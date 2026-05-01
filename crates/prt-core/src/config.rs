//! Configuration loading from `~/.config/prt/config.toml`.
//!
//! The config is mostly read-only — `prt` only writes to it through
//! [`write_tunnels`] when the user explicitly requests "save tunnels".
//! Missing file or parse errors fall back to defaults silently
//! (a warning is printed to stderr on parse failure).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::core::ssh_tunnel::{SshTunnelSpec, TunnelKind};

/// Raw TOML representation (TOML table keys are always strings).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawConfig {
    known_ports: HashMap<String, String>,
    alerts: Vec<AlertRuleConfig>,
    ssh_hosts: Vec<SshHostConfig>,
    ssh_tunnels: Vec<SshTunnelConfig>,
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

    /// Saved SSH hosts (in addition to `~/.ssh/config`).
    pub ssh_hosts: Vec<SshHostConfig>,

    /// Saved SSH tunnels (auto-restore on launch).
    pub ssh_tunnels: Vec<SshTunnelConfig>,
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
            ssh_hosts: raw.ssh_hosts,
            ssh_tunnels: raw.ssh_tunnels,
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

/// User-defined SSH host (extends `~/.ssh/config`).
///
/// ```toml
/// [[ssh_hosts]]
/// alias = "prod-db"
/// hostname = "10.0.1.5"
/// user = "deploy"
/// port = 22
/// identity_file = "~/.ssh/id_ed25519_prod"
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SshHostConfig {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
}

/// Saved SSH tunnel.
///
/// ```toml
/// [[ssh_tunnels]]
/// name = "prod-postgres"
/// kind = "local"
/// local_port = 5433
/// remote_host = "127.0.0.1"
/// remote_port = 5432
/// host_alias = "prod-db"
/// ```
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SshTunnelConfig {
    pub name: Option<String>,
    #[serde(default = "default_tunnel_kind")]
    pub kind: String,
    pub local_port: u16,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
    pub host_alias: String,
}

fn default_tunnel_kind() -> String {
    "local".into()
}

impl SshTunnelConfig {
    /// Convert to runtime [`SshTunnelSpec`]. Returns `None` for unknown kinds
    /// or invalid combinations (logged at the call site, not here).
    pub fn to_spec(&self) -> Option<SshTunnelSpec> {
        let kind = match self.kind.to_ascii_lowercase().as_str() {
            "local" => TunnelKind::Local,
            "dynamic" => TunnelKind::Dynamic,
            _ => return None,
        };
        Some(SshTunnelSpec {
            name: self.name.clone(),
            kind,
            local_port: self.local_port,
            remote_host: self.remote_host.clone(),
            remote_port: self.remote_port,
            host_alias: self.host_alias.clone(),
        })
    }

    pub fn from_spec(spec: &SshTunnelSpec) -> Self {
        Self {
            name: spec.name.clone(),
            kind: spec.kind.label().into(),
            local_port: spec.local_port,
            remote_host: spec.remote_host.clone(),
            remote_port: spec.remote_port,
            host_alias: spec.host_alias.clone(),
        }
    }
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

/// Persist `[[ssh_tunnels]]` to the config file at `path`.
///
/// Reads any existing TOML, strips all existing `[[ssh_tunnels]]` blocks,
/// and appends fresh ones rebuilt from `specs`. The rest of the file is
/// preserved verbatim. Creates the file (and parent directory) if missing.
pub fn write_tunnels(path: &Path, specs: &[SshTunnelSpec]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Only fall back to empty content when the file genuinely doesn't exist.
    // Other I/O errors (permission denied, transient disk error, decoding)
    // must propagate so we don't blow away unrelated sections like
    // `known_ports` / `alerts` / `ssh_hosts`.
    let existing = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let stripped = strip_ssh_tunnels_section(&existing);

    let configs: Vec<SshTunnelConfig> = specs.iter().map(SshTunnelConfig::from_spec).collect();

    #[derive(Serialize)]
    struct Wrap<'a> {
        ssh_tunnels: &'a [SshTunnelConfig],
    }
    let appended = if configs.is_empty() {
        String::new()
    } else {
        toml::to_string(&Wrap {
            ssh_tunnels: &configs,
        })
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    };

    let mut out = stripped.trim_end().to_string();
    if !out.is_empty() {
        out.push('\n');
        out.push('\n');
    }
    out.push_str(&appended);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    std::fs::write(path, out)
}

