//! SSH tunnel specification — a serializable description of one tunnel.
//!
//! This module is process-agnostic: it only describes *what* tunnel to spawn.
//! Actual `ssh` subprocess handling lives in the binary crate (`prt::forward`).

use serde::{Deserialize, Serialize};

/// Type of SSH tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelKind {
    /// `ssh -L LOCAL:remote_host:REMOTE host` — bring a remote service to localhost.
    Local,
    /// `ssh -D LOCAL host` — SOCKS5 proxy on localhost.
    Dynamic,
}

impl TunnelKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Dynamic => "dynamic",
        }
    }
}

/// Description of one SSH tunnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTunnelSpec {
    /// Optional human-friendly name.
    pub name: Option<String>,
    pub kind: TunnelKind,
    /// Local port that ssh will bind on `localhost`.
    pub local_port: u16,
    /// Remote target host (only used for `Local`). For `Dynamic`, ignored.
    pub remote_host: Option<String>,
    /// Remote target port (only used for `Local`).
    pub remote_port: Option<u16>,
    /// SSH host argument (alias from `~/.ssh/config` or `user@host`).
    pub host_alias: String,
}

impl SshTunnelSpec {
    /// Build the argument list passed to `ssh`.
    /// Always includes `-N` (no remote command).
    pub fn ssh_args(&self) -> Vec<String> {
        match self.kind {
            TunnelKind::Local => {
                let host = self.remote_host.as_deref().unwrap_or("localhost");
                let port = self.remote_port.unwrap_or(0);
                vec![
                    "-N".into(),
                    "-L".into(),
                    format!("{}:{}:{}", self.local_port, host, port),
                    self.host_alias.clone(),
                ]
            }
            TunnelKind::Dynamic => vec![
                "-N".into(),
                "-D".into(),
                self.local_port.to_string(),
                self.host_alias.clone(),
            ],
        }
    }

    /// Human-readable one-line summary.
    pub fn summary(&self) -> String {
        match self.kind {
            TunnelKind::Local => {
                let host = self.remote_host.as_deref().unwrap_or("?");
                let port = self
                    .remote_port
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "?".into());
                format!(
                    "L localhost:{} \u{2192} {}:{}:{}",
                    self.local_port, self.host_alias, host, port
                )
            }
            TunnelKind::Dynamic => format!(
                "D socks5://localhost:{} \u{2192} {}",
                self.local_port, self.host_alias
            ),
        }
    }

    /// Validate that the spec is internally consistent.
    pub fn validate(&self) -> Result<(), String> {
        if self.host_alias.trim().is_empty() {
            return Err("host_alias is empty".into());
        }
        if self.local_port == 0 {
            return Err("local_port must be > 0".into());
        }
        if self.kind == TunnelKind::Local {
            if self
                .remote_host
                .as_deref()
                .map(str::is_empty)
                .unwrap_or(true)
            {
                return Err("remote_host required for Local tunnel".into());
            }
            match self.remote_port {
                Some(p) if p > 0 => {}
                _ => return Err("remote_port required for Local tunnel".into()),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_spec() -> SshTunnelSpec {
        SshTunnelSpec {
            name: Some("pg".into()),
            kind: TunnelKind::Local,
            local_port: 5433,
            remote_host: Some("127.0.0.1".into()),
            remote_port: Some(5432),
            host_alias: "prod".into(),
        }
    }

    fn dynamic_spec() -> SshTunnelSpec {
        SshTunnelSpec {
            name: None,
            kind: TunnelKind::Dynamic,
            local_port: 1080,
            remote_host: None,
            remote_port: None,
            host_alias: "prod".into(),
        }
    }

    #[test]
    fn local_args() {
        let args = local_spec().ssh_args();
        assert_eq!(
            args,
            vec!["-N", "-L", "5433:127.0.0.1:5432", "prod"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn dynamic_args() {
        let args = dynamic_spec().ssh_args();
        assert_eq!(
            args,
            vec!["-N", "-D", "1080", "prod"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn summary_local_contains_endpoints() {
        let s = local_spec().summary();
        assert!(s.contains("5433"));
        assert!(s.contains("prod"));
        assert!(s.contains("127.0.0.1"));
        assert!(s.contains("5432"));
    }

    #[test]
    fn summary_dynamic_mentions_socks() {
        let s = dynamic_spec().summary();
        assert!(s.contains("1080"));
        assert!(s.to_lowercase().contains("socks"));
        assert!(s.contains("prod"));
    }

    #[test]
    fn validate_local_ok_and_errors() {
        assert!(local_spec().validate().is_ok());

        let mut bad = local_spec();
        bad.host_alias = "".into();
        assert!(bad.validate().is_err());

        let mut bad = local_spec();
        bad.local_port = 0;
        assert!(bad.validate().is_err());

        let mut bad = local_spec();
        bad.remote_host = None;
        assert!(bad.validate().is_err());

        let mut bad = local_spec();
        bad.remote_port = None;
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_dynamic_ok_with_no_remote() {
        assert!(dynamic_spec().validate().is_ok());
    }

    #[test]
    fn kind_serde_lowercase() {
        let s: TunnelKind = serde_json::from_str("\"local\"").unwrap();
        assert_eq!(s, TunnelKind::Local);
        let s: TunnelKind = serde_json::from_str("\"dynamic\"").unwrap();
        assert_eq!(s, TunnelKind::Dynamic);
    }
}
