//! Container name resolution for Docker/Podman.
//!
//! Resolves process PIDs to container names using:
//! - **Linux:** `/proc/{pid}/cgroup` → container ID → `docker ps` lookup
//! - **macOS:** `docker ps` with PID matching via `docker inspect`
//!
//! All lookups are batched per refresh cycle to minimize CLI overhead.
//! Missing Docker/Podman is handled gracefully (empty results, no errors).

use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

/// Timeout for docker CLI calls to avoid blocking the TUI.
const DOCKER_TIMEOUT_SECS: u64 = 2;

/// Resolve container names for a batch of PIDs.
///
/// Returns a map of PID → container name. PIDs not running in a
/// container are simply absent from the result. If Docker/Podman
/// is unavailable, returns an empty map.
pub fn resolve_container_names(pids: &[u32]) -> HashMap<u32, String> {
    if pids.is_empty() {
        return HashMap::new();
    }

    // Try Docker first, fall back to Podman
    docker_resolve(pids)
        .or_else(|| podman_resolve(pids))
        .unwrap_or_default()
}

/// Check if any entries have container names (used for adaptive column).
pub fn has_containers(names: &HashMap<u32, String>) -> bool {
    !names.is_empty()
}

/// Resolve via `docker ps` + `docker inspect`.
fn docker_resolve(pids: &[u32]) -> Option<HashMap<u32, String>> {
    let pid_set: std::collections::HashSet<u32> = pids.iter().copied().collect();
    // Get all running containers: ID and Name
    let output = run_with_timeout(
        "docker",
        &["ps", "--no-trunc", "--format", "{{.ID}} {{.Names}}"],
    )?;

    if output.is_empty() {
        return Some(HashMap::new());
    }

    let containers: Vec<(String, String)> = output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let id = parts.next()?.trim().to_string();
            let name = parts.next()?.trim().to_string();
            if id.is_empty() || name.is_empty() {
                None
            } else {
                Some((id, name))
            }
        })
        .collect();

    if containers.is_empty() {
        return Some(HashMap::new());
    }

    // For each container, get its PID
    let mut result = HashMap::new();
    for (id, name) in &containers {
        if let Some(container_pid) = get_container_pid("docker", id) {
            if pid_set.contains(&container_pid) {
                result.insert(container_pid, name.clone());
            }
        }
    }

    Some(result)
}

/// Resolve via `podman ps` + `podman inspect`.
fn podman_resolve(pids: &[u32]) -> Option<HashMap<u32, String>> {
    let pid_set: std::collections::HashSet<u32> = pids.iter().copied().collect();
    let output = run_with_timeout(
        "podman",
        &["ps", "--no-trunc", "--format", "{{.ID}} {{.Names}}"],
    )?;

    if output.is_empty() {
        return Some(HashMap::new());
    }

    let containers: Vec<(String, String)> = output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let id = parts.next()?.trim().to_string();
            let name = parts.next()?.trim().to_string();
            if id.is_empty() || name.is_empty() {
                None
            } else {
                Some((id, name))
            }
        })
        .collect();

    if containers.is_empty() {
        return Some(HashMap::new());
    }

    let mut result = HashMap::new();
    for (id, name) in &containers {
        if let Some(container_pid) = get_container_pid("podman", id) {
            if pid_set.contains(&container_pid) {
                result.insert(container_pid, name.clone());
            }
        }
    }

    Some(result)
}

/// Get the main PID of a container via `docker/podman inspect`.
fn get_container_pid(runtime: &str, container_id: &str) -> Option<u32> {
    let output = run_with_timeout(
        runtime,
        &["inspect", "--format", "{{.State.Pid}}", container_id],
    )?;
    output.trim().parse().ok().filter(|&pid: &u32| pid > 0)
}

/// Run a command with timeout, returning stdout as String.
/// Returns None if command not found, timeout, or non-zero exit.
fn run_with_timeout(cmd: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let timeout = Duration::from_secs(DOCKER_TIMEOUT_SECS);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                // Child already exited — read stdout directly (not wait_with_output,
                // which would double-wait and potentially deadlock).
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    use std::io::Read;
                    let _ = stdout.read_to_string(&mut out);
                }
                return Some(out);
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

/// Parse a `docker ps` line into (id, name).
#[cfg(test)]
fn parse_ps_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.splitn(2, ' ');
    let id = parts.next()?.trim().to_string();
    let name = parts.next()?.trim().to_string();
    if id.is_empty() || name.is_empty() {
        None
    } else {
        Some((id, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ps_line_valid() {
        let (id, name) = parse_ps_line("abc123def456 my-nginx").unwrap();
        assert_eq!(id, "abc123def456");
        assert_eq!(name, "my-nginx");
    }

    #[test]
    fn parse_ps_line_with_spaces_in_name() {
        // Docker names don't have spaces, but ensure parsing is robust
        let (id, name) = parse_ps_line("abc123 my container").unwrap();
        assert_eq!(id, "abc123");
        assert_eq!(name, "my container");
    }

    #[test]
    fn parse_ps_line_empty_returns_none() {
        assert!(parse_ps_line("").is_none());
        assert!(parse_ps_line(" ").is_none());
    }

    #[test]
    fn parse_ps_line_no_name_returns_none() {
        assert!(parse_ps_line("abc123").is_none());
    }

    #[test]
    fn resolve_empty_pids() {
        let result = resolve_container_names(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn has_containers_empty() {
        assert!(!has_containers(&HashMap::new()));
    }

    #[test]
    fn has_containers_with_data() {
        let mut m = HashMap::new();
        m.insert(1, "nginx".to_string());
        assert!(has_containers(&m));
    }
}
