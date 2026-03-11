use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "dmeter")]
#[command(version = "0.1.0")]
#[command(about = "A fast, beautiful terminal system monitor", long_about = None)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = 2, help = "Refresh interval in seconds")]
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub interval: u64,
    pub show_swap: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: 2,
            show_swap: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();

        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => return config,
                    Err(e) => log::warn!("Failed to parse config: {}", e),
                },
                Err(e) => log::warn!("Failed to read config: {}", e),
            }
        }

        Self::default()
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dmeter")
            .join("config.toml")
    }

    pub fn merge_cli(&mut self, cli: &CliArgs) {
        self.interval = cli.interval;
    }
}
