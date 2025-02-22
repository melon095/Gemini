use crate::window::GeminiRootWindow;
use iced::Font;

mod document;
mod network;
mod window;

const DEJA_VU_MONO: &[u8] = include_bytes!("../../../assets/DejaVuSansMono.ttf");
const NOTO_COLOR_EMOJI: &[u8] = include_bytes!("../../../assets/NotoColorEmoji-Regular.ttf");

fn main() {
    env_logger::builder()
        .filter_module("wgpu_hal", log::LevelFilter::Off)
        .filter_module("gemini", log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    iced::application(
        "Gemini Browser",
        GeminiRootWindow::update,
        GeminiRootWindow::view,
    )
    .font(DEJA_VU_MONO)
    .font(NOTO_COLOR_EMOJI)
    .default_font(Font::with_name("DejaVu Sans"))
    .run_with(GeminiRootWindow::new)
    .unwrap();
}
