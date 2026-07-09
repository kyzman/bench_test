pub mod draw;
pub mod pipeline;

// Делаем реэкспорт основных функций наружу, чтобы в файлах app/mod.rs
// и app/state.rs не пришлось переписывать пути импорта.
pub use draw::{draw_pixels_frame, draw_softbuffer_frame};
pub use pipeline::CustomRenderPipeline;
