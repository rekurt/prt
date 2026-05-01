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

/// Resolved SSH connection settings used to expand a spec into concrete
/// command-line flags. Lets `[[ssh_hosts]]` aliases that don't appear in
/// `~/.ssh/config` actually resolve at the OS level.
#[derive(Debug, Clone, Default)]
pub struct ResolvedHost<'a> {
    pub hostname: Option<&'a str>,
    pub user: Option<&'a str>,
    pub port: Option<u16>,
    pub identity_file: Option<&'a str>,
}

impl SshTunnelSpec {
    /// `-N -L LOCAL:host:PORT` / `-N -D LOCAL` — without the trailing host arg.
    fn forward_args(&self) -> Vec<String> {
        match self.kind {
            TunnelKind::Local => {
                let host = self.remote_host.as_deref().unwrap_or("localhost");
                let port = self.remote_port.unwrap_or(0);
                vec![
                    "-N".into(),
                    "-L".into(),
                    format!("{}:{}:{}", self.local_port, host, port),
                ]
            }
            TunnelKind::Dynamic => {
                vec!["-N".into(), "-D".into(), self.local_port.to_string()]
            }
        }
    }

    /// Build the argument list passed to `ssh`.
    /// Always includes `-N` (no remote command). Uses only `host_alias`
    /// — relies on `~/.ssh/config` (or DNS) to resolve it.
    pub fn ssh_args(&self) -> Vec<String> {
        let mut args = self.forward_args();
        args.push(self.host_alias.clone());
        args
    }

    /// Like [`ssh_args`] but injects `-l user`, `-p port`, `-i identity_file`
    /// from a resolved host, and uses `hostname` (when provided) as the
    /// positional target so prt-config-only aliases resolve correctly.
    pub fn ssh_args_with(&self, host: &ResolvedHost<'_>) -> Vec<String> {
        let mut args = self.forward_args();
        if let Some(u) = host.user {
            args.push("-l".into());
            args.push(u.into());
        }
        if let Some(p) = host.port {
            args.push("-p".into());
            args.push(p.to_string());
        }
        if let Some(id) = host.identity_file {
            args.push("-i".into());
            args.push(id.into());
        }
        let target = host.hostname.unwrap_or(self.host_alias.as_str());
        args.push(target.into());
        args
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
    fn ssh_args_with_resolved_host_local() {
        let spec = local_spec();
        let host = ResolvedHost {
            hostname: Some("real.example.com"),
            user: Some("deploy"),
            port: Some(2222),
            identity_file: Some("/home/u/.ssh/id"),
        };
        let args = spec.ssh_args_with(&host);
        assert_eq!(
            args,
            vec![
                "-N",
                "-L",
                "5433:127.0.0.1:5432",
                "-l",
                "deploy",
                "-p",
                "2222",
                "-i",
                "/home/u/.ssh/id",
                "real.example.com",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ssh_args_with_empty_host_falls_back_to_alias() {
        let spec = local_spec();
        let host = ResolvedHost::default();
        let args = spec.ssh_args_with(&host);
        // No -l/-p/-i and the alias is the positional target.
        assert_eq!(args.last().map(String::as_str), Some("prod"));
        assert!(!args.contains(&"-l".to_string()));
        assert!(!args.contains(&"-p".to_string()));
    }

    #[test]
    fn kind_serde_lowercase() {
        let s: TunnelKind = serde_json::from_str("\"local\"").unwrap();
        assert_eq!(s, TunnelKind::Local);
        let s: TunnelKind = serde_json::from_str("\"dynamic\"").unwrap();
        assert_eq!(s, TunnelKind::Dynamic);
    }
}
