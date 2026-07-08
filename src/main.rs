use pixels::{Pixels, SurfaceTexture};
use ril::draw::{Ellipse, Rectangle};
use ril::prelude::*;
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 400;
const BALL_RADIUS: i32 = 8;
// Константа количества шариков
const BALL_COUNT: usize = 40;

#[derive(Clone)]
struct Ball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

impl Ball {
    fn new(x: f32, y: f32, vx: f32, vy: f32) -> Self {
        Self { x, y, vx, vy }
    }

    fn update(balls: &mut [Ball], width: f32, height: f32) {
        // 1. Движение и отскок от стен
        for ball in balls.iter_mut() {
            ball.x += ball.vx;
            ball.y += ball.vy;

            if ball.x - BALL_RADIUS as f32 <= 0.0 {
                ball.x = BALL_RADIUS as f32;
                ball.vx = -ball.vx;
            } else if ball.x + BALL_RADIUS as f32 >= width {
                ball.x = width - BALL_RADIUS as f32;
                ball.vx = -ball.vx;
            }

            if ball.y - BALL_RADIUS as f32 <= 0.0 {
                ball.y = BALL_RADIUS as f32;
                ball.vy = -ball.vy;
            } else if ball.y + BALL_RADIUS as f32 >= height {
                ball.y = height - BALL_RADIUS as f32;
                ball.vy = -ball.vy;
            }
        }

        // 2. Обработка столкновений между шариками
        for i in 0..balls.len() {
            for j in (i + 1)..balls.len() {
                let dx = balls[j].x - balls[i].x;
                let dy = balls[j].y - balls[i].y;
                let distance = (dx * dx + dy * dy).sqrt();
                let min_dist = (BALL_RADIUS * 2) as f32;

                if distance < min_dist && distance > 0.0 {
                    // Исправление наложения (расталкиваем шары)
                    let overlap = min_dist - distance;
                    let nx = dx / distance;
                    let ny = dy / distance;

                    balls[i].x -= nx * overlap * 0.5;
                    balls[i].y -= ny * overlap * 0.5;
                    balls[j].x += nx * overlap * 0.5;
                    balls[j].y += ny * overlap * 0.5;

                    // Считаем проекции скоростей на вектор столкновения
                    let kx = balls[i].vx - balls[j].vx;
                    let ky = balls[i].vy - balls[j].vy;
                    let p = nx * kx + ny * ky;

                    if p > 0.0 {
                        balls[i].vx -= nx * p;
                        balls[i].vy -= ny * p;
                        balls[j].vx += nx * p;
                        balls[j].vy += ny * p;
                    }
                }
            }
        }
    }
}

struct App<'win> {
    win_pixels: Option<Arc<Window>>,
    id_pixels: Option<WindowId>,
    pixels: Option<Pixels<'win>>,
    canvas_pixels: Image<Rgba>,
    balls_pixels: Vec<Ball>,

    win_softbuffer: Option<Arc<Window>>,
    id_softbuffer: Option<WindowId>,
    sb_context: Option<SbContext<Arc<Window>>>,
    sb_surface: Option<SbSurface<Arc<Window>, Arc<Window>>>,
    canvas_softbuffer: Image<Rgb>,
    balls_softbuffer: Vec<Ball>,
}

impl<'win> Default for App<'win> {
    fn default() -> Self {
        // Указываем явный тип Vec<Ball> для исправления ошибки "type annotations needed"
        let mut balls_p: Vec<Ball> = Vec::with_capacity(BALL_COUNT);
        let mut balls_sb: Vec<Ball> = Vec::with_capacity(BALL_COUNT);

        // Генерация шариков по сетке во избежание мгновенного застревания на старте
        let cols = (BALL_COUNT as f32).sqrt().ceil() as usize;
        let spacing = WIDTH / (cols + 1) as u32;

        for i in 0..BALL_COUNT {
            let row = i / cols;
            let col = i % cols;
            let x = ((col + 1) * spacing as usize) as f32;
            let y = ((row + 1) * spacing as usize) as f32;

            // Разные скорости для асинхронности
            let vx1 = 2.0 + (i as f32 * 0.4);
            let vy1 = 1.5 + (i as f32 * 0.3);
            let vx2 = -1.5 - (i as f32 * 0.3);
            let vy2 = 2.5 + (i as f32 * 0.2);

            balls_p.push(Ball::new(x, y, vx1, vy1));
            balls_sb.push(Ball::new(x, y, vx2, vy2));
        }

        Self {
            win_pixels: None,
            id_pixels: None,
            pixels: None,
            canvas_pixels: Image::new(WIDTH, HEIGHT, Rgba::new(0, 0, 0, 255)),
            balls_pixels: balls_p,

            win_softbuffer: None,
            id_softbuffer: None,
            sb_context: None,
            sb_surface: None,
            canvas_softbuffer: Image::new(WIDTH, HEIGHT, Rgb::new(0, 0, 0)),
            balls_softbuffer: balls_sb,
        }
    }
}

