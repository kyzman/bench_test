use crate::ball::{Ball, Playfield, ShapeMarker};
use pixels::Pixels;
use pixels::wgpu;
use rayon::prelude::*;
use ril::draw::{Ellipse, Rectangle};
use ril::{Image, Pixel};
use softbuffer::Surface;
use std::sync::Arc; // Импортируем низкоуровневый wgpu из pixels

// 1. Структура одного шара для GPU (Выравнивание Std430)
// Все поля f32 занимают по 4 байта, итого структура весит ровно 24 байта (кратно 4)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuBall {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub marker: f32, // <-- НОВОЕ ПОЛЕ
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

// 2. Глобальные параметры для Uniform-буфера шейдера (Выравнивание Std140)
// Видеокарта требует, чтобы размер Uniform-структуры был кратен 16 байтам.
// У нас 4 поля по 4 байта (u32/f32) = ровно 16 байт.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuGlobals {
    pub ball_count: u32,
    pub screen_width: f32,
    pub screen_height: f32,
    pub panel_height: f32,
}

// 3. Главный контейнер кастомного конвейера рендеринга GPU
// Он будет хранить скомпилированный шейдер и буферы видеопамяти внутри PixelsState
pub struct CustomRenderPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub balls_buffer: wgpu::Buffer,
    pub globals_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

impl CustomRenderPipeline {
    pub fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        max_balls: usize,
    ) -> Self {
        // Компилируем WGSL-шейдер
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Custom GPU Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        // Создаем Storage Buffer под массив шаров
        let balls_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Balls Storage Buffer"),
            size: (max_balls * std::mem::size_of::<GpuBall>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Создаем Uniform Buffer под глобальные параметры
        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals Uniform Buffer"),
            size: std::mem::size_of::<GpuGlobals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Описываем схему привязки ресурсов
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Custom Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Создаем саму Bind Group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Custom Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: balls_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: globals_buffer.as_entire_binding(),
                },
            ],
        });

        // ИСПРАВЛЕНО 1 и 3: Исправлено под новые требования PipelineLayoutDescriptor (immediate_size и BindGroupLayout)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Custom Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0, // Задаем обязательный нулевой размер для непосредственных констант
        });

        // Строим финальный графический конвейер
        // ИСПРАВЛЕНО 4, 5 и 6: Добавлены compilation_options, типы Option<&str> и заменен multiview на multiview_mask
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Custom Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // Теперь это Option<&str>
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                // ИСПРАВЛЕНО: wgpu не имеет PrimitiveFace, лицевая сторона настраивается через FrontFace
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"), // Теперь это Option<&str>
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview_mask: None, // Заменили устаревший multiview
            cache: None,
        });

        Self {
            render_pipeline,
            balls_buffer,
            globals_buffer,
            bind_group,
        }
    }
}

fn get_safe_circle_params(
    bx: f32,
    by: f32,
    radius: f32,
    field: &Playfield,
) -> Option<(u32, u32, u32)> {
    let r = radius.round() as i32;
    let cx = bx.round() as i32;
    let cy = by.round() as i32;

    // Проверяем, что шар физически не вылезает за внутренние границы Playfield
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
// ОБНОВЛЕНО: Функция теперь принимает encoder и render_target_view напрямую от pixels
pub fn draw_pixels_frame(
    encoder: &mut wgpu::CommandEncoder,
    render_target_view: &wgpu::TextureView,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    balls: &[Ball],
    width: u32,
    height: u32,
    playfield: &Playfield,
    bg_color: (u8, u8, u8),
    pipeline: &CustomRenderPipeline,
) {
    // 1. Упаковываем глобальные параметры для Uniform-буфера шейдера
    let gpu_globals = GpuGlobals {
        ball_count: balls.len() as u32,
        screen_width: width as f32,
        screen_height: height as f32,
        panel_height: crate::PANEL_HEIGHT as f32,
    };
    queue.write_buffer(
        &pipeline.globals_buffer,
        0,
        bytemuck::bytes_of(&gpu_globals),
    );

    // 2. Преобразуем массив шаров для GPU
    let gpu_balls: Vec<GpuBall> = balls
        .iter()
        .map(|b| {
            // Переводим наше перечисление ShapeMarker в число f32 для шейдера
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
                marker: marker_id, // Передаем ID фигуры на GPU
                r: b.color.0 as f32 / 255.0,
                g: b.color.1 as f32 / 255.0,
                b: b.color.2 as f32 / 255.0,
            }
        })
        .collect();

    if !gpu_balls.is_empty() {
        queue.write_buffer(&pipeline.balls_buffer, 0, bytemuck::cast_slice(&gpu_balls));
    }

    // 3. Открываем проход рендеринга видеокарты, используя переданный render_target_view
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Custom Shader Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target_view, // Использован вьюпорт от pixels
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

pub fn draw_softbuffer_frame(
    canvas: &mut Image<ril::Rgb>,
    surface: &mut Surface<Arc<winit::window::Window>, Arc<winit::window::Window>>,
    balls: &[Ball],
    width: u32,
    height: u32,
    playfield: &Playfield,
    bg_color: (u8, u8, u8),
) {
    // 1. Задний фон всего окна
    let window_bg = Rectangle::at(0, 0)
        .with_size(width, height)
        .with_fill(ril::Rgb::new(bg_color.0, bg_color.1, bg_color.2));
    canvas.draw(&window_bg);

    // 2. Визуализация игрового поля
    let field_rect = Rectangle::at(playfield.x as u32, playfield.y as u32)
        .with_size(playfield.w as u32, playfield.h as u32)
        .with_fill(ril::Rgb::new(40, 40, 40));
    canvas.draw(&field_rect);

    // 3. Рисуем шары
    for ball in balls {
        if let Some((x, y, r)) = get_safe_circle_params(ball.x, ball.y, ball.radius, playfield) {
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

    // 4. Рисуем нижнюю панель
    let panel_y = height - crate::PANEL_HEIGHT;
    let panel_rect = Rectangle::at(0, panel_y)
        .with_size(width, crate::PANEL_HEIGHT)
        .with_fill(ril::Rgb::new(15, 15, 15));
    canvas.draw(&panel_rect);

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
