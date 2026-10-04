use crate::types::TimeFormat;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Telegram notification configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TelegramConfig {
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
    pub endpoint: String,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: None,
            chat_id: None,
            endpoint: "https://api.telegram.org".to_string(),
        }
    }
}

impl TelegramConfig {
    pub fn get_token(&self) -> Option<String> {
        self.bot_token
            .clone()
            .or_else(|| std::env::var("RMD_TELEGRAM_BOT_TOKEN").ok())
    }

    pub fn get_chat_id(&self) -> Option<String> {
        self.chat_id
            .clone()
            .or_else(|| std::env::var("RMD_TELEGRAM_CHAT_ID").ok())
    }

    pub fn get_endpoint(&self) -> String {
        std::env::var("RMD_TELEGRAM_ENDPOINT").unwrap_or_else(|_| self.endpoint.clone())
    }
}

/// Application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub time_format: TimeFormat,
    pub limit: usize,
    pub default_time: String,
    pub dbus_service: String,
    pub backends: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<TelegramConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            time_format: TimeFormat::Human,
            limit: 5,
            default_time: "09:00".to_string(),
            dbus_service: "org.freedesktop.Notifications".to_string(),
            backends: vec!["dbus".to_string()],
            telegram: None,
        }
    }
}

/// Configuration directory
pub fn get_config_dir() -> PathBuf {
    if let Ok(path) = std::env::var("RMD_CONFIG_DIR") {
        PathBuf::from(path)
    } else {
        dirs::config_dir()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("Cannot find home dir")
                    .join(".config")
            })
            .join("rmd")
    }
}

/// Get the config file path: $RMD_CONFIG_DIR/config.json or ~/.config/rmd/config.json
pub fn get_config_path() -> PathBuf {
    let dir = get_config_dir();
    std::fs::create_dir_all(&dir).ok();
    dir.join("config.json")
}

/// Load configuration from disk or return default
pub fn load_config() -> Config {
    let path = get_config_path();
    if !path.exists() {
        return Config::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

/// Save configuration to disk atomically
pub fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path();
    let tmp_path = path.with_extension("json.tmp");

    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(&tmp_path, data)?;

    let file = std::fs::File::open(&tmp_path)?;
    file.sync_all()?;
    std::fs::rename(tmp_path, path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = Config::default();
        assert_eq!(config.dbus_service, "org.freedesktop.Notifications");
        assert_eq!(config.limit, 5);
        assert_eq!(config.default_time, "09:00");
        assert_eq!(config.backends, vec!["dbus"]);
        assert!(config.telegram.is_none());
    }

    #[test]
    fn test_config_deserialization_fallback() {
        // Test that old JSON without dbus_service or backends populates default values
        let json_data = r#"{"limit": 10, "default_time": "10:00"}"#;
        let config: Config = serde_json::from_str(json_data).unwrap();
        assert_eq!(config.dbus_service, "org.freedesktop.Notifications");
        assert_eq!(config.limit, 10);
        assert_eq!(config.backends, vec!["dbus"]);
        assert!(config.telegram.is_none());
    }

    #[test]
    fn test_telegram_config_defaults_and_env() {
        let tg = TelegramConfig::default();
        assert_eq!(tg.endpoint, "https://api.telegram.org");
        assert_eq!(tg.get_endpoint(), "https://api.telegram.org");
        assert!(tg.get_token().is_none());
        assert!(tg.get_chat_id().is_none());
    }
}
