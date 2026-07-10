use pixels::wgpu;
use ril::prelude::{Image, Rgb, Rgba};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use crate::app::handlers::ThreadCommand;
use crate::app::state::{BG_COLOR, CpuThreadContext, GpuThreadContext};
use crate::ball::Ball;
use crate::render::{draw_pixels_frame, draw_softbuffer_frame};

// СКОРОСТЬ ИЗМЕНЕНИЯ РАДИУСА ЗА ОДИН КАДР СИМУЛЯЦИИ
const GROWTH_SPEED: f32 = 0.15;

pub fn run_gpu_thread(mut ctx: GpuThreadContext<'static>, rx: Receiver<ThreadCommand>) {
    let mut last_fps_update = Instant::now();
    let mut frame_count = 0u32;
    let mut current_fps = 0u32;

    // СОСТОЯНИЕ МЫШИ ДЛЯ ТЕКУЩЕГО ПОТОКА
    let mut l_pressed = false;
    let mut r_pressed = false;
    let mut mx = 0.0f32;
    let mut my = 0.0f32;
    let mut growing_ball_idx: Option<usize> = None;

    loop {
        // 1. Асинхронно обрабатываем входящие системные команды мыши
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                ThreadCommand::Resize { w, h } => {
                    ctx.w = w;
                    ctx.h = h;
                    ctx.playfield.w = w as f32;
                    ctx.playfield.h = (h - crate::PANEL_HEIGHT) as f32;
                    ctx.canvas = Image::new(w, h, Rgba::new(0, 0, 0, 255));
                    ctx.pixels.resize_buffer(w, h).unwrap();
                    ctx.pixels.resize_surface(w, h).unwrap();
                }
                ThreadCommand::MousePressed { x, y, is_left } => {
                    mx = x;
                    my = y;
                    if is_left {
                        l_pressed = true;

                        // Если ЛКМ нажата на пустом месте — МГНОВЕННО спавним фантомный шар
                        let mut hit = false;
                        for ball in ctx.balls.iter_mut() {
                            if ball.check_click(x, y, BG_COLOR) {
                                hit = true;
                                break;
                            }
                        }
                        if !hit {
                            ctx.balls.push(Ball::spawn_at(x, y, ctx.default_color, 0.0));
                            // Запоминаем индекс только что созданного шара (он последний в векторе)
                            growing_ball_idx = Some(ctx.balls.len() - 1);
                            // Обнуляем ему скорость на время удержания
                            if let Some(b) = ctx.balls.last_mut() {
                                b.vx = 0.0;
                                b.vy = 0.0;
                            }
                        }
                    } else {
                        r_pressed = true;
                        // Классическое удаление по ПКМ работает ТОЛЬКО если ЛКМ не зажата
                        if !l_pressed {
                            if let Some(index) =
                                ctx.balls.iter().position(|b| b.is_point_inside(x, y))
                            {
                                ctx.balls.remove(index);
                            }
                        }
                    }
                }
                ThreadCommand::MouseReleased { is_left } => {
                    if is_left {
                        l_pressed = false;
                        // Отпустили ЛКМ — шар созрел! Даем ему начальный случайный импульс полета
                        if let Some(idx) = growing_ball_idx.take() {
                            if idx < ctx.balls.len() {
                                let mut rng = rand::rng();
                                ctx.balls[idx].vx = rand::RngExt::random_range(&mut rng, -4.0..4.0);
                                ctx.balls[idx].vy = rand::RngExt::random_range(&mut rng, -4.0..4.0);
                                if ctx.balls[idx].vx == 0.0 {
                                    ctx.balls[idx].vx = 2.0;
                                }
                                if ctx.balls[idx].vy == 0.0 {
                                    ctx.balls[idx].vy = 2.0;
                                }
                            }
                        }
                    } else {
                        r_pressed = false;
                    }
                }
                ThreadCommand::MouseMove { x, y } => {
                    mx = x;
                    my = y;
                }
            }
        }

        // 2. ДИНАМИЧЕСКИЙ РОСТ ИЛИ СЖАТИЕ ШАРА НА КАЖДОМ КАДРЕ В РЕАЛЬНОМ ВРЕМЕНИ
        if let Some(idx) = growing_ball_idx {
            if idx < ctx.balls.len() {
                // Шар намертво следует за курсором в процессе раздувания
                ctx.balls[idx].x = mx;
                ctx.balls[idx].y = my;

                if l_pressed && !r_pressed {
                    // Зажата только ЛКМ — плавно увеличиваем радиус
                    ctx.balls[idx].radius += GROWTH_SPEED;
                } else if l_pressed && r_pressed {
                    // Зажаты ОБЕ кнопки — плавно уменьшаем радиус (но не меньше 4 пикселей)
                    ctx.balls[idx].radius = (ctx.balls[idx].radius - GROWTH_SPEED).max(4.0);
                }

                // Пересчитываем массу в зависимости от текущего динамического радиуса
                ctx.balls[idx].mass = (ctx.balls[idx].radius * ctx.balls[idx].radius) / 100.0;
            }
        }

        // 3. Расчет FPS потока GPU
        frame_count += 1;
        let elapsed = last_fps_update.elapsed();
        if elapsed.as_secs_f32() >= 0.5 {
            current_fps = (frame_count as f32 / elapsed.as_secs_f32()).round() as u32;
            frame_count = 0;
            last_fps_update = Instant::now();
        }

        // 4. Обновляем физику (исключая растущий шар из коллизий, чтобы избежать взрыва)
        if let Some(idx) = growing_ball_idx {
            // Временно вытаскиваем растущий шар из вектора, чтобы обсчитать физику остальных
            let mut flying_balls = ctx.balls.clone();
            if idx < flying_balls.len() {
                flying_balls.remove(idx);
            }
            Ball::update_physics(&mut flying_balls, &ctx.playfield);

            // Возвращаем физические координаты летящих шаров обратно в контекст
            let mut f_idx = 0;
            for i in 0..ctx.balls.len() {
                if i != idx {
                    ctx.balls[i].x = flying_balls[f_idx].x;
                    ctx.balls[i].y = flying_balls[f_idx].y;
                    ctx.balls[i].vx = flying_balls[f_idx].vx;
                    ctx.balls[i].vy = flying_balls[f_idx].vy;
                    f_idx += 1;
                }
            }
        } else {
            // Если никто не растет — обсчитываем честную физику для всех объектов
            Ball::update_physics(&mut ctx.balls, &ctx.playfield);
        }

        // 5. Рендеринг кадра GPU
        let balls_ref = &ctx.balls;
        let w = ctx.w;
        let h = ctx.h;
        let pf = &ctx.playfield;
        let pipe = &ctx.custom_pipeline;
        let _ = ctx
            .pixels
            .render_with(|encoder, render_target_view, context| {
                draw_pixels_frame(
                    encoder,
                    render_target_view,
                    &context.device,
                    &context.queue,
                    balls_ref,
                    w,
                    h,
                    pf,
                    BG_COLOR,
                    pipe,
                    current_fps,
                );
                Ok(())
            });
    }
}

