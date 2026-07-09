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
use crate::app::state::{BG_COLOR, PixelsState, SoftbufferState};
use crate::ball::Ball;
use crate::render::{draw_pixels_frame, draw_softbuffer_frame};

pub struct App<'win> {
    gpu_state: PixelsState<'win>,
    cpu_state: SoftbufferState,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    mouse_start_press: Option<Instant>,

    // НОВЫЕ ПОЛЯ ДЛЯ ОЖИВЛЕНИЯ FPS
    last_fps_update: Instant, // Время последнего замера FPS
    frames_gpu: u32,          // Накопленный счётчик кадров для GPU (Pixels)
    frames_cpu: u32,          // Накопленный счётчик кадров для CPU (Softbuffer)
    current_fps_gpu: u32,     // Текущее рассчитанное значение FPS для GPU
    current_fps_cpu: u32,     // Текущее рассчитанное значение FPS для CPU
}

impl<'win> Default for App<'win> {
    fn default() -> Self {
        Self {
            gpu_state: PixelsState::new(BALL_COUNT),
            cpu_state: SoftbufferState::new(BALL_COUNT),
            cursor_pos: winit::dpi::PhysicalPosition::new(0.0, 0.0),
            mouse_start_press: None,
            // ИНИЦИАЛИЗАЦИЯ ТАЙМЕРОВ
            last_fps_update: Instant::now(),
            frames_gpu: 0,
            frames_cpu: 0,
            current_fps_gpu: 0,
            current_fps_cpu: 0,
        }
    }
}

