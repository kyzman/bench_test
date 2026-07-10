use crate::ball::Ball;
use winit::window::Window;

// Команды, которые главный поток будет слать в фоновые потоки рендеринга
#[derive(Debug, Clone, Copy)]
pub enum ThreadCommand {
    Resize { w: u32, h: u32 },
    // НОВЫЕ СИГНАЛЫ: передают сырые системные события кнопок мыши и координаты
    MousePressed { x: f32, y: f32, is_left: bool },
    MouseReleased { is_left: bool },
    MouseMove { x: f32, y: f32 },
}

// Статичная функция обновления минимальных размеров (вызывается на основе дефолтных или переданных расчетов)
pub fn set_window_min_size(window: &Window, ball_count: usize) {
    // Временный вектор-пустышка для расчета лимитов по площади
    let dummy_balls = vec![Ball::spawn_at(0.0, 0.0, (0, 0, 0), 0.0); ball_count];
    let (min_w, min_h) = Ball::calculate_min_window_size(&dummy_balls);
    let logical_size = winit::dpi::LogicalSize::new(min_w as f64, min_h as f64);
    window.set_min_inner_size(Some(logical_size));
}

// Функция синхронизации положения окон при изменении размера (как раньше, работает в главном потоке)
pub fn sync_positions_on_resize(win_p: &Window, win_sb: &Window) {
    if let (Ok(p_pos), Ok(sb_pos)) = (win_p.outer_position(), win_sb.outer_position()) {
        let p_outer_size = win_p.outer_size();
        let expected_sb_x = p_pos.x + p_outer_size.width as i32;
        if sb_pos.x != expected_sb_x || sb_pos.y != p_pos.y {
            win_sb.set_outer_position(winit::dpi::PhysicalPosition::new(expected_sb_x, p_pos.y));
        }
    }
}

// Функция мягкой сцепки окон в реальном времени при перемещении
pub fn handle_window_moved(
    window_id: winit::window::WindowId,
    new_position: winit::dpi::PhysicalPosition<i32>,
    id_p: Option<winit::window::WindowId>,
    id_sb: Option<winit::window::WindowId>,
    win_p: Option<&Window>,
    win_sb: Option<&Window>,
) {
    if Some(window_id) == id_p {
        if let (Some(wp), Some(wsb)) = (win_p, win_sb) {
            let target_pos = winit::dpi::PhysicalPosition::new(
                new_position.x + wp.outer_size().width as i32,
                new_position.y,
            );
            wsb.set_outer_position(target_pos);
        }
    } else if Some(window_id) == id_sb {
        if let (Some(wp), Some(wsb)) = (win_p, win_sb) {
            let target_pos = winit::dpi::PhysicalPosition::new(
                new_position.x - wp.outer_size().width as i32,
                new_position.y,
            );
            wp.set_outer_position(target_pos);
        }
    }
}
