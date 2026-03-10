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
    get_display_mode, render_apps, render_cpu, render_disk, render_gpu, render_header,
    render_memory, render_minimum_size_warning, render_network, render_system_info,
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
    is_paused: bool,
    show_gpu: bool,
    show_swap: bool,
    interval: u64,
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
            is_paused: false,
            show_gpu: !cli.no_gpu,
            show_swap: config.show_swap,
            interval: config.interval,
        }
    }

    fn update(&mut self) {
        if self.is_paused {
            return;
        }

        self.data = self.collector.collect(self.show_swap);
        self.cpu_history.push(self.data.cpu.usage);

        if self.show_gpu && self.data.gpu.available {
            self.gpu_history.push(self.data.gpu.usage);
        }
    }
}

fn main() -> io::Result<()> {
    let result = std::panic::catch_unwind(|| main_inner());

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
                        KeyCode::Char('p') => {
                            app.is_paused = !app.is_paused;
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

        let size = terminal.size()?;
        let mode = get_display_mode(size.height);

        if size.width < 80 || size.height < 24 {
            // Skip rendering when too small to avoid crashes
            // terminal.draw(|f| {
            //     render_minimum_size_warning(f, f.area());
            // })?;
        } else {
            terminal.draw(|f| {
                render_ui(f, &app, mode);
            })?;
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_ui(f: &mut Frame, app: &App, mode: crate::ui::DisplayMode) {
    let area = f.area();

    if area.width < 40 || area.height < 10 {
        return;
    }

    // Header (1 row)
    let header_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), 1);
    render_header(f, header_area, &app.data);

    // Footer (last row)
    let footer_area = Rect::new(
        area.x,
        area.y.saturating_add(area.height.saturating_sub(1)),
        area.width.saturating_sub(1),
        1,
    );
    render_system_info(f, footer_area, &app.data);

    // Middle panels based on display mode
    let middle_area = Rect::new(
        area.x,
        area.y.saturating_add(1),
        area.width.saturating_sub(1),
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

fn render_compact_mode(f: &mut Frame, area: Rect, app: &App) {
    // Panel height for top row
    let panel_height = 4u16;

    // Calculate width for each column (width / 3)
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

    // Bottom row panels
    let net_area = Rect::new(
        area.x,
        area.y + panel_height + 1,
        col1_width,
        area.height - panel_height - 1,
    );
    let disk_area = Rect::new(
        area.x + col1_width,
        area.y + panel_height + 1,
        col2_width + col3_width,
        area.height - panel_height - 1,
    );

    render_cpu(
        f,
        cpu_area,
        &app.data.cpu,
        crate::ui::DisplayMode::Compact,
        app.cpu_history.get(),
    );
    render_gpu(
        f,
        gpu_area,
        &app.data.gpu,
        crate::ui::DisplayMode::Compact,
        app.gpu_history.get(),
    );
    render_memory(f, mem_area, &app.data.memory, app.show_swap);
    render_network(f, net_area, &app.data.network);
    render_disk(f, disk_area, &app.data.disks);
}

fn render_standard_mode(f: &mut Frame, area: Rect, app: &App) {
    // Panel heights
    let panel_height = 5u16;
    let history_height = 2u16;
    let network_height = 4u16;

    // Calculate width for each column
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

    // History row (spans all columns)
    let history_y = area.y + panel_height;
    let history_area = Rect::new(area.x, history_y, area.width, history_height);

    // Network row
    let network_y = history_y + history_height;
    let net_area = Rect::new(area.x, network_y, col1_width, network_height);
    let disk_area = Rect::new(
        area.x + col1_width,
        network_y,
        col2_width + col3_width,
        network_height,
    );

    // Apps row (takes remaining space below network/disk)
    let apps_y = network_y + network_height;
    let apps_height = area
        .height
        .saturating_sub(panel_height + history_height + network_height);
    let apps_area = Rect::new(area.x, apps_y, area.width, apps_height);

    render_cpu(
        f,
        cpu_area,
        &app.data.cpu,
        crate::ui::DisplayMode::Standard,
        app.cpu_history.get(),
    );
    render_gpu(
        f,
        gpu_area,
        &app.data.gpu,
        crate::ui::DisplayMode::Standard,
        app.gpu_history.get(),
    );
    render_memory(f, mem_area, &app.data.memory, app.show_swap);
    render_cpu_history(f, history_area, app.cpu_history.get());
    render_network(f, net_area, &app.data.network);
    render_disk(f, disk_area, &app.data.disks);
    render_apps(f, apps_area, &app.data.processes);
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
