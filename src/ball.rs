use rand::RngExt; // ИСПРАВЛЕНО И ЗАФИКСИРОВАНО!

pub const BALL_RADIUS: f32 = 10.0;

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
}

impl Ball {
    pub fn new(x: f32, y: f32, vx: f32, vy: f32, marker: ShapeMarker, color: (u8, u8, u8)) -> Self {
        Self {
            x,
            y,
            vx,
            vy,
            marker,
            color,
        }
    }

    pub fn check_click(&mut self, click_x: f32, click_y: f32, bg_color: (u8, u8, u8)) -> bool {
        let dx = self.x - click_x;
        let dy = self.y - click_y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance <= BALL_RADIUS {
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

    pub fn update_physics(balls: &mut [Ball], width: f32, height: f32) {
        let count = balls.len();

        for ball in balls.iter_mut() {
            ball.x += ball.vx;
            ball.y += ball.vy;

            // Отскок от динамической левой/правой стены
            if ball.x - BALL_RADIUS <= 0.0 {
                ball.x = BALL_RADIUS;
                ball.vx = ball.vx.abs();
            } else if ball.x + BALL_RADIUS >= width {
                ball.x = width - BALL_RADIUS;
                ball.vx = -ball.vx.abs();
            }

            // Отскок от динамической верхней/нижней стены
            if ball.y - BALL_RADIUS <= 0.0 {
                ball.y = BALL_RADIUS;
                ball.vy = ball.vy.abs();
            } else if ball.y + BALL_RADIUS >= height {
                ball.y = height - BALL_RADIUS;
                ball.vy = -ball.vy.abs();
            }
        }

        for i in 0..count {
            for j in (i + 1)..count {
                let dx = balls[j].x - balls[i].x;
                let dy = balls[j].y - balls[i].y;
                let distance = (dx * dx + dy * dy).sqrt();
                let min_dist = BALL_RADIUS * 2.0;

                if distance < min_dist && distance > 0.0 {
                    let nx = dx / distance;
                    let ny = dy / distance;

                    let overlap = min_dist - distance;
                    balls[i].x -= nx * overlap * 0.5;
                    balls[i].y -= ny * overlap * 0.5;
                    balls[j].x += nx * overlap * 0.5;
                    balls[j].y += ny * overlap * 0.5;

                    let kx = balls[i].vx - balls[j].vx;
                    let ky = balls[i].vy - balls[j].vy;
                    let vel_along_normal = kx * nx + ky * ny;

                    if vel_along_normal > 0.0 {
                        let impulse_scalar = 2.0 * vel_along_normal / 2.0;
                        balls[i].vx -= impulse_scalar * nx;
                        balls[i].vy -= impulse_scalar * ny;
                        balls[j].vx += impulse_scalar * nx;
                        balls[j].vy += impulse_scalar * ny;
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

            loop {
                let x = rng.random_range(BALL_RADIUS..(width - BALL_RADIUS));
                let y = rng.random_range(BALL_RADIUS..(height - BALL_RADIUS));

                let is_overlapping = balls.iter().any(|b: &Ball| {
                    let dx = b.x - x;
                    let dy = b.y - y;
                    (dx * dx + dy * dy).sqrt() < BALL_RADIUS * 2.0
                });

                if !is_overlapping || attempts > 200 {
                    let vx = rng.random_range(-4.0..4.0);
                    let vy = rng.random_range(-4.0..4.0);
                    balls.push(Ball::new(x, y, vx, vy, marker, default_color));
                    break;
                }
                attempts += 1;
            }
        }
        balls
    }
}
