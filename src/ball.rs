use crate::app::state::{START_HEIGHT, START_WIDTH};
use crate::{PANEL_HEIGHT, PANEL_MIN_WIDTH};
use rand::RngExt;
pub const MIN_PADDING_PIXELS: f32 = 50.0; // Дополнительное пространство, которое требуется для разлёта шариков(для минимального размера окна)
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

    pub fn calculate_min_window_size(balls: &[Ball], current_win_w: u32) -> (u32, u32) {
        if balls.is_empty() {
            return (
                START_WIDTH.max(PANEL_MIN_WIDTH),
                START_HEIGHT + PANEL_HEIGHT,
            );
        }

        // 1. Создаем список диаметров (сторон описанных квадратов) всех текущих шаров
        let mut sizes: Vec<f32> = balls.iter().map(|b| b.radius * 2.0).collect();

        // Сортируем от больших к маленьким для максимально плотной и предсказуемой укладки
        sizes.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        // Находим диаметр самого большого шара — окно физически не может быть меньше него
        let max_diameter = sizes[0];

        // 2. Рассчитываем эффективную доступную ширину для укладки стопки шаров
        // Вычитаем обязательный внутренний отступ
        let effective_win_w = (current_win_w as f32 - MIN_PADDING_PIXELS).max(max_diameter);

        // Переменные симулятора укладки рядов
        let mut current_row_x = 0.0f32;
        let mut current_row_y = 0.0f32;
        let mut current_row_max_h = 0.0f32;
        let mut total_packed_height = 0.0f32;

        // Идем по каждому квадрату шара и укладываем в ряды
        for size in sizes {
            // Если шар не помещается в текущую строку по ширине — переносим ряд
            if current_row_x + size > effective_win_w {
                current_row_y += current_row_max_h; // Сдвигаем Y вниз на высоту прошлого ряда
                current_row_x = 0.0; // Начинаем с левого края
                current_row_max_h = 0.0; // Сбрасываем высоту текущего ряда
            }

            // Ставим шар в текущий ряд
            current_row_x += size;
            if size > current_row_max_h {
                current_row_max_h = size; // Запоминаем самый высокий элемент в ряду
            }

            // Обновляем общую высоту всей получившейся стопки
            let current_total_h = current_row_y + current_row_max_h;
            if current_total_h > total_packed_height {
                total_packed_height = current_total_h;
            }
        }

        // 3. Формируем финальные лимиты окна
        // Минимальная ширина: должна вмещать как минимум один самый большой шар + отступы
        let final_min_w = (max_diameter + MIN_PADDING_PIXELS).round() as u32;

        // Минимальная высота: высота получившейся виртуальной стопки + отступы + высота нижней панели
        let final_min_h = (total_packed_height + MIN_PADDING_PIXELS).round() as u32 + PANEL_HEIGHT;

        // ИСПРАВЛЕНО: Убрали жесткую привязку к START_HEIGHT!
        // Теперь минимальная высота рассчитывается ЧЕСТНО на основе уложенной стопки,
        // но не падает ниже 150 пикселей (защита от схлопывания, если шаров мало или их нет).
        let final_width = final_min_w.max(START_WIDTH).max(PANEL_MIN_WIDTH).max(150);
        let final_height = final_min_h.max(150); // <-- ЗАМЕНИЛИ СТАРЫЙ START_HEIGHT НА 150

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
