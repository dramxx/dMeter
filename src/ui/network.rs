use crate::state::NetworkData;
use crate::utils::{format_bytes_per_sec, render_sparkline};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_network(
    f: &mut Frame,
    area: Rect,
    data: &NetworkData,
    rx_history: &[f32],
    tx_history: &[f32],
) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" NETWORK ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 {
        return;
    }

    let mut y = inner.y;

    let up_speed = format_bytes_per_sec(data.upload_speed);
    let down_speed = format_bytes_per_sec(data.download_speed);

    // Calculate responsive sparkline width
    let text_content = format!("  ↑ {} | ↓ {}", up_speed, down_speed);
    let text_width = text_content.len() as u16 + 2;
    let sparkline_width = (inner.width.saturating_sub(text_width) as usize).max(20);

    // Render sparkline on the left with adaptive scaling
    if !rx_history.is_empty() && !tx_history.is_empty() {
        let combined_sparkline = render_sparkline(rx_history, sparkline_width);
        f.render_widget(
            Paragraph::new(Span::raw(combined_sparkline))
                .style(Style::default().fg(ratatui::style::Color::Green)),
            Rect::new(inner.x, y, sparkline_width as u16, 1),
        );
    }

    // Render network text on the right (after sparkline)
    use ratatui::text::Line;
    let network_text = Line::from(vec![
        Span::styled("  ↑ ", Style::default().fg(ratatui::style::Color::White)),
        Span::styled(up_speed, Style::default().fg(ratatui::style::Color::Green)),
        Span::styled(" | ↓ ", Style::default().fg(ratatui::style::Color::White)),
        Span::styled(down_speed, Style::default().fg(ratatui::style::Color::Cyan)),
    ]);
    f.render_widget(
        Paragraph::new(network_text),
        Rect::new(
            inner.x + sparkline_width as u16,
            y,
            inner.width.saturating_sub(sparkline_width as u16),
            1,
        ),
    );
    y = y.saturating_add(1);

    // Render adapter info below
    let adapter_info = if data.adapter_name.is_empty() {
        "No adapter".to_string()
    } else {
        format!("{} · {}", data.adapter_name, data.ip_address)
    };

    f.render_widget(
        Paragraph::new(Span::raw(adapter_info))
            .style(Style::default().fg(ratatui::style::Color::White)),
        Rect::new(inner.x, y, inner.width, 1),
    );
}
