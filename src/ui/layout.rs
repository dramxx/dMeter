use ratatui::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    Warning,
    Compact,
    Standard,
    Spacious,
}

pub fn get_display_mode(height: u16) -> DisplayMode {
    if height < 24 {
        DisplayMode::Warning
    } else if height < 36 {
        DisplayMode::Compact
    } else if height < 50 {
        DisplayMode::Standard
    } else {
        DisplayMode::Spacious
    }
}

pub fn safe_inner(area: Rect, margin: u16) -> Rect {
    let x = area.x.saturating_add(margin);
    let y = area.y.saturating_add(margin);
    let width = area.width.saturating_sub(margin * 2);
    let height = area.height.saturating_sub(margin * 2);
    Rect::new(x, y, width, height)
}
