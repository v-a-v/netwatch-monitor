use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    1000
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExternalIpConfig {
    #[serde(default = "default_ifconfig_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_whois_endpoint")]
    pub whois_endpoint: String,
    #[serde(default = "default_check_interval")]
    pub check_interval_sec: u64,
}

fn default_ifconfig_endpoint() -> String {
    "https://ifconfig.io/ip".to_string()
}

fn default_whois_endpoint() -> String {
    "https://ifconfig.io/whois/".to_string()
}

fn default_check_interval() -> u64 {
    300
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_interval")]
    pub interval: u64,
    #[serde(default = "default_history_size")]
    pub history_size: usize,
    #[serde(default)]
    pub servers: Vec<ServerConfig>,
    #[serde(default)]
    pub external_ip: ExternalIpConfig,
}

fn default_interval() -> u64 {
    2
}

fn default_history_size() -> usize {
    60
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default() -> anyhow::Result<Self> {
        let default_config = Config {
            interval: default_interval(),
            history_size: default_history_size(),
            servers: vec![
                ServerConfig {
                    name: "Google DNS".to_string(),
                    host: "8.8.8.8".to_string(),
                    timeout_ms: default_timeout(),
                },
                ServerConfig {
                    name: "Cloudflare DNS".to_string(),
                    host: "1.1.1.1".to_string(),
                    timeout_ms: default_timeout(),
                },
                ServerConfig {
                    name: "Yandex DNS".to_string(),
                    host: "77.88.8.8".to_string(),
                    timeout_ms: default_timeout(),
                },
            ],
            external_ip: ExternalIpConfig::default(),
        };

        // Check in order of priority
        if let Ok(config) = Self::load("config.toml") {
            Ok(config)
        } else if let Ok(config) = Self::load("config.toml.example") {
            Ok(config)
        } else if let Ok(config) = Self::load(
            dirs::home_dir()
                .map(|h| h.join(".config/netwatch/config.toml"))
                .unwrap_or_else(|| std::path::PathBuf::from("config.toml"))
        ) {
            Ok(config)
        } else if let Ok(config) = Self::load("/etc/netwatch/config.toml") {
            Ok(config)
        } else {
            Ok(default_config)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            interval: default_interval(),
            history_size: default_history_size(),
            servers: vec![],
            external_ip: ExternalIpConfig::default(),
        }
    }
}
