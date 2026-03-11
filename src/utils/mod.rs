use ratatui::style::Color;

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_bytes_per_sec(bytes: u64) -> String {
    format!("{}/s", format_bytes(bytes))
}

pub fn format_frequency(freq: u64) -> String {
    if freq >= 1000 {
        format!("{:.1} GHz", freq as f64 / 1000.0)
    } else {
        format!("{} MHz", freq)
    }
}

pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

pub fn get_usage_color(usage: f32) -> Color {
    if usage >= 85.0 {
        Color::Red
    } else if usage >= 60.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub fn get_temp_color(temp: f32) -> Color {
    if temp >= 80.0 {
        Color::Red
    } else if temp >= 60.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub fn render_bar(percent: f32, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f32).ceil() as usize;
    let filled = filled.min(width);

    let bar: String = "█".repeat(filled) + &"░".repeat(width - filled);
    bar
}

pub fn render_sparkline(data: &[f32], width: usize) -> String {
    if data.is_empty() || width == 0 {
        return "░".repeat(width);
    }

    // Use fixed scale from 0 to max to prevent visual jumps during buffer fill
    let max = data.iter().cloned().fold(0.0f32, f32::max).max(1.0); // Minimum max of 1.0

    let chars = ['░', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let step = if data.len() > width {
        data.len() / width
    } else {
        1
    };

    let mut result = String::new();

    for i in 0..width {
        let idx = (i * step).min(data.len() - 1);
        let value = data[idx];

        // Normalize from 0 to max (fixed scale)
        let normalized = (value / max).clamp(0.0, 1.0);

        let char_idx = ((normalized * (chars.len() - 1) as f32) as usize).min(chars.len() - 1);
        result.push(chars[char_idx]);
    }

    result
}

