use ril::prelude::{Image, Rgb, Rgba};
use std::num::NonZeroU32;
use winit::event::{ElementState, MouseButton};
use winit::window::WindowId;

use crate::app::state::{BG_COLOR, PixelsState, SoftbufferState};
use crate::ball::Ball;

// Функция мягкой сцепки окон в реальном времени при перемещении (Moved)
pub fn handle_window_moved(
    window_id: WindowId,
    new_position: winit::dpi::PhysicalPosition<i32>,
    gpu_state: &PixelsState,
    cpu_state: &SoftbufferState,
) {
    if Some(window_id) == gpu_state.id {
        if let (Some(win_p), Some(win_sb)) = (&gpu_state.window, &cpu_state.window) {
            let target_pos = winit::dpi::PhysicalPosition::new(
                new_position.x + win_p.outer_size().width as i32,
                new_position.y,
            );
            win_sb.set_outer_position(target_pos);
        }
    } else if Some(window_id) == cpu_state.id {
        if let (Some(win_p), Some(win_sb)) = (&gpu_state.window, &cpu_state.window) {
            let target_pos = winit::dpi::PhysicalPosition::new(
                new_position.x - win_p.outer_size().width as i32,
                new_position.y,
            );
            win_p.set_outer_position(target_pos);
        }
    }
}

// Функция синхронизации положения окон при изменении размера (Resized)
pub fn sync_positions_on_resize(gpu_state: &PixelsState, cpu_state: &SoftbufferState) {
    if let (Some(win_p), Some(win_sb)) = (&gpu_state.window, &cpu_state.window) {
        if let (Ok(p_pos), Ok(sb_pos)) = (win_p.outer_position(), win_sb.outer_position()) {
            let p_outer_size = win_p.outer_size();

            // Если правое окно сместилось относительно новой границы левого — возвращаем его на место встык
            let expected_sb_x = p_pos.x + p_outer_size.width as i32;
            if sb_pos.x != expected_sb_x || sb_pos.y != p_pos.y {
                win_sb
                    .set_outer_position(winit::dpi::PhysicalPosition::new(expected_sb_x, p_pos.y));
            }
        }
    }
}

// ОБНОВЛЕНО: Пересчитывает игровое поле GPU-окна при изменении размеров
pub fn resize_gpu_window(gpu_state: &mut PixelsState, new_size: winit::dpi::PhysicalSize<u32>) {
    gpu_state.w = new_size.width;
    gpu_state.h = new_size.height;

    // Пересчитываем локальное игровое поле: ширина равна ширине окна, высота урезана под панель
    gpu_state.playfield.w = gpu_state.w as f32;
    gpu_state.playfield.h = (gpu_state.h - crate::PANEL_HEIGHT) as f32;

    gpu_state.canvas = Image::new(gpu_state.w, gpu_state.h, Rgba::new(0, 0, 0, 255));
    if let Some(pixels) = &mut gpu_state.pixels {
        pixels.resize_buffer(gpu_state.w, gpu_state.h).unwrap();
        pixels.resize_surface(gpu_state.w, gpu_state.h).unwrap();
    }
}

// ОБНОВЛЕНО: Пересчитывает игровое поле CPU-окна при изменении размеров
pub fn resize_cpu_window(cpu_state: &mut SoftbufferState, new_size: winit::dpi::PhysicalSize<u32>) {
    cpu_state.w = new_size.width;
    cpu_state.h = new_size.height;

    // Пересчитываем локальное игровое поле: ширина равна ширине окна, высота урезана под панель
    cpu_state.playfield.w = cpu_state.w as f32;
    cpu_state.playfield.h = (cpu_state.h - crate::PANEL_HEIGHT) as f32;

    cpu_state.canvas = Image::new(cpu_state.w, cpu_state.h, Rgb::new(0, 0, 0));
    if let Some(surface) = &mut cpu_state.surface {
        if let (Some(w), Some(h)) = (NonZeroU32::new(cpu_state.w), NonZeroU32::new(cpu_state.h)) {
            surface.resize(w, h).unwrap();
        }
    }
}

