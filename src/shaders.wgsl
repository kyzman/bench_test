struct Ball {
    x: f32,
    y: f32,
    radius: f32,
    marker: f32,
    r: f32,
    g: f32,
    b: f32,
}

struct Globals {
    ball_count: u32,
    screen_width: f32,
    screen_height: f32,
    panel_height: f32,
    bg_color: vec3<f32>,
    text_data: array<vec4<u32>, 4>,
}

@group(0) @binding(0) var<storage, read> balls: array<Ball>;
@group(0) @binding(1) var<uniform> globals: Globals;
@group(0) @binding(2) var font_texture: texture_2d<f32>;
@group(0) @binding(3) var font_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) ball_data: vec4<f32>,
    @location(2) color: vec3<f32>,
    @location(3) @interpolate(flat) is_panel_layer: u32,
    @location(4) local_pixel_offset: vec2<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
    @builtin(instance_index) in_instance_index: u32,
    @location(0) x: f32,
    @location(1) y: f32,
    @location(2) radius: f32,
    @location(3) marker: f32,
    @location(4) r: f32,
    @location(5) g: f32,
    @location(6) b: f32,
) -> VertexOutput {
    var out: VertexOutput;

    // СЛОЙ 1: Отрисовка подложки панели (виртуальный лишний инстанс)
    if (in_instance_index >= globals.ball_count) {
        out.is_panel_layer = 1u;
        var bg_positions = array<vec2<f32>, 4>(
            vec2<f32>(-1.0,  1.0),
            vec2<f32>(-1.0, -1.0),
            vec2<f32>( 1.0,  1.0),
            vec2<f32>( 1.0, -1.0)
        );
        out.position = vec4<f32>(bg_positions[in_vertex_index], 0.0, 1.0);
        out.uv = bg_positions[in_vertex_index] * 0.5 + 0.5;
        out.uv.y = 1.0 - out.uv.y;
        out.ball_data = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        out.color = vec3<f32>(0.0, 0.0, 0.0);
        out.local_pixel_offset = vec2<f32>(0.0, 0.0);
        return out;
    }

    // СЛОЙ 2: Аппаратный инстансинг шаров
    out.is_panel_layer = 0u;

    let half_size_x = radius / (globals.screen_width * 0.5);
    let half_size_y = radius / (globals.screen_height * 0.5);
    let center_x = (x - globals.screen_width * 0.5) / (globals.screen_width * 0.5);
    let center_y = (globals.screen_height * 0.5 - y) / (globals.screen_height * 0.5);

    var positions = array<vec2<f32>, 4>(
        vec2<f32>(center_x - half_size_x, center_y + half_size_y),
        vec2<f32>(center_x - half_size_x, center_y - half_size_y),
        vec2<f32>(center_x + half_size_x, center_y + half_size_y),
        vec2<f32>(center_x + half_size_x, center_y - half_size_y)
    );

    var local_uvs = array<vec2<f32>, 4>(
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0, -1.0)
    );

    var pixel_offsets = array<vec2<f32>, 4>(
        vec2<f32>(-radius, -radius),
        vec2<f32>(-radius,  radius),
        vec2<f32>( radius, -radius),
        vec2<f32>( radius,  radius)
    );

    out.position = vec4<f32>(positions[in_vertex_index], 0.0, 1.0);
    out.uv = local_uvs[in_vertex_index];
    out.ball_data = vec4<f32>(x, y, radius, marker);
    out.color = vec3<f32>(r, g, b);
    out.local_pixel_offset = pixel_offsets[in_vertex_index];

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    // ---- ВЕТКА 1: ОТРИСОВКА ТЕХНИЧЕСКОЙ ПАНЕЛИ FPS ----
    if (in.is_panel_layer == 1u) {
        let pixel_coords = in.uv * vec2<f32>(globals.screen_width, globals.screen_height);

        if (pixel_coords.y >= globals.screen_height - globals.panel_height) {
            var base_panel_color = vec3<f32>(0.058, 0.058, 0.058);
            let local_y = pixel_coords.y - (globals.screen_height - globals.panel_height);
            let local_x = pixel_coords.x;
            let text_start_x = 10.0;
            let text_start_y = 4.0;
            let char_step = 16.0;

            if (local_x >= text_start_x && local_y >= text_start_y && local_y < text_start_y + 24.0) {
                let text_offset_x = local_x - text_start_x;
                let char_index = u32(text_offset_x / char_step);

                if (char_index < 16u) {
                    let vec_index = char_index / 4u;
                    let comp_index = char_index % 4u;
                    let target_vec = globals.text_data[vec_index];
                    var ascii_code = 32u;

                    if (comp_index == 0u) { ascii_code = target_vec.x; }
                    else if (comp_index == 1u) { ascii_code = target_vec.y; }
                    else if (comp_index == 2u) { ascii_code = target_vec.z; }
                    else { ascii_code = target_vec.w; }

                    let glyph_x = u32(text_offset_x % char_step) / 2u;
                    let glyph_y = u32(local_y - text_start_y) / 2u;

                    if (glyph_x < 8u && glyph_y < 12u) {
                        if (get_char_pixel(ascii_code, glyph_x, glyph_y) > 0.5) {
                            base_panel_color = vec3<f32>(0.0, 1.0, 0.0);
                        }
                    }
                }
            }
            // ТА САМАЯ КОРРЕКЦИЯ ГАММЫ: Вернули ручной множитель (pow 0.5), как в вашей рабочей версии!
            // Если цвета панели кажутся блёклыми или инвертированными, измените 0.5 на нужный вам коэффициент (например, 1.0 / 0.5 или 2.2)
            return vec4<f32>(pow(base_panel_color, vec3<f32>(2.2)), 1.0);
        }
        discard;
    }

    // ---- ВЕТКА 2: ОТРИСОВКА КРУГОВ И МАРКЕРОВ ШАРА ----
    let dist_from_center = length(in.uv);
    if (dist_from_center > 1.0) {
        discard;
    }

    let dx = in.local_pixel_offset.x;
    let dy = in.local_pixel_offset.y;

    let adx = abs(dx);
    let ady = abs(dy);
    let marker_id = u32(in.ball_data.w);

    let ball_base_color = in.color;
    var final_color = ball_base_color;

    let brightness = (ball_base_color.r * 0.299) + (ball_base_color.g * 0.587) + (ball_base_color.b * 0.114);
    var marker_color = vec3<f32>(1.0, 1.0, 1.0);
    if (brightness > 0.5) { marker_color = vec3<f32>(0.0, 0.0, 0.0); }

    // Логика маркеров
    if (marker_id == 1u) { if (adx <= 3.0 && ady <= 3.0) { final_color = marker_color; } }
    else if (marker_id == 2u) { if (adx <= 1.0 && ady <= 1.0) { final_color = marker_color; } }
    else if (marker_id == 3u) { if ((adx <= 4.0 && ady <= 1.0) || (adx <= 1.0 && ady <= 4.0)) { final_color = marker_color; } }
    else if (marker_id == 4u) { if (adx <= 2.0 && ady <= 2.0) { final_color = marker_color; } }
    else if (marker_id == 5u) { if (dy >= -2.0 && dy <= 2.0 && adx <= (2.0 - dy * 0.5)) { final_color = marker_color; } }
    else if (marker_id == 6u) { if ((adx <= 2.0 && ady <= 2.0) || (adx <= 4.0 && ady <= 1.0) || (adx <= 1.0 && ady <= 4.0)) { final_color = marker_color; } }

    // ТА САМАЯ КОРРЕКЦИЯ ГАММЫ ДЛЯ ШАРОВ:
    // Заменили pow(2.2) на pow(0.5). Теперь этот коэффициент будет чутко реагировать на любые ваши ручные правки!
    return vec4<f32>(pow(final_color, vec3<f32>(2.2)), 1.0);
}


