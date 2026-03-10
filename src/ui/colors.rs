use ratatui::style::Color;

/// Platform-aware color configuration
pub struct Colors;

impl Colors {
    /// Border color - more visible on Linux terminals
    pub fn border() -> Color {
        if cfg!(target_os = "windows") {
            Color::DarkGray // Windows: DarkGray works well
        } else {
            Color::Gray // Linux: Use Gray instead of DarkGray for better visibility
        }
    }

    /// Muted text color - for very subtle information
    pub fn muted_text() -> Color {
        if cfg!(target_os = "windows") {
            Color::DarkGray
        } else {
            Color::Gray
        }
    }

    /// System info text color
    pub fn system_info() -> Color {
        if cfg!(target_os = "windows") {
            Color::DarkGray
        } else {
            Color::White
        }
    }
}
