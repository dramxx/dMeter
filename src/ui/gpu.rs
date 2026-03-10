use crate::state::GpuData;
use crate::ui::layout::DisplayMode;
use crate::utils::{get_temp_color, get_usage_color, render_bar, render_sparkline};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn render_gpu(f: &mut Frame, area: Rect, data: &GpuData, mode: DisplayMode, history: &[f32]) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" GPU ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 || inner.height < 1 {
        return;
    }

    let bar_width = (inner.width as usize).saturating_sub(8).max(10);

    let mut y = inner.y;

    if !data.available {
        f.render_widget(
            Paragraph::new(Span::raw(data.name.clone()))
                .style(Style::default().fg(crate::ui::colors::Colors::muted_text())),
            Rect::new(
                inner.x,
                y,
                inner.x.saturating_add(inner.width),
                y.saturating_add(1),
            ),
        );
        return;
    }

    let gpu_bar = render_bar(data.usage, bar_width);
    let gpu_color = get_usage_color(data.usage);

    f.render_widget(
        Paragraph::new(Span::raw(&data.name))
            .style(Style::default().fg(ratatui::style::Color::White)),
        Rect::new(
            inner.x,
            y,
            inner.x.saturating_add(inner.width),
            y.saturating_add(1),
        ),
    );
    y = y.saturating_add(1);

    f.render_widget(
        Paragraph::new(Span::raw(format!("GPU  [{}] {:.1}%", gpu_bar, data.usage)))
            .style(Style::default().fg(gpu_color)),
        Rect::new(
            inner.x,
            y,
            inner.x.saturating_add(inner.width),
            y.saturating_add(1),
        ),
    );
    y = y.saturating_add(1);

    let mem_percent = if data.memory_total > 0 {
        (data.memory_used as f32 / data.memory_total as f32) * 100.0
    } else {
        0.0
    };
    let mem_bar = render_bar(mem_percent, bar_width);
    let mem_used = format_bytes(data.memory_used);
    let mem_total = format_bytes(data.memory_total);

    f.render_widget(
        Paragraph::new(Span::raw(format!(
            "VRAM [{}] {} / {}",
            mem_bar, mem_used, mem_total
        )))
        .style(Style::default().fg(ratatui::style::Color::Cyan)),
        Rect::new(
            inner.x,
            y,
            inner.x.saturating_add(inner.width),
            y.saturating_add(1),
        ),
    );
    y = y.saturating_add(1);

    if let Some(temp) = data.temperature {
        let temp_color = get_temp_color(temp);
        let mut line = format!("Temp: {:.0}°C", temp);

        if let Some(fan) = data.fan_speed {
            line.push_str(&format!("  Fan: {}%", fan));
        }
        if let Some(power) = data.power_draw {
            line.push_str(&format!("  Power: {}W", power));
        }

        f.render_widget(
            Paragraph::new(Span::raw(line)).style(Style::default().fg(temp_color)),
            Rect::new(
                inner.x,
                y,
                inner.x.saturating_add(inner.width),
                y.saturating_add(1),
            ),
        );
        y = y.saturating_add(1);
    }

    if (mode == DisplayMode::Standard || mode == DisplayMode::Spacious) && !history.is_empty() {
        let sparkline = render_sparkline(history, bar_width);
        f.render_widget(
            Paragraph::new(Span::raw(sparkline))
                .style(Style::default().fg(ratatui::style::Color::Magenta)),
            Rect::new(
                inner.x,
                y,
                inner.x.saturating_add(bar_width as u16),
                y.saturating_add(1),
            ),
        );
    }
}