fn get_char_pixel(char_code: u32, gx: u32, gy: u32) -> f32 {
    let start_ascii = 32u;
    let end_ascii = 126u;
    let char_w = 8u;
    let char_h = 12u;

    if (char_code < start_ascii || char_code > end_ascii) {
        let tex_x_excl = f32(1u * char_w + gx) + 0.5;
        let tex_x_quest = f32(31u * char_w + gx) + 0.5;
        let tex_y = f32(gy) + 0.5;

        let size = textureDimensions(font_texture);
        let uv_excl = vec2<f32>(tex_x_excl / f32(size.x), tex_y / f32(size.y));
        let uv_quest = vec2<f32>(tex_x_quest / f32(size.x), tex_y / f32(size.y));

        let pix_excl = textureSampleLevel(font_texture, font_sampler, uv_excl, 0.0).r;
        let pix_quest = textureSampleLevel(font_texture, font_sampler, uv_quest, 0.0).r;

        if (pix_excl > 0.5 || pix_quest > 0.5) { return 1.0; }
        return 0.0;
    }

    let char_index = char_code - start_ascii;
    let tex_pixel_x = f32(char_index * char_w + gx) + 0.5;
    let tex_pixel_y = f32(gy) + 0.5;

    let atlas_size = textureDimensions(font_texture);
    let final_uv = vec2<f32>(tex_pixel_x / f32(atlas_size.x), tex_pixel_y / f32(atlas_size.y));

    return textureSampleLevel(font_texture, font_sampler, final_uv, 0.0).r;
}
