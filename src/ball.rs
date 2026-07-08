use rand::RngExt;

pub const BALL_RADIUS: f32 = 8.0;

#[derive(Clone, Copy)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

impl Ball {
    pub fn new(x: f32, y: f32, vx: f32, vy: f32) -> Self {
        Self { x, y, vx, vy }
    }

    pub fn update_physics(balls: &mut [Ball], width: f32, height: f32) {
        let count = balls.len();

        // 1. Движение и отскок от стен
        for ball in balls.iter_mut() {
            ball.x += ball.vx;
            ball.y += ball.vy;

            if ball.x - BALL_RADIUS <= 0.0 {
                ball.x = BALL_RADIUS;
                ball.vx = ball.vx.abs();
            } else if ball.x + BALL_RADIUS >= width {
                ball.x = width - BALL_RADIUS;
                ball.vx = -ball.vx.abs();
            }

            if ball.y - BALL_RADIUS <= 0.0 {
                ball.y = BALL_RADIUS;
                ball.vy = ball.vy.abs();
            } else if ball.y + BALL_RADIUS >= height {
                ball.y = height - BALL_RADIUS;
                ball.vy = -ball.vy.abs();
            }
        }

        // 2. Отскок шаров друг от друга (Импульсы)
        for i in 0..count {
            for j in (i + 1)..count {
                let b1 = balls[i];
                let b2 = balls[j];

                let dx = b2.x - b1.x;
                let dy = b2.y - b1.y;
                let distance = (dx * dx + dy * dy).sqrt();
                let min_dist = BALL_RADIUS * 2.0;

                if distance < min_dist && distance > 0.0 {
                    // Коррекция наложения
                    let overlap = min_dist - distance;
                    let nx = dx / distance;
                    let ny = dy / distance;

                    balls[i].x -= nx * overlap * 0.5;
                    balls[i].y -= ny * overlap * 0.5;
                    balls[j].x += nx * overlap * 0.5;
                    balls[j].y += ny * overlap * 0.5;

                    // Расчет скоростей
                    let kx = balls[i].vx - balls[j].vx;
                    let ky = balls[i].vy - balls[j].vy;
                    let p = nx * kx + ny * ky;

                    if p > 0.0 {
                        balls[i].vx -= p * nx;
                        balls[i].vy -= p * ny;
                        balls[j].vx += p * nx;
                        balls[j].vy += p * ny;
                    }
                }
            }
        }
    }

    pub fn generate_scene(count: usize, width: f32, height: f32) -> Vec<Ball> {
        let mut balls = Vec::with_capacity(count);
        let mut rng = rand::rng();

        for _ in 0..count {
            let mut attempts = 0;
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
                    balls.push(Ball::new(
                        x,
                        y,
                        if vx == 0.0 { 2.0 } else { vx },
                        if vy == 0.0 { 2.0 } else { vy },
                    ));
                    break;
                }
                attempts += 1;
            }
        }
        balls
    }
}
