mod app;
mod ball;
mod render;

use app::App;
use winit::event_loop::EventLoop;

// Глобальные константы конфигурации приложения
pub const WIDTH: u32 = 400;
pub const HEIGHT: u32 = 400;
pub const BALL_COUNT: usize = 60; // Легко менять количество шаров для бенчмарка

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
