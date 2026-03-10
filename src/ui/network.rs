use crate::state::NetworkData;
use crate::utils::format_bytes_per_sec;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_network(f: &mut Frame, area: Rect, data: &NetworkData) {
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
    y = y.saturating_add(1);

    let up_speed = format_bytes_per_sec(data.upload_speed);
    let down_speed = format_bytes_per_sec(data.download_speed);

    f.render_widget(
        Paragraph::new(Span::raw(format!("↑ {}", up_speed)))
            .style(Style::default().fg(ratatui::style::Color::Green)),
        Rect::new(inner.x, y, inner.width, 1),
    );
    y = y.saturating_add(1);

    f.render_widget(
        Paragraph::new(Span::raw(format!("↓ {}", down_speed)))
            .style(Style::default().fg(ratatui::style::Color::Red)),
        Rect::new(inner.x, y, inner.width, 1),
    );
}
