use crate::state::DiskIOData;
use crate::ui::layout::render_rate_line;
use crate::utils::format_bytes_per_sec;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

pub fn render_disk_io(
    f: &mut Frame,
    area: Rect,
    data: &DiskIOData,
    read_history: &[f32],
    write_history: &[f32],
) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" DISK I/O ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 || inner.height < 1 {
        return;
    }

    // Fixed-width speeds keep the sparkline from jittering as the numbers change
    let read = Span::styled(
        format!("{:>11}", format_bytes_per_sec(data.read_speed)),
        Style::default().fg(Color::Blue),
    );
    let write = Span::styled(
        format!("{:>11}", format_bytes_per_sec(data.write_speed)),
        Style::default().fg(Color::Yellow),
    );
    let label = |text: &'static str| Span::styled(text, Style::default().fg(Color::White));

    let row = |offset: u16| Rect::new(inner.x, inner.y + offset, inner.width, 1);

    if inner.height >= 2 {
        render_rate_line(f, row(0), read_history, Color::Blue, Line::from(vec![label("Read  "), read]));
        render_rate_line(f, row(1), write_history, Color::Yellow, Line::from(vec![label("Write "), write]));
    } else {
        render_rate_line(
            f,
            row(0),
            read_history,
            Color::Blue,
            Line::from(vec![label("R "), read, label(" W "), write]),
        );
    }
}
