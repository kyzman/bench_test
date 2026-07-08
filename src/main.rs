mod app;
mod ball;
mod render;

use app::App;
use winit::event_loop::EventLoop;

pub const BALL_COUNT: usize = 15;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
