use pixels::{Pixels, SurfaceTexture};
use ril::prelude::*;
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::num::NonZeroU32;
use std::sync::Arc;
// Добавили импорт WindowButtons для точечной настройки кнопок управления окном
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowButtons, WindowId},
};

use crate::BALL_COUNT;
use crate::ball::Ball;
use crate::render::{draw_pixels_frame, draw_softbuffer_frame};

const START_WIDTH: u32 = 400;
const START_HEIGHT: u32 = 400;
const BG_COLOR: (u8, u8, u8) = (30, 30, 30);

pub struct App<'win> {
    win_pixels: Option<Arc<Window>>,
    id_pixels: Option<WindowId>,
    pixels: Option<Pixels<'win>>,
    canvas_pixels: Image<Rgba>,
    balls_pixels: Vec<Ball>,
    w_pixels: u32,
    h_pixels: u32,

    win_softbuffer: Option<Arc<Window>>,
    id_softbuffer: Option<WindowId>,
    sb_context: Option<SbContext<Arc<Window>>>,
    sb_surface: Option<SbSurface<Arc<Window>, Arc<Window>>>,
    canvas_softbuffer: Image<Rgb>,
    balls_softbuffer: Vec<Ball>,
    w_softbuffer: u32,
    h_softbuffer: u32,

    cursor_pos: winit::dpi::PhysicalPosition<f64>,
}

impl<'win> Default for App<'win> {
    fn default() -> Self {
        Self {
            win_pixels: None,
            id_pixels: None,
            pixels: None,
            canvas_pixels: Image::new(START_WIDTH, START_HEIGHT, Rgba::new(0, 0, 0, 255)),
            balls_pixels: Ball::generate_scene(
                BALL_COUNT,
                START_WIDTH as f32,
                START_HEIGHT as f32,
                (255, 255, 255),
            ),
            w_pixels: START_WIDTH,
            h_pixels: START_HEIGHT,

            win_softbuffer: None,
            id_softbuffer: None,
            sb_context: None,
            sb_surface: None,
            canvas_softbuffer: Image::new(START_WIDTH, START_HEIGHT, Rgb::new(0, 0, 0)),
            balls_softbuffer: Ball::generate_scene(
                BALL_COUNT,
                START_WIDTH as f32,
                START_HEIGHT as f32,
                (255, 255, 0),
            ),
            w_softbuffer: START_WIDTH,
            h_softbuffer: START_HEIGHT,

            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
        }
    }
}

