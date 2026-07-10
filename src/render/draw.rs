use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use pixels::wgpu;
use rayon::prelude::*;
use ril::draw::{Draw, Ellipse, Rectangle};
use ril::{Image, Pixel, Rgb};
use softbuffer::Surface;
use std::ops::DerefMut;
use std::sync::Arc;

use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X12},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

use crate::ball::{Ball, Playfield, ShapeMarker};
use crate::render::pipeline::{CustomRenderPipeline, GpuBall, GpuGlobals};

/// Кастомный примитив для отрисовки текста в экосистеме RIL
pub struct TextPrimitive<'a, P: Pixel> {
    pub text: &'a str,
    pub x: f32,
    pub y: f32,
    pub scale_px: f32,
    pub font_bytes: &'a [u8],
    pub color: P,
}

impl<'a, P: Pixel> TextPrimitive<'a, P> {
    pub fn new(
        text: &'a str,
        x: f32,
        y: f32,
        scale_px: f32,
        font_bytes: &'a [u8],
        color: P,
    ) -> Self {
        Self {
            text,
            x,
            y,
            scale_px,
            font_bytes,
            color,
        }
    }
}

// 1. Дженерик переносим в сам трейт: Draw<P>
impl<'a, P: Pixel + Copy> Draw<P> for TextPrimitive<'a, P> {
    // Убираем ассоциированный тип, так как его нет в трейте

    fn draw<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        // Получаем прямую изменяемую ссылку на сам Image<P> через разыменование
        let img = image.deref_mut();

        let font = FontRef::try_from_slice(self.font_bytes).expect("Ошибка TTF");
        let scale = PxScale::from(self.scale_px);
        let scaled_font = font.as_scaled(scale);

        let mut glyphs = Vec::new();
        let mut caret = point(self.x, self.y);
        let mut last_glyph = None;

        for c in self.text.chars() {
            let glyph_id = font.glyph_id(c);
            if let Some(last) = last_glyph {
                caret.x += scaled_font.kern(last, glyph_id);
            }
            let glyph = glyph_id.with_scale_and_position(scale, caret);
            caret.x += scaled_font.h_advance(glyph_id);
            last_glyph = Some(glyph_id);
            glyphs.push(glyph);
        }

        for glyph in glyphs {
            if let Some(outline) = font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|px, py, coverage| {
                    if coverage > 0.4 {
                        let target_x = (bounds.min.x + px as f32) as i32;
                        let target_y = (bounds.min.y + py as f32) as i32;

                        // 2. Теперь методы width() и set_pixel() доступны, так как мы вызываем их у img (&mut Image<P>)
                        if target_x >= 0
                            && target_x < img.width() as i32
                            && target_y >= 0
                            && target_y < img.height() as i32
                        {
                            img.set_pixel(target_x as u32, target_y as u32, self.color);
                        }
                    }
                });
            }
        }
    }
}

struct CpuTextTarget<'a> {
    canvas: &'a mut Image<Rgb>,
    color: Rgb,
    // Теперь передаем размеры окна внутрь самой структуры как её собственные поля
    width: u32,
    height: u32,
}
impl<'a> DrawTarget for CpuTextTarget<'a> {
    type Color = BinaryColor;
    type Error = std::convert::Infallible;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for embedded_graphics::Pixel(point, col) in pixels {
            // Масштабируем шрифт в 2 раза прямо на процессоре!
            // Родной размер 6x12 превратится в крупный 12x24, заполнив всю панель
            if col == BinaryColor::On && point.x >= 0 && point.y >= 0 {
                let base_x = (point.x as u32) * 2 + 10; // Отступ слева 10px
                let base_y = (point.y as u32) * 2 + (self.height - crate::PANEL_HEIGHT); // Пишем строго на панели

                // Рисуем жирный пиксель 2х2
                if base_x + 1 < self.width && base_y + 1 < self.height {
                    let dot = Rectangle::at(base_x, base_y)
                        .with_size(2, 2)
                        .with_fill(self.color);
                    self.canvas.draw(&dot);
                }
            }
        }
        Ok(())
    }
}
impl<'a> OriginDimensions for CpuTextTarget<'a> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

// Вспомогательная функция проверки границ (только для CPU-окна)
fn get_safe_circle_params(
    bx: f32,
    by: f32,
    radius: f32,
    field: &Playfield,
) -> Option<(u32, u32, u32)> {
    let r = radius.round() as i32;
    let cx = bx.round() as i32;
    let cy = by.round() as i32;

    if cx - r < field.x as i32
        || cx + r >= (field.x + field.w) as i32
        || cy - r < field.y as i32
        || cy + r >= (field.y + field.h) as i32
    {
        None
    } else {
        Some((cx as u32, cy as u32, r as u32))
    }
}

// Рендерер маркеров для CPU-окна
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

fn get_contrast_color(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    let brightness = (rgb.0 as f32 * 0.299) + (rgb.1 as f32 * 0.587) + (rgb.2 as f32 * 0.114);
    if brightness > 128.0 {
        (0, 0, 0)
    } else {
        (255, 255, 255)
    }
}

