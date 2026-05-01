//! Lightweight `~/.ssh/config` parser + merge with prt's own host config.
//!
//! Only the directives needed to identify a destination are recognised:
//! `Host`, `HostName`, `User`, `Port`, `IdentityFile`. Everything else is
//! silently ignored. Wildcard host blocks (`Host *` / `Host foo?bar`) are
//! skipped — they match patterns rather than name a concrete target.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::SshHostConfig;

/// Where a host definition came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshHostSource {
    SshConfig,
    PrtConfig,
}

impl SshHostSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::SshConfig => "ssh_config",
            Self::PrtConfig => "prt",
        }
    }
}

/// One concrete SSH destination (no wildcards).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<PathBuf>,
    pub source: SshHostSource,
}

impl SshHost {
    /// Display string: `user@hostname:port` (with sensible fallbacks).
    pub fn target(&self) -> String {
        let host = self.hostname.as_deref().unwrap_or(&self.alias);
        let mut s = String::new();
        if let Some(u) = &self.user {
            s.push_str(u);
            s.push('@');
        }
        s.push_str(host);
        if let Some(p) = self.port {
            s.push(':');
            s.push_str(&p.to_string());
        }
        s
    }
}

/// Default `~/.ssh/config` path.
pub fn default_ssh_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ssh").join("config"))
}

/// Parse `~/.ssh/config` (or any file with that grammar). On failure, returns
/// an empty list — this is best-effort enrichment.
pub fn parse_ssh_config(path: &Path) -> Vec<SshHost> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    parse_ssh_config_str(&content)
}

fn parse_ssh_config_str(content: &str) -> Vec<SshHost> {
    let mut result: Vec<SshHost> = Vec::new();
    let mut current: Vec<usize> = Vec::new(); // indices into result for active aliases

    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (key, value) = match split_kv(trimmed) {
            Some(kv) => kv,
            None => continue,
        };
        let key_lc = key.to_ascii_lowercase();

        if key_lc == "host" {
            current.clear();
            for token in value.split_whitespace() {
                let alias = strip_quotes(token);
                if alias.is_empty() || alias.contains('*') || alias.contains('?') {
                    continue;
                }
                result.push(SshHost {
                    alias: alias.to_string(),
                    hostname: None,
                    user: None,
                    port: None,
                    identity_file: None,
                    source: SshHostSource::SshConfig,
                });
                current.push(result.len() - 1);
            }
            continue;
        }

        if current.is_empty() {
            continue;
        }
        let value = strip_quotes(value).to_string();
        for &idx in &current {
            let host = &mut result[idx];
            match key_lc.as_str() {
                "hostname" => host.hostname = Some(value.clone()),
                "user" => host.user = Some(value.clone()),
                "port" => {
                    if let Ok(p) = value.parse() {
                        host.port = Some(p);
                    }
                }
                "identityfile" => host.identity_file = Some(expand_tilde(&value)),
                _ => {}
            }
        }
    }

    result
}

fn split_kv(line: &str) -> Option<(&str, &str)> {
    // ssh_config(5): key and value separated by whitespace and/or '='.
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let key = &line[..i];
    let mut j = i;
    while j < bytes.len() && (bytes[j].is_ascii_whitespace() || bytes[j] == b'=') {
        j += 1;
    }
    if j >= bytes.len() {
        return None;
    }
    Some((key, line[j..].trim()))
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(s)
}

/// Convert a prt-config host entry to an `SshHost`.
pub fn from_prt_config(cfg: &SshHostConfig) -> Option<SshHost> {
    if cfg.alias.trim().is_empty() {
        return None;
    }
    Some(SshHost {
        alias: cfg.alias.clone(),
        hostname: cfg.hostname.clone(),
        user: cfg.user.clone(),
        port: cfg.port,
        identity_file: cfg.identity_file.as_ref().map(|p| expand_tilde(p)),
        source: SshHostSource::PrtConfig,
    })
}

