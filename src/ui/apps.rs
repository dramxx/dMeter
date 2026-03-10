use crate::state::ProcessData;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_apps(f: &mut Frame, area: Rect, processes: &[ProcessData]) {
    if area.width < 4 || area.height < 2 {
        return;
    }

    let block = Block::default()
        .title(" Apps ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    if inner.width < 4 || inner.height < 1 {
        return;
    }

    if processes.is_empty() {
        f.render_widget(
            Paragraph::new(Span::raw("No user applications running"))
                .style(Style::default().fg(crate::ui::colors::Colors::muted_text())),
            Rect::new(
                inner.x,
                inner.y,
                inner.x.saturating_add(inner.width),
                inner.y.saturating_add(1),
            ),
        );
        return;
    }

    let mut y = inner.y;
    let name_width = (inner.width as usize).saturating_sub(20).max(10);

    for process in processes.iter() {
        if y >= inner.y + inner.height {
            break;
        }

        let cpu_str = format!("{:5.1}%", process.cpu_usage);
        let mem_str = format!("{:>6} MB", process.memory_mb);

        // Truncate name if too long
        let name = if process.name.len() > name_width {
            format!("{}...", &process.name[..name_width])
        } else {
            process.name.clone()
        };

        let line = format!(
            "{:<width$} {} {}",
            name,
            cpu_str,
            mem_str,
            width = name_width
        );

        f.render_widget(
            Paragraph::new(Span::raw(line))
                .style(Style::default().fg(ratatui::style::Color::White)),
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
