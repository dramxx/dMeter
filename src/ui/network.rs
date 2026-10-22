use crate::state::NetworkData;
use crate::ui::layout::render_rate_line;
use crate::utils::format_bytes_per_sec;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
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

    // Adapter info lives in the title so both directions get their own row
    let title = if data.adapter_name.is_empty() {
        " NETWORK ".to_string()
    } else {
        format!(" NETWORK · {} · {} ", data.adapter_name, data.ip_address)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 || inner.height < 1 {
        return;
    }

    // Fixed-width speeds keep the sparkline from jittering as the numbers change
    let down = Span::styled(
        format!("{:>11}", format_bytes_per_sec(data.download_speed)),
        Style::default().fg(Color::Cyan),
    );
    let up = Span::styled(
        format!("{:>11}", format_bytes_per_sec(data.upload_speed)),
        Style::default().fg(Color::Green),
    );
    let label = |arrow: &'static str| Span::styled(arrow, Style::default().fg(Color::White));

    let row = |offset: u16| Rect::new(inner.x, inner.y + offset, inner.width, 1);

    if inner.height >= 2 {
        render_rate_line(f, row(0), rx_history, Color::Cyan, Line::from(vec![label("↓ "), down]));
        render_rate_line(f, row(1), tx_history, Color::Green, Line::from(vec![label("↑ "), up]));
    } else {
        render_rate_line(
            f,
            row(0),
            rx_history,
            Color::Cyan,
            Line::from(vec![label("↓ "), down, label(" ↑ "), up]),
        );
    }
}
