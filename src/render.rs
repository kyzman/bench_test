use crate::ball::{BALL_RADIUS, Ball, ShapeMarker};
use pixels::Pixels;
use ril::draw::{Ellipse, Rectangle};
use ril::{Image, Pixel};
use softbuffer::Surface;
use std::sync::Arc;

fn get_safe_circle_params(bx: f32, by: f32, max_w: u32, max_h: u32) -> Option<(u32, u32, u32)> {
    let r = BALL_RADIUS.round() as i32;
    let cx = bx.round() as i32;
    let cy = by.round() as i32;

    if cx - r < 0 || cx + r >= max_w as i32 || cy - r < 0 || cy + r >= max_h as i32 {
        None
    } else {
        Some((cx as u32, cy as u32, r as u32))
    }
}

fn draw_marker_generic<P: Pixel>(
    canvas: &mut Image<P>,
    x: u32,
    y: u32,
    marker: ShapeMarker,
    color: P,
) {
    match marker {
        ShapeMarker::None => {}
        ShapeMarker::Square => {
            canvas.draw(
                &Rectangle::at(x.saturating_sub(3), y.saturating_sub(3))
                    .with_size(6, 6)
                    .with_fill(color),
            );
        }
        ShapeMarker::Dot => {
            canvas.draw(
                &Rectangle::at(x.saturating_sub(1), y.saturating_sub(1))
                    .with_size(2, 2)
                    .with_fill(color),
            );
        }
        ShapeMarker::Cross => {
            canvas.draw(
                &Rectangle::at(x.saturating_sub(4), y.saturating_sub(1))
                    .with_size(8, 2)
                    .with_fill(color),
            );
            canvas.draw(
                &Rectangle::at(x.saturating_sub(1), y.saturating_sub(4))
                    .with_size(2, 8)
                    .with_fill(color),
            );
        }
        ShapeMarker::Rhombus => {
            canvas.draw(
                &Rectangle::at(x.saturating_sub(2), y.saturating_sub(2))
                    .with_size(4, 4)
                    .with_fill(color),
            );
        }
        ShapeMarker::Triangle => {
            canvas.draw(
                &Rectangle::at(x.saturating_sub(3), y + 2)
                    .with_size(7, 2)
                    .with_fill(color),
            );
            canvas.draw(
                &Rectangle::at(x.saturating_sub(2), y)
                    .with_size(5, 2)
                    .with_fill(color),
            );
            canvas.draw(
                &Rectangle::at(x.saturating_sub(1), y.saturating_sub(2))
                    .with_size(3, 2)
                    .with_fill(color),
            );
        }
        ShapeMarker::Star => {
            canvas.draw(
                &Rectangle::at(x.saturating_sub(2), y.saturating_sub(2))
                    .with_size(5, 5)
                    .with_fill(color),
            );
            canvas.draw(
                &Rectangle::at(x.saturating_sub(4), y.saturating_sub(1))
                    .with_size(9, 3)
                    .with_fill(color),
            );
            canvas.draw(
                &Rectangle::at(x.saturating_sub(1), y.saturating_sub(4))
                    .with_size(3, 9)
                    .with_fill(color),
            );
        }
    }
}

// Вычисляем контрастный цвет для маркера (черный или белый) в зависимости от яркости шара
fn get_contrast_color(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    let brightness = (rgb.0 as f32 * 0.299) + (rgb.1 as f32 * 0.587) + (rgb.2 as f32 * 0.114);
    if brightness > 128.0 {
        (0, 0, 0)
    } else {
        (255, 255, 255)
    }
}

pub fn draw_pixels_frame(
    canvas: &mut Image<ril::Rgba>,
    pixels: &mut Pixels,
    balls: &[Ball],
    width: u32,
    height: u32,
    bg_color: (u8, u8, u8),
) {
    let background = Rectangle::at(0, 0)
        .with_size(width, height)
        .with_fill(ril::Rgba::new(bg_color.0, bg_color.1, bg_color.2, 255));
    canvas.draw(&background);

    for ball in balls {
        if let Some((x, y, r)) = get_safe_circle_params(ball.x, ball.y, width, height) {
            // Рисуем шар его собственным цветом
            canvas.draw(&Ellipse::circle(x, y, r).with_fill(ril::Rgba::new(
                ball.color.0,
                ball.color.1,
                ball.color.2,
                255,
            )));

            let m_color = get_contrast_color(ball.color);
            draw_marker_generic(
                canvas,
                x,
                y,
                ball.marker,
                ril::Rgba::new(m_color.0, m_color.1, m_color.2, 255),
            );
        }
    }

    let pixels_buffer = pixels.frame_mut();
    let pixel_slice: &[ril::Rgba] = &canvas.data;
    let raw_bytes = unsafe {
        std::slice::from_raw_parts(pixel_slice.as_ptr() as *const u8, pixel_slice.len() * 4)
    };
    pixels_buffer.copy_from_slice(raw_bytes);
    pixels.render().unwrap();
}

pub fn draw_softbuffer_frame(
    canvas: &mut Image<ril::Rgb>,
    surface: &mut Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>,
    balls: &[Ball],
    width: u32,
    height: u32,
    bg_color: (u8, u8, u8),
) {
    let background = Rectangle::at(0, 0)
        .with_size(width, height)
        .with_fill(ril::Rgb::new(bg_color.0, bg_color.1, bg_color.2));
    canvas.draw(&background);

    for ball in balls {
        if let Some((x, y, r)) = get_safe_circle_params(ball.x, ball.y, width, height) {
            // Рисуем шар его собственным цветом
            canvas.draw(&Ellipse::circle(x, y, r).with_fill(ril::Rgb::new(
                ball.color.0,
                ball.color.1,
                ball.color.2,
            )));

            let m_color = get_contrast_color(ball.color);
            draw_marker_generic(
                canvas,
                x,
                y,
                ball.marker,
                ril::Rgb::new(m_color.0, m_color.1, m_color.2),
            );
        }
    }

    let mut buffer = surface.buffer_mut().unwrap();
    let ril_pixels = &canvas.data;
    for (i, pixel) in ril_pixels.iter().enumerate() {
        buffer[i] = ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32);
    }
    buffer.present().unwrap();
}
