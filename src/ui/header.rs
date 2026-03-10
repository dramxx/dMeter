use crate::state::SystemData;
use ratatui::style::Stylize;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_header(f: &mut Frame, area: Rect, data: &SystemData) {
    let block = Block::default()
        .style(Style::default().fg(ratatui::style::Color::Cyan))
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray));

    f.render_widget(block, area);

    if area.width < 4 || area.height < 1 {
        return;
    }

    let left = "dmeter";
    f.render_widget(
        Paragraph::new(Span::raw(left).bold())
            .style(Style::default().fg(ratatui::style::Color::Cyan)),
        Rect::new(
            area.x.saturating_add(1),
            area.y,
            area.x.saturating_add(1).saturating_add(left.len() as u16),
            area.y.saturating_add(1),
        ),
    );

    let hostname = &data.system.hostname;
    let now = chrono::Local::now();
    let time_str = now.format("%a %d %b %H:%M").to_string();
    let right = format!("{}  |  {}", hostname, time_str);
    let right_len = right.len() as u16;
    let right_x = area
        .x
        .saturating_add(area.width)
        .saturating_sub(right_len + 2);

    if right_x >= area.x {
        f.render_widget(
            Paragraph::new(Span::raw(right)),
            Rect::new(
                right_x,
                area.y,
                right_x.saturating_add(right_len),
                area.y.saturating_add(1),
            ),
        );
    }
}

pub fn render_system_info(f: &mut Frame, area: Rect, data: &SystemData) {
    let block = Block::default()
        .style(Style::default().fg(ratatui::style::Color::DarkGray))
        .borders(Borders::TOP)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray));

    f.render_widget(block, area);

    if area.width < 4 || area.height < 1 {
        return;
    }

    let os = format!("{} {}", data.system.os_name, data.system.os_version);
    let uptime = crate::utils::format_uptime(data.system.uptime);
    let load = format!(
        "{:.2} / {:.2} / {:.2}",
        data.system.load_avg.0, data.system.load_avg.1, data.system.load_avg.2
    );
    let info = format!("{}  ·  Uptime: {}  ·  Load: {}", os, uptime, load);

    let info_len = info.len() as u16;
    if info_len.saturating_add(2) < area.width {
        f.render_widget(
            Paragraph::new(Span::raw(info))
                .style(Style::default().fg(ratatui::style::Color::White)),
            Rect::new(
                area.x.saturating_add(1),
                area.y,
                area.x.saturating_add(1).saturating_add(info_len),
                area.y.saturating_add(1),
            ),
        );
    }
}

pub fn render_minimum_size_warning(f: &mut Frame, area: Rect) {
    let text = "Terminal too small. Please resize to at least 80x24.";

    let block = Block::default()
        .title(" Warning ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Yellow));

    let paragraph = Paragraph::new(Span::raw(text))
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    let width = (text.len() as u16 + 4).min(area.width);
    let height = 3u16.min(area.height);

    if width < 10 || height < 3 {
        return;
    }

    let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
    let y = area
        .y
        .saturating_add(area.height.saturating_sub(height) / 2);

    f.render_widget(
        paragraph,
        Rect::new(x, y, x.saturating_add(width), y.saturating_add(height)),
    );
}