/// Load known hosts: parse `~/.ssh/config` and merge with prt-config hosts.
/// Aliases from prt-config win on collision.
pub fn load_known_hosts(extra: &[SshHostConfig]) -> Vec<SshHost> {
    let mut hosts: Vec<SshHost> = match default_ssh_config_path() {
        Some(p) => parse_ssh_config(&p),
        None => Vec::new(),
    };

    for cfg in extra {
        if let Some(host) = from_prt_config(cfg) {
            if let Some(pos) = hosts.iter().position(|h| h.alias == host.alias) {
                hosts[pos] = host;
            } else {
                hosts.push(host);
            }
        }
    }

    hosts.sort_by(|a, b| a.alias.cmp(&b.alias));
    hosts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_host() {
        let cfg = "Host prod\n  HostName 10.0.0.5\n  User deploy\n  Port 2222\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        let h = &hosts[0];
        assert_eq!(h.alias, "prod");
        assert_eq!(h.hostname.as_deref(), Some("10.0.0.5"));
        assert_eq!(h.user.as_deref(), Some("deploy"));
        assert_eq!(h.port, Some(2222));
    }

    #[test]
    fn parse_skips_wildcards() {
        let cfg = "Host *\n  User everyone\nHost prod\n  HostName p\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "prod");
    }

    #[test]
    fn parse_handles_comments_and_indent() {
        let cfg = "# comment\n\n   Host foo\n     # nested\n     HostName f.example\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "foo");
        assert_eq!(hosts[0].hostname.as_deref(), Some("f.example"));
    }

    #[test]
    fn parse_multiple_aliases_share_block() {
        let cfg = "Host a b c\n  HostName shared\n  User root\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 3);
        for h in &hosts {
            assert_eq!(h.hostname.as_deref(), Some("shared"));
            assert_eq!(h.user.as_deref(), Some("root"));
        }
    }

    #[test]
    fn parse_case_insensitive_keys_and_equals() {
        let cfg = "Host abc\n  HOSTNAME=h.example\n  user=joe\n  PORT = 22\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("h.example"));
        assert_eq!(hosts[0].user.as_deref(), Some("joe"));
        assert_eq!(hosts[0].port, Some(22));
    }

    #[test]
    fn parse_quoted_values() {
        let cfg = "Host abc\n  HostName \"example.com\"\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts[0].hostname.as_deref(), Some("example.com"));
    }

    #[test]
    fn parse_unknown_keys_ignored() {
        let cfg = "Host foo\n  ProxyCommand whatever\n  HostName ok\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("ok"));
    }

    #[test]
    fn parse_empty_returns_empty() {
        assert!(parse_ssh_config_str("").is_empty());
    }

    #[test]
    fn parse_missing_file_returns_empty() {
        let path = PathBuf::from("/nonexistent/.ssh/config_xxx");
        assert!(parse_ssh_config(&path).is_empty());
    }

    #[test]
    fn merge_prt_config_overrides_ssh_config() {
        let prt = vec![SshHostConfig {
            alias: "prod".into(),
            hostname: Some("override".into()),
            user: None,
            port: None,
            identity_file: None,
        }];
        // Simulate by manually parsing then merging
        let mut hosts = parse_ssh_config_str("Host prod\n  HostName original\n");
        for cfg in &prt {
            if let Some(host) = from_prt_config(cfg) {
                if let Some(pos) = hosts.iter().position(|h| h.alias == host.alias) {
                    hosts[pos] = host;
                } else {
                    hosts.push(host);
                }
            }
        }
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("override"));
        assert_eq!(hosts[0].source, SshHostSource::PrtConfig);
    }

    #[test]
    fn target_formats_user_host_port() {
        let host = SshHost {
            alias: "prod".into(),
            hostname: Some("h".into()),
            user: Some("u".into()),
            port: Some(2222),
            identity_file: None,
            source: SshHostSource::SshConfig,
        };
        assert_eq!(host.target(), "u@h:2222");
    }

    #[test]
    fn target_falls_back_to_alias() {
        let host = SshHost {
            alias: "prod".into(),
            hostname: None,
            user: None,
            port: None,
            identity_file: None,
            source: SshHostSource::SshConfig,
        };
        assert_eq!(host.target(), "prod");
    }
}
