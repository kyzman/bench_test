struct Ball {
    x: f32,
    y: f32,
    radius: f32,
    marker: f32,
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
    text_data: array<vec4<u32>, 4>,
}
@group(0) @binding(1) var<uniform> globals: Globals;

// Новые слоты для текстуры атласа шрифта
@group(0) @binding(2) var font_texture: texture_2d<f32>;
@group(0) @binding(3) var font_sampler: sampler;

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

// Вспомогательная функция: считывает цвет пикселя конкретного символа из текстуры атласа
fn get_char_pixel(char_code: u32, gx: u32, gy: u32) -> f32 {
    let start_ascii = 32u;
    let end_ascii = 126u;
    let char_w = 8u;
    let char_h = 12u;

    // ПРОВЕРКА НАЛИЧИЯ СИМВОЛА В ЮНИКОДЕ
    if (char_code < start_ascii || char_code > end_ascii) {
        // СИМВОЛА НЕТ — СУПЕР-ЗАГЛУШКА: Смешиваем (накладываем) знак '!' и знак '?'
        // Индекс для '!' это 33 - 32 = 1u
        // Индекс для '?' это 63 - 32 = 31u
        let tex_x_excl = f32(1u * char_w + gx) + 0.5;
        let tex_x_quest = f32(31u * char_w + gx) + 0.5;
        let tex_y = f32(gy) + 0.5;

        let size = textureDimensions(font_texture);
        let uv_excl = vec2<f32>(tex_x_excl / f32(size.x), tex_y / f32(size.y));
        let uv_quest = vec2<f32>(tex_x_quest / f32(size.x), tex_y / f32(size.y));

        let pix_excl = textureSampleLevel(font_texture, font_sampler, uv_excl, 0.0).r;
        let pix_quest = textureSampleLevel(font_texture, font_sampler, uv_quest, 0.0).r;

        // Если горит хотя бы один из пикселей наложения — возвращаем активный бит
        if (pix_excl > 0.5 || pix_quest > 0.5) {
            return 1.0;
        }
        return 0.0;
    }

    // Если символ есть в атласе — вычисляем его честную координату по сетке 8x12
    let char_index = char_code - start_ascii;
    let tex_pixel_x = f32(char_index * char_w + gx) + 0.5;
    let tex_pixel_y = f32(gy) + 0.5;

    let atlas_size = textureDimensions(font_texture);
    let final_uv = vec2<f32>(tex_pixel_x / f32(atlas_size.x), tex_pixel_y / f32(atlas_size.y));

    // Сэмплируем пиксель из текстуры атласа
    return textureSampleLevel(font_texture, font_sampler, final_uv, 0.0).r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_coords = vec2<f32>(
        in.uv.x * globals.screen_width,
        in.uv.y * globals.screen_height
    );

    // 1. ОТРИСОВКА НИЖНЕЙ ТЕХНИЧЕСКОЙ ПАНЕЛИ + КРУПНОГО ТЕКСТА ИЗ АТЛАСА
    if (pixel_coords.y >= globals.screen_height - globals.panel_height) {
        var base_panel_color = vec3<f32>(0.058, 0.058, 0.058); // Фон панели (15, 15, 15)

        let local_y = pixel_coords.y - (globals.screen_height - globals.panel_height);
        let local_x = pixel_coords.x;

        // ИСПРАВЛЕНО: Поскольку шрифт 12px при умножении на 2 занимает ровно 24px,
        // стартовый отступ по вертикали теперь равен 0.0 (буква занимает панель от верха до низа)
        let text_start_x = 10.0;
        let text_start_y = 0.0;

        // ИСПРАВЛЕНО: Шаг сетки для символа шириной 8px при масштабе х2 теперь равен строго 16 пикселей!
        let char_step = 16.0;

        if (local_x >= text_start_x && local_y >= text_start_y && local_y < text_start_y + 24.0) {
            let text_offset_x = local_x - text_start_x;
            let char_index = u32(text_offset_x / char_step);

            if (char_index < 16u) {
                let vec_index = char_index / 4u;
                let comp_index = char_index % 4u;

                let target_vec = globals.text_data[vec_index];
                var ascii_code: u32 = 0u;
                if (comp_index == 0u) { ascii_code = target_vec.x; }
                else if (comp_index == 1u) { ascii_code = target_vec.y; }
                else if (comp_index == 2u) { ascii_code = target_vec.z; }
                else { ascii_code = target_vec.w; }

                // Целочисленное деление на 2u переводит экранные координаты х2 в координаты оригинальной ячейки 8х12
                let glyph_x = u32(text_offset_x % char_step) / 2u;
                let glyph_y = u32(local_y - text_start_y) / 2u;

                // ИСПРАВЛЕНО: Проверяем новые честные границы сетки промышленного шрифта 8х12
                if (glyph_x < 8u && glyph_y < 12u) {
                    if (get_char_pixel(ascii_code, glyph_x, glyph_y) > 0.5) {
                        base_panel_color = vec3<f32>(0.0, 1.0, 0.0); // Неоново-зеленый текст
                    }
                }
            }
        }

        return vec4<f32>(pow(base_panel_color, vec3<f32>(2.2)), 1.0);
    }

    // 2. ОТРИСОВКА ИГРОВОГО ПОЛЯ И ШАРОВ
    var final_color = vec3<f32>(0.156, 0.156, 0.156);

    for (var i: u32 = 0u; i < globals.ball_count; i = i + 1u) {
        let ball = balls[i];
        let dx = ball.x - pixel_coords.x;
        let dy = ball.y - pixel_coords.y;
        let distance_sq = dx * dx + dy * dy;

        if (distance_sq <= ball.radius * ball.radius) {
            let ball_base_color = vec3<f32>(ball.r, ball.g, ball.b);
            final_color = ball_base_color;

            let brightness = (ball.r * 0.299) + (ball.g * 0.587) + (ball.b * 0.114);
            var marker_color = vec3<f32>(1.0, 1.0, 1.0);
            if (brightness > 0.5) { marker_color = vec3<f32>(0.0, 0.0, 0.0); }

            let adx = abs(dx);
            let ady = abs(dy);
            let marker_id = u32(ball.marker);

            if (marker_id == 1u) { if (adx <= 3.0 && ady <= 3.0) { final_color = marker_color; } }
            else if (marker_id == 2u) { if (adx <= 1.0 && ady <= 1.0) { final_color = marker_color; } }
            else if (marker_id == 3u) { if ((adx <= 4.0 && ady <= 1.0) || (adx <= 1.0 && ady <= 4.0)) { final_color = marker_color; } }
            else if (marker_id == 4u) { if (adx <= 2.0 && ady <= 2.0) { final_color = marker_color; } }
            else if (marker_id == 5u) { if (dy >= -2.0 && dy <= 2.0 && adx <= (2.0 - dy * 0.5)) { final_color = marker_color; } }
            else if (marker_id == 6u) { if ((adx <= 2.0 && ady <= 2.0) || (adx <= 4.0 && ady <= 1.0) || (adx <= 1.0 && ady <= 4.0)) { final_color = marker_color; } }
            break;
        }
    }

    let gamma_corrected = pow(final_color, vec3<f32>(1.0 / 0.5));
    return vec4<f32>(gamma_corrected, 1.0);
}
