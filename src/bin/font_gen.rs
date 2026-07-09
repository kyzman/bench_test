use ril::{Image, ImageFormat, Rgb};

// Явно импортируем элементы embedded-graphics
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_6X12},
    pixelcolor::BinaryColor,
    prelude::OriginDimensions,
    text::Text,
};

const CHAR_WIDTH: u32 = 8;
const CHAR_HEIGHT: u32 = 12;

// Наш кастомный таргет для рисования
struct RilDrawTarget {
    pub image: Image<Rgb>,
    pub white: Rgb,
}

// ИСПРАВЛЕНО: Используем embedded_graphics::Pixel вместо embedded_graphics::primitives::Pixel
impl DrawTarget for RilDrawTarget {
    type Color = BinaryColor;
    type Error = std::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for embedded_graphics::Pixel(point, color) in pixels {
            if color == BinaryColor::On && point.x >= 0 && point.y >= 0 {
                let x = point.x as u32;
                let y = point.y as u32;
                if x < self.image.width() && y < self.image.height() {
                    // Рисуем пиксель буквы как микро-квадратик 1x1 в RIL
                    let dot = ril::draw::Rectangle::at(x, y)
                        .with_size(1, 1)
                        .with_fill(self.white);
                    self.image.draw(&dot);
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for RilDrawTarget {
    fn size(&self) -> embedded_graphics::prelude::Size {
        embedded_graphics::prelude::Size::new(self.image.width(), self.image.height())
    }
}

fn main() {
    println!("Запуск универсального генератора текстурного атласа шрифта...");

    let start_ascii = 32u8;
    let end_ascii = 126u8;
    let total_chars = (end_ascii - start_ascii + 1) as u32;
    let atlas_width = total_chars * CHAR_WIDTH;

    let mut target = RilDrawTarget {
        image: Image::<Rgb>::new(atlas_width, CHAR_HEIGHT, Rgb::new(0, 0, 0)),
        white: Rgb::new(255, 255, 255),
    };

    let text_style = MonoTextStyle::new(&FONT_6X12, BinaryColor::On);

    for code in start_ascii..=end_ascii {
        let ch = code as char;
        let char_str = ch.to_string();
        let char_index = (code - start_ascii) as u32;

        let cell_x = char_index * CHAR_WIDTH;

        // Отрисовка символа (смещение Y на 9 пикселей вниз под базовую линию embedded-graphics)
        Text::new(&char_str, Point::new(cell_x as i32 + 1, 9), text_style)
            .draw(&mut target)
            .unwrap();
    }

    std::fs::create_dir_all("assets").unwrap();
    target
        .image
        .save(ImageFormat::Png, "assets/font_atlas.png")
        .expect("Не удалось сохранить файл font_atlas.png");

    println!("Успех! Текстурный атлас сохранен в 'assets/font_atlas.png'.");
    println!(
        "Размер атласа: {}x{} пикселей. Символов: {}",
        atlas_width, CHAR_HEIGHT, total_chars
    );
}
