struct Ball {
    x: f32,
    y: f32,
    radius: f32,
    marker: f32, // <-- Принимаем ID маркера
    r: f32,
    g: f32,
    b: f32,
}

@group(0) @binding(0) var<storage, read> balls: array<Ball>;

struct Globals {
    ball_count: u32,
    screen_width: f32,
    screen_height: f32,
    panel_height: f32,
}
@group(0) @binding(1) var<uniform> globals: Globals;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0, -1.0)
    );
    out.position = vec4<f32>(positions[in_vertex_index], 0.0, 1.0);
    out.uv = positions[in_vertex_index] * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_coords = vec2<f32>(
        in.uv.x * globals.screen_width,
        in.uv.y * globals.screen_height
    );

    // 1. Отрисовка нижней технической панели
    if (pixel_coords.y >= globals.screen_height - globals.panel_height) {
        // Переводим цвет панели (15, 15, 15) в sRGB для выравнивания гаммы
        return vec4<f32>(pow(vec3<f32>(0.058, 0.058, 0.058), vec3<f32>(2.2)), 1.0);
    }

    // Линейный дефолтный фон поля (40, 40, 40)
    var final_color = vec3<f32>(0.156, 0.156, 0.156);

    // 2. Перебор шаров на GPU
    for (var i: u32 = 0u; i < globals.ball_count; i = i + 1u) {
        let ball = balls[i];
        let dx = ball.x - pixel_coords.x;
        let dy = ball.y - pixel_coords.y;
        let distance_sq = dx * dx + dy * dy;

        if (distance_sq <= ball.radius * ball.radius) {
            // Исходный цвет шара
            let ball_base_color = vec3<f32>(ball.r, ball.g, ball.b);
            final_color = ball_base_color;

            // Вычисляем контрастный цвет для маркера
            let brightness = (ball.r * 0.299) + (ball.g * 0.587) + (ball.b * 0.114);
            var marker_color = vec3<f32>(1.0, 1.0, 1.0);
            if (brightness > 0.5) { marker_color = vec3<f32>(0.0, 0.0, 0.0); }

            // Локальные абсолютные координаты пикселя относительно центра шара
            let adx = abs(dx);
            let ady = abs(dy);
            let marker_id = u32(ball.marker);

            // ИСПРАВЛЕНО: Рисуем кастомную геометрию на GPU в зависимости от типа маркера
            if (marker_id == 1u) { // Square (Квадрат 6х6)
                if (adx <= 3.0 && ady <= 3.0) { final_color = marker_color; }
            } else if (marker_id == 2u) { // Dot (Точка 2х2)
                if (adx <= 1.0 && ady <= 1.0) { final_color = marker_color; }
            } else if (marker_id == 3u) { // Cross (Крестик)
                if ((adx <= 4.0 && ady <= 1.0) || (adx <= 1.0 && ady <= 4.0)) { final_color = marker_color; }
            } else if (marker_id == 4u) { // Rhombus (Ромб 4х4)
                if (adx <= 2.0 && ady <= 2.0) { final_color = marker_color; }
            } else if (marker_id == 5u) { // Triangle (Треугольник)
                if (dy >= -2.0 && dy <= 2.0 && adx <= (2.0 - dy * 0.5)) { final_color = marker_color; }
            } else if (marker_id == 6u) { // Star (Звёздочка)
                if ((adx <= 2.0 && ady <= 2.0) || (adx <= 4.0 && ady <= 1.0) || (adx <= 1.0 && ady <= 4.0)) { final_color = marker_color; }
            }
            break;
        }
    }

    // ИСПРАВЛЕНО: Применяем обратную гамма-коррекцию перед выводом на экран ( sRGB Фикс )
    // Из-за этого цвета в окне pixels станут такими же насыщенными и темными, как в softbuffer.
    let gamma_corrected = pow(final_color, vec3<f32>(1.0 / 2.2));
    return vec4<f32>(gamma_corrected, 1.0);
}
