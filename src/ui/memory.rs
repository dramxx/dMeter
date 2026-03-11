use crate::state::MemoryData;
use crate::utils::{format_bytes, get_usage_color, render_bar};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_memory(f: &mut Frame, area: Rect, data: &MemoryData, show_swap: bool) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" MEMORY ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 {
        return;
    }

    let bar_width = (((inner.width / 2) as f32 * 1.2) as usize).max(8);

    let mem_percent = if data.total > 0 {
        (data.used as f32 / data.total as f32) * 100.0
    } else {
        0.0
    };

    let mem_bar = render_bar(mem_percent, bar_width);
    let mem_color = get_usage_color(mem_percent);
    let mem_used = format_bytes(data.used);
    let mem_total = format_bytes(data.total);

    let mut y = inner.y;

    f.render_widget(
        Paragraph::new(Span::raw(format!(
            "RAM  [{}] {} / {}",
            mem_bar, mem_used, mem_total
        )))
        .style(Style::default().fg(mem_color)),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y = y.saturating_add(1);

    if show_swap && data.swap_total > 0 {
        let swap_percent = (data.swap_used as f32 / data.swap_total as f32) * 100.0;
        let swap_bar = render_bar(swap_percent, bar_width);
        let swap_used = format_bytes(data.swap_used);
        let swap_total = format_bytes(data.swap_total);

        f.render_widget(
            Paragraph::new(Span::raw(format!(
                "SWAP [{}] {} / {}",
                swap_bar, swap_used, swap_total
            )))
            .style(Style::default().fg(ratatui::style::Color::Yellow)),
            Rect::new(inner.x, y, inner.width, 1),
        );
        y = y.saturating_add(1);
    }

    // Add empty row for spacing
    y = y.saturating_add(1);

    // Display Commit and Cached memory or loading indicator
    if data.commit_total > 0 || data.cached > 0 {
        let commit_used = format_bytes(data.commit_total - data.commit_used);
        let _commit_total = format_bytes(data.commit_total);
        let cached = format_bytes(data.cached);

        f.render_widget(
            Paragraph::new(Span::raw(format!(
                "Commit {}  Cached {}",
                commit_used, cached
            )))
            .style(Style::default().fg(ratatui::style::Color::Cyan)),
            Rect::new(inner.x, y, inner.width, 1),
        );
    } else {
        // Show loading indicator while memory data is being collected
        let loading_chars = ["|", "/", "-", "\\"];
        let loading_index = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() / 250) as usize % 4; // Rotate every 250ms
        
        f.render_widget(
            Paragraph::new(Span::raw(format!(
                "Loading memory info {}",
                loading_chars[loading_index]
            )))
            .style(Style::default().fg(ratatui::style::Color::Cyan)),
            Rect::new(inner.x, y, inner.width, 1),
        );
    }
}
