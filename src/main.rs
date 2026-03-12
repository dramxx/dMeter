use std::io;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ratatui::crossterm::{
    event::{poll, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, prelude::*, widgets::*, Frame, Terminal};

use crate::collectors::SystemCollector;
use crate::config::CliArgs;
use crate::state::{HistoryBuffer, SystemData};
use crate::ui::{
    get_display_mode, render_cpu, render_disk, render_disk_io, render_gpu, render_header,
    render_memory, render_network, render_system_info, GameOfLife,
};
use clap::Parser;

mod collectors;
mod config;
mod state;
mod ui;
mod utils;

struct App {
    collector: SystemCollector,
    data: SystemData,
    cpu_history: HistoryBuffer,
    gpu_history: HistoryBuffer,
    ram_history: HistoryBuffer,
    vram_history: HistoryBuffer,
    network_rx_history: HistoryBuffer,
    network_tx_history: HistoryBuffer,
    disk_read_history: HistoryBuffer,
    disk_write_history: HistoryBuffer,
    gol: Option<GameOfLife>,
    interval: u64,
    gol_tick: std::time::Instant,
}

impl App {
    fn new(cli: CliArgs) -> Self {
        let mut config = crate::config::Config::load();
        config.merge_cli(&cli);

        let collector = SystemCollector::new();

        Self {
            collector,
            data: SystemData::default(),
            cpu_history: HistoryBuffer::new(60),
            gpu_history: HistoryBuffer::new(60),
            ram_history: HistoryBuffer::new(60),
            vram_history: HistoryBuffer::new(60),
            network_rx_history: HistoryBuffer::new(60),
            network_tx_history: HistoryBuffer::new(60),
            disk_read_history: HistoryBuffer::new(60),
            disk_write_history: HistoryBuffer::new(60),
            gol: None,
            interval: config.interval,
            gol_tick: std::time::Instant::now(),
        }
    }

    fn update(&mut self) {
        self.data = self.collector.collect(true);
        self.cpu_history.push(self.data.cpu.usage);

        // RAM usage percentage
        let ram_usage = if self.data.memory.total > 0 {
            (self.data.memory.used as f32 / self.data.memory.total as f32) * 100.0
        } else {
            0.0
        };
        self.ram_history.push(ram_usage);

        if self.data.gpu.available {
            self.gpu_history.push(self.data.gpu.usage);

            // VRAM usage percentage
            let vram_usage = if self.data.gpu.memory_total > 0 {
                (self.data.gpu.memory_used as f32 / self.data.gpu.memory_total as f32) * 100.0
            } else {
                0.0
            };
            self.vram_history.push(vram_usage);
        }

        // Network history (already in bytes/s, convert to KB/s for better scaling)
        self.network_rx_history
            .push(self.data.network.download_speed as f32 / 1024.0);
        self.network_tx_history
            .push(self.data.network.upload_speed as f32 / 1024.0);

        // Disk I/O history (already in bytes/s, convert to MB/s for better scaling)
        self.disk_read_history
            .push(self.data.disk_io.read_speed as f32 / 1024.0 / 1024.0);
        self.disk_write_history
            .push(self.data.disk_io.write_speed as f32 / 1024.0 / 1024.0);
    }
}

fn main() -> io::Result<()> {
    let result = std::panic::catch_unwind(main_inner);

    if let Err(e) = result {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        if let Some(s) = e.downcast_ref::<&str>() {
            eprintln!("\n\nPANIC: {}\n", s);
        } else if let Some(s) = e.downcast_ref::<String>() {
            eprintln!("\n\nPANIC: {}\n", s);
        } else {
            eprintln!("\n\nPANIC: {:?}\n", e);
        }
        std::process::exit(1);
    }

    Ok(())
}

fn main_inner() -> io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = CliArgs::parse();

    let running = Arc::new(AtomicBool::new(true));

    panic::set_hook(Box::new(move |_| {
        let _ = execute!(io::stdout(), LeaveAlternateScreen,);
    }));

    execute!(io::stdout(), EnterAlternateScreen, Clear(ClearType::All))?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut app = App::new(cli);

    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        app.update();
    })) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        eprintln!("Error during data collection: {:?}", e);
        return Ok(());
    }

    let mut tick_timer = std::time::Instant::now();

    while running.load(Ordering::SeqCst) {
        let timeout = Duration::from_millis(100);

        if poll(timeout)? {
            if let Ok(Event::Key(key)) = crossterm::event::read() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            running.store(false, Ordering::SeqCst);
                        }
                        KeyCode::Char('r') => {
                            if let Err(e) =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    app.update();
                                }))
                            {
                                eprintln!("Error during refresh: {:?}", e);
                            }
                        }
                        KeyCode::Char('c') => {
                            if key
                                .modifiers
                                .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
                            {
                                running.store(false, Ordering::SeqCst);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if tick_timer.elapsed() >= Duration::from_secs(app.interval) {
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                app.update();
            })) {
                eprintln!("Error during update: {:?}", e);
            }
            tick_timer = std::time::Instant::now();
        }

        if app.gol_tick.elapsed() >= Duration::from_millis(200) {
            if let Some(ref mut gol) = app.gol {
                gol.step();
            }
            app.gol_tick = std::time::Instant::now();
        }

        let size = terminal.size()?;
        let mode = get_display_mode(size.height);

        if size.width >= 80 && size.height >= 24 {
            terminal.draw(|f| {
                render_ui(f, &mut app, mode);
            })?;
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_ui(f: &mut Frame, app: &mut App, mode: crate::ui::DisplayMode) {
    let area = f.area();

    if area.width < 40 || area.height < 10 {
        return;
    }

    // Add slight left padding (2 characters)
    let left_padding = 2;
    let available_width = area.width.saturating_sub(left_padding).saturating_sub(1);

    // Header (1 row)
    let header_area = Rect::new(area.x + left_padding, area.y, available_width, 1);
    render_header(f, header_area, &app.data);

    // Footer (last row)
    let footer_area = Rect::new(
        area.x + left_padding,
        area.y.saturating_add(area.height.saturating_sub(1)),
        available_width,
        1,
    );
    render_system_info(f, footer_area, &app.data);

    // Middle panels based on display mode
    let middle_area = Rect::new(
        area.x + left_padding,
        area.y.saturating_add(1),
        available_width,
        area.height.saturating_sub(2),
    );

    match mode {
        crate::ui::DisplayMode::Warning => {}
        crate::ui::DisplayMode::Compact => {
            render_compact_mode(f, middle_area, app);
        }
        crate::ui::DisplayMode::Standard | crate::ui::DisplayMode::Spacious => {
            render_standard_mode(f, middle_area, app);
        }
    }
}

fn has_gpu(app: &App) -> bool {
    app.data.gpu.available
}

fn render_compact_mode(f: &mut Frame, area: Rect, app: &mut App) {
    let panel_height = 4u16;
    let history_height = 3u16;

    if has_gpu(app) {
        // Original 3-column layout with GPU
        let col1_width = area.width / 3;
        let col2_width = area.width / 3;
        let col3_width = area.width - col1_width - col2_width;

        // Top row panels
        let cpu_area = Rect::new(area.x, area.y, col1_width, panel_height);
        let gpu_area = Rect::new(area.x + col1_width, area.y, col2_width, panel_height);
        let mem_area = Rect::new(
            area.x + col1_width + col2_width,
            area.y,
            col3_width,
            panel_height,
        );

        let gap_size = 0;
        let available_width = area.width - gap_size;
        let history_widget_width = available_width / 2;

        // First history row: CPU and RAM side by side
        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, history_widget_width, history_height);
        let ram_history_area = Rect::new(
            area.x + history_widget_width + gap_size,
            history_y,
            history_widget_width,
            history_height,
        );

        // Second history row: GPU and VRAM side by side
        let gpu_history_y = history_y + history_height;
        let gpu_history_area = Rect::new(area.x, gpu_history_y, history_widget_width, history_height);
        let vram_history_area = Rect::new(
            area.x + history_widget_width + gap_size,
            gpu_history_y,
            history_widget_width,
            history_height,
        );

        // Bottom row panels
        let network_y = gpu_history_y + history_height;
        let net_area = Rect::new(
            area.x,
            network_y,
            col1_width,
            area.height - panel_height - (history_height * 2) - 1,
        );
        let disk_area = Rect::new(
            area.x + col1_width,
            network_y,
            col2_width,
            area.height - panel_height - (history_height * 2) - 1,
        );
        let disk_io_area = Rect::new(
            area.x + col1_width + col2_width,
            network_y,
            col3_width,
            area.height - panel_height - (history_height * 2) - 1,
        );

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Compact, app.cpu_history.get());
        render_gpu(f, gpu_area, &app.data.gpu);
        render_memory(f, mem_area, &app.data.memory, true);
        render_cpu_history(f, cpu_history_area, app.cpu_history.get());
        render_ram_history(f, ram_history_area, app.ram_history.get());
        render_gpu_history(f, gpu_history_area, app.gpu_history.get());
        render_vram_history(f, vram_history_area, app.vram_history.get());
        render_network(f, net_area, &app.data.network, app.network_rx_history.get(), app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, app.disk_read_history.get(), app.disk_write_history.get());
    } else {
        // No GPU: 2-column layout
        let col_width = area.width / 2;

        // Top row: CPU and Memory (2 columns)
        let cpu_area = Rect::new(area.x, area.y, col_width, panel_height);
        let mem_area = Rect::new(area.x + col_width, area.y, col_width, panel_height);

        // History row: CPU and RAM side by side
        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, col_width, history_height);
        let ram_history_area = Rect::new(area.x + col_width, history_y, col_width, history_height);

        // Network and Disk row (2 columns)
        let network_y = history_y + history_height;
        let bottom_height = 5u16;
        let net_area = Rect::new(area.x, network_y, col_width, bottom_height);
        let disk_area = Rect::new(area.x + col_width, network_y, col_width, bottom_height);

        // Disk I/O (full width)
        let disk_io_y = network_y + bottom_height;
        let disk_io_height = area.height.saturating_sub(panel_height + history_height + bottom_height);
        let disk_io_area = Rect::new(area.x, disk_io_y, area.width, disk_io_height);

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Compact, app.cpu_history.get());
        render_memory(f, mem_area, &app.data.memory, true);
        render_cpu_history(f, cpu_history_area, app.cpu_history.get());
        render_ram_history(f, ram_history_area, app.ram_history.get());
        render_network(f, net_area, &app.data.network, app.network_rx_history.get(), app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, app.disk_read_history.get(), app.disk_write_history.get());
    }
}

