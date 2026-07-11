use crate::ball::{Ball, Playfield};
use crate::render::CustomRenderPipeline;
use pixels::Pixels;
use ril::prelude::{Image, Rgb, Rgba};
use softbuffer::Surface as SbSurface;
use std::sync::Arc;
use winit::window::{Window, WindowId};

pub const START_WIDTH: u32 = 400;
pub const START_HEIGHT: u32 = 400;
pub const BG_COLOR: (u8, u8, u8) = (50, 30, 30);

// Структура данных, которая уйдет жить в независимый GPU-поток
pub struct GpuThreadContext<'win> {
    pub pixels: Pixels<'win>,
    pub custom_pipeline: CustomRenderPipeline,
    pub balls: Vec<Ball>,
    pub playfield: Playfield,
    pub canvas: Image<Rgba>,
    pub w: u32,
    pub h: u32,
    pub default_color: (u8, u8, u8),
}

// Структура данных, которая уйдет жить в независимый CPU-поток
pub struct CpuThreadContext {
    pub surface: SbSurface<Arc<Window>, Arc<Window>>,
    pub balls: Vec<Ball>,
    pub playfield: Playfield,
    pub canvas: Image<Rgb>,
    pub w: u32,
    pub h: u32,
    pub default_color: (u8, u8, u8),
}

pub struct PixelsState {
    pub window: Option<Arc<Window>>,
    pub id: Option<WindowId>,
}

pub struct SoftbufferState {
    pub window: Option<Arc<Window>>,
    pub id: Option<WindowId>,
}

impl PixelsState {
    pub fn new() -> Self {
        Self {
            window: None,
            id: None,
        }
    }
}

impl SoftbufferState {
    pub fn new() -> Self {
        Self {
            window: None,
            id: None,
        }
    }
}
