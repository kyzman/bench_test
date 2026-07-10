use crate::{PANEL_HEIGHT, PANEL_MIN_WIDTH};
use rand::RngExt;

pub const MIN_PADDING_PIXELS: f32 = 50.0;
pub const SMALL_BALL_THRESHOLD: f32 = 12.0; // Шары с радиусом меньше этого считаются мелкими
pub const MIN_CLICK_RADIUS: f32 = 14.0; // Минимальный радиус хитбокса, до которого расширяются мелкие шары

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ShapeMarker {
    None,
    Square,
    Dot,
    Cross,
    Rhombus,
    Triangle,
    Star,
}

// НОВАЯ СТРУКТУРА: Описывает изолированную геометрию игрового поля внутри окна
#[derive(Clone, Copy, Debug)]
pub struct Playfield {
    pub x: f32, // Смещение поля по горизонтали от края окна
    pub y: f32, // Смещение поля по вертикали от края окна
    pub w: f32, // Собственная ширина поля
    pub h: f32, // Собственная высота поля
}

#[derive(Clone)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub marker: ShapeMarker,
    pub color: (u8, u8, u8),
    pub radius: f32,
    pub mass: f32,
    pub restitution: f32,
}

impl Ball {
    pub fn new(
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        marker: ShapeMarker,
        color: (u8, u8, u8),
        radius: f32,
        mass: f32,
        restitution: f32,
    ) -> Self {
        Self {
            x,
            y,
            vx,
            vy,
            marker,
            color,
            radius,
            mass,
            restitution,
        }
    }

    // ОБНОВЛЕНО: Проверка клика теперь использует увеличенный виртуальный хитбокс для мелких шаров
    pub fn check_click(&mut self, click_x: f32, click_y: f32, bg_color: (u8, u8, u8)) -> bool {
        let dx = self.x - click_x;
        let dy = self.y - click_y;
        let distance = (dx * dx + dy * dy).sqrt();

        // Определяем эффективный радиус клика: если шар мелкий, расширяем хитбокс до MIN_CLICK_RADIUS
        let effective_radius = if self.radius < SMALL_BALL_THRESHOLD {
            MIN_CLICK_RADIUS
        } else {
            self.radius
        };

        if distance <= effective_radius {
            if self.marker != ShapeMarker::None {
                self.marker = ShapeMarker::None;
            } else {
                let mut rng = rand::rng();
                loop {
                    let r = rng.random_range(0..=255);
                    let g = rng.random_range(0..=255);
                    let b = rng.random_range(0..=255);

                    let dr = r as f32 - bg_color.0 as f32;
                    let dg = g as f32 - bg_color.1 as f32;
                    let db = b as f32 - bg_color.2 as f32;
                    let color_diff = (dr * dr + dg * dg + db * db).sqrt();

                    if color_diff > 80.0 {
                        self.color = (r, b, g);
                        break;
                    }
                }
            }
            true
        } else {
            false
        }
    }

    // ОБНОВЛЕНО: Поиск шара для удаления по ПКМ теперь тоже учитывает увеличенный хитбокс
    pub fn is_point_inside(&self, click_x: f32, click_y: f32) -> bool {
        let dx = self.x - click_x;
        let dy = self.y - click_y;
        let distance = (dx * dx + dy * dy).sqrt();

        let effective_radius = if self.radius < SMALL_BALL_THRESHOLD {
            MIN_CLICK_RADIUS
        } else {
            self.radius
        };

        distance <= effective_radius
    }

    pub fn spawn_at(x: f32, y: f32, default_color: (u8, u8, u8), _duration_ms: f32) -> Self {
        let mut rng = rand::rng();
        let markers = [
            ShapeMarker::Square,
            ShapeMarker::Dot,
            ShapeMarker::Cross,
            ShapeMarker::Rhombus,
            ShapeMarker::Triangle,
            ShapeMarker::Star,
        ];
        let marker = markers[rng.random_range(0..markers.len())];
        let vx = rng.random_range(-4.0..4.0);
        let vy = rng.random_range(-4.0..4.0);
        let radius = rng.random_range(6.0..24.0);
        let mass = (radius * radius) / 100.0;

        Self::new(
            x,
            y,
            if vx == 0.0 { 2.0 } else { vx },
            if vy == 0.0 { 2.0 } else { vy },
            marker,
            default_color,
            radius,
            mass,
            1.0,
        )
    }

