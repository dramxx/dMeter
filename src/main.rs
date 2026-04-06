use std::io;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use ratatui::crossterm::{
    event::{poll, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

#[cfg(not(windows))]
use ratatui::crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use ratatui::{backend::CrosstermBackend, prelude::*, widgets::*, Frame, Terminal};

use crate::collectors::SystemCollector;
use crate::config::CliArgs;
use crate::state::{HistoryBuffer, SystemData};
use crate::ui::{
    get_display_mode, render_cpu, render_disk, render_disk_io, render_gpu, render_header,
    render_memory, render_network, render_processes, render_system_info, GameOfLife,
};
use clap::Parser;

mod collectors;
mod config;
mod state;
mod ui;
mod utils;

struct App {
    data: SystemData,
    data_receiver: mpsc::Receiver<SystemData>,
    show_processes_flag: Arc<AtomicBool>,
    force_refresh_flag: Arc<AtomicBool>,
    collector_shutdown: Arc<AtomicBool>,
    collector_thread: Option<std::thread::JoinHandle<()>>,
    cpu_history: HistoryBuffer,
    gpu_history: HistoryBuffer,
    ram_history: HistoryBuffer,
    vram_history: HistoryBuffer,
    network_rx_history: HistoryBuffer,
    network_tx_history: HistoryBuffer,
    disk_read_history: HistoryBuffer,
    disk_write_history: HistoryBuffer,
    gol: Option<GameOfLife>,
    show_processes: bool,
    interval: u64,
    gol_tick: std::time::Instant,
}

impl Drop for App {
    fn drop(&mut self) {
        self.collector_shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.collector_thread.take() {
            let _ = handle.join();
        }
    }
}

impl App {
    fn new(cli: CliArgs) -> Self {
        let mut config = crate::config::Config::load();
        config.merge_cli(&cli);
        let interval = config.interval;

        let mut collector = SystemCollector::new();

        // Initial synchronous collection so we have data immediately
        let initial_data = collector.collect(true);

        // Spawn background collection thread — ALL collection happens off the main thread
        let (tx, rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);
        let show_procs = Arc::new(AtomicBool::new(false));
        let show_procs_clone = Arc::clone(&show_procs);
        let force_refresh = Arc::new(AtomicBool::new(false));
        let force_refresh_clone = Arc::clone(&force_refresh);

        let collector_thread = std::thread::spawn(move || {
            loop {
                // Sleep first (initial data already collected synchronously)
                for _ in 0..(interval * 10) {
                    if shutdown_clone.load(Ordering::Relaxed) {
                        return;
                    }
                    if force_refresh_clone.load(Ordering::Relaxed) {
                        force_refresh_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }

                if shutdown_clone.load(Ordering::Relaxed) {
                    break;
                }

                let collect_procs = show_procs_clone.load(Ordering::Relaxed);
                let data = collector.collect(collect_procs);
                if tx.send(data).is_err() {
                    break;
                }
            }
        });

        Self {
            data: initial_data,
            data_receiver: rx,
            show_processes_flag: show_procs,
            force_refresh_flag: force_refresh,
            collector_shutdown: shutdown,
            collector_thread: Some(collector_thread),
            cpu_history: HistoryBuffer::new(60),
            gpu_history: HistoryBuffer::new(60),
            ram_history: HistoryBuffer::new(60),
            vram_history: HistoryBuffer::new(60),
            network_rx_history: HistoryBuffer::new(60),
            network_tx_history: HistoryBuffer::new(60),
            disk_read_history: HistoryBuffer::new(60),
            disk_write_history: HistoryBuffer::new(60),
            gol: None,
            show_processes: false,
            interval,
            gol_tick: std::time::Instant::now(),
        }
    }

    fn update(&mut self) {
        // Sync the show_processes flag to the background thread
        self.show_processes_flag.store(self.show_processes, Ordering::Relaxed);

        // Non-blocking: read latest data from background thread
        while let Ok(data) = self.data_receiver.try_recv() {
            self.data = data;
        }

        self.cpu_history.push(self.data.cpu.usage);

        let ram_usage = if self.data.memory.total > 0 {
            (self.data.memory.used as f32 / self.data.memory.total as f32) * 100.0
        } else {
            0.0
        };
        self.ram_history.push(ram_usage);

        if self.data.gpu.available {
            self.gpu_history.push(self.data.gpu.usage);

            let vram_usage = if self.data.gpu.memory_total > 0 {
                (self.data.gpu.memory_used as f32 / self.data.gpu.memory_total as f32) * 100.0
            } else {
                0.0
            };
            self.vram_history.push(vram_usage);
        }

        self.network_rx_history
            .push(self.data.network.download_speed as f32 / 1024.0);
        self.network_tx_history
            .push(self.data.network.upload_speed as f32 / 1024.0);

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
        #[cfg(not(windows))]
        let _ = disable_raw_mode();
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

    #[cfg(not(windows))]
    let _ = enable_raw_mode();

    let mut app = App::new(cli);

    if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        app.update();
    })) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        eprintln!("Error during data collection: {:?}", e);
        return Ok(());
    }

    let mut tick_timer = std::time::Instant::now();
    let mut last_render = std::time::Instant::now();
    let frame_duration = Duration::from_millis(50); // 20 FPS limit

    while running.load(Ordering::SeqCst) {
        let timeout = Duration::from_millis(16); // ~60Hz polling

        if poll(timeout)? {
            if let Ok(Event::Key(key)) = crossterm::event::read() {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            running.store(false, Ordering::SeqCst);
                        }
                        KeyCode::Char('r') => {
                            app.force_refresh_flag.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Char('c') => {
                            if key
                                .modifiers
                                .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
                            {
                                running.store(false, Ordering::SeqCst);
                            }
                        }
                        KeyCode::Char(' ') => {
                            app.show_processes = !app.show_processes;
                            app.show_processes_flag.store(app.show_processes, Ordering::Relaxed);
                        }
                        KeyCode::Char('g') => {
                            // Force restart Game of Life
                            app.gol = None;
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

        if !app.show_processes && app.gol_tick.elapsed() >= Duration::from_millis(200) {
            if let Some(ref mut gol) = app.gol {
                gol.step();
            }
            app.gol_tick = std::time::Instant::now();
        }

        // Frame rate limiting: only render at 20 FPS
        if last_render.elapsed() >= frame_duration {
            let size = terminal.size()?;
            let mode = get_display_mode(size.height);

            if size.width >= 80 && size.height >= 24 {
                terminal.draw(|f| {
                    render_ui(f, &mut app, mode);
                })?;
            }
            last_render = std::time::Instant::now();
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    #[cfg(not(windows))]
    let _ = disable_raw_mode();

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
        let col1_width = area.width / 3;
        let col2_width = area.width / 3;
        let col3_width = area.width - col1_width - col2_width;

        let cpu_area = Rect::new(area.x, area.y, col1_width, panel_height);
        let gpu_area = Rect::new(area.x + col1_width, area.y, col2_width, panel_height);
        let mem_area = Rect::new(
            area.x + col1_width + col2_width,
            area.y,
            col3_width,
            panel_height,
        );

        let history_widget_width = area.width / 2;

        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, history_widget_width, history_height);
        let ram_history_area = Rect::new(
            area.x + history_widget_width,
            history_y,
            area.width - history_widget_width,
            history_height,
        );

        let gpu_history_y = history_y + history_height;
        let gpu_history_area = Rect::new(area.x, gpu_history_y, history_widget_width, history_height);
        let vram_history_area = Rect::new(
            area.x + history_widget_width,
            gpu_history_y,
            area.width - history_widget_width,
            history_height,
        );

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

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Compact, &&app.cpu_history.get());
        render_gpu(f, gpu_area, &app.data.gpu);
        render_memory(f, mem_area, &app.data.memory, true);
        render_history(f, cpu_history_area, &&app.cpu_history.get(), "CPU History", Color::Blue);
        render_history(f, ram_history_area, &&app.ram_history.get(), "RAM History", Color::Green);
        render_history(f, gpu_history_area, &&app.gpu_history.get(), "GPU History", Color::Cyan);
        render_history(f, vram_history_area, &&app.vram_history.get(), "VRAM History", Color::Magenta);
        render_network(f, net_area, &app.data.network, &app.network_rx_history.get(), &app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, &app.disk_read_history.get(), &app.disk_write_history.get());
    } else {
        let col_width = area.width / 2;

        let cpu_area = Rect::new(area.x, area.y, col_width, panel_height);
        let mem_area = Rect::new(area.x + col_width, area.y, area.width - col_width, panel_height);

        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, col_width, history_height);
        let ram_history_area = Rect::new(area.x + col_width, history_y, area.width - col_width, history_height);

        let network_y = history_y + history_height;
        let bottom_height = 4u16;
        let net_area = Rect::new(area.x, network_y, col_width, bottom_height);
        let disk_area = Rect::new(area.x + col_width, network_y, col_width, bottom_height);

        let disk_io_y = network_y + bottom_height;
        let disk_io_height = 3u16;
        let disk_io_area = Rect::new(area.x, disk_io_y, area.width, disk_io_height);

        let gol_y = disk_io_y + disk_io_height;
        let gol_height = area.height.saturating_sub(panel_height + history_height + bottom_height + disk_io_height);
        let gol_area = Rect::new(area.x, gol_y, area.width, gol_height);

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Compact, &&app.cpu_history.get());
        render_memory(f, mem_area, &app.data.memory, true);
        render_history(f, cpu_history_area, &&app.cpu_history.get(), "CPU History", Color::Blue);
        render_history(f, ram_history_area, &&app.ram_history.get(), "RAM History", Color::Green);
        render_network(f, net_area, &app.data.network, &app.network_rx_history.get(), &app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, &app.disk_read_history.get(), &app.disk_write_history.get());

        render_bottom_widget(f, gol_area, app);
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

        let history_widget_width = area.width / 2;

        // First history row: CPU and RAM side by side
        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, history_widget_width, history_height);
        let ram_history_area = Rect::new(area.x + history_widget_width, history_y, area.width - history_widget_width, history_height);

        // Second history row: GPU and VRAM side by side
        let gpu_history_y = history_y + history_height;
        let gpu_history_area = Rect::new(area.x, gpu_history_y, history_widget_width, history_height);
        let vram_history_area = Rect::new(area.x + history_widget_width, gpu_history_y, area.width - history_widget_width, history_height);

        // Network row
        let network_y = gpu_history_y + history_height;
        let net_area = Rect::new(area.x, network_y, col1_width, network_height);
        let disk_area = Rect::new(area.x + col1_width, network_y, col2_width, network_height);
        let disk_io_area = Rect::new(area.x + col1_width + col2_width, network_y, col3_width, network_height);

        // Game of Life row (fills remaining height)
        let gol_y = network_y + network_height + 1;
        let gol_height = area.height.saturating_sub(panel_height + (history_height * 2) + network_height + 2);
        let gol_area = Rect::new(area.x, gol_y, area.width, gol_height);

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Standard, &app.cpu_history.get());
        render_gpu(f, gpu_area, &app.data.gpu);
        render_memory(f, mem_area, &app.data.memory, true);
        render_history(f, cpu_history_area, &app.cpu_history.get(), "CPU History", Color::Blue);
        render_history(f, ram_history_area, &app.ram_history.get(), "RAM History", Color::Green);
        render_history(f, gpu_history_area, &app.gpu_history.get(), "GPU History", Color::Cyan);
        render_history(f, vram_history_area, &app.vram_history.get(), "VRAM History", Color::Magenta);
        render_network(f, net_area, &app.data.network, &app.network_rx_history.get(), &app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, &app.disk_read_history.get(), &app.disk_write_history.get());

        render_bottom_widget(f, gol_area, app);
    } else {
        // No GPU: optimized layout with full-width rows
        let col_width = area.width / 2;

        // Row 1: CPU and Memory (2 columns)
        let cpu_area = Rect::new(area.x, area.y, col_width, panel_height);
        let mem_area = Rect::new(area.x + col_width, area.y, area.width - col_width, panel_height);

        // Row 2: CPU History and RAM History (2 columns)
        let history_y = area.y + panel_height;
        let cpu_history_area = Rect::new(area.x, history_y, col_width, history_height);
        let ram_history_area = Rect::new(area.x + col_width, history_y, area.width - col_width, history_height);

        // Row 3: Network (full width)
        let network_y = history_y + history_height;
        let net_area = Rect::new(area.x, network_y, area.width, network_height);

        // Row 4: Disk and Disk I/O (2 columns)
        let disk_y = network_y + network_height;
        let disk_height = 4u16;
        let disk_area = Rect::new(area.x, disk_y, col_width, disk_height);
        let disk_io_area = Rect::new(area.x + col_width, disk_y, area.width - col_width, disk_height);

        // Game of Life (expands to fill all remaining space)
        let gol_y = disk_y + disk_height + 1;
        let gol_height = area.height.saturating_sub(panel_height + history_height + network_height + disk_height + 1);
        let gol_area = Rect::new(area.x, gol_y, area.width, gol_height);

        render_cpu(f, cpu_area, &app.data.cpu, crate::ui::DisplayMode::Standard, &app.cpu_history.get());
        render_memory(f, mem_area, &app.data.memory, true);
        render_history(f, cpu_history_area, &app.cpu_history.get(), "CPU History", Color::Blue);
        render_history(f, ram_history_area, &app.ram_history.get(), "RAM History", Color::Green);
        render_network(f, net_area, &app.data.network, &app.network_rx_history.get(), &app.network_tx_history.get());
        render_disk(f, disk_area, &app.data.disks);
        render_disk_io(f, disk_io_area, &app.data.disk_io, &app.disk_read_history.get(), &app.disk_write_history.get());

        render_bottom_widget(f, gol_area, app);
    }
}