fn render_standard_mode(f: &mut Frame, area: Rect, app: &mut App) {
    let panel_height = 6u16;
    let history_height = 3u16;
    let network_height = 4u16;

    if has_gpu(app) {
        // Original 3-column layout with GPU
        let col1_width = area.width / 3;
        let col2_width = area.width / 3;
        let col3_width = area.width - col1_width - col2_width;

        // Top row panels
        let cpu_area = Rect::new(area.x, area.y, col1_width, panel_height);
        let gpu_area = Rect::new(area.x + col1_width, area.y, col2_width, panel_height);
        let mem_area = Rect::new(area.x + col1_width + col2_width, area.y, col3_width, panel_height);

        let gap_size = 0;
        let available_width = area.width - gap_size;
        let history_widget_width = available_width / 2;

        // First history row: CPU and RAM side by side
        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, history_widget_width, history_height);
        let ram_history_area = Rect::new(area.x + history_widget_width + gap_size, history_y, history_widget_width, history_height);

        // Second history row: GPU and VRAM side by side
        let gpu_history_y = history_y + history_height;
        let gpu_history_area = Rect::new(area.x, gpu_history_y, history_widget_width, history_height);
        let vram_history_area = Rect::new(area.x + history_widget_width + gap_size, gpu_history_y, history_widget_width, history_height);

        // Network row
        let network_y = gpu_history_y + history_height;
        let net_area = Rect::new(area.x, network_y, col1_width, network_height);
        let disk_area = Rect::new(area.x + col1_width, network_y, col2_width, network_height);
        let disk_io_area = Rect::new(area.x + col1_width + col2_width, network_y, col3_width, network_height);

        // Game of Life row (fills remaining height)
        let gol_y = network_y + network_height + 1;
        let gol_height = area.height.saturating_sub(panel_height + (history_height * 2) + network_height + 2);
        let gol_area = Rect::new(area.x, gol_y, area.width, gol_height);

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Standard, app.cpu_history.get());
        render_gpu(f, gpu_area, &app.data.gpu);
        render_memory(f, mem_area, &app.data.memory, true);
        render_cpu_history(f, cpu_history_area, app.cpu_history.get());
        render_ram_history(f, ram_history_area, app.ram_history.get());
        render_gpu_history(f, gpu_history_area, app.gpu_history.get());
        render_vram_history(f, vram_history_area, app.vram_history.get());
        render_network(f, net_area, &app.data.network, app.network_rx_history.get(), app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, app.disk_read_history.get(), app.disk_write_history.get());

        render_game_of_life(f, gol_area, app);
    } else {
        // No GPU: 2-column layout with expanded Game of Life
        let col_width = area.width / 2;

        // Top row: CPU and Memory (2 columns)
        let cpu_area = Rect::new(area.x, area.y, col_width, panel_height);
        let mem_area = Rect::new(area.x + col_width, area.y, col_width, panel_height);

        // History row: CPU and RAM side by side
        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, col_width, history_height);
        let ram_history_area = Rect::new(area.x + col_width, history_y, col_width, history_height);

        // Network and Disk row (2 columns)
        let network_y = history_y + history_height;
        let net_area = Rect::new(area.x, network_y, col_width, network_height);
        let disk_area = Rect::new(area.x + col_width, network_y, col_width, network_height);

        // Disk I/O (full width)
        let disk_io_y = network_y + network_height;
        let disk_io_height = 4u16;
        let disk_io_area = Rect::new(area.x, disk_io_y, area.width, disk_io_height);

        // Game of Life (expands to fill all remaining space)
        let gol_y = disk_io_y + disk_io_height + 1;
        let gol_height = area.height.saturating_sub(panel_height + history_height + network_height + disk_io_height + 1);
        let gol_area = Rect::new(area.x, gol_y, area.width, gol_height);

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Standard, app.cpu_history.get());
        render_memory(f, mem_area, &app.data.memory, true);
        render_cpu_history(f, cpu_history_area, app.cpu_history.get());
        render_ram_history(f, ram_history_area, app.ram_history.get());
        render_network(f, net_area, &app.data.network, app.network_rx_history.get(), app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, app.disk_read_history.get(), app.disk_write_history.get());

        render_game_of_life(f, gol_area, app);
    }
}

