use pixels::{Pixels, SurfaceTexture};
use ril::prelude::*;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    // Добавили MouseButton, MouseScrollDelta и ElementState для мыши
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

// Константное логическое разрешение нашего холста
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

struct App<'win> {
    // Окно оборачивается в Arc, так как pixels требует потокобезопасную ссылку на окно
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'win>>,
    // Наш холст в оперативной памяти от RIL
    canvas: Image<Rgba>,
}

impl<'win> Default for App<'win> {
    fn default() -> Self {
        Self {
            window: None,
            pixels: None,
            // Создаем пустое изображение, заполненное черным цветом
            canvas: Image::new(WIDTH, HEIGHT, Rgba::black()),
        }
    }
}

impl<'win> ApplicationHandler for App<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("Winit + Pixels + RIL (Fixed & Working)")
                .with_inner_size(winit::dpi::LogicalSize::new(WIDTH as f64, HEIGHT as f64));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, window.clone());
            let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();

            self.window = Some(window.clone());
            self.pixels = Some(pixels);

            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }

            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state.is_pressed() {
                    match key_event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            event_loop.exit();
                        }
                        _ => (),
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                // Координаты мыши относительно левого верхнего угла окна
                println!("Cursor moved to: x={:.1}, y={:.1}", position.x, position.y);
            }

            // --- НОВОЕ: ОБРАБОТКА НАЖАТИЙ КНОПОК МЫШИ ---
            WindowEvent::MouseInput { state, button, .. } => {
                // state показывает нажата кнопка (Pressed) или отпущена (Released)
                let action = match state {
                    ElementState::Pressed => "Pressed",
                    ElementState::Released => "Released",
                };

                // button определяет, какая именно кнопка была задействована
                match button {
                    MouseButton::Left => println!("Left mouse button: {}", action),
                    MouseButton::Right => println!("Right mouse button: {}", action),
                    MouseButton::Middle => {
                        println!("Middle mouse button (wheel click): {}", action)
                    }
                    MouseButton::Back => println!("Back mouse button: {}", action),
                    MouseButton::Forward => println!("Forward mouse button: {}", action),
                    MouseButton::Other(id) => {
                        println!("Other mouse button (ID {}): {}", id, action)
                    }
                }
            }

            // --- НОВОЕ: ОБРАБОТКА КОЛЁСИКА МЫШИ ---
            WindowEvent::MouseWheel {
                delta,
                phase: _phase,
                ..
            } => {
                // delta может приходить в двух форматах в зависимости от ОС и мыши
                match delta {
                    // LineDelta возвращает количество прокрученных строк (обычно на Windows/Linux)
                    MouseScrollDelta::LineDelta(x, y) => {
                        println!("Scroll by lines: x={}, y={}", x, y);
                    }
                    // PixelDelta возвращает точные пиксели (обычно на macOS с трекпадами или плавными мышами)
                    MouseScrollDelta::PixelDelta(physical_position) => {
                        println!(
                            "Scroll by pixels: x={}, y={}",
                            physical_position.x, physical_position.y
                        );
                    }
                }
            }
            // Перерисовываем экран, когда ОС запрашивает обновление окна
            WindowEvent::RedrawRequested => {
                if let (Some(pixels), Some(window)) = (&mut self.pixels, &self.window) {
                    // 1. Очищаем холст RIL (например, темно-серым цветом)
                    // ИСПРАВЛЕНО 1: Очищаем экран, рисуя фоновый прямоугольник на весь холст
                    let background = Rectangle::at(0, 0)
                        .with_size(WIDTH, HEIGHT)
                        .with_fill(Rgba::new(30, 30, 30, 255));
                    self.canvas.draw(&background);

                    // 2. Рисуем что-нибудь средствами RIL
                    // Нарисуем красный прямоугольник
                    let rect = Rectangle::at(100, 100)
                        .with_size(200, 150)
                        .with_fill(Rgba::new(255, 0, 0, 255));
                    self.canvas.draw(&rect);

                    // Нарисуем синий круг
                    let circle = Ellipse::circle(500, 300, 80).with_fill(Rgba::new(0, 0, 255, 255));
                    self.canvas.draw(&circle);

                    // 3. Копируем пиксели из RIL в буфер pixels
                    // RIL хранит данные как &[Rgba], а pixels принимает &[u8].
                    // С помощью as_bytes() мы безопасно приводим типы без лишнего копирования элементов.
                    let pixels_buffer = pixels.frame_mut();

                    let pixel_slice: &[Rgba] = &self.canvas.data; // Автоматическое разыменование структуры Image в &[P]

                    // Безопасно преобразуем срез пикселей &[Rgba] в срез байт &[u8]
                    let raw_bytes = unsafe {
                        std::slice::from_raw_parts(
                            pixel_slice.as_ptr() as *const u8,
                            pixel_slice.len() * 4,
                        )
                    };

                    pixels_buffer.copy_from_slice(raw_bytes);

                    // 4. Отрисовываем буфер на видеокарту
                    if let Err(err) = pixels.render() {
                        println!("Ошибка рендеринга: {:?}", err);
                        event_loop.exit();
                    }

                    // Запрашиваем следующий кадр для плавной анимации
                    window.request_redraw();
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