    // ИСПРАВЛЕНО: Физика теперь заперта СТРОГО внутри локальных границ Playfield
    pub fn update_physics(balls: &mut [Ball], playfield: &Playfield) {
        let count = balls.len();

        for ball in balls.iter_mut() {
            ball.x += ball.vx;
            ball.y += ball.vy;

            // Шар отбивается от внутренних левой и правой границ поля
            if ball.x - ball.radius <= playfield.x {
                ball.x = playfield.x + ball.radius;
                ball.vx = ball.vx.abs();
            } else if ball.x + ball.radius >= playfield.x + playfield.w {
                ball.x = playfield.x + playfield.w - ball.radius;
                ball.vx = -ball.vx.abs();
            }

            // Шар отбивается от внутренних верхней и нижней границ поля
            if ball.y - ball.radius <= playfield.y {
                ball.y = playfield.y + ball.radius;
                ball.vy = ball.vy.abs();
            } else if ball.y + ball.radius >= playfield.y + playfield.h {
                ball.y = playfield.y + playfield.h - ball.radius;
                ball.vy = -ball.vy.abs();
            }
        }

        // Обсчет коллизий шаров между собой (остается прежним)
        for i in 0..count {
            for j in (i + 1)..count {
                let dx = balls[j].x - balls[i].x;
                let dy = balls[j].y - balls[i].y;
                let distance = (dx * dx + dy * dy).sqrt();
                let min_dist = balls[i].radius + balls[j].radius;
                if distance < min_dist && distance > 0.0 {
                    let nx = dx / distance;
                    let ny = dy / distance;
                    let overlap = min_dist - distance;
                    let total_mass = balls[i].mass + balls[j].mass;
                    let mass_ratio_i = balls[j].mass / total_mass;
                    let mass_ratio_j = balls[i].mass / total_mass;
                    balls[i].x -= nx * overlap * mass_ratio_i;
                    balls[i].y -= ny * overlap * mass_ratio_i;
                    balls[j].x += nx * overlap * mass_ratio_j;
                    balls[j].y += ny * overlap * mass_ratio_j;
                    let kx = balls[i].vx - balls[j].vx;
                    let ky = balls[i].vy - balls[j].vy;
                    let vel_along_normal = kx * nx + ky * ny;
                    if vel_along_normal > 0.0 {
                        let e = balls[i].restitution.min(balls[j].restitution);
                        let impulse_scalar = (1.0 + e) * vel_along_normal
                            / (1.0 / balls[i].mass + 1.0 / balls[j].mass);
                        balls[i].vx -= (impulse_scalar / balls[i].mass) * nx;
                        balls[i].vy -= (impulse_scalar / balls[i].mass) * ny;
                        balls[j].vx += (impulse_scalar / balls[j].mass) * nx;
                        balls[j].vy += (impulse_scalar / balls[j].mass) * ny;
                    }
                }
            }
        }
    }

    // ИСПРАВЛЕНО: Расчет минимального размера Окна складывается из потребностей Поля + интерфейса
    pub fn calculate_min_window_size(balls: &[Ball]) -> (u32, u32) {
        let mut total_occupied_area = 0.0;
        for ball in balls {
            let diameter = ball.radius * 2.0;
            total_occupied_area += diameter * diameter;
        }

        let min_field_side = total_occupied_area.sqrt() + MIN_PADDING_PIXELS;

        // Минимальное окно по ширине должно вместить поле шаров (но не меньше PANEL_MIN_WIDTH)
        let final_width = (min_field_side.round() as u32)
            .max(PANEL_MIN_WIDTH)
            .max(150);

        // Минимальное окно по высоте должно вместить поле шаров + нижнюю интерфейсную панель
        let final_height = (min_field_side.round() as u32).max(150) + PANEL_HEIGHT;

        (final_width, final_height)
    }

    // Вспомогательный метод генерации начальной сцены внутри стартовых границ поля
    pub fn generate_scene_in_field(
        count: usize,
        playfield: &Playfield,
        default_color: (u8, u8, u8),
    ) -> Vec<Ball> {
        let mut balls = Vec::with_capacity(count);
        let mut rng = rand::rng();
        let markers = [
            ShapeMarker::Square,
            ShapeMarker::Dot,
            ShapeMarker::Cross,
            ShapeMarker::Rhombus,
            ShapeMarker::Triangle,
            ShapeMarker::Star,
        ];

        for i in 0..count {
            let mut attempts = 0;
            let marker = markers[i % markers.len()];
            let radius = 10.0;

            loop {
                // Спавним строго внутри Playfield координат
                let x =
                    rng.random_range((playfield.x + radius)..(playfield.x + playfield.w - radius));
                let y =
                    rng.random_range((playfield.y + radius)..(playfield.y + playfield.h - radius));

                let is_overlapping = balls.iter().any(|b: &Ball| {
                    let dx = b.x - x;
                    let dy = b.y - y;
                    (dx * dx + dy * dy).sqrt() < (b.radius + radius)
                });

                if !is_overlapping || attempts > 200 {
                    let vx = rng.random_range(-4.0..4.0);
                    let vy = rng.random_range(-4.0..4.0);
                    balls.push(Ball::new(
                        x,
                        y,
                        vx,
                        vy,
                        marker,
                        default_color,
                        radius,
                        1.0,
                        1.0,
                    ));
                    break;
                }
                attempts += 1;
            }
        }
        balls
    }
}