fn render_game_of_life(f: &mut Frame, gol_area: Rect, app: &mut App) {
    // Create inner container with padding (no border)
    let inner_gol_area = gol_area.inner(Margin::new(2, 2)); // 2-char padding on all sides
    let gol_width = inner_gol_area.width as u32;
    let gol_height = (inner_gol_area.height as u32) * 2; // 2 game rows per terminal row

    if gol_width > 2 && gol_height > 2 {
        if app.gol.is_none()
            || app
                .gol
                .as_ref()
                .map(|g| g.width != gol_width || g.height != gol_height)
                .unwrap_or(true)
        {
            app.gol = Some(GameOfLife::new(gol_width, gol_height));
        }

        if let Some(ref gol) = app.gol {
            let cells = gol.get_cells();
            let gen = gol.generation();

            let title = format!(" Conway's Game of Life | Generation: {} ", gen);
            let gol_block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(crate::ui::colors::Colors::border()));
            f.render_widget(gol_block, gol_area);

            // Check if game is dead and show appropriate content
            if gol.is_dead() {
                // Show "all died." text in center
                let text = "all died.";
                let text_x =
                    inner_gol_area.x + (inner_gol_area.width.saturating_sub(text.len() as u16)) / 2;
                let text_y = inner_gol_area.y + inner_gol_area.height / 2;

                f.render_widget(
                    Paragraph::new(Span::raw(text))
                        .style(Style::default().fg(ratatui::style::Color::DarkGray)),
                    Rect::new(text_x, text_y, text.len() as u16, 1),
                );
            } else {
                let cell_color = Color::Rgb(60, 60, 60);

                for term_y in 0..inner_gol_area.height {
                    for term_x in 0..inner_gol_area.width {
                        let game_x = term_x as u32;
                        let top_y = (term_y as u32) * 2;
                        let bot_y = top_y + 1;

                        let top = cells.contains(&(game_x, top_y));
                        let bot = bot_y < gol.height && cells.contains(&(game_x, bot_y));

                        let ch = match (top, bot) {
                            (true, true) => "█",
                            (true, false) => "▀",
                            (false, true) => "▄",
                            (false, false) => continue, // skip empty, avoid unnecessary renders
                        };

                        f.render_widget(
                            Span::raw(ch).fg(cell_color),
                            Rect::new(inner_gol_area.x + term_x, inner_gol_area.y + term_y, 1, 1),
                        );
                    }
                }
            }
        }
    }
}

