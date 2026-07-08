use pixels::{Pixels, SurfaceTexture};
use ril::draw::{Ellipse, Rectangle};
use ril::prelude::*;
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 400;

struct App<'win> {
    // Первое окно (GPU - Pixels)
    win_pixels: Option<Arc<Window>>,
    id_pixels: Option<WindowId>,
    pixels: Option<Pixels<'win>>,
    canvas_pixels: Image<Rgba>,

    // Второе окно (CPU - Softbuffer)
    win_softbuffer: Option<Arc<Window>>,
    id_softbuffer: Option<WindowId>,
    sb_context: Option<SbContext<Arc<Window>>>,
    sb_surface: Option<SbSurface<Arc<Window>, Arc<Window>>>,
    canvas_softbuffer: Image<Rgb>,
}

impl<'win> Default for App<'win> {
    fn default() -> Self {
        Self {
            win_pixels: None,
            id_pixels: None,
            pixels: None,
            canvas_pixels: Image::new(WIDTH, HEIGHT, Rgba::new(0, 0, 0, 255)),

            win_softbuffer: None,
            id_softbuffer: None,
            sb_context: None,
            sb_surface: None,
            canvas_softbuffer: Image::new(WIDTH, HEIGHT, Rgb::new(0, 0, 0)),
        }
    }
}

impl<'win> ApplicationHandler for App<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.win_pixels.is_none() {
            // 1. Создаем Главное окно (Pixels)
            let attr_pixels = Window::default_attributes()
                .with_title("GPU (Pixels)")
                .with_inner_size(winit::dpi::LogicalSize::new(WIDTH as f64, HEIGHT as f64));
            let win_p = Arc::new(event_loop.create_window(attr_pixels).unwrap());
            self.id_pixels = Some(win_p.id());

            let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, win_p.clone());
            let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();
            self.pixels = Some(pixels);
            self.win_pixels = Some(win_p.clone());

            // Получаем позицию главного окна, чтобы прикрепить второе рядом
            let p_pos = win_p
                .outer_position()
                .unwrap_or(winit::dpi::PhysicalPosition::new(100, 100));

            // 2. Создаем Второе окно (Softbuffer) справа от первого
            // Сдвигаем по оси X на ширину окна (WIDTH) + рамки окна (примерно 16 пикселей для Windows)
            let sb_pos = winit::dpi::PhysicalPosition::new(p_pos.x + WIDTH as i32 + 16, p_pos.y);

            let attr_sb = Window::default_attributes()
                .with_title("CPU (Softbuffer)")
                .with_inner_size(winit::dpi::LogicalSize::new(WIDTH as f64, HEIGHT as f64))
                .with_position(sb_pos);
            let win_sb = Arc::new(event_loop.create_window(attr_sb).unwrap());
            self.id_softbuffer = Some(win_sb.id());

            let sb_context = SbContext::new(win_sb.clone()).unwrap();
            let mut sb_surface = SbSurface::new(&sb_context, win_sb.clone()).unwrap();
            sb_surface
                .resize(
                    NonZeroU32::new(WIDTH).unwrap(),
                    NonZeroU32::new(HEIGHT).unwrap(),
                )
                .unwrap();

            self.sb_context = Some(sb_context);
            self.sb_surface = Some(sb_surface);
            self.win_softbuffer = Some(win_sb);

            // Запрашиваем отрисовку обоих окон
            self.win_pixels.as_ref().unwrap().request_redraw();
            self.win_softbuffer.as_ref().unwrap().request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            // СЦЕПЛЕНИЕ 1: Закрытие любого окна закрывает всю программу
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            // СЦЕПЛЕНИЕ 2: Синхронное перемещение окон
            WindowEvent::Moved(new_position) => {
                if Some(window_id) == self.id_pixels {
                    // Если движется окно Pixels, двигаем Softbuffer вслед за ним справа
                    if let Some(win_sb) = &self.win_softbuffer {
                        let target_pos = winit::dpi::PhysicalPosition::new(
                            new_position.x + WIDTH as i32 + 16, // Учитываем ширину главного окна
                            new_position.y,
                        );
                        win_sb.set_outer_position(target_pos);
                    }
                } else if Some(window_id) == self.id_softbuffer {
                    // Если пользователь потащил окно Softbuffer, двигаем Pixels вслед за ним слева
                    if let Some(win_p) = &self.win_pixels {
                        let target_pos = winit::dpi::PhysicalPosition::new(
                            new_position.x - WIDTH as i32 - 16,
                            new_position.y,
                        );
                        win_p.set_outer_position(target_pos);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                // ОТРИСОВКА ОКНА 1: Pixels (GPU)
                if Some(window_id) == self.id_pixels {
                    if let (Some(pixels), Some(window)) = (&mut self.pixels, &self.win_pixels) {
                        // Очистка фона и отрисовка красного квадрата
                        // ИСПРАВЛЕНО 1: Очищаем экран, рисуя фоновый прямоугольник на весь холст
                        let background = Rectangle::at(0, 0)
                            .with_size(WIDTH, HEIGHT)
                            .with_fill(Rgba::new(30, 30, 30, 255));
                        self.canvas_pixels.draw(&background);
                        let rect = Rectangle::at(100, 125)
                            .with_size(200, 150)
                            .with_fill(Rgba::new(255, 0, 0, 255));
                        self.canvas_pixels.draw(&rect);

                        // Копирование буфера
                        let pixels_buffer = pixels.frame_mut();
                        let pixel_slice: &[Rgba] = &self.canvas_pixels.data;
                        let raw_bytes = unsafe {
                            std::slice::from_raw_parts(
                                pixel_slice.as_ptr() as *const u8,
                                pixel_slice.len() * 4,
                            )
                        };
                        pixels_buffer.copy_from_slice(raw_bytes);
                        pixels.render().unwrap();
                        window.request_redraw();
                    }
                }

                // ОТРИСОВКА ОКНА 2: Softbuffer (CPU)
                if Some(window_id) == self.id_softbuffer {
                    if let (Some(surface), Some(window)) =
                        (&mut self.sb_surface, &self.win_softbuffer)
                    {
                        // Очистка фона и отрисовка синего круга
                        let background = Rectangle::at(0, 0)
                            .with_size(WIDTH, HEIGHT)
                            .with_fill(Rgb::new(30, 30, 30));
                        self.canvas_softbuffer.draw(&background);

                        let circle = Ellipse::circle(200, 200, 80).with_fill(Rgb::new(0, 0, 255));
                        self.canvas_softbuffer.draw(&circle);

                        // Копирование буфера в softbuffer формат
                        let mut buffer = surface.buffer_mut().unwrap();
                        let ril_pixels = &self.canvas_softbuffer.data;
                        for (i, pixel) in ril_pixels.iter().enumerate() {
                            buffer[i] = ((pixel.r as u32) << 16)
                                | ((pixel.g as u32) << 8)
                                | (pixel.b as u32);
                        }
                        buffer.present().unwrap();
                        window.request_redraw();
                    }
                }
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
