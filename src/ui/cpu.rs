use crate::state::CpuData;
use crate::ui::layout::DisplayMode;
use crate::utils::{format_frequency, get_usage_color, render_bar};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_cpu(f: &mut Frame, area: Rect, data: &CpuData, _mode: DisplayMode, _history: &[f32]) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" CPU ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 || inner.height < 1 {
        return;
    }

    let bar_width = (inner.width as usize).saturating_sub(8).max(10);
    let bar = render_bar(data.usage, bar_width);
    let color = get_usage_color(data.usage);

    let cpu_name = data.name.trim();

    let mut y = inner.y;

    f.render_widget(
        Paragraph::new(Span::raw(cpu_name))
            .style(Style::default().fg(ratatui::style::Color::White)),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y = y.saturating_add(1);

    f.render_widget(
        Paragraph::new(Span::raw(format!("[{}] {:.1}%", bar, data.usage)))
            .style(Style::default().fg(color)),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y = y.saturating_add(1);

    // Add empty row for spacing
    y = y.saturating_add(1);

    // Display clock, plus temperature, fan speed and power where available
    let mut temp_parts = vec![format!("Clock: {}", format_frequency(data.frequency))];

    if let Some(temp) = data.temperature {
        temp_parts.push(format!("Temp: {:.0}°C", temp));
    }

    if let Some(fan) = data.fan_speed {
        temp_parts.push(format!("Fan: {}%", fan));
    }

    if let Some(power) = data.power_draw {
        temp_parts.push(format!("Power: {}W", power));
    }

    // Compact mode only has room for name and usage
    if y >= inner.bottom() {
        return;
    }

    f.render_widget(
        Paragraph::new(Span::raw(temp_parts.join("  ")))
            .style(Style::default().fg(ratatui::style::Color::Yellow)),
        Rect::new(inner.x, y, inner.width, 1),
    );
}