impl<'win> ApplicationHandler for App<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.win_pixels.is_none() {
            let attr_pixels = Window::default_attributes()
                .with_title("GPU (Pixels)")
                .with_inner_size(winit::dpi::LogicalSize::new(WIDTH as f64, HEIGHT as f64));
            let win_p = Arc::new(event_loop.create_window(attr_pixels).unwrap());
            self.id_pixels = Some(win_p.id());

            let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, win_p.clone());
            let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();
            self.pixels = Some(pixels);
            self.win_pixels = Some(win_p.clone());

            let p_pos = win_p
                .outer_position()
                .unwrap_or(winit::dpi::PhysicalPosition::new(100, 100));
            let sb_pos = winit::dpi::PhysicalPosition::new(p_pos.x + WIDTH as i32 + 16, p_pos.y);

            let attr_sb = Window::default_attributes()
                .with_title("CPU (Softbuffer)")
                .with_inner_size(winit::dpi::LogicalSize::new(WIDTH as f64, HEIGHT as f64))
                .with_position(sb_pos);
            let win_sb = Arc::new(event_loop.create_window(attr_sb).unwrap());
            self.id_softbuffer = Some(win_sb.id());

            let sb_context = SbContext::new(win_sb.clone()).unwrap();
            let mut sb_surface = SbSurface::new(&sb_context, win_sb.clone()).unwrap();
            sb_surface
                .resize(
                    NonZeroU32::new(WIDTH).unwrap(),
                    NonZeroU32::new(HEIGHT).unwrap(),
                )
                .unwrap();

            self.sb_context = Some(sb_context);
            self.sb_surface = Some(sb_surface);
            self.win_softbuffer = Some(win_sb);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // --- GPU ОКНО ---
        if let (Some(pixels), Some(window)) = (&mut self.pixels, &self.win_pixels) {
            Ball::update(&mut self.balls_pixels, WIDTH as f32, HEIGHT as f32);

            let background = Rectangle::at(0, 0)
                .with_size(WIDTH, HEIGHT)
                .with_fill(Rgba::new(30, 30, 30, 255));
            self.canvas_pixels.draw(&background);

            for ball in &self.balls_pixels {
                // ЗАЩИТА ОТ OVERFLOW: Жестко зажимаем координаты отрисовки круга в рамки холста RIL
                let draw_x = ball
                    .x
                    .clamp(BALL_RADIUS as f32, (WIDTH as i32 - BALL_RADIUS) as f32)
                    as u32;
                let draw_y = ball
                    .y
                    .clamp(BALL_RADIUS as f32, (HEIGHT as i32 - BALL_RADIUS) as f32)
                    as u32;

                let circle = Ellipse::circle(draw_x, draw_y, BALL_RADIUS as u32)
                    .with_fill(Rgba::new(255, 255, 255, 255));
                self.canvas_pixels.draw(&circle);
            }

            let pixels_buffer = pixels.frame_mut();
            let pixel_slice: &[Rgba] = &self.canvas_pixels.data;
            let raw_bytes = unsafe {
                std::slice::from_raw_parts(pixel_slice.as_ptr() as *const u8, pixel_slice.len() * 4)
            };
            pixels_buffer.copy_from_slice(raw_bytes);
            pixels.render().unwrap();

            window.request_redraw();
        }

        // --- CPU ОКНО ---
        if let (Some(surface), Some(window)) = (&mut self.sb_surface, &self.win_softbuffer) {
            Ball::update(&mut self.balls_softbuffer, WIDTH as f32, HEIGHT as f32);

            let background = Rectangle::at(0, 0)
                .with_size(WIDTH, HEIGHT)
                .with_fill(Rgb::new(30, 30, 30));
            self.canvas_softbuffer.draw(&background);

            for ball in &self.balls_softbuffer {
                // ЗАЩИТА ОТ OVERFLOW: Аналогично зажимаем координаты для CPU окна
                let draw_x = ball
                    .x
                    .clamp(BALL_RADIUS as f32, (WIDTH as i32 - BALL_RADIUS) as f32)
                    as u32;
                let draw_y = ball
                    .y
                    .clamp(BALL_RADIUS as f32, (HEIGHT as i32 - BALL_RADIUS) as f32)
                    as u32;

                let circle = Ellipse::circle(draw_x, draw_y, BALL_RADIUS as u32)
                    .with_fill(Rgb::new(255, 255, 0));
                self.canvas_softbuffer.draw(&circle);
            }

            let mut buffer = surface.buffer_mut().unwrap();
            let ril_pixels = &self.canvas_softbuffer.data;
            for (i, pixel) in ril_pixels.iter().enumerate() {
                buffer[i] = ((pixel.r as u32) << 16) | ((pixel.g as u32) << 8) | (pixel.b as u32);
            }
            buffer.present().unwrap();

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

            WindowEvent::Moved(new_position) => {
                if Some(window_id) == self.id_pixels {
                    if let Some(win_sb) = &self.win_softbuffer {
                        let target_pos = winit::dpi::PhysicalPosition::new(
                            new_position.x + WIDTH as i32 + 16,
                            new_position.y,
                        );
                        win_sb.set_outer_position(target_pos);
                    }
                } else if Some(window_id) == self.id_softbuffer {
                    if let Some(win_p) = &self.win_pixels {
                        let target_pos = winit::dpi::PhysicalPosition::new(
                            new_position.x - WIDTH as i32 - 16,
                            new_position.y,
                        );
                        win_p.set_outer_position(target_pos);
                    }
                }
            }
            _ => (),
        }
    }
}
fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
