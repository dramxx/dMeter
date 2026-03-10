use crate::state::CpuData;
use crate::ui::layout::DisplayMode;
use crate::utils::{
    format_frequency, get_temp_color, get_usage_color, render_bar, render_sparkline,
};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_cpu(f: &mut Frame, area: Rect, data: &CpuData, mode: DisplayMode, history: &[f32]) {
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

    let cpu_name = &data.name;
    let freq = format_frequency(data.frequency);
    let cores = format!(
        "{} cores ({}P/{}L)",
        data.core_usage.len(),
        data.physical_cores,
        data.logical_cores
    );

    let mut y = inner.y;

    f.render_widget(
        Paragraph::new(Span::raw(format!("{} {}", cpu_name, freq)))
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

    if let Some(temp) = data.temperature {
        let temp_color = get_temp_color(temp);
        f.render_widget(
            Paragraph::new(Span::raw(format!("Temperature: {:.0}°C", temp)))
                .style(Style::default().fg(temp_color)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y = y.saturating_add(1);
    }

    f.render_widget(
        Paragraph::new(Span::raw(cores))
            .style(Style::default().fg(crate::ui::colors::Colors::muted_text())),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y = y.saturating_add(1);

    if (mode == DisplayMode::Standard || mode == DisplayMode::Spacious) && !history.is_empty() {
        let sparkline = render_sparkline(history, bar_width);
        f.render_widget(
            Paragraph::new(Span::raw(sparkline))
                .style(Style::default().fg(ratatui::style::Color::Blue)),
            Rect::new(inner.x, y, bar_width as u16, 1),
        );
    }
}
