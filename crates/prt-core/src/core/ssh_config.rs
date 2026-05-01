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

/// Parse `~/.ssh/config` (or any file with that grammar). Resolves
/// `Include` directives relative to the SSH config root (the parent
/// directory of the top-level file, e.g. `~/.ssh/`), matching OpenSSH
/// semantics. Recursion is capped at [`MAX_INCLUDE_DEPTH`] so circular
/// includes don't loop. On failure, returns an empty list — this is
/// best-effort enrichment.
pub fn parse_ssh_config(path: &Path) -> Vec<SshHost> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut result: Vec<SshHost> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    parse_ssh_config_inner(&content, path.parent(), 0, &mut result, &mut current);
    result
}

#[cfg(test)]
fn parse_ssh_config_str(content: &str) -> Vec<SshHost> {
    let mut result: Vec<SshHost> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    parse_ssh_config_inner(content, None, 0, &mut result, &mut current);
    result
}

const MAX_INCLUDE_DEPTH: u32 = 16;

/// Recursive parser body.
///
/// `root_dir` is the SSH config root (the directory of the *top-level*
/// file) and stays the same across nested `Include` calls. Relative
/// include paths are resolved against it.
///
/// `result` and `current` are shared across recursion. When an `Include`
/// is encountered the outer `current` is snapshotted and a fresh copy is
/// passed into the nested parse — that way directives without their own
/// `Host` block in the included file still apply to the outer block,
/// while the outer `current` is restored unchanged after the include
/// returns (matching OpenSSH).
fn parse_ssh_config_inner(
    content: &str,
    root_dir: Option<&Path>,
    depth: u32,
    result: &mut Vec<SshHost>,
    current: &mut Vec<usize>,
) {
    for raw_line in content.lines() {
        let trimmed = strip_inline_comment(raw_line.trim());
        if trimmed.is_empty() {
            continue;
        }

        let (key, value) = match split_kv(trimmed) {
            Some(kv) => kv,
            None => continue,
        };
        let key_lc = key.to_ascii_lowercase();

        if key_lc == "include" {
            if depth >= MAX_INCLUDE_DEPTH {
                continue;
            }
            for token in value.split_whitespace() {
                let raw = strip_quotes(token);
                for include_path in resolve_include(raw, root_dir) {
                    if let Ok(included) = fs::read_to_string(&include_path) {
                        // Snapshot outer scope; nested file inherits it
                        // but cannot mutate the caller's selection.
                        let mut nested = current.clone();
                        parse_ssh_config_inner(&included, root_dir, depth + 1, result, &mut nested);
                    }
                }
            }
            // `current` deliberately left intact: directives following
            // an Include inside the same Host block still apply.
            continue;
        }

        if key_lc == "host" {
            current.clear();
            for token in value.split_whitespace() {
                let alias = strip_quotes(token);
                if alias.is_empty()
                    || alias.starts_with('!')
                    || alias.contains('*')
                    || alias.contains('?')
                {
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
        for &idx in current.iter() {
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
}

/// Resolve one `Include` token into a list of concrete file paths.
///
/// - `~/...` is expanded via `dirs::home_dir`.
/// - Relative paths are resolved against `root_dir` (the SSH config root,
///   typically `~/.ssh/` or `/etc/ssh/`), matching OpenSSH semantics —
///   including for nested includes.
/// - If the final path contains a `*` or `?` glob in its basename,
///   the parent directory is listed and entries matching the basename
///   pattern are returned. Globs in directory components are not supported
///   (rare in real configs).
fn resolve_include(raw: &str, root_dir: Option<&Path>) -> Vec<PathBuf> {
    if raw.is_empty() {
        return Vec::new();
    }
    let expanded = if let Some(rest) = raw.strip_prefix("~/") {
        match dirs::home_dir() {
            Some(h) => h.join(rest),
            None => return Vec::new(),
        }
    } else {
        let p = PathBuf::from(raw);
        if p.is_absolute() {
            p
        } else {
            match root_dir {
                Some(b) => b.join(p),
                None => p,
            }
        }
    };

    let basename = match expanded.file_name().and_then(|s| s.to_str()) {
        Some(s) => s.to_string(),
        None => return Vec::new(),
    };

    if !basename.contains('*') && !basename.contains('?') {
        return vec![expanded];
    }

    let parent = match expanded.parent() {
        Some(p) => p,
        None => return Vec::new(),
    };
    let read = match fs::read_dir(parent) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in read.flatten() {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        if match_glob(&basename, name_str) {
            out.push(entry.path());
        }
    }
    out.sort();
    out
}

/// Minimal fnmatch-style matcher: `*` matches any sequence (including empty),
/// `?` matches exactly one character. No bracket classes, no escaping.
fn match_glob(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    fn rec(p: &[char], n: &[char]) -> bool {
        match p.first() {
            None => n.is_empty(),
            Some('*') => {
                if rec(&p[1..], n) {
                    return true;
                }
                if let Some((_, rest)) = n.split_first() {
                    rec(p, rest)
                } else {
                    false
                }
            }
            Some('?') => {
                if let Some((_, rest)) = n.split_first() {
                    rec(&p[1..], rest)
                } else {
                    false
                }
            }
            Some(c) => match n.split_first() {
                Some((nc, rest)) if nc == c => rec(&p[1..], rest),
                _ => false,
            },
        }
    }
    rec(&p, &n)
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

/// Drop everything from the first unquoted `#` onward and trim trailing
/// whitespace. OpenSSH treats `#` as the start of a comment anywhere on a
/// line, including after a directive value (e.g. `Port 22 # ssh`).
fn strip_inline_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut in_quotes = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_quotes = !in_quotes,
            b'#' if !in_quotes => return s[..i].trim_end(),
            _ => {}
        }
        i += 1;
    }
    s
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
    fn parse_skips_negated_aliases() {
        let cfg = "Host !bastion good\n  HostName ok\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "good");
    }

    #[test]
    fn parse_resolves_include_directive() {
        // Build a small fixture: parent file with an `Include` pointing at
        // a sibling fragment.
        let dir = tmpdir();
        let frag = dir.join("frag.conf");
        std::fs::write(&frag, "Host included-alias\n  HostName included.example\n").unwrap();
        let main = dir.join("config");
        std::fs::write(
            &main,
            format!("Host top\n  HostName t\nInclude {}\n", frag.display()),
        )
        .unwrap();

        let hosts = parse_ssh_config(&main);
        let aliases: Vec<_> = hosts.iter().map(|h| h.alias.as_str()).collect();
        assert!(aliases.contains(&"top"), "{aliases:?}");
        assert!(aliases.contains(&"included-alias"), "{aliases:?}");
    }

    #[test]
    fn parse_include_with_glob_pattern() {
        let dir = tmpdir();
        let sub = dir.join("conf.d");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("a.conf"), "Host a\n  HostName ah\n").unwrap();
        std::fs::write(sub.join("b.conf"), "Host b\n  HostName bh\n").unwrap();
        std::fs::write(sub.join("ignore.txt"), "garbage\n").unwrap();

        let main = dir.join("config");
        std::fs::write(&main, format!("Include {}/*.conf\n", sub.display())).unwrap();

        let hosts = parse_ssh_config(&main);
        let aliases: Vec<_> = hosts.iter().map(|h| h.alias.as_str()).collect();
        assert!(aliases.contains(&"a"));
        assert!(aliases.contains(&"b"));
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn parse_include_resolves_relative_to_ssh_root() {
        // Layout:
        //   <root>/config       — top-level (root_dir = <root>)
        //   <root>/sibling.conf — referenced as `Include sibling.conf`
        //                         from inside <root>/sub/nested.conf
        //   <root>/sub/nested.conf — referenced from top-level
        //
        // OpenSSH resolves relative includes against the SSH config root
        // (parent of top-level file), regardless of where the include
        // statement appears. Resolving against the parent of the *current*
        // file would look for <root>/sub/sibling.conf and miss the alias.
        let dir = tmpdir();
        let sub = dir.join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(dir.join("sibling.conf"), "Host sibling\n  HostName s\n").unwrap();
        std::fs::write(
            sub.join("nested.conf"),
            "Host inner\n  HostName i\nInclude sibling.conf\n",
        )
        .unwrap();
        let main = dir.join("config");
        std::fs::write(
            &main,
            format!("Include {}\n", sub.join("nested.conf").display()),
        )
        .unwrap();

        let hosts = parse_ssh_config(&main);
        let aliases: Vec<_> = hosts.iter().map(|h| h.alias.as_str()).collect();
        assert!(aliases.contains(&"inner"), "{aliases:?}");
        assert!(
            aliases.contains(&"sibling"),
            "relative Include resolved against wrong root: {aliases:?}"
        );
    }

    #[test]
    fn parse_include_inside_host_keeps_block_active() {
        // `Host prod` is followed by `Include` and then `Port 2222`.
        // The trailing Port must apply to prod despite the Include.
        let dir = tmpdir();
        let frag = dir.join("frag.conf");
        std::fs::write(&frag, "Host other\n  HostName o\n").unwrap();
        let main = dir.join("config");
        std::fs::write(
            &main,
            format!(
                "Host prod\n  HostName p\nInclude {}\n  Port 2222\n  User deploy\n",
                frag.display()
            ),
        )
        .unwrap();

        let hosts = parse_ssh_config(&main);
        let prod = hosts
            .iter()
            .find(|h| h.alias == "prod")
            .expect("prod missing");
        assert_eq!(prod.hostname.as_deref(), Some("p"));
        assert_eq!(prod.port, Some(2222), "Port lost after Include");
        assert_eq!(
            prod.user.as_deref(),
            Some("deploy"),
            "User lost after Include"
        );

        // Included `Host other` must not contaminate prod nor disappear.
        let other = hosts
            .iter()
            .find(|h| h.alias == "other")
            .expect("other missing");
        assert_eq!(other.hostname.as_deref(), Some("o"));
        // Outer Port must NOT have leaked into the included host.
        assert_eq!(other.port, None);
    }

    #[test]
    fn match_glob_basics() {
        assert!(match_glob("*.conf", "a.conf"));
        assert!(match_glob("*.conf", ".conf"));
        assert!(!match_glob("*.conf", "a.txt"));
        assert!(match_glob("?.conf", "a.conf"));
        assert!(!match_glob("?.conf", "ab.conf"));
        assert!(match_glob("a*b", "axyzb"));
        assert!(match_glob("a*", "abc"));
        assert!(match_glob("*", "anything"));
    }

    fn tmpdir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "prt-ssh-cfg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parse_strips_inline_comments() {
        let cfg = "Host prod # primary db\n  HostName 10.0.0.5  # internal\n  Port 22 # ssh\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "prod");
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.0.5"));
        assert_eq!(hosts[0].port, Some(22));
    }

    #[test]
    fn parse_keeps_hash_inside_quotes() {
        let cfg = "Host abc\n  HostName \"h#1.example\"\n";
        let hosts = parse_ssh_config_str(cfg);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("h#1.example"));
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
