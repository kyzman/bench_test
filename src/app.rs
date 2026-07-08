use pixels::{Pixels, SurfaceTexture};
use ril::prelude::*;
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::num::NonZeroU32;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::ball::Ball;
use crate::render::{draw_pixels_frame, draw_softbuffer_frame};
use crate::{BALL_COUNT, HEIGHT, WIDTH};

pub struct App<'win> {
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
        Self {
            win_pixels: None,
            id_pixels: None,
            pixels: None,
            canvas_pixels: Image::new(WIDTH, HEIGHT, Rgba::new(0, 0, 0, 255)),
            balls_pixels: Ball::generate_scene(BALL_COUNT, WIDTH as f32, HEIGHT as f32),

            win_softbuffer: None,
            id_softbuffer: None,
            sb_context: None,
            sb_surface: None,
            canvas_softbuffer: Image::new(WIDTH, HEIGHT, Rgb::new(0, 0, 0)),
            balls_softbuffer: Ball::generate_scene(BALL_COUNT, WIDTH as f32, HEIGHT as f32),
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
        // Логика и отрисовка GPU окна
        if let (Some(pixels), Some(window)) = (&mut self.pixels, &self.win_pixels) {
            Ball::update_physics(&mut self.balls_pixels, WIDTH as f32, HEIGHT as f32);
            draw_pixels_frame(
                &mut self.canvas_pixels,
                pixels,
                &self.balls_pixels,
                WIDTH,
                HEIGHT,
            );
            window.request_redraw();
        }

        // Логика и отрисовка CPU окна
        if let (Some(surface), Some(window)) = (&mut self.sb_surface, &self.win_softbuffer) {
            Ball::update_physics(&mut self.balls_softbuffer, WIDTH as f32, HEIGHT as f32);
            draw_softbuffer_frame(
                &mut self.canvas_softbuffer,
                surface,
                &self.balls_softbuffer,
                WIDTH,
                HEIGHT,
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
            WindowEvent::Moved(new_position) => {
                if Some(window_id) == self.id_pixels {
                    if let Some(win_sb) = &self.win_softbuffer {
                        win_sb.set_outer_position(winit::dpi::PhysicalPosition::new(
                            new_position.x + WIDTH as i32 + 16,
                            new_position.y,
                        ));
                    }
                } else if Some(window_id) == self.id_softbuffer {
                    if let Some(win_p) = &self.win_pixels {
                        win_p.set_outer_position(winit::dpi::PhysicalPosition::new(
                            new_position.x - WIDTH as i32 - 16,
                            new_position.y,
                        ));
                    }
                }
            }
            _ => (),
        }
    }
}
