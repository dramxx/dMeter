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

pub fn format_bytes_per_sec(bytes: f64) -> String {
    format!("{}/s", format_bytes(bytes as u64))
}

pub fn format_frequency(freq: f32) -> String {
    if freq >= 1000.0 {
        format!("{:.1} GHz", freq / 1000.0)
    } else {
        format!("{:.0} MHz", freq)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
        assert_eq!(format_bytes(1099511627776), "1.0 TB");
    }

    #[test]
    fn test_format_bytes_per_sec() {
        assert_eq!(format_bytes_per_sec(1024.0), "1.0 KB/s");
        assert_eq!(format_bytes_per_sec(1048576.0), "1.0 MB/s");
        assert_eq!(format_bytes_per_sec(1073741824.0), "1.0 GB/s");
    }

    #[test]
    fn test_format_frequency() {
        assert_eq!(format_frequency(1500.0), "1.5 GHz");
        assert_eq!(format_frequency(1000.0), "1.0 GHz");
        assert_eq!(format_frequency(999.0), "999 MHz");
        assert_eq!(format_frequency(2500.0), "2.5 GHz");
        assert_eq!(format_frequency(0.0), "0 MHz");
    }

    #[test]
    fn test_render_bar() {
        let empty_bar = render_bar(0.0, 10);
        assert_eq!(empty_bar.chars().count(), 10);
        assert_eq!(empty_bar.chars().filter(|&c| c == '░').count(), 10);
        
        let full_bar = render_bar(100.0, 10);
        assert_eq!(full_bar.chars().count(), 10);
        assert_eq!(full_bar.chars().filter(|&c| c == '█').count(), 10);
        
        let half_bar = render_bar(50.0, 10);
        assert_eq!(half_bar.chars().count(), 10);
        assert_eq!(half_bar.chars().filter(|&c| c == '█').count(), 5);
        assert_eq!(half_bar.chars().filter(|&c| c == '░').count(), 5);
    }

    #[test]
    fn test_get_usage_color() {
        use ratatui::style::Color;
        assert_eq!(get_usage_color(0.0), Color::Green);
        assert_eq!(get_usage_color(50.0), Color::Green);
        assert_eq!(get_usage_color(60.0), Color::Yellow);
        assert_eq!(get_usage_color(75.0), Color::Yellow);
        assert_eq!(get_usage_color(79.9), Color::Yellow);
        assert_eq!(get_usage_color(100.0), Color::Red);
    }

    #[test]
    fn test_render_sparkline() {
        let empty = render_sparkline(&[], 10);
        assert_eq!(empty.chars().count(), 10);
        assert_eq!(render_sparkline(&[1.0, 2.0, 3.0], 0), "");
        
        let data = vec![0.0, 50.0, 100.0];
        let sparkline = render_sparkline(&data, 3);
        assert_eq!(sparkline.chars().count(), 3);
        assert!(sparkline.contains('░'));
        assert!(sparkline.contains('█'));
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0), "0m");
        assert_eq!(format_uptime(60), "1m");
        assert_eq!(format_uptime(3600), "1h 0m");
        assert_eq!(format_uptime(86400), "1d 0h 0m");
        assert_eq!(format_uptime(90061), "1d 1h 1m");
    }
}

