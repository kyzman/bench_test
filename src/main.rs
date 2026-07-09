mod app;
mod ball;
mod render;

use app::App;
use winit::event_loop::EventLoop;

pub const BALL_COUNT: usize = 15;

// НОВЫЕ КОНСТАНТЫ ДЛЯ НИЖНЕЙ ПАНЕЛИ
pub const PANEL_HEIGHT: u32 = 24; // Высота технической зоны в пикселях
pub const PANEL_MIN_WIDTH: u32 = 100; // Минимальная длина этой зоны (и всего окна)

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
