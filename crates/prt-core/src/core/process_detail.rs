//! Enhanced process detail: cwd, environment, open files, CPU/RAM.
//!
//! Used by the Connection tab (or a dedicated Info tab) to show
//! deeper info about the selected process. Data is fetched lazily —
//! only when the detail panel is visible and the selected PID changes.
//!
//! - **Linux:** reads `/proc/{pid}/cwd`, `/proc/{pid}/environ`, `/proc/{pid}/fd/*`, `/proc/{pid}/stat`
//! - **macOS:** uses `lsof -p {pid}` and `ps -o %cpu,rss -p {pid}`

use std::path::PathBuf;

/// Detailed information about a single process, fetched on demand.
#[derive(Debug, Clone)]
pub struct ProcessDetail {
    pub cwd: Option<PathBuf>,
    pub env_vars: Vec<(String, String)>,
    pub open_files: Vec<String>,
    pub cpu_percent: Option<f32>,
    pub rss_kb: Option<u64>,
}

/// Fetch detailed process information for the given PID.
/// Returns `None` if the process no longer exists.
/// Individual fields gracefully degrade to `None`/empty on permission errors.
pub fn fetch(pid: u32) -> Option<ProcessDetail> {
    if cfg!(target_os = "linux") {
        fetch_linux(pid)
    } else if cfg!(target_os = "macos") {
        fetch_macos(pid)
    } else {
        Some(ProcessDetail {
            cwd: None,
            env_vars: Vec::new(),
            open_files: Vec::new(),
            cpu_percent: None,
            rss_kb: None,
        })
    }
}

/// Linux: read directly from /proc/{pid}/
#[allow(dead_code)]
fn fetch_linux(pid: u32) -> Option<ProcessDetail> {
    let proc_dir = PathBuf::from(format!("/proc/{pid}"));
    if !proc_dir.exists() {
        return None;
    }

    let cwd = std::fs::read_link(proc_dir.join("cwd")).ok();

    let env_vars = std::fs::read(proc_dir.join("environ"))
        .ok()
        .map(|data| parse_environ(&data))
        .unwrap_or_default();

    let open_files = read_fd_links(pid);

    let (cpu_percent, rss_kb) = parse_proc_stat_status(pid);

    Some(ProcessDetail {
        cwd,
        env_vars,
        open_files,
        cpu_percent,
        rss_kb,
    })
}

