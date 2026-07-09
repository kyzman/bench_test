use crate::ball::Ball;
use pixels::Pixels;
use ril::prelude::{Image, Rgb, Rgba};
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::sync::Arc;
use winit::window::{Window, WindowId};

pub const START_WIDTH: u32 = 400;
pub const START_HEIGHT: u32 = 400;
pub const BG_COLOR: (u8, u8, u8) = (30, 30, 30);

// Состояние окна, работающего на GPU (Pixels)
pub struct PixelsState<'win> {
    pub window: Option<Arc<Window>>,
    pub id: Option<WindowId>,
    pub pixels: Option<Pixels<'win>>,
    pub canvas: Image<Rgba>,
    pub balls: Vec<Ball>,
    pub w: u32,
    pub h: u32,
    pub default_color: (u8, u8, u8),
}

// Состояние окна, работающего на CPU (Softbuffer)
pub struct SoftbufferState {
    pub window: Option<Arc<Window>>,
    pub id: Option<WindowId>,
    pub context: Option<SbContext<Arc<Window>>>,
    pub surface: Option<SbSurface<Arc<Window>, Arc<Window>>>,
    pub canvas: Image<Rgb>,
    pub balls: Vec<Ball>,
    pub w: u32,
    pub h: u32,
    pub default_color: (u8, u8, u8),
}

impl<'win> PixelsState<'win> {
    pub fn new(count: usize) -> Self {
        let color = (255, 255, 255); // Белый
        Self {
            window: None,
            id: None,
            pixels: None,
            canvas: Image::new(START_WIDTH, START_HEIGHT, Rgba::new(0, 0, 0, 255)),
            balls: Ball::generate_scene(count, START_WIDTH as f32, START_HEIGHT as f32, color),
            w: START_WIDTH,
            h: START_HEIGHT,
            default_color: color,
        }
    }
}

impl SoftbufferState {
    pub fn new(count: usize) -> Self {
        let color = (255, 255, 0); // Желтый
        Self {
            window: None,
            id: None,
            context: None,
            surface: None,
            canvas: Image::new(START_WIDTH, START_HEIGHT, Rgb::new(0, 0, 0)),
            balls: Ball::generate_scene(count, START_WIDTH as f32, START_HEIGHT as f32, color),
            w: START_WIDTH,
            h: START_HEIGHT,
            default_color: color,
        }
    }
}