// Бесконечный автономный цикл для CPU-окна (Softbuffer)
pub fn run_cpu_thread(mut ctx: CpuThreadContext, rx: Receiver<ThreadCommand>) {
    let mut last_fps_update = Instant::now();
    let mut frame_count = 0u32;
    let mut current_fps = 0u32;

    let mut l_pressed = false;
    let mut r_pressed = false;
    let mut mx = 0.0f32;
    let mut my = 0.0f32;
    let mut growing_ball_idx: Option<usize> = None;

    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                ThreadCommand::Resize { w, h } => {
                    ctx.w = w;
                    ctx.h = h;
                    ctx.playfield.w = w as f32;
                    ctx.playfield.h = (h - crate::PANEL_HEIGHT) as f32;
                    ctx.canvas = Image::new(w, h, Rgb::new(0, 0, 0));
                    if let (Some(w_nz), Some(h_nz)) =
                        (std::num::NonZeroU32::new(w), std::num::NonZeroU32::new(h))
                    {
                        ctx.surface.resize(w_nz, h_nz).unwrap();
                    }
                }
                ThreadCommand::MousePressed { x, y, is_left } => {
                    mx = x;
                    my = y;
                    if is_left {
                        l_pressed = true;
                        let mut hit = false;
                        for ball in ctx.balls.iter_mut() {
                            if ball.check_click(x, y, BG_COLOR) {
                                hit = true;
                                break;
                            }
                        }
                        if !hit {
                            ctx.balls.push(Ball::spawn_at(x, y, ctx.default_color, 0.0));
                            growing_ball_idx = Some(ctx.balls.len() - 1);
                            if let Some(b) = ctx.balls.last_mut() {
                                b.vx = 0.0;
                                b.vy = 0.0;
                            }
                        }
                    } else {
                        r_pressed = true;
                        if !l_pressed {
                            if let Some(index) =
                                ctx.balls.iter().position(|b| b.is_point_inside(x, y))
                            {
                                ctx.balls.remove(index);
                            }
                        }
                    }
                }
                ThreadCommand::MouseReleased { is_left } => {
                    if is_left {
                        l_pressed = false;
                        if let Some(idx) = growing_ball_idx.take() {
                            if idx < ctx.balls.len() {
                                let mut rng = rand::rng();
                                ctx.balls[idx].vx = rand::RngExt::random_range(&mut rng, -4.0..4.0);
                                ctx.balls[idx].vy = rand::RngExt::random_range(&mut rng, -4.0..4.0);
                                if ctx.balls[idx].vx == 0.0 {
                                    ctx.balls[idx].vx = 2.0;
                                }
                                if ctx.balls[idx].vy == 0.0 {
                                    ctx.balls[idx].vy = 2.0;
                                }
                            }
                        }
                    } else {
                        r_pressed = false;
                    }
                }
                ThreadCommand::MouseMove { x, y } => {
                    mx = x;
                    my = y;
                }
            }
        }

        if let Some(idx) = growing_ball_idx {
            if idx < ctx.balls.len() {
                ctx.balls[idx].x = mx;
                ctx.balls[idx].y = my;
                if l_pressed && !r_pressed {
                    ctx.balls[idx].radius += GROWTH_SPEED;
                } else if l_pressed && r_pressed {
                    ctx.balls[idx].radius = (ctx.balls[idx].radius - GROWTH_SPEED).max(4.0);
                }
                ctx.balls[idx].mass = (ctx.balls[idx].radius * ctx.balls[idx].radius) / 100.0;
            }
        }
        frame_count += 1;
        let elapsed = last_fps_update.elapsed();
        if elapsed.as_secs_f32() >= 0.5 {
            current_fps = (frame_count as f32 / elapsed.as_secs_f32()).round() as u32;
            frame_count = 0;
            last_fps_update = Instant::now();
        } // Изолированная физика для CPU потока
        if let Some(idx) = growing_ball_idx {
            let mut flying_balls = ctx.balls.clone();
            if idx < flying_balls.len() {
                flying_balls.remove(idx);
            }
            Ball::update_physics(&mut flying_balls, &ctx.playfield);
            let mut f_idx = 0;
            for i in 0..ctx.balls.len() {
                if i != idx {
                    ctx.balls[i].x = flying_balls[f_idx].x;
                    ctx.balls[i].y = flying_balls[f_idx].y;
                    ctx.balls[i].vx = flying_balls[f_idx].vx;
                    ctx.balls[i].vy = flying_balls[f_idx].vy;
                    f_idx += 1;
                }
            }
        } else {
            Ball::update_physics(&mut ctx.balls, &ctx.playfield);
        }
        draw_softbuffer_frame(
            &mut ctx.canvas,
            &mut ctx.surface,
            &ctx.balls,
            ctx.w,
            ctx.h,
            &ctx.playfield,
            BG_COLOR,
            current_fps,
        );
    }
}