/// Read /proc/{pid}/fd/* symlinks to list open files.
#[allow(dead_code)]
fn read_fd_links(pid: u32) -> Vec<String> {
    let fd_dir = format!("/proc/{pid}/fd");
    let entries = match std::fs::read_dir(&fd_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut files = Vec::new();
    for entry in entries.flatten() {
        if let Ok(target) = std::fs::read_link(entry.path()) {
            let s = target.to_string_lossy().to_string();
            // Skip anonymous pipes, sockets without paths
            if !s.starts_with("pipe:") && !s.starts_with("anon_inode:") {
                files.push(s);
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

/// Parse /proc/{pid}/environ (NUL-separated KEY=VALUE pairs).
#[allow(dead_code)]
fn parse_environ(data: &[u8]) -> Vec<(String, String)> {
    data.split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .filter_map(|s| {
            let s = String::from_utf8_lossy(s);
            let eq = s.find('=')?;
            Some((s[..eq].to_string(), s[eq + 1..].to_string()))
        })
        .collect()
}

/// Parse CPU% from /proc/{pid}/stat and RSS from /proc/{pid}/status.
#[allow(dead_code)]
fn parse_proc_stat_status(pid: u32) -> (Option<f32>, Option<u64>) {
    // RSS from /proc/{pid}/status (VmRSS line, in kB)
    let rss_kb = std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|content| {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("VmRSS:") {
                    let num_str = rest.trim().trim_end_matches(" kB").trim();
                    return num_str.parse::<u64>().ok();
                }
            }
            None
        });

    // CPU% — we'd need two samples to compute; use ps as a simpler approach
    let cpu_percent = std::process::Command::new("ps")
        .args(["-o", "%cpu=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.trim().parse::<f32>().ok()
        });

    (cpu_percent, rss_kb)
}

/// macOS: use lsof and ps commands.
#[allow(dead_code)]
fn fetch_macos(pid: u32) -> Option<ProcessDetail> {
    // Check process exists
    let exists = std::process::Command::new("ps")
        .args(["-p", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !exists {
        return None;
    }

    let cwd = std::process::Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()
        .and_then(|o| parse_lsof_cwd(&String::from_utf8_lossy(&o.stdout)));

    let open_files = std::process::Command::new("lsof")
        .args(["-p", &pid.to_string(), "-Fn"])
        .output()
        .ok()
        .map(|o| parse_lsof_files(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();

    let (cpu_percent, rss_kb) = std::process::Command::new("ps")
        .args(["-o", "%cpu=,rss=", "-p", &pid.to_string()])
        .output()
        .ok()
        .map(|o| parse_ps_cpu_rss(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or((None, None));

    // environ not easily accessible on macOS without SIP issues
    let env_vars = Vec::new();

    Some(ProcessDetail {
        cwd,
        env_vars,
        open_files,
        cpu_percent,
        rss_kb,
    })
}

/// Parse `lsof -d cwd -Fn` output for the cwd path.
/// Lines: `p<pid>`, `fcwd`, `n<path>`
fn parse_lsof_cwd(output: &str) -> Option<PathBuf> {
    let mut in_cwd = false;
    for line in output.lines() {
        if line == "fcwd" {
            in_cwd = true;
        } else if in_cwd && line.starts_with('n') {
            return Some(PathBuf::from(&line[1..]));
        } else if line.starts_with('f') {
            in_cwd = false;
        }
    }
    None
}

/// Parse `lsof -Fn` output for file names.
fn parse_lsof_files(output: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in output.lines() {
        if let Some(path) = line.strip_prefix('n') {
            // Skip internal pseudo-entries
            if !path.is_empty() && !path.starts_with("->") && path != "pipe" {
                files.push(path.to_string());
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

/// Parse `ps -o %cpu=,rss=` output.
fn parse_ps_cpu_rss(output: &str) -> (Option<f32>, Option<u64>) {
    let trimmed = output.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    let cpu = parts.first().and_then(|s| s.parse::<f32>().ok());
    let rss = parts.get(1).and_then(|s| s.parse::<u64>().ok());
    (cpu, rss)
}

/// Format RSS in human-readable form.
pub fn format_rss(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1} GB", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{:.1} MB", kb as f64 / 1024.0)
    } else {
        format!("{kb} KB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_environ_basic() {
        let data = b"HOME=/home/user\0PATH=/usr/bin\0LANG=en_US\0";
        let vars = parse_environ(data);
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0], ("HOME".to_string(), "/home/user".to_string()));
        assert_eq!(vars[1], ("PATH".to_string(), "/usr/bin".to_string()));
    }

    #[test]
    fn parse_environ_empty() {
        let vars = parse_environ(b"");
        assert!(vars.is_empty());
    }

    #[test]
    fn parse_environ_value_with_equals() {
        let data = b"FOO=bar=baz\0";
        let vars = parse_environ(data);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0], ("FOO".to_string(), "bar=baz".to_string()));
    }

    #[test]
    fn parse_lsof_cwd_valid() {
        let output = "p1234\nfcwd\nn/home/user/project\nf0\nn/dev/null\n";
        let cwd = parse_lsof_cwd(output);
        assert_eq!(cwd, Some(PathBuf::from("/home/user/project")));
    }

    #[test]
    fn parse_lsof_cwd_missing() {
        let output = "p1234\nf0\nn/dev/null\n";
        assert_eq!(parse_lsof_cwd(output), None);
    }

    #[test]
    fn parse_lsof_files_basic() {
        let output = "p1234\nf0\nn/dev/null\nf1\nn/home/user/file.txt\nf2\nn/tmp/log\n";
        let files = parse_lsof_files(output);
        assert!(files.contains(&"/dev/null".to_string()));
        assert!(files.contains(&"/home/user/file.txt".to_string()));
    }

    #[test]
    fn parse_ps_cpu_rss_valid() {
        let (cpu, rss) = parse_ps_cpu_rss("  3.2 12345\n");
        assert!((cpu.unwrap() - 3.2).abs() < 0.01);
        assert_eq!(rss, Some(12345));
    }

    #[test]
    fn parse_ps_cpu_rss_empty() {
        let (cpu, rss) = parse_ps_cpu_rss("");
        assert!(cpu.is_none());
        assert!(rss.is_none());
    }

    #[test]
    fn format_rss_kilobytes() {
        assert_eq!(format_rss(512), "512 KB");
    }

    #[test]
    fn format_rss_megabytes() {
        assert_eq!(format_rss(2048), "2.0 MB");
    }

    #[test]
    fn format_rss_gigabytes() {
        assert_eq!(format_rss(2_097_152), "2.0 GB");
    }
}
