use rand::RngExt;

pub const MIN_PADDING_PIXELS: f32 = 50.0; // Константа свободного пространства в пикселях

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

#[derive(Clone)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub marker: ShapeMarker,
    pub color: (u8, u8, u8),
    // НОВЫЕ ИНДИВИДУАЛЬНЫЕ ХАРАКТЕРИСТИКИ
    pub radius: f32,      // Индивидуальный размер шара
    pub mass: f32,        // Масса (инерция) — чем больше, тем сложнее сдвинуть шар
    pub restitution: f32, // Упругость (от 0.0 до 1.0) — сохранение энергии при ударе
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

    pub fn check_click(&mut self, click_x: f32, click_y: f32, bg_color: (u8, u8, u8)) -> bool {
        let dx = self.x - click_x;
        let dy = self.y - click_y;
        let distance = (dx * dx + dy * dy).sqrt();

        // Проверка клика теперь опирается на индивидуальный радиус шара
        if distance <= self.radius {
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

    pub fn is_point_inside(&self, click_x: f32, click_y: f32) -> bool {
        let dx = self.x - click_x;
        let dy = self.y - click_y;
        let distance = (dx * dx + dy * dy).sqrt();
        distance <= self.radius // Опираемся на индивидуальный радиус
    }

    // ОБНОВЛЕНО: Спавн шара со случайным радиусом и массой, пропорциональной его площади.
    // Параметр duration_ms заложен на будущее для привязки к длительности зажатия кнопки.
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

        // Генерируем случайный радиус от 6.0 до 24.0 пикселей
        let radius = rng.random_range(6.0..24.0);

        // Масса жестко зависит от площади круга (пропорционально квадрату радиуса).
        // Делим на 100.0, чтобы базовый шар с радиусом 10.0 имел массу ровно 1.0.
        let mass = (radius * radius) / 100.0;

        let restitution = 1.0; // Идеальная упругость

        Self::new(
            x,
            y,
            if vx == 0.0 { 2.0 } else { vx },
            if vy == 0.0 { 2.0 } else { vy },
            marker,
            default_color,
            radius,
            mass,
            restitution,
        )
    }

    pub fn update_physics(balls: &mut [Ball], width: f32, height: f32) {
        let count = balls.len();

        // 1. Движение и отскок от динамических стен с учетом индивидуального радиуса
        for ball in balls.iter_mut() {
            ball.x += ball.vx;
            ball.y += ball.vy;

            if ball.x - ball.radius <= 0.0 {
                ball.x = ball.radius;
                ball.vx = ball.vx.abs();
            } else if ball.x + ball.radius >= width {
                ball.x = width - ball.radius;
                ball.vx = -ball.vx.abs();
            }

            if ball.y - ball.radius <= 0.0 {
                ball.y = ball.radius;
                ball.vy = ball.vy.abs();
            } else if ball.y + ball.radius >= height {
                ball.y = height - ball.radius;
                ball.vy = -ball.vy.abs();
            }
        }

        // 2. Сложный физический расчет столкновений с учетом ИНЕРЦИИ (массы) и УПРУГОСТИ
        for i in 0..count {
            for j in (i + 1)..count {
                let dx = balls[j].x - balls[i].x;
                let dy = balls[j].y - balls[i].y;
                let distance = (dx * dx + dy * dy).sqrt();

                // Сумма индивидуальных радиусов двух сталкивающихся шаров
                let min_dist = balls[i].radius + balls[j].radius;

                if distance < min_dist && distance > 0.0 {
                    let nx = dx / distance;
                    let ny = dy / distance;

                    // Расталкивание шаров пропорционально их массам
                    // (более легкий шар сдвигается сильнее, чем тяжелый)
                    let overlap = min_dist - distance;
                    let total_mass = balls[i].mass + balls[j].mass;

                    // Доля сдвига для каждого шара обратно пропорциональна массе
                    let mass_ratio_i = balls[j].mass / total_mass;
                    let mass_ratio_j = balls[i].mass / total_mass;

                    balls[i].x -= nx * overlap * mass_ratio_i;
                    balls[i].y -= ny * overlap * mass_ratio_i;
                    balls[j].x += nx * overlap * mass_ratio_j;
                    balls[j].y += ny * overlap * mass_ratio_j;

                    // Проекция относительной скорости на нормаль столкновения
                    let kx = balls[i].vx - balls[j].vx;
                    let ky = balls[i].vy - balls[j].vy;
                    let vel_along_normal = kx * nx + ky * ny;

                    if vel_along_normal > 0.0 {
                        // Выбираем наименьшую упругость из двух шаров (или среднее арифметическое)
                        let e = balls[i].restitution.min(balls[j].restitution);

                        // Формула закона сохранения импульса для упругого удара
                        let impulse_scalar = (1.0 + e) * vel_along_normal
                            / (1.0 / balls[i].mass + 1.0 / balls[j].mass);

                        // Изменение скоростей жестко зависит от массы шара (тяжелый шар почти не реагирует)
                        balls[i].vx -= (impulse_scalar / balls[i].mass) * nx;
                        balls[i].vy -= (impulse_scalar / balls[i].mass) * ny;
                        balls[j].vx += (impulse_scalar / balls[j].mass) * nx;
                        balls[j].vy += (impulse_scalar / balls[j].mass) * ny;
                    }
                }
            }
        }
    }

    pub fn generate_scene(
        count: usize,
        width: f32,
        height: f32,
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

            // Базовые характеристики по умолчанию (сохраняем старое поведение)
            let radius = 10.0;
            let mass = 1.0;
            let restitution = 1.0;

            loop {
                let x = rng.random_range(radius..(width - radius));
                let y = rng.random_range(radius..(height - radius));

                let is_overlapping = balls.iter().any(|b: &Ball| {
                    let dx = b.x - x;
                    let dy = b.y - y;
                    let min_dist = b.radius + radius;
                    (dx * dx + dy * dy).sqrt() < min_dist
                });

                if !is_overlapping || attempts > 200 {
                    let vx = rng.random_range(-4.0..4.0);
                    let vy = rng.random_range(-4.0..4.0);
                    balls.push(Ball::new(
                        x,
                        y,
                        if vx == 0.0 { 2.0 } else { vx },
                        if vy == 0.0 { 2.0 } else { vy },
                        marker,
                        default_color,
                        radius,
                        mass,
                        restitution,
                    ));
                    break;
                }
                attempts += 1;
            }
        }
        balls
    }
    // НОВОЕ: Вычисление минимально допустимого размера стороны окна на основе площадей шаров
    pub fn calculate_min_window_size(balls: &[Ball]) -> u32 {
        let mut total_occupied_area = 0.0;

        for ball in balls {
            // Эффективная площадь, которую шар занимает на плоскости (площадь описанного квадрата)
            let diameter = ball.radius * 2.0;
            total_occupied_area += diameter * diameter;
        }

        // Извлекаем корень, чтобы получить сторону эквивалентного большого квадрата
        let min_side = total_occupied_area.sqrt();

        // Прибавляем константу свободного пространства
        let final_size = min_side + MIN_PADDING_PIXELS;

        // Возвращаем размер, но не меньше базового разумного порога (например, 150 пикселей),
        // чтобы окно не схлопнулось до нуля, если удалить все шары.
        (final_size.round() as u32).max(150)
    }
}

