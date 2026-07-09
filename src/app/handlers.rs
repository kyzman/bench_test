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

// Функция изменения размеров холстов для GPU (Pixels)
pub fn resize_gpu_window(gpu_state: &mut PixelsState, new_size: winit::dpi::PhysicalSize<u32>) {
    gpu_state.w = new_size.width;
    gpu_state.h = new_size.height;
    gpu_state.canvas = Image::new(gpu_state.w, gpu_state.h, Rgba::new(0, 0, 0, 255));
    if let Some(pixels) = &mut gpu_state.pixels {
        pixels.resize_buffer(gpu_state.w, gpu_state.h).unwrap();
        pixels.resize_surface(gpu_state.w, gpu_state.h).unwrap();
    }
}

// Функция изменения размеров холстов для CPU (Softbuffer)
pub fn resize_cpu_window(cpu_state: &mut SoftbufferState, new_size: winit::dpi::PhysicalSize<u32>) {
    cpu_state.w = new_size.width;
    cpu_state.h = new_size.height;
    cpu_state.canvas = Image::new(cpu_state.w, cpu_state.h, Rgb::new(0, 0, 0));
    if let Some(surface) = &mut cpu_state.surface {
        if let (Some(w), Some(h)) = (NonZeroU32::new(cpu_state.w), NonZeroU32::new(cpu_state.h)) {
            surface.resize(w, h).unwrap();
        }
    }
}

// Универсальный обработчик кликов (ЛКМ и ПКМ)
pub fn handle_mouse_input(
    window_id: WindowId,
    state: ElementState,
    button: MouseButton,
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    gpu_state: &mut PixelsState,
    cpu_state: &mut SoftbufferState,
) {
    if state != ElementState::Pressed {
        return;
    }

    let mouse_x = cursor_pos.x as f32;
    let mouse_y = cursor_pos.y as f32;

    if Some(window_id) == gpu_state.id {
        match button {
            MouseButton::Left => {
                let mut hit_any_ball = false;
                for ball in gpu_state.balls.iter_mut() {
                    if ball.check_click(mouse_x, mouse_y, BG_COLOR) {
                        hit_any_ball = true;
                        break;
                    }
                }
                if !hit_any_ball {
                    gpu_state
                        .balls
                        .push(Ball::spawn_at(mouse_x, mouse_y, gpu_state.default_color));
                }
            }
            MouseButton::Right => {
                if let Some(index) = gpu_state
                    .balls
                    .iter()
                    .position(|b| b.is_point_inside(mouse_x, mouse_y))
                {
                    gpu_state.balls.remove(index);
                }
            }
            _ => (),
        }
    } else if Some(window_id) == cpu_state.id {
        match button {
            MouseButton::Left => {
                let mut hit_any_ball = false;
                for ball in cpu_state.balls.iter_mut() {
                    if ball.check_click(mouse_x, mouse_y, BG_COLOR) {
                        hit_any_ball = true;
                        break;
                    }
                }
                if !hit_any_ball {
                    cpu_state
                        .balls
                        .push(Ball::spawn_at(mouse_x, mouse_y, cpu_state.default_color));
                }
            }
            MouseButton::Right => {
                if let Some(index) = cpu_state
                    .balls
                    .iter()
                    .position(|b| b.is_point_inside(mouse_x, mouse_y))
                {
                    cpu_state.balls.remove(index);
                }
            }
            _ => (),
        }
    }
}
