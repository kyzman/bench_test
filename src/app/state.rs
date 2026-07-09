use crate::ball::{Ball, Playfield};
use pixels::Pixels;
use ril::prelude::{Image, Rgb, Rgba};
use softbuffer::{Context as SbContext, Surface as SbSurface};
use std::sync::Arc;
use winit::window::{Window, WindowId};
// Импортируем наш кастомный конвейер из render
use crate::render::CustomRenderPipeline;

pub const START_WIDTH: u32 = 400; // Чистая стартовая ширина игрового поля
pub const START_HEIGHT: u32 = 400; // Чистая стартовая высота игрового поля
pub const BG_COLOR: (u8, u8, u8) = (30, 30, 30);

// Состояние окна, работающего на GPU (Pixels)
pub struct PixelsState<'win> {
    pub window: Option<Arc<Window>>,
    pub id: Option<WindowId>,
    pub pixels: Option<Pixels<'win>>,
    pub canvas: Image<Rgba>,
    pub balls: Vec<Ball>,
    pub playfield: Playfield,
    pub w: u32,
    pub h: u32,
    pub default_color: (u8, u8, u8),

    // НОВОЕ ПОЛЕ: Хранит скомпилированные шейдеры и буферы GPU
    pub custom_pipeline: Option<CustomRenderPipeline>,
}

pub struct SoftbufferState {
    pub window: Option<Arc<Window>>,
    pub id: Option<WindowId>,
    pub context: Option<SbContext<Arc<Window>>>,
    pub surface: Option<SbSurface<Arc<Window>, Arc<Window>>>,
    pub canvas: Image<Rgb>,
    pub balls: Vec<Ball>,
    pub playfield: Playfield,
    pub w: u32,
    pub h: u32,
    pub default_color: (u8, u8, u8),
}

impl<'win> PixelsState<'win> {
    pub fn new(count: usize) -> Self {
        let color = (255, 255, 255);
        let playfield = Playfield {
            x: 0.0,
            y: 0.0,
            w: START_WIDTH as f32,
            h: START_HEIGHT as f32,
        };
        let total_w = START_WIDTH.max(crate::PANEL_MIN_WIDTH);
        let total_h = START_HEIGHT + crate::PANEL_HEIGHT;

        Self {
            window: None,
            id: None,
            pixels: None,
            canvas: Image::new(total_w, total_h, Rgba::new(0, 0, 0, 255)),
            balls: Ball::generate_scene_in_field(count, &playfield, color),
            playfield,
            w: total_w,
            h: total_h,
            default_color: color,
            custom_pipeline: None, // Изначально конвейер пуст, пока Pixels не создан
        }
    }

    pub fn get_window_start_size(&self) -> (u32, u32) {
        (self.w, self.h)
    }
}

impl SoftbufferState {
    pub fn new(count: usize) -> Self {
        let color = (255, 255, 0);

        let playfield = Playfield {
            x: 0.0,
            y: 0.0,
            w: START_WIDTH as f32,
            h: START_HEIGHT as f32,
        };

        // ИСПРАВЛЕНО: Полная ширина холста окна учитывает PANEL_MIN_WIDTH
        let total_w = START_WIDTH.max(crate::PANEL_MIN_WIDTH);
        // Полная высота холста окна учитывает PANEL_HEIGHT
        let total_h = START_HEIGHT + crate::PANEL_HEIGHT;

        Self {
            window: None,
            id: None,
            context: None,
            surface: None,
            canvas: Image::new(total_w, total_h, Rgb::new(0, 0, 0)),
            balls: Ball::generate_scene_in_field(count, &playfield, color),
            playfield,
            w: total_w,
            h: total_h,
            default_color: color,
        }
    }

    pub fn get_window_start_size(&self) -> (u32, u32) {
        (self.w, self.h)
    }
}