fn render_bottom_widget(f: &mut Frame, gol_area: Rect, app: &mut App) {
    if app.show_processes {
        // Render process widget
        render_processes(f, gol_area, &app.data.processes);
    } else {
        // Render Game of Life
        // Create inner container with padding (no border)
        let inner_gol_area = gol_area.inner(Margin::new(2, 2)); // 2-char padding on all sides
        let gol_width = inner_gol_area.width as u32;
        let gol_height = (inner_gol_area.height as u32) * 2; // 2 game rows per terminal row

        if gol_width > 2 && gol_height > 2 {
            if app.gol.as_ref().is_none_or(|g| g.width != gol_width || g.height != gol_height) {
                app.gol = Some(GameOfLife::new(gol_width, gol_height));
            }

            if let Some(ref gol) = app.gol {
                let gen = gol.generation();

                let title = format!(" Conway's Game of Life | Generation: {gen} ");
                let gol_block = Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(crate::ui::colors::Colors::border()));
                f.render_widget(gol_block, gol_area);

                if gol.is_dead() {
                    let text = "all died.";
                    let text_len = u16::try_from(text.len()).unwrap_or(u16::MAX);
                    let text_x =
                        inner_gol_area.x + (inner_gol_area.width.saturating_sub(text_len)) / 2;
                    let text_y = inner_gol_area.y + inner_gol_area.height / 2;

                    f.render_widget(
                        Paragraph::new(Span::raw(text))
                            .style(Style::default().fg(ratatui::style::Color::DarkGray)),
                        Rect::new(text_x, text_y, text_len, 1),
                    );
                } else {
                    let cell_color = Color::Rgb(60, 60, 60);

                    for term_y in 0..inner_gol_area.height {
                        for term_x in 0..inner_gol_area.width {
                            let game_x = u32::from(term_x);
                            let top_y = u32::from(term_y) * 2;
                            let bot_y = top_y + 1;

                            let top = gol.cell_alive(game_x, top_y);
                            let bot = gol.cell_alive(game_x, bot_y);

                            let ch = match (top, bot) {
                                (true, true) => "█",
                                (true, false) => "▀",
                                (false, true) => "▄",
                                (false, false) => continue,
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
}

fn render_history(f: &mut Frame, area: Rect, history: &[f32], title: &str, color: Color) {
    use ratatui::widgets::{Block, Borders};

    if history.is_empty() {
        return;
    }

    let block = Block::default()
        .title(format!(" {title} "))
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
        Paragraph::new(Span::raw(sparkline)).style(Style::default().fg(color)),
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
            assert!((app.data.gpu.usage - 0.0).abs() < f32::EPSILON);
            assert_eq!(app.data.gpu.memory_used, 0);
            assert_eq!(app.data.gpu.memory_total, 0);
            
            // GPU history should still work (just with zeros)
            assert!(app.gpu_history.get().is_empty() || (app.gpu_history.get()[0] - 0.0).abs() < f32::EPSILON);
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
        for &cpu_val in &app.cpu_history.get() {
            assert!((0.0..=100.0).contains(&cpu_val));
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

    #[test]
    fn test_app_process_toggle() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        
        // Should start with Game of Life (show_processes = false)
        assert!(!app.show_processes);
        
        // Toggle to processes
        app.show_processes = !app.show_processes;
        assert!(app.show_processes);
        
        // Toggle back to Game of Life
        app.show_processes = !app.show_processes;
        assert!(!app.show_processes);
    }

    #[test]
    fn test_app_process_data_collection() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        
        // Enable process widget to trigger process collection
        app.show_processes = true;
        app.update();
        
        // Should have collected process data
        assert!(!app.data.processes.is_empty());
        
        // Should be limited to 100 processes
        assert!(app.data.processes.len() <= 100);
        
        // Processes should be sorted by combined resource usage
        if app.data.processes.len() > 1 {
            let first_score = app.data.processes[0].cpu_usage + app.data.processes[0].memory_usage;
            let second_score = app.data.processes[1].cpu_usage + app.data.processes[1].memory_usage;
            assert!(first_score >= second_score);
        }
    }

    #[test]
    fn test_app_process_data_validity() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        
        // Enable process widget to trigger process collection
        app.show_processes = true;
        app.update();
        
        // Check that process data is valid
        for process in &app.data.processes {
            assert!(!process.name.is_empty());
            assert!(process.cpu_usage >= 0.0);
            assert!(process.memory_usage >= 0.0);
            assert!(process.memory_usage <= 100.0);
        }
    }

    #[test]
    fn test_app_gol_restart() {
        let cli = CliArgs { interval: 2 };
        let mut app = App::new(cli);
        
        // Initially GoL should be None
        assert!(app.gol.is_none());
        
        // Simulate GoL initialization (would happen during rendering)
        app.gol = Some(GameOfLife::new(50, 50));
        assert!(app.gol.is_some());
        
        // Simulate 'g' key press - restart GoL
        app.gol = None;
        assert!(app.gol.is_none());
        
        // GoL will be recreated on next render
        app.gol = Some(GameOfLife::new(50, 50));
        assert!(app.gol.is_some());
    }
}