impl<'win> ApplicationHandler for App<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.win_pixels.is_none() {
            // Настраиваем маску кнопок: оставляем только ЗАКРЫТЬ (CLOSE) и СВЕРНУТЬ (MINIMIZE)
            let enabled_buttons = WindowButtons::CLOSE | WindowButtons::MINIMIZE;

            // ИСПРАВЛЕНО: Окна ресайзятся (.with_resizable(true)), но кнопка максимизации отключена через .with_enabled_buttons
            let attr_pixels = Window::default_attributes()
                .with_title("GPU (Pixels)")
                .with_resizable(true)
                .with_enabled_buttons(enabled_buttons)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    START_WIDTH as f64,
                    START_HEIGHT as f64,
                ));
            let win_p = Arc::new(event_loop.create_window(attr_pixels).unwrap());
            self.id_pixels = Some(win_p.id());

            let surface_texture = SurfaceTexture::new(START_WIDTH, START_HEIGHT, win_p.clone());
            let pixels = Pixels::new(START_WIDTH, START_HEIGHT, surface_texture).unwrap();
            self.pixels = Some(pixels);
            self.win_pixels = Some(win_p.clone());

            let p_pos = win_p
                .outer_position()
                .unwrap_or(winit::dpi::PhysicalPosition::new(100, 100));
            let p_outer_size = win_p.outer_size();
            let sb_pos =
                winit::dpi::PhysicalPosition::new(p_pos.x + p_outer_size.width as i32, p_pos.y);

            let attr_sb = Window::default_attributes()
                .with_title("CPU (Softbuffer)")
                .with_resizable(true)
                .with_enabled_buttons(enabled_buttons)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    START_WIDTH as f64,
                    START_HEIGHT as f64,
                ))
                .with_position(sb_pos);
            let win_sb = Arc::new(event_loop.create_window(attr_sb).unwrap());
            self.id_softbuffer = Some(win_sb.id());

            let sb_context = SbContext::new(win_sb.clone()).unwrap();
            let mut sb_surface = SbSurface::new(&sb_context, win_sb.clone()).unwrap();
            sb_surface
                .resize(
                    NonZeroU32::new(START_WIDTH).unwrap(),
                    NonZeroU32::new(START_HEIGHT).unwrap(),
                )
                .unwrap();

            self.sb_context = Some(sb_context);
            self.sb_surface = Some(sb_surface);
            self.win_softbuffer = Some(win_sb);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let (Some(pixels), Some(window)) = (&mut self.pixels, &self.win_pixels) {
            Ball::update_physics(
                &mut self.balls_pixels,
                self.w_pixels as f32,
                self.h_pixels as f32,
            );
            draw_pixels_frame(
                &mut self.canvas_pixels,
                pixels,
                &self.balls_pixels,
                self.w_pixels,
                self.h_pixels,
                BG_COLOR,
            );
            window.request_redraw();
        }

        if let (Some(surface), Some(window)) = (&mut self.sb_surface, &self.win_softbuffer) {
            Ball::update_physics(
                &mut self.balls_softbuffer,
                self.w_softbuffer as f32,
                self.h_softbuffer as f32,
            );
            draw_softbuffer_frame(
                &mut self.canvas_softbuffer,
                surface,
                &self.balls_softbuffer,
                self.w_softbuffer,
                self.h_softbuffer,
                BG_COLOR,
            );
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            // Обрабатываем изменение размеров окон в реальном времени
            WindowEvent::Resized(new_size) => {
                if Some(window_id) == self.id_pixels {
                    self.w_pixels = new_size.width;
                    self.h_pixels = new_size.height;
                    self.canvas_pixels =
                        Image::new(self.w_pixels, self.h_pixels, Rgba::new(0, 0, 0, 255));
                    if let Some(pixels) = &mut self.pixels {
                        pixels.resize_buffer(self.w_pixels, self.h_pixels).unwrap();
                        pixels.resize_surface(self.w_pixels, self.h_pixels).unwrap();
                    }

                    // СЦЕПКА ПРИ РЕЗАЙЗЕ: Если изменился размер левого окна (Pixels),
                    // мы считываем его текущую позицию и мгновенно двигаем правое окно (Softbuffer) встык
                    if let (Some(win_p), Some(win_sb)) = (&self.win_pixels, &self.win_softbuffer) {
                        if let Ok(p_pos) = win_p.outer_position() {
                            let p_outer_size = win_p.outer_size();
                            let target_pos = winit::dpi::PhysicalPosition::new(
                                p_pos.x + p_outer_size.width as i32,
                                p_pos.y,
                            );
                            win_sb.set_outer_position(target_pos);
                        }
                    }
                } else if Some(window_id) == self.id_softbuffer {
                    self.w_softbuffer = new_size.width;
                    self.h_softbuffer = new_size.height;
                    self.canvas_softbuffer =
                        Image::new(self.w_softbuffer, self.h_softbuffer, Rgb::new(0, 0, 0));
                    if let Some(surface) = &mut self.sb_surface {
                        if let (Some(w), Some(h)) = (
                            NonZeroU32::new(self.w_softbuffer),
                            NonZeroU32::new(self.h_softbuffer),
                        ) {
                            surface.resize(w, h).unwrap();
                        }
                    }

                    // СЦЕПКА ПРИ РЕЗАЙЗЕ: Если вы растягиваете правое окно,
                    // левое должно оставаться на месте, но если логика требует жесткой склейки сдвигом влево:
                    if let (Some(win_p), Some(win_sb)) = (&self.win_pixels, &self.win_softbuffer) {
                        if let Ok(sb_pos) = win_sb.outer_position() {
                            let p_outer_size = win_p.outer_size();
                            let target_pos = winit::dpi::PhysicalPosition::new(
                                sb_pos.x - p_outer_size.width as i32,
                                sb_pos.y,
                            );
                            win_p.set_outer_position(target_pos);
                        }
                    }
                }
            }

            WindowEvent::Moved(new_position) => {
                if Some(window_id) == self.id_pixels {
                    if let (Some(win_p), Some(win_sb)) = (&self.win_pixels, &self.win_softbuffer) {
                        let p_outer_size = win_p.outer_size();
                        let target_pos = winit::dpi::PhysicalPosition::new(
                            new_position.x + p_outer_size.width as i32,
                            new_position.y,
                        );
                        win_sb.set_outer_position(target_pos);
                    }
                } else if Some(window_id) == self.id_softbuffer {
                    if let (Some(win_p), Some(win_sb)) = (&self.win_pixels, &self.win_softbuffer) {
                        let p_outer_size = win_p.outer_size();
                        let target_pos = winit::dpi::PhysicalPosition::new(
                            new_position.x - p_outer_size.width as i32,
                            new_position.y,
                        );
                        win_p.set_outer_position(target_pos);
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed && button == MouseButton::Left {
                    let mouse_x = self.cursor_pos.x as f32;
                    let mouse_y = self.cursor_pos.y as f32;

                    if Some(window_id) == self.id_pixels {
                        for ball in self.balls_pixels.iter_mut() {
                            if ball.check_click(mouse_x, mouse_y, BG_COLOR) {
                                break;
                            }
                        }
                    } else if Some(window_id) == self.id_softbuffer {
                        for ball in self.balls_softbuffer.iter_mut() {
                            if ball.check_click(mouse_x, mouse_y, BG_COLOR) {
                                break;
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
