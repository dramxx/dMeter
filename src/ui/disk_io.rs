use crate::state::DiskIOData;
use crate::utils::{format_bytes_per_sec, render_sparkline};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_disk_io(f: &mut Frame, area: Rect, data: &DiskIOData, read_history: &[f32], write_history: &[f32]) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" DISK I/O ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 {
        return;
    }

    let read_speed = format_bytes_per_sec(data.read_speed);
    let write_speed = format_bytes_per_sec(data.write_speed);

    // Calculate responsive sparkline width
    let text_content = format!("  Read {} | Write {}", read_speed, write_speed);
    let text_width = text_content.len() as u16 + 2;
    let sparkline_width = (inner.width.saturating_sub(text_width) as usize).max(20);

    // Render sparkline on the left
    if !read_history.is_empty() && !write_history.is_empty() {
        let combined_sparkline = render_sparkline(read_history, sparkline_width);
        f.render_widget(
            Paragraph::new(Span::raw(combined_sparkline))
                .style(Style::default().fg(ratatui::style::Color::Green)),
            Rect::new(inner.x, inner.y, sparkline_width as u16, 1),
        );
    }

    // Render disk I/O text on the right (after sparkline)
    let io_text = Line::from(vec![
        Span::styled("  Read ", Style::default().fg(ratatui::style::Color::White)),
        Span::styled(read_speed, Style::default().fg(ratatui::style::Color::Blue)),
        Span::styled(" | Write ", Style::default().fg(ratatui::style::Color::White)),
        Span::styled(write_speed, Style::default().fg(ratatui::style::Color::Yellow)),
    ]);

    f.render_widget(
        Paragraph::new(io_text),
        Rect::new(inner.x + sparkline_width as u16, inner.y, inner.width.saturating_sub(sparkline_width as u16), 1),
    );
}