impl<'win> ApplicationHandler for App<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_state.window.is_none() {
            let enabled_buttons = WindowButtons::CLOSE | WindowButtons::MINIMIZE;

            // 1. Инициализация GPU-окна (Pixels)
            // ИСПРАВЛЕНО: Запрашиваем честные стартовые размеры окна из его состояния
            let (gpu_win_w, gpu_win_h) = self.gpu_state.get_window_start_size();

            let attr_pixels = Window::default_attributes()
                .with_title("GPU (Pixels)")
                .with_resizable(true)
                .with_enabled_buttons(enabled_buttons)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    gpu_win_w as f64,
                    gpu_win_h as f64,
                ));
            let win_p = Arc::new(event_loop.create_window(attr_pixels).unwrap());
            self.gpu_state.id = Some(win_p.id());

            let surface_texture = SurfaceTexture::new(gpu_win_w, gpu_win_h, win_p.clone());
            // ... (предыдущий код создания Pixels в resumed)
            let pixels = Pixels::new(gpu_win_w, gpu_win_h, surface_texture).unwrap();

            // НОВЫЙ КОД: Извлекаем устройство wgpu и компилируем наш конвейер шейдеров
            let custom_pipeline = crate::render::CustomRenderPipeline::new(
                pixels.device(),
                pixels.queue(),
                pixels.render_texture_format(),
                1000, // Максимальное количество шаров, под которое резервируется буфер VRAM
            );
            self.gpu_state.custom_pipeline = Some(custom_pipeline);

            self.gpu_state.pixels = Some(pixels);
            self.gpu_state.window = Some(win_p.clone());
            // ... (дальнейший код инициализации)

            // Рассчитываем позицию для правого окна
            let p_pos = win_p
                .outer_position()
                .unwrap_or(winit::dpi::PhysicalPosition::new(100, 100));
            let p_outer_size = win_p.outer_size();
            let sb_pos =
                winit::dpi::PhysicalPosition::new(p_pos.x + p_outer_size.width as i32, p_pos.y);

            // 2. Инициализация CPU-окна (Softbuffer)
            // ИСПРАВЛЕНО: Запрашиваем честные стартовые размеры для второго окна
            let (cpu_win_w, cpu_win_h) = self.cpu_state.get_window_start_size();

            let attr_sb = Window::default_attributes()
                .with_title("CPU (Softbuffer)")
                .with_resizable(true)
                .with_enabled_buttons(enabled_buttons)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    cpu_win_w as f64,
                    cpu_win_h as f64,
                ))
                .with_position(sb_pos);
            let win_sb = Arc::new(event_loop.create_window(attr_sb).unwrap());
            self.cpu_state.id = Some(win_sb.id());

            let sb_context = SbContext::new(win_sb.clone()).unwrap();
            let mut sb_surface = SbSurface::new(&sb_context, win_sb.clone()).unwrap();
            sb_surface
                .resize(
                    NonZeroU32::new(cpu_win_w).unwrap(),
                    NonZeroU32::new(cpu_win_h).unwrap(),
                )
                .unwrap();

            self.cpu_state.context = Some(sb_context);
            self.cpu_state.surface = Some(sb_surface);
            self.cpu_state.window = Some(win_sb);

            handlers::update_window_min_sizes(&self.gpu_state, &self.cpu_state);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 1. Считаем дельту времени и обновляем FPS дважды в секунду для точности
        let elapsed = self.last_fps_update.elapsed();
        if elapsed.as_secs_f32() >= 0.5 {
            let secs = elapsed.as_secs_f32();

            // Количество кадров делим на реальное прошедшее время в секундах
            self.current_fps_gpu = (self.frames_gpu as f32 / secs).round() as u32;
            self.current_fps_cpu = (self.frames_cpu as f32 / secs).round() as u32;

            // Сбрасываем таймеры и счётчики для следующего круга
            self.frames_gpu = 0;
            self.frames_cpu = 0;
            self.last_fps_update = Instant::now();
        }
        // Симуляция и отрисовка левого окна (GPU)
        if let (Some(pixels), Some(window), Some(pipeline)) = (
            &mut self.gpu_state.pixels,
            &self.gpu_state.window,
            &self.gpu_state.custom_pipeline,
        ) {
            // Обсчитываем физику шаров на CPU
            Ball::update_physics(&mut self.gpu_state.balls, &self.gpu_state.playfield);

            // Наращиваем счётчик кадров GPU
            self.frames_gpu += 1;

            // Запоминаем нужные ссылки локально, чтобы замыкание не конфликтовало по заимствованиям (borrow checker)
            let balls = &self.gpu_state.balls;
            let w = self.gpu_state.w;
            let h = self.gpu_state.h;
            let playfield = &self.gpu_state.playfield;
            let fps_to_draw = self.current_fps_gpu; // Локальная переменная для замыкания

            // ИСПРАВЛЕНО: Рендерим сцену через официальный метод pixels.render_with
            let render_result = pixels.render_with(|encoder, render_target_view, context| {
                draw_pixels_frame(
                    encoder,
                    render_target_view,
                    &context.device,
                    &context.queue,
                    balls,
                    w,
                    h,
                    playfield,
                    BG_COLOR,
                    pipeline,
                    fps_to_draw,
                );
                Ok(())
            });

            if let Err(err) = render_result {
                println!("Pixels render error: {:?}", err);
            }

            window.request_redraw();
        }

        // Симуляция и отрисовка правого окна (CPU - Softbuffer остаётся прежней)
        if let (Some(surface), Some(window)) = (&mut self.cpu_state.surface, &self.cpu_state.window)
        {
            Ball::update_physics(&mut self.cpu_state.balls, &self.cpu_state.playfield);

            // Наращиваем счётчик кадров CPU
            self.frames_cpu += 1;

            draw_softbuffer_frame(
                &mut self.cpu_state.canvas,
                surface,
                &self.cpu_state.balls,
                self.cpu_state.w,
                self.cpu_state.h,
                &self.cpu_state.playfield,
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

                if Some(window_id) == self.gpu_state.id {
                    handlers::resize_gpu_window(&mut self.gpu_state, new_size);
                } else if Some(window_id) == self.cpu_state.id {
                    handlers::resize_cpu_window(&mut self.cpu_state, new_size);
                }

                handlers::update_window_min_sizes(&self.gpu_state, &self.cpu_state);
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
                        self.mouse_start_press = Some(Instant::now());
                    } else if state == ElementState::Released {
                        if let Some(start_time) = self.mouse_start_press.take() {
                            duration_ms = start_time.elapsed().as_secs_f32() * 1000.0;
                        }
                    }
                }

                handlers::handle_mouse_input(
                    window_id,
                    state,
                    button,
                    self.cursor_pos,
                    &mut self.gpu_state,
                    &mut self.cpu_state,
                    duration_ms,
                );

                if state == ElementState::Released || state == ElementState::Pressed {
                    handlers::update_window_min_sizes(&self.gpu_state, &self.cpu_state);
                }
            }
            _ => (),
        }
    }
}
