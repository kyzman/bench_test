pub mod handlers;
pub mod state;

use pixels::{Pixels, SurfaceTexture};
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowButtons, WindowId},
};

use crate::BALL_COUNT;
use crate::app::state::{BG_COLOR, PixelsState, START_HEIGHT, START_WIDTH, SoftbufferState};
use crate::ball::Ball;
use crate::render::{draw_pixels_frame, draw_softbuffer_frame};

pub struct App<'win> {
    gpu_state: PixelsState<'win>,
    cpu_state: SoftbufferState,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,

    // НОВОЕ ПОЛЕ: Хранит время начала зажатия ЛКМ
    mouse_start_press: Option<Instant>,
}

impl<'win> Default for App<'win> {
    fn default() -> Self {
        Self {
            gpu_state: PixelsState::new(BALL_COUNT),
            cpu_state: SoftbufferState::new(BALL_COUNT),
            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_start_press: None, // По умолчанию мышь не зажата
        }
    }
}

impl<'win> ApplicationHandler for App<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_state.window.is_none() {
            let enabled_buttons = WindowButtons::CLOSE | WindowButtons::MINIMIZE;

            // 1. Инициализация GPU-окна (Pixels)
            let attr_pixels = Window::default_attributes()
                .with_title("GPU (Pixels)")
                .with_resizable(true)
                .with_enabled_buttons(enabled_buttons)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    START_WIDTH as f64,
                    START_HEIGHT as f64,
                ));
            let win_p = Arc::new(event_loop.create_window(attr_pixels).unwrap());
            self.gpu_state.id = Some(win_p.id());

            let surface_texture = SurfaceTexture::new(START_WIDTH, START_HEIGHT, win_p.clone());
            let pixels = Pixels::new(START_WIDTH, START_HEIGHT, surface_texture).unwrap();
            self.gpu_state.pixels = Some(pixels);
            self.gpu_state.window = Some(win_p.clone());

            // Рассчитываем позицию для правого окна
            let p_pos = win_p
                .outer_position()
                .unwrap_or(winit::dpi::PhysicalPosition::new(100, 100));
            let p_outer_size = win_p.outer_size();
            let sb_pos =
                winit::dpi::PhysicalPosition::new(p_pos.x + p_outer_size.width as i32, p_pos.y);

            // 2. Инициализация CPU-окна (Softbuffer)
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
            self.cpu_state.id = Some(win_sb.id());

            let sb_context = SbContext::new(win_sb.clone()).unwrap();
            let mut sb_surface = SbSurface::new(&sb_context, win_sb.clone()).unwrap();
            sb_surface
                .resize(
                    NonZeroU32::new(START_WIDTH).unwrap(),
                    NonZeroU32::new(START_HEIGHT).unwrap(),
                )
                .unwrap();

            self.cpu_state.context = Some(sb_context);
            self.cpu_state.surface = Some(sb_surface);
            self.cpu_state.window = Some(win_sb);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Симуляция и отрисовка левого окна (GPU)
        if let (Some(pixels), Some(window)) = (&mut self.gpu_state.pixels, &self.gpu_state.window) {
            Ball::update_physics(
                &mut self.gpu_state.balls,
                self.gpu_state.w as f32,
                self.gpu_state.h as f32,
            );
            draw_pixels_frame(
                &mut self.gpu_state.canvas,
                pixels,
                &self.gpu_state.balls,
                self.gpu_state.w,
                self.gpu_state.h,
                BG_COLOR,
            );
            window.request_redraw();
        }

        // Симуляция и отрисовка правого окна (CPU)
        if let (Some(surface), Some(window)) = (&mut self.cpu_state.surface, &self.cpu_state.window)
        {
            Ball::update_physics(
                &mut self.cpu_state.balls,
                self.cpu_state.w as f32,
                self.cpu_state.h as f32,
            );
            draw_softbuffer_frame(
                &mut self.cpu_state.canvas,
                surface,
                &self.cpu_state.balls,
                self.cpu_state.w,
                self.cpu_state.h,
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

            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 {
                    return;
                }

                // ИСПРАВЛЕНО: Вычисляем и обновляем минимальный размер окон СТРОГО в момент изменения размеров окна
                handlers::update_window_min_sizes(&self.gpu_state, &self.cpu_state);

                if Some(window_id) == self.gpu_state.id {
                    handlers::resize_gpu_window(&mut self.gpu_state, new_size);
                } else if Some(window_id) == self.cpu_state.id {
                    handlers::resize_cpu_window(&mut self.cpu_state, new_size);
                }

                handlers::sync_positions_on_resize(&self.gpu_state, &self.cpu_state);
            }

            WindowEvent::Moved(new_position) => {
                handlers::handle_window_moved(
                    window_id,
                    new_position,
                    &self.gpu_state,
                    &self.cpu_state,
                );
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let mut duration_ms = 0.0;

                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        // Зажимаем кнопку — запускаем секундомер
                        self.mouse_start_press = Some(Instant::now());
                    } else if state == ElementState::Released {
                        // Отпускаем кнопку — останавливаем секундомер и считаем время
                        if let Some(start_time) = self.mouse_start_press.take() {
                            duration_ms = start_time.elapsed().as_secs_f32() * 1000.0;
                        }
                    }
                }

                // ИСПРАВЛЕНО: Передаем duration_ms седьмым аргументом, как требует handlers.rs
                handlers::handle_mouse_input(
                    window_id,
                    state,
                    button,
                    self.cursor_pos,
                    &mut self.gpu_state,
                    &mut self.cpu_state,
                    duration_ms,
                );

                // Вычисляем лимиты окон только в момент кликов (а не в цикле),
                // чтобы подготовить систему к будущему изменению рамки окна
                if state == ElementState::Released || state == ElementState::Pressed {
                    handlers::update_window_min_sizes(&self.gpu_state, &self.cpu_state);
                }
            }
            _ => (),
        }
    }
}