// РАДИУСО-НЕЗАВИСИМЫЕ ТЕСТЫ ФИЗИКИ
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heavy_and_light_ball_collision() {
        let width = 400.0;
        let height = 400.0;

        // Создаем тяжелый шар (масса 10.0, радиус 20.0) движущийся вправо
        let mut b1 = Ball::new(
            100.0,
            200.0,
            2.0,
            0.0,
            ShapeMarker::None,
            (0, 0, 0),
            20.0,
            10.0,
            1.0,
        );
        // Легкий шар (масса 1.0, радиус 10.0) движущийся навстречу влево
        let mut b2 = Ball::new(
            130.0,
            200.0,
            -2.0,
            0.0,
            ShapeMarker::None,
            (0, 0, 0),
            10.0,
            1.0,
            1.0,
        );

        let mut balls = vec![b1, b2];
        Ball::update_physics(&mut balls, width, height);

        // Тяжелый шар (индекс 0) из-за огромной инерции должен продолжить лететь вправо,
        // лишь слегка замедлившись, а легкий шар (индекс 1) должен резко отскочить вправо.
        assert!(
            balls[0].vx > 0.0,
            "Тяжелый шар не должен был изменить знак скорости!"
        );
        assert!(
            balls[1].vx > 2.0,
            "Легкий шар должен был отлететь с высокой скоростью!"
        );
    }
}