/// Remove every `[[ssh_tunnels]]` block (including its key/value lines)
/// from a raw TOML string. A block runs until the next `[`-prefixed line
/// or EOF. Comments belonging to the block are removed too.
fn strip_ssh_tunnels_section(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut skipping = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[[ssh_tunnels]]") || trimmed.starts_with("[ssh_tunnels]") {
            skipping = true;
            continue;
        }
        if skipping {
            // A new top-level table or array-of-tables ends the skipped block.
            if trimmed.starts_with('[') {
                skipping = false;
            } else {
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        let config = PrtConfig::default();
        assert!(config.known_ports.is_empty());
        assert!(config.alerts.is_empty());
        assert!(config.ssh_hosts.is_empty());
        assert!(config.ssh_tunnels.is_empty());
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
    fn parse_ssh_hosts() {
        let toml_str = r#"
[[ssh_hosts]]
alias = "prod"
hostname = "10.0.0.5"
user = "deploy"
port = 22
identity_file = "~/.ssh/id_ed25519"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let config: PrtConfig = raw.into();
        assert_eq!(config.ssh_hosts.len(), 1);
        assert_eq!(config.ssh_hosts[0].alias, "prod");
        assert_eq!(config.ssh_hosts[0].hostname.as_deref(), Some("10.0.0.5"));
        assert_eq!(config.ssh_hosts[0].port, Some(22));
    }

    #[test]
    fn parse_ssh_tunnels() {
        let toml_str = r#"
[[ssh_tunnels]]
name = "pg"
kind = "local"
local_port = 5433
remote_host = "127.0.0.1"
remote_port = 5432
host_alias = "prod"

[[ssh_tunnels]]
kind = "dynamic"
local_port = 1080
host_alias = "prod"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let config: PrtConfig = raw.into();
        assert_eq!(config.ssh_tunnels.len(), 2);
        let s0 = config.ssh_tunnels[0].to_spec().unwrap();
        assert_eq!(s0.kind, TunnelKind::Local);
        assert_eq!(s0.local_port, 5433);
        let s1 = config.ssh_tunnels[1].to_spec().unwrap();
        assert_eq!(s1.kind, TunnelKind::Dynamic);
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

    #[test]
    fn strip_ssh_tunnels_preserves_other_sections() {
        let content = r#"
[known_ports]
9090 = "prom"

[[alerts]]
port = 22

[[ssh_tunnels]]
name = "old"
kind = "local"
local_port = 1
remote_host = "x"
remote_port = 1
host_alias = "y"

[[ssh_hosts]]
alias = "z"
"#;
        let stripped = strip_ssh_tunnels_section(content);
        assert!(stripped.contains("[known_ports]"));
        assert!(stripped.contains("[[alerts]]"));
        assert!(stripped.contains("[[ssh_hosts]]"));
        assert!(!stripped.contains("[[ssh_tunnels]]"));
        assert!(!stripped.contains("\"old\""));
    }

    #[test]
    fn write_tunnels_roundtrip() {
        let dir = tempdir();
        let path = dir.join("config.toml");

        let specs = vec![
            SshTunnelSpec {
                name: Some("pg".into()),
                kind: TunnelKind::Local,
                local_port: 5433,
                remote_host: Some("127.0.0.1".into()),
                remote_port: Some(5432),
                host_alias: "prod".into(),
            },
            SshTunnelSpec {
                name: None,
                kind: TunnelKind::Dynamic,
                local_port: 1080,
                remote_host: None,
                remote_port: None,
                host_alias: "prod".into(),
            },
        ];
        write_tunnels(&path, &specs).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let raw: RawConfig = toml::from_str(&content).unwrap();
        let cfg: PrtConfig = raw.into();
        assert_eq!(cfg.ssh_tunnels.len(), 2);
        let s0 = cfg.ssh_tunnels[0].to_spec().unwrap();
        assert_eq!(s0.local_port, 5433);
        assert_eq!(s0.kind, TunnelKind::Local);
        let s1 = cfg.ssh_tunnels[1].to_spec().unwrap();
        assert_eq!(s1.kind, TunnelKind::Dynamic);

        // Re-write replaces, doesn't duplicate.
        write_tunnels(&path, &specs[..1]).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        let raw: RawConfig = toml::from_str(&content).unwrap();
        let cfg: PrtConfig = raw.into();
        assert_eq!(cfg.ssh_tunnels.len(), 1);
    }

    #[test]
    fn write_tunnels_propagates_read_errors() {
        // A directory in place of the config file forces read_to_string to
        // fail with a non-NotFound error (IsADirectory / Other on Linux).
        let dir = tempdir();
        let path = dir.join("not-a-file");
        std::fs::create_dir(&path).unwrap();

        let specs = vec![SshTunnelSpec {
            name: None,
            kind: TunnelKind::Local,
            local_port: 1,
            remote_host: Some("h".into()),
            remote_port: Some(2),
            host_alias: "a".into(),
        }];
        let err = write_tunnels(&path, &specs).expect_err("should not silently overwrite");
        assert_ne!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn write_tunnels_preserves_other_sections() {
        let dir = tempdir();
        let path = dir.join("config.toml");
        let initial = "[known_ports]\n9090 = \"prom\"\n\n[[alerts]]\nport = 22\n";
        std::fs::write(&path, initial).unwrap();

        let specs = vec![SshTunnelSpec {
            name: None,
            kind: TunnelKind::Local,
            local_port: 1,
            remote_host: Some("h".into()),
            remote_port: Some(2),
            host_alias: "a".into(),
        }];
        write_tunnels(&path, &specs).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[known_ports]"));
        assert!(content.contains("[[alerts]]"));
        assert!(content.contains("[[ssh_tunnels]]"));
    }

    fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "prt-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            n,
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
