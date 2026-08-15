pub mod calendar;
pub mod radar;

pub use calendar::draw_monthly_calendar;
pub use radar::draw_radar_chart;

static FONT_INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
pub const FONT_DATA: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");

pub fn init_fonts() {
    FONT_INIT.get_or_init(|| {
        let _ = plotters::style::register_font("sans-serif", plotters::style::FontStyle::Normal, FONT_DATA);
    });
}

