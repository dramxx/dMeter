use crate::state::DiskData;
use crate::utils::{format_bytes, get_usage_color, render_bar};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_disk(f: &mut Frame, area: Rect, disks: &[DiskData]) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" DISK ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = crate::ui::layout::safe_inner(area, 1);
    if inner.width < 4 {
        return;
    }

    let bar_width = ((inner.width / 3) as usize).max(8);

    let disks: Vec<_> = disks.iter().take(3).collect();
    let max_label_width = disks
        .iter()
        .map(|d| {
            if d.name.is_empty() {
                d.mount_point.len()
            } else {
                d.mount_point.len() + d.filesystem.len() + 3
            }
        })
        .max()
        .unwrap_or(0);

    let mut y = inner.y;

    for disk in disks {
        if y >= inner.y.saturating_add(inner.height) {
            break;
        }

        let percent = if disk.total > 0 {
            (disk.used as f32 / disk.total as f32) * 100.0
        } else {
            0.0
        };

        let bar = render_bar(percent, bar_width);
        let color = get_usage_color(percent);

        let used = format_bytes(disk.used);
        let total = format_bytes(disk.total);

        let label = if disk.name.is_empty() {
            disk.mount_point.clone()
        } else {
            format!("{} ({})", disk.mount_point, disk.filesystem)
        };

        let padded_label = format!("{:<width$}", label, width = max_label_width);

        f.render_widget(
            Paragraph::new(Span::raw(format!(
                "{} [{}] {} / {}",
                padded_label, bar, used, total
            )))
            .style(Style::default().fg(color)),
            Rect::new(
                inner.x,
                y,
                inner.x.saturating_add(inner.width),
                y.saturating_add(1),
            ),
        );

        y = y.saturating_add(1);
    }
}