// ОБНОВЛЕНО: Добавлен параметр duration_ms, передающийся в Ball::spawn_at
pub fn handle_mouse_input(
    window_id: WindowId,
    state: ElementState,
    button: MouseButton,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    gpu_state: &mut PixelsState,
    cpu_state: &mut SoftbufferState,
    duration_ms: f32, // Задел на будущее
) {
    let mouse_x = cursor_pos.x as f32;
    let mouse_y = cursor_pos.y as f32;

    if Some(window_id) == gpu_state.id {
        match button {
            MouseButton::Left => {
                // Спавн или клик обрабатываем только при ОТПУСКАНИИ (Released) кнопки мыши,
                // чтобы мы успели замерить длительность зажатия!
                if state == ElementState::Released {
                    let mut hit_any_ball = false;
                    for ball in gpu_state.balls.iter_mut() {
                        if ball.check_click(mouse_x, mouse_y, BG_COLOR) {
                            hit_any_ball = true;
                            break;
                        }
                    }
                    if !hit_any_ball {
                        // Передаем duration_ms в метод спавна
                        gpu_state.balls.push(Ball::spawn_at(
                            mouse_x,
                            mouse_y,
                            gpu_state.default_color,
                            duration_ms,
                        ));
                    }
                }
            }
            MouseButton::Right => {
                if state == ElementState::Pressed {
                    if let Some(index) = gpu_state
                        .balls
                        .iter()
                        .position(|b| b.is_point_inside(mouse_x, mouse_y))
                    {
                        gpu_state.balls.remove(index);
                    }
                }
            }
            _ => (),
        }
    } else if Some(window_id) == cpu_state.id {
        match button {
            MouseButton::Left => {
                if state == ElementState::Released {
                    let mut hit_any_ball = false;
                    for ball in cpu_state.balls.iter_mut() {
                        if ball.check_click(mouse_x, mouse_y, BG_COLOR) {
                            hit_any_ball = true;
                            break;
                        }
                    }
                    if !hit_any_ball {
                        cpu_state.balls.push(Ball::spawn_at(
                            mouse_x,
                            mouse_y,
                            cpu_state.default_color,
                            duration_ms,
                        ));
                    }
                }
            }
            MouseButton::Right => {
                if state == ElementState::Pressed {
                    if let Some(index) = cpu_state
                        .balls
                        .iter()
                        .position(|b| b.is_point_inside(mouse_x, mouse_y))
                    {
                        cpu_state.balls.remove(index);
                    }
                }
            }
            _ => (),
        }
    }
}

// ИСПРАВЛЕНО: Правильная распаковка кортежа (u32, u32) перед приведением типов к f64
pub fn update_window_min_sizes(gpu_state: &PixelsState, cpu_state: &SoftbufferState) {
    // 1. Лимиты для GPU окна
    let (min_gpu_w, min_gpu_h) = Ball::calculate_min_window_size(&gpu_state.balls);
    // Приводим к f64 ширину и высоту раздельно
    let gpu_logical_size = winit::dpi::LogicalSize::new(min_gpu_w as f64, min_gpu_h as f64);

    if let Some(window) = &gpu_state.window {
        window.set_min_inner_size(Some(gpu_logical_size));
    }

    // 2. Лимиты для CPU окна
    let (min_cpu_w, min_cpu_h) = Ball::calculate_min_window_size(&cpu_state.balls);
    // Приводим к f64 ширину и высоту раздельно
    let cpu_logical_size = winit::dpi::LogicalSize::new(min_cpu_w as f64, min_cpu_h as f64);

    if let Some(window) = &cpu_state.window {
        window.set_min_inner_size(Some(cpu_logical_size));
    }
}