// 1. АППАРАТНЫЙ РЕНДЕРИНГ КАДРА НА GPU (WGPU / Шейдер)
// ОБНОВЛЕНО: Добавлен аргумент current_fps: u32 в самый конец сигнатуры
pub fn draw_pixels_frame(
    encoder: &mut wgpu::CommandEncoder,
    render_target_view: &wgpu::TextureView,
    _device: &wgpu::Device,
    queue: &wgpu::Queue,
    balls: &[Ball],
    width: u32,
    height: u32,
    _playfield: &Playfield,
    bg_color: (u8, u8, u8),
    pipeline: &CustomRenderPipeline,
    current_fps: u32, // <-- ПРИНИМАЕМ ДИНАМИЧЕСКИЙ FPS
) {
    // Формируем Юникод-строку для шейдера на нижней панели
    let display_string = format!("FPS: {}  Ш: {:<3}", current_fps, balls.len());
    let mut text_unicode = [0u32; 16];
    for (idx, ch) in display_string.chars().take(16).enumerate() {
        text_unicode[idx] = ch as u32;
    }

    let gpu_globals = GpuGlobals {
        ball_count: balls.len() as u32,
        screen_width: width as f32,
        screen_height: height as f32,
        panel_height: crate::PANEL_HEIGHT as f32,
        text_data: text_unicode,
    };
    queue.write_buffer(
        &pipeline.globals_buffer,
        0,
        bytemuck::bytes_of(&gpu_globals),
    );

    let gpu_balls: Vec<GpuBall> = balls
        .iter()
        .map(|b| {
            let marker_id = match b.marker {
                ShapeMarker::None => 0.0,
                ShapeMarker::Square => 1.0,
                ShapeMarker::Dot => 2.0,
                ShapeMarker::Cross => 3.0,
                ShapeMarker::Rhombus => 4.0,
                ShapeMarker::Triangle => 5.0,
                ShapeMarker::Star => 6.0,
            };
            GpuBall {
                x: b.x,
                y: b.y,
                radius: b.radius,
                marker: marker_id,
                r: b.color.0 as f32 / 255.0,
                g: b.color.1 as f32 / 255.0,
                b: b.color.2 as f32 / 255.0,
            }
        })
        .collect();

    if !gpu_balls.is_empty() {
        queue.write_buffer(&pipeline.balls_buffer, 0, bytemuck::cast_slice(&gpu_balls));
    }

    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Custom Shader Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg_color.0 as f64 / 255.0,
                        g: bg_color.1 as f64 / 255.0,
                        b: bg_color.2 as f64 / 255.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            multiview_mask: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        rpass.set_pipeline(&pipeline.render_pipeline);
        rpass.set_bind_group(0, &pipeline.bind_group, &[]);
        rpass.draw(0..4, 0..1);
    }
}

// 2. ПРОГРАММНЫЙ РЕНДЕРИНГ КАДРА НА CPU (Softbuffer + Rayon)
// ОБНОВЛЕНО: Добавлен аргумент current_fps: u32 в самый конец сигнатуры
pub fn draw_softbuffer_frame(
    canvas: &mut Image<Rgb>,
    surface: &mut Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>,
    balls: &[Ball],
    width: u32,
    height: u32,
    playfield: &Playfield,
    bg_color: (u8, u8, u8),
    current_fps: u32, // <-- ПРИНИМАЕМ ЖИВОЙ FPS ДЛЯ CPU
) {
    let font_bytes = include_bytes!("../../assets/font.ttf");

    let window_bg = Rectangle::at(0, 0)
        .with_size(width, height)
        .with_fill(Rgb::new(bg_color.0, bg_color.1, bg_color.2));
    canvas.draw(&window_bg);

    let field_rect = Rectangle::at(playfield.x as u32, playfield.y as u32)
        .with_size(playfield.w as u32, playfield.h as u32)
        .with_fill(Rgb::new(40, 40, 40));
    canvas.draw(&field_rect);

    for ball in balls {
        if let Some((x, y, r)) = get_safe_circle_params(ball.x, ball.y, ball.radius, playfield) {
            canvas.draw(&Ellipse::circle(x, y, r).with_fill(Rgb::new(
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
                Rgb::new(m_color.0, m_color.1, m_color.2),
            );
        }
    }

    let panel_y = height - crate::PANEL_HEIGHT;
    let panel_rect = Rectangle::at(0, panel_y)
        .with_size(width, crate::PANEL_HEIGHT)
        .with_fill(Rgb::new(15, 15, 15));
    canvas.draw(&panel_rect);

    let test_text = TextPrimitive::new(
        "TEST",
        10.0,
        40.0, // С запасом на базовую линию (baseline)
        16.0,
        font_bytes,
        ril::Rgb::new(255, 255, 255),
    );
    canvas.draw(&test_text);

    // --- НОВЫЙ БЛОК: РЕНДЕРИНГ КРУПНОГО ТЕКСТА НА CPU ПАНЕЛИ ---
    // Наша структура-таргет для выжигания пикселей embedded-graphics прямо на RIL-холсте кадра
    // Формируем живую строку статуса и выжигаем её на холсте в ярко-зеленом цвете
    let display_string = format!("FPS: {:<3}  B: {:<3}", current_fps, balls.len());
    let mut text_target = CpuTextTarget {
        canvas,
        color: Rgb::new(0, 255, 0),
        width,
        height,
    };
    let text_style = MonoTextStyle::new(&FONT_6X12, BinaryColor::On);

    // Смещение по Y выставляем в 11, чтобы шрифт встал ровно по сетке панели
    Text::new(&display_string, Point::new(0, 11), text_style)
        .draw(&mut text_target)
        .unwrap();
    // -----------------------------------------------------------

    let mut buffer = surface.buffer_mut().unwrap();
    let ril_pixels = &canvas.data;
    buffer
        .par_iter_mut()
        .enumerate()
        .for_each(|(i, target_pixel)| {
            let pixel = ril_pixels[i];
            *target_pixel = ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32);
        });
    buffer.present().unwrap();
}
