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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: 2,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_defaults() {
        let args = CliArgs { interval: 2 };
        assert_eq!(args.interval, 2);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.interval, 2);
    }

    #[test]
    fn test_config_merge_cli() {
        let mut config = Config::default();
        let cli = CliArgs { interval: 5 };
        
        config.merge_cli(&cli);
        
        assert_eq!(config.interval, 5);
    }

    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
interval = 3
"#;
        
        let config: Config = toml::from_str(toml_str).unwrap();
        
        assert_eq!(config.interval, 3);
    }

    #[test]
    fn test_config_partial_toml() {
        let partial_toml = r#"
interval = 4
"#;
        
        let config: Config = toml::from_str(partial_toml).unwrap();
        
        assert_eq!(config.interval, 4);
    }

    #[test]
    fn test_config_invalid_interval() {
        let invalid_toml = r#"
interval = -1
"#;
        
        let result: Result<Config, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }
}
