use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::ProcessData;

pub fn render_processes(f: &mut Frame, area: Rect, processes: &[ProcessData]) {
    // Create bordered block with title
    let block = Block::default()
        .title(" Processes | CPU + Memory Usage ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));
    
    f.render_widget(block, area);

    // Create inner area with padding (2 chars on all sides, same as Game of Life)
    let inner_area = area.inner(Margin::new(2, 2));
    
    if inner_area.width < 30 || inner_area.height < 2 {
        return; // Not enough space to render
    }

    // Split into 3 columns with 2-char padding between columns
    let padding = 2;
    let total_usable_width = inner_area.width.saturating_sub(padding * 2); // 2 gaps between 3 columns
    let col_width = total_usable_width / 3;
    
    let col1_area = Rect::new(inner_area.x, inner_area.y, col_width, inner_area.height);
    let col2_area = Rect::new(inner_area.x + col_width + padding, inner_area.y, col_width, inner_area.height);
    let col3_area = Rect::new(inner_area.x + (col_width + padding) * 2, inner_area.y, col_width, inner_area.height);

    // Calculate column widths for each column (tighter spacing)
    let total_width = col_width as usize;
    let cpu_width = 4;  // Reduced: just enough for "99.9"
    let mem_width = 4;  // Reduced: just enough for "99.9"
    let name_width = total_width.saturating_sub(cpu_width + mem_width + 2); // -2 for minimal spacing

    // Calculate how many processes fit per column (no header row)
    let available_height = inner_area.height as usize;
    let max_processes_per_column = available_height;
    
    // Render Column 1
    render_process_column(f, col1_area, processes, 0, max_processes_per_column, (name_width, cpu_width, mem_width));
    
    // Render Column 2
    render_process_column(f, col2_area, processes, max_processes_per_column, max_processes_per_column, (name_width, cpu_width, mem_width));
    
    // Render Column 3
    render_process_column(f, col3_area, processes, max_processes_per_column * 2, max_processes_per_column, (name_width, cpu_width, mem_width));
}

fn render_process_column(
    f: &mut Frame,
    area: Rect,
    processes: &[ProcessData],
    start_idx: usize,
    count: usize,
    widths: (usize, usize, usize),
) {
    let (name_width, cpu_width, mem_width) = widths;
    let mut y = area.y;

    // Render processes (no header)
    for process in processes.iter().skip(start_idx).take(count) {
        if y >= area.y + area.height {
            break;
        }

        // Truncate process name if too long
        let name = if process.name.len() > name_width {
            format!("{}...", &process.name[..name_width.saturating_sub(3)])
        } else {
            format!("{:<width$}", process.name, width = name_width)
        };

        let cpu = format!("{:>width$.1}", process.cpu_usage, width = cpu_width);
        let mem = format!("{:>width$.1}", process.memory_usage, width = mem_width);
        
        // Tighter spacing: name + 1 space + cpu + 1 space + mem
        let line = format!("{} {} {}", name, cpu, mem);
        
        f.render_widget(
            Paragraph::new(line)
                .style(Style::default().fg(crate::ui::colors::Colors::muted_text())),
            Rect::new(area.x, y, area.width, 1),
        );
        
        y += 1;
    }
}
