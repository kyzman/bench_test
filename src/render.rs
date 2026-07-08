use crate::ball::{BALL_RADIUS, Ball};
use pixels::Pixels;
use ril::draw::{Ellipse, Rectangle};
use ril::prelude::*;
use softbuffer::Surface;
use std::sync::Arc;

// Функция для безопасного получения координат отрисовки радиуса
fn get_safe_circle_params(bx: f32, by: f32, max_w: u32, max_h: u32) -> Option<(u32, u32, u32)> {
    let r = BALL_RADIUS.round() as i32;
    let cx = bx.round() as i32;
    let cy = by.round() as i32;

    if cx - r < 0 || cx + r >= max_w as i32 || cy - r < 0 || cy + r >= max_h as i32 {
        None // Пропускаем отрисовку кадра, если шар вышел за границы холста
    } else {
        Some((cx as u32, cy as u32, r as u32))
    }
}

pub fn draw_pixels_frame(
    canvas: &mut Image<Rgba>,
    pixels: &mut Pixels,
    balls: &[Ball],
    width: u32,
    height: u32,
) {
    // Очистка фона
    let background = Rectangle::at(0, 0)
        .with_size(width, height)
        .with_fill(Rgba::new(30, 30, 30, 255));
    canvas.draw(&background);

    // Рисование шаров
    for ball in balls {
        if let Some((x, y, r)) = get_safe_circle_params(ball.x, ball.y, width, height) {
            let circle = Ellipse::circle(x, y, r).with_fill(Rgba::new(255, 255, 255, 255));
            canvas.draw(&circle);
        }
    }

    // Копирование в буфер GPU
    let pixels_buffer = pixels.frame_mut();
    let pixel_slice: &[Rgba] = &canvas.data;
    let raw_bytes = unsafe {
        std::slice::from_raw_parts(pixel_slice.as_ptr() as *const u8, pixel_slice.len() * 4)
    };
    pixels_buffer.copy_from_slice(raw_bytes);
    pixels.render().unwrap();
}

pub fn draw_softbuffer_frame(
    canvas: &mut Image<Rgb>,
    surface: &mut Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>,
    balls: &[Ball],
    width: u32,
    height: u32,
) {
    // Очистка фона
    let background = Rectangle::at(0, 0)
        .with_size(width, height)
        .with_fill(Rgb::new(30, 30, 30));
    canvas.draw(&background);

    // Рисование шаров
    for ball in balls {
        if let Some((x, y, r)) = get_safe_circle_params(ball.x, ball.y, width, height) {
            let circle = Ellipse::circle(x, y, r).with_fill(Rgb::new(255, 255, 0));
            canvas.draw(&circle);
        }
    }

    // Копирование и конвертация для CPU вывода
    let mut buffer = surface.buffer_mut().unwrap();
    let ril_pixels = &canvas.data;
    for (i, pixel) in ril_pixels.iter().enumerate() {
        buffer[i] = ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32);
    }
    buffer.present().unwrap();
}
