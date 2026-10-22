use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "dmeter")]
#[command(version)]
#[command(about = "A fast, beautiful terminal system monitor", long_about = None)]
pub struct CliArgs {
    /// Refresh interval in seconds [default: 2, or the value in config.toml]
    #[arg(short, long, value_parser = clap::value_parser!(u64).range(1..))]
    pub interval: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
                    Err(e) => eprintln!("dmeter: failed to parse {}: {}", config_path.display(), e),
                },
                Err(e) => eprintln!("dmeter: failed to read {}: {}", config_path.display(), e),
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
        if let Some(interval) = cli.interval {
            self.interval = interval;
        }
        // An interval of 0 would make the collector loop without sleeping
        self.interval = self.interval.max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_defaults() {
        let args = CliArgs::parse_from(["dmeter"]);
        assert_eq!(args.interval, None);
    }

    #[test]
    fn test_cli_rejects_zero_interval() {
        assert!(CliArgs::try_parse_from(["dmeter", "-i", "0"]).is_err());
    }

    #[test]
    fn test_config_file_interval_kept_without_cli() {
        let mut config: Config = toml::from_str("interval = 7").unwrap();
        config.merge_cli(&CliArgs { interval: None });
        assert_eq!(config.interval, 7);
    }

    #[test]
    fn test_config_zero_interval_clamped() {
        let mut config: Config = toml::from_str("interval = 0").unwrap();
        config.merge_cli(&CliArgs { interval: None });
        assert_eq!(config.interval, 1);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.interval, 2);
    }

    #[test]
    fn test_config_merge_cli() {
        let mut config = Config::default();
        let cli = CliArgs { interval: Some(5) };
        
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
        let config: Config = toml::from_str("").unwrap();

        assert_eq!(config.interval, 2);
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
