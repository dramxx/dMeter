use ratatui::{prelude::*, widgets::Paragraph};

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

/// One row of a rate panel: a history sparkline filling the space left of a fixed label.
pub fn render_rate_line(f: &mut Frame, area: Rect, history: &[f32], color: Color, label: Line) {
    // Measure in terminal columns, not bytes ("↑" is 3 bytes but 1 column)
    let label_width = (label.width() as u16).min(area.width);
    let sparkline_width = area.width.saturating_sub(label_width + 1);

    if sparkline_width >= 5 {
        f.render_widget(
            Paragraph::new(crate::utils::render_sparkline(history, sparkline_width as usize))
                .style(Style::default().fg(color)),
            Rect::new(area.x, area.y, sparkline_width, 1),
        );
    }

    f.render_widget(
        Paragraph::new(label),
        Rect::new(area.right() - label_width, area.y, label_width, 1),
    );
}
