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
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    if area.width < 4 || area.height < 1 {
        return;
    }

    let left = "dMeter";
    f.render_widget(
        Paragraph::new(Span::raw(left).bold())
            .style(Style::default().fg(ratatui::style::Color::Cyan)),
        Rect::new(area.x.saturating_add(1), area.y, left.len() as u16, 1),
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
            Rect::new(right_x, area.y, right_len, 1),
        );
    }
}

pub fn render_system_info(f: &mut Frame, area: Rect, data: &SystemData) {
    let block = Block::default()
        .style(Style::default().fg(crate::ui::colors::Colors::system_info()))
        .borders(Borders::TOP)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    if area.width < 4 || area.height < 1 {
        return;
    }

    let os = format!("{} {}", data.system.os_name, data.system.os_version);
    let uptime = crate::utils::format_uptime(data.system.uptime);
    let info = format!("{}  ·  Uptime: {}", os, uptime);

    let info_len = info.len() as u16;
    if info_len.saturating_add(2) < area.width {
        f.render_widget(
            Paragraph::new(Span::raw(info))
                .style(Style::default().fg(ratatui::style::Color::White)),
            Rect::new(area.x.saturating_add(1), area.y, info_len, 1),
        );
    }
}

#[allow(dead_code)]
pub fn render_minimum_size_warning(f: &mut Frame, area: Rect) {
    let text = "Terminal too small. Please resize to at least 80x24.";

    let block = Block::default()
        .title(" Warning ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

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

    f.render_widget(paragraph, Rect::new(x, y, width, height));
}