fn render_cpu_history(f: &mut Frame, area: Rect, history: &[f32]) {
    use ratatui::widgets::{Block, Borders};

    if history.is_empty() {
        return;
    }

    let block = Block::default()
        .title(" CPU History ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = area.inner(Margin::new(1, 1));
    let inner_width = inner.width as usize;

    if inner_width < 10 {
        return;
    }

    let sparkline = crate::utils::render_sparkline(history, inner_width);

    f.render_widget(
        Paragraph::new(Span::raw(sparkline)).style(Style::default().fg(Color::Blue)),
        inner,
    );
}

fn render_ram_history(f: &mut Frame, area: Rect, history: &[f32]) {
    use ratatui::widgets::{Block, Borders};

    if history.is_empty() {
        return;
    }

    let block = Block::default()
        .title(" RAM History ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = area.inner(Margin::new(1, 1));
    let inner_width = inner.width as usize;

    if inner_width < 10 {
        return;
    }

    let sparkline = crate::utils::render_sparkline(history, inner_width);

    f.render_widget(
        Paragraph::new(Span::raw(sparkline)).style(Style::default().fg(Color::Green)),
        inner,
    );
}

fn render_gpu_history(f: &mut Frame, area: Rect, history: &[f32]) {
    use ratatui::widgets::{Block, Borders};

    if history.is_empty() {
        return;
    }

    let block = Block::default()
        .title(" GPU History ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = area.inner(Margin::new(1, 1));
    let inner_width = inner.width as usize;

    if inner_width < 10 {
        return;
    }

    let sparkline = crate::utils::render_sparkline(history, inner_width);

    f.render_widget(
        Paragraph::new(Span::raw(sparkline)).style(Style::default().fg(Color::Cyan)),
        inner,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        // App should be created without panicking
        let cli = CliArgs { interval: 2 };
        let app = App::new(cli);
        
        // Verify app was created successfully
        assert_eq!(app.interval, 2);
        assert!(app.gol.is_none());
        
        // History buffers may or may not be empty depending on initialization
        // The important thing is the app doesn't crash
    }

    #[test]
    fn test_app_update_no_crash() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        
        // Update should not crash even without GPU
        app.update();
        
        // Verify data was collected
        assert!(app.data.cpu.usage >= 0.0);
        assert!(app.data.memory.total > 0);
    }

    #[test]
    fn test_app_update_multiple_times() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        
        // Update multiple times - should not leak memory or crash
        for _ in 0..10 {
            app.update();
            assert!(app.data.cpu.usage >= 0.0);
        }
    }

    #[test]
    fn test_app_with_no_gpu() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        app.update();
        
        // If no GPU, should have default values
        if !app.data.gpu.available {
            assert_eq!(app.data.gpu.usage, 0.0);
            assert_eq!(app.data.gpu.memory_used, 0);
            assert_eq!(app.data.gpu.memory_total, 0);
            
            // GPU history should still work (just with zeros)
            assert!(app.gpu_history.get().is_empty() || app.gpu_history.get()[0] == 0.0);
        }
    }

    #[test]
    fn test_app_history_buffers() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        
        // Update to populate history
        app.update();
        
        // History buffers should have data
        assert!(!app.cpu_history.get().is_empty());
        assert!(!app.ram_history.get().is_empty());
        
        // Values should be within valid ranges
        for &cpu_val in app.cpu_history.get() {
            assert!(cpu_val >= 0.0 && cpu_val <= 100.0);
        }
    }

    #[test]
    fn test_app_cleanup() {
        // Create and drop app multiple times
        for _ in 0..5 {
            let cli = CliArgs { interval: 2 };
            let mut app = App::new(cli);
            app.update();
            drop(app);
        }
        
        // If we get here, cleanup is working properly
    }

    #[test]
    fn test_has_gpu_function() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        app.update();
        
        let has_gpu_result = has_gpu(&app);
        
        // Should match the GPU available flag
        assert_eq!(has_gpu_result, app.data.gpu.available);
    }

    #[test]
    fn test_app_data_consistency() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        app.update();
        
        // CPU usage should be valid
        assert!(app.data.cpu.usage >= 0.0 && app.data.cpu.usage <= 100.0);
        
        // Memory should be consistent
        assert!(app.data.memory.used <= app.data.memory.total);
        
        // GPU memory should be consistent
        assert!(app.data.gpu.memory_used <= app.data.gpu.memory_total);
        
        // Network speeds should be non-negative
        assert!(app.data.network.upload_speed >= 0.0);
        assert!(app.data.network.download_speed >= 0.0);
    }

    #[test]
    fn test_app_game_of_life_initialization() {
        let cli = CliArgs { interval: 2 };
        let app = App::new(cli);
        
        // Game of Life should be None initially
        assert!(app.gol.is_none());
    }
}

fn render_vram_history(f: &mut Frame, area: Rect, history: &[f32]) {
    use ratatui::widgets::{Block, Borders};

    if history.is_empty() {
        return;
    }

    let block = Block::default()
        .title(" VRAM History ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(crate::ui::colors::Colors::border()));

    f.render_widget(block, area);

    let inner = area.inner(Margin::new(1, 1));
    let inner_width = inner.width as usize;

    if inner_width < 10 {
        return;
    }

    let sparkline = crate::utils::render_sparkline(history, inner_width);

    f.render_widget(
        Paragraph::new(Span::raw(sparkline)).style(Style::default().fg(Color::Magenta)),
        inner,
    );
}
