pub mod handlers;
pub mod state;
pub mod threads; // Подключаем наш новый модуль потоков

use pixels::{Pixels, SurfaceTexture};
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::num::NonZeroU32;
use std::sync::{
    Arc,
    mpsc::{Sender, channel},
};
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowButtons, WindowId},
};

use crate::BALL_COUNT;
use crate::app::handlers::ThreadCommand;
use crate::app::state::{
    CpuThreadContext, GpuThreadContext, PixelsState, START_HEIGHT, START_WIDTH, SoftbufferState,
};
use crate::ball::Ball;

pub struct App {
    gpu_state: PixelsState,
    cpu_state: SoftbufferState,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    mouse_start_press: Option<Instant>,

    // Каналы для отправки команд в фоновые независимые потоки
    gpu_tx: Option<Sender<ThreadCommand>>,
    cpu_tx: Option<Sender<ThreadCommand>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            gpu_state: PixelsState::new(),
            cpu_state: SoftbufferState::new(),
            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_start_press: None,
            gpu_tx: None,
            cpu_tx: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_state.window.is_none() {
            let enabled_buttons = WindowButtons::CLOSE | WindowButtons::MINIMIZE;
            let total_w = START_WIDTH.max(crate::PANEL_MIN_WIDTH);
            let total_h = START_HEIGHT + crate::PANEL_HEIGHT;

            // 1. Создаем окна в главном потоке (требование ОС)
            let attr_p = Window::default_attributes()
                .with_title("GPU (Pixels)")
                .with_resizable(true)
                .with_enabled_buttons(enabled_buttons)
                .with_inner_size(winit::dpi::LogicalSize::new(total_w as f64, total_h as f64));
            let win_p = Arc::new(event_loop.create_window(attr_p).unwrap());
            self.gpu_state.id = Some(win_p.id());
            self.gpu_state.window = Some(win_p.clone());

            let p_pos = win_p
                .outer_position()
                .unwrap_or(winit::dpi::PhysicalPosition::new(100, 100));
            let sb_pos = winit::dpi::PhysicalPosition::new(
                p_pos.x + win_p.outer_size().width as i32,
                p_pos.y,
            );

            let attr_sb = Window::default_attributes()
                .with_title("CPU (Softbuffer)")
                .with_resizable(true)
                .with_enabled_buttons(enabled_buttons)
                .with_inner_size(winit::dpi::LogicalSize::new(total_w as f64, total_h as f64))
                .with_position(sb_pos);
            let win_sb = Arc::new(event_loop.create_window(attr_sb).unwrap());
            self.cpu_state.id = Some(win_sb.id());
            self.cpu_state.window = Some(win_sb.clone());

            // 2. Инициализируем графические контексты
            let surface_texture = SurfaceTexture::new(total_w, total_h, win_p.clone());
            let pixels = Pixels::new(total_w, total_h, surface_texture).unwrap();
            let custom_pipeline = crate::render::pipeline::CustomRenderPipeline::new(
                pixels.device(),
                pixels.queue(),
                pixels.render_texture_format(),
                1000,
            );

            let sb_context = SbContext::new(win_sb.clone()).unwrap();
            let mut sb_surface = SbSurface::new(&sb_context, win_sb.clone()).unwrap();
            sb_surface
                .resize(
                    NonZeroU32::new(total_w).unwrap(),
                    NonZeroU32::new(total_h).unwrap(),
                )
                .unwrap();

            // Настраиваем геометрию игрового поля
            let playfield = crate::ball::Playfield {
                x: 0.0,
                y: 0.0,
                w: START_WIDTH as f32,
                h: START_HEIGHT as f32,
            };

            // 3. Собираем контексты данных для отправки в потоки
            let gpu_context = GpuThreadContext {
                pixels,
                custom_pipeline,
                playfield,
                balls: Ball::generate_scene_in_field(BALL_COUNT, &playfield, (255, 255, 255)),
                canvas: ril::prelude::Image::new(
                    total_w,
                    total_h,
                    ril::prelude::Rgba::new(0, 0, 0, 255),
                ),
                w: total_w,
                h: total_h,
                default_color: (255, 255, 255),
            };

            let cpu_context = CpuThreadContext {
                surface: sb_surface,
                playfield,
                balls: Ball::generate_scene_in_field(BALL_COUNT, &playfield, (255, 255, 0)),
                canvas: ril::prelude::Image::new(total_w, total_h, ril::prelude::Rgb::new(0, 0, 0)),
                w: total_w,
                h: total_h,
                default_color: (255, 255, 0),
            };

            // 4. ЗАПУСКАЕМ НАСТОЯЩИЕ ПОТОКИ ОС
            let (gpu_tx, gpu_rx) = channel();
            let (cpu_tx, cpu_rx) = channel();
            self.gpu_tx = Some(gpu_tx);
            self.cpu_tx = Some(cpu_tx);

            // Уводим контексты жить в фоновые loop-циклы навечно
            // Используем небезопасный каст времени жизни, так как Pixels<'win> привязан к Arc<Window>, который не умрет до конца программы
            let gpu_context_static: GpuThreadContext<'static> =
                unsafe { std::mem::transmute(gpu_context) };
            std::thread::spawn(move || {
                threads::run_gpu_thread(gpu_context_static, gpu_rx);
            });
            std::thread::spawn(move || {
                threads::run_cpu_thread(cpu_context, cpu_rx);
            });

            handlers::set_window_min_size(&win_p, BALL_COUNT);
            handlers::set_window_min_size(&win_sb, BALL_COUNT);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Главный поток просто отдыхает и опрашивает ОС, фоновые потоки фигачат сами по себе
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
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

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    // Проверяем, что нажата именно клавиша Пробел
                    if event.logical_key
                        == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space)
                    {
                        if let Some(tx) = &self.gpu_tx {
                            let _ = tx.send(ThreadCommand::TogglePause);
                        }
                        if let Some(tx) = &self.cpu_tx {
                            let _ = tx.send(ThreadCommand::TogglePause);
                        }
                    }
                }
            }

            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 {
                    return;
                }

                // Передаем команду изменения размера в нужный поток по каналу
                if Some(window_id) == self.gpu_state.id {
                    if let Some(tx) = &self.gpu_tx {
                        let _ = tx.send(ThreadCommand::Resize {
                            w: new_size.width,
                            h: new_size.height,
                        });
                    }
                } else if Some(window_id) == self.cpu_state.id {
                    if let Some(tx) = &self.cpu_tx {
                        let _ = tx.send(ThreadCommand::Resize {
                            w: new_size.width,
                            h: new_size.height,
                        });
                    }
                }

                if let (Some(wp), Some(wsb)) = (&self.gpu_state.window, &self.cpu_state.window) {
                    handlers::sync_positions_on_resize(wp, wsb);
                }
            }
            WindowEvent::Moved(new_position) => {
                handlers::handle_window_moved(
                    window_id,
                    new_position,
                    self.gpu_state.id,
                    self.cpu_state.id,
                    self.gpu_state.window.as_deref(),
                    self.cpu_state.window.as_deref(),
                );
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;
                let x = position.x as f32;
                let y = position.y as f32;
                // Отправляем новые координаты мыши в потоки для обновления позиции "растущего" шара
                if let Some(tx) = &self.gpu_tx {
                    let _ = tx.send(ThreadCommand::MouseMove { x, y });
                }
                if let Some(tx) = &self.cpu_tx {
                    let _ = tx.send(ThreadCommand::MouseMove { x, y });
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let x = self.cursor_pos.x as f32;
                let y = self.cursor_pos.y as f32;
                let is_left = button == MouseButton::Left;
                let is_right = button == MouseButton::Right;

                // Передаем точные фазыPressed/Released в нужный поток по каналу
                if Some(window_id) == self.gpu_state.id {
                    if let Some(tx) = &self.gpu_tx {
                        if state == ElementState::Pressed {
                            if is_left {
                                let _ = tx.send(ThreadCommand::MousePressed {
                                    x,
                                    y,
                                    is_left: true,
                                });
                            }
                            if is_right {
                                let _ = tx.send(ThreadCommand::MousePressed {
                                    x,
                                    y,
                                    is_left: false,
                                });
                            }
                        } else {
                            if is_left {
                                let _ = tx.send(ThreadCommand::MouseReleased { is_left: true });
                            }
                            if is_right {
                                let _ = tx.send(ThreadCommand::MouseReleased { is_left: false });
                            }
                        }
                    }
                } else if Some(window_id) == self.cpu_state.id {
                    if let Some(tx) = &self.cpu_tx {
                        if state == ElementState::Pressed {
                            if is_left {
                                let _ = tx.send(ThreadCommand::MousePressed {
                                    x,
                                    y,
                                    is_left: true,
                                });
                            }
                            if is_right {
                                let _ = tx.send(ThreadCommand::MousePressed {
                                    x,
                                    y,
                                    is_left: false,
                                });
                            }
                        } else {
                            if is_left {
                                let _ = tx.send(ThreadCommand::MouseReleased { is_left: true });
                            }
                            if is_right {
                                let _ = tx.send(ThreadCommand::MouseReleased { is_left: false });
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
