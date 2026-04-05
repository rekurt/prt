//! Configuration loading from `~/.config/prt/config.toml`.
//!
//! The config is read-only — `prt` never writes to this file.
//! Missing file or parse errors fall back to defaults silently
//! (a warning is printed to stderr on parse failure).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Raw TOML representation (TOML table keys are always strings).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawConfig {
    known_ports: HashMap<String, String>,
    alerts: Vec<AlertRuleConfig>,
}

/// Top-level configuration.
#[derive(Debug, Clone, Default)]
pub struct PrtConfig {
    /// User-defined port → service name overrides.
    /// These take precedence over the built-in known ports database.
    ///
    /// ```toml
    /// [known_ports]
    /// 9090 = "prometheus"
    /// 3000 = "grafana"
    /// ```
    pub known_ports: HashMap<u16, String>,

    /// Alert rules (populated by the alerts feature).
    ///
    /// ```toml
    /// [[alerts]]
    /// port = 22
    /// action = "bell"
    /// ```
    pub alerts: Vec<AlertRuleConfig>,
}

impl From<RawConfig> for PrtConfig {
    fn from(raw: RawConfig) -> Self {
        let known_ports = raw
            .known_ports
            .into_iter()
            .filter_map(|(k, v)| k.parse::<u16>().ok().map(|port| (port, v)))
            .collect();
        Self {
            known_ports,
            alerts: raw.alerts,
        }
    }
}

/// A single alert rule from the TOML config.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AlertRuleConfig {
    pub port: Option<u16>,
    pub process: Option<String>,
    pub state: Option<String>,
    pub connections_gt: Option<usize>,
    #[serde(default = "default_action")]
    pub action: String,
}

fn default_action() -> String {
    "highlight".into()
}

/// Returns the config directory path: `~/.config/prt/`.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("prt"))
}

/// Returns the path to the main config file: `~/.config/prt/config.toml`.
pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// Load configuration from `~/.config/prt/config.toml`.
///
/// Returns [`PrtConfig::default()`] if the file does not exist.
/// Prints a warning to stderr and returns defaults if the file
/// exists but cannot be parsed.
pub fn load_config() -> PrtConfig {
    let path = match config_path() {
        Some(p) => p,
        None => return PrtConfig::default(),
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return PrtConfig::default();
        }
        Err(e) => {
            eprintln!("prt: warning: cannot read {}: {e}", path.display());
            return PrtConfig::default();
        }
    };

    match toml::from_str::<RawConfig>(&content) {
        Ok(raw) => raw.into(),
        Err(e) => {
            eprintln!("prt: warning: cannot parse {}: {e}", path.display());
            PrtConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        let config = PrtConfig::default();
        assert!(config.known_ports.is_empty());
        assert!(config.alerts.is_empty());
    }

    #[test]
    fn parse_known_ports() {
        let toml_str = r#"
[known_ports]
9090 = "prometheus"
3000 = "grafana"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let config: PrtConfig = raw.into();
        assert_eq!(config.known_ports.get(&9090).unwrap(), "prometheus");
        assert_eq!(config.known_ports.get(&3000).unwrap(), "grafana");
    }

    #[test]
    fn parse_alert_rules() {
        let toml_str = r#"
[[alerts]]
port = 22
action = "bell"

[[alerts]]
process = "python"
state = "LISTEN"
action = "highlight"

[[alerts]]
connections_gt = 100
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let config: PrtConfig = raw.into();
        assert_eq!(config.alerts.len(), 3);
        assert_eq!(config.alerts[0].port, Some(22));
        assert_eq!(config.alerts[0].action, "bell");
        assert_eq!(config.alerts[1].process.as_deref(), Some("python"));
        assert_eq!(config.alerts[2].connections_gt, Some(100));
        assert_eq!(config.alerts[2].action, "highlight"); // default
    }

    #[test]
    fn parse_empty_toml_returns_defaults() {
        let raw: RawConfig = toml::from_str("").unwrap();
        let config: PrtConfig = raw.into();
        assert!(config.known_ports.is_empty());
        assert!(config.alerts.is_empty());
    }

    #[test]
    fn parse_invalid_port_key_is_skipped() {
        let toml_str = r#"
[known_ports]
9090 = "prometheus"
not_a_port = "ignored"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let config: PrtConfig = raw.into();
        assert_eq!(config.known_ports.len(), 1);
        assert_eq!(config.known_ports.get(&9090).unwrap(), "prometheus");
    }

    #[test]
    fn load_config_returns_defaults_when_no_file() {
        // In test environment, config_path() likely points to a nonexistent file
        let config = load_config();
        assert!(config.known_ports.is_empty());
    }
}
