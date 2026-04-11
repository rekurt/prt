use crate::model::{ConnectionState, PortEntry, ProcessInfo, Protocol};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// (user, parent_pid, command, comm)
type PsInfo = HashMap<u32, (Option<String>, Option<u32>, Option<String>, Option<String>)>;

pub fn scan() -> Result<Vec<PortEntry>> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-iUDP", "-nP", "+c0", "-FnPpTtc"])
        .output()
        .context("failed to run lsof — is it installed?")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_lsof_output(&stdout)
}

pub fn has_elevated_access() -> bool {
    Command::new("sudo")
        .args(["-n", "true"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn scan_elevated() -> Result<Vec<PortEntry>> {
    let output = Command::new("sudo")
        .args(["-n", "lsof", "-iTCP", "-iUDP", "-nP", "+c0", "-FnPpTtc"])
        .output()
        .context("failed to run sudo lsof")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_lsof_output(&stdout)
    } else {
        scan()
    }
}

pub fn scan_with_sudo(password: &str) -> Result<Vec<PortEntry>> {
    let mut child = Command::new("sudo")
        .args(["-S", "lsof", "-iTCP", "-iUDP", "-nP", "+c0", "-FnPpTtc"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run sudo")?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "{password}");
    }

    let output = child.wait_with_output().context("sudo lsof failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("sudo: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_lsof_output(&stdout)
}

fn parse_lsof_output(output: &str) -> Result<Vec<PortEntry>> {
    // lsof -F field order per file descriptor: P (proto) → n (addr) → T (state)
    // State arrives AFTER the address, so we buffer the addr and flush on the
    // next fd/process boundary (f, p tags) or at end-of-output.
    let mut raw_entries: Vec<(String, u32, String, Protocol, ConnectionState)> = Vec::new();
    let mut current_pid: Option<u32> = None;
    let mut current_name = String::new();
    let mut current_proto = Protocol::Tcp;
    let mut current_state = ConnectionState::Unknown;
    let mut pending_addr: Option<String> = None;

    // Flush buffered addr + state into raw_entries
    let mut flush = |pending: &mut Option<String>,
                     pid: Option<u32>,
                     name: &str,
                     proto: Protocol,
                     state: &mut ConnectionState| {
        if let (Some(addr), Some(pid)) = (pending.take(), pid) {
            raw_entries.push((addr, pid, name.to_string(), proto, *state));
        }
        *state = ConnectionState::Unknown;
    };

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        let tag = line.as_bytes()[0];
        let value = &line[1..].trim_end_matches('\0');

        match tag {
            b'p' => {
                flush(
                    &mut pending_addr,
                    current_pid,
                    &current_name,
                    current_proto,
                    &mut current_state,
                );
                current_pid = value.parse().ok();
            }
            b'c' => {
                current_name = value.to_string();
            }
            b'f' => {
                // New file descriptor — flush previous fd's entry
                flush(
                    &mut pending_addr,
                    current_pid,
                    &current_name,
                    current_proto,
                    &mut current_state,
                );
            }
            b'P' => {
                current_proto = if value.eq_ignore_ascii_case("UDP") {
                    Protocol::Udp
                } else {
                    Protocol::Tcp
                };
            }
            b'n' => {
                // Buffer the address; state (T) will follow
                pending_addr = Some(value.to_string());
            }
            b'T' => {
                if let Some(st) = value.strip_prefix("ST=") {
                    current_state = parse_state(st);
                }
            }
            _ => {}
        }
    }

    // Flush last buffered entry
    flush(
        &mut pending_addr,
        current_pid,
        &current_name,
        current_proto,
        &mut current_state,
    );

    if raw_entries.is_empty() {
        return scan_fallback();
    }

    let pids: Vec<u32> = raw_entries
        .iter()
        .map(|(_, pid, _, _, _)| *pid)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let ps_info = batch_ps_info(&pids);

    let ppids: Vec<u32> = ps_info
        .values()
        .filter_map(|(_, ppid, _, _)| *ppid)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let parent_names = batch_parent_names(&ppids);

    let mut entries = Vec::new();
    let mut process_cache: HashMap<u32, ProcessInfo> = HashMap::new();

    for (addr_str, pid, name, proto, state) in &raw_entries {
        if let Some(entry) = parse_connection_line(
            addr_str,
            *pid,
            name,
            *proto,
            *state,
            &mut process_cache,
            &ps_info,
            &parent_names,
        ) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn scan_fallback() -> Result<Vec<PortEntry>> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-iUDP", "-nP"])
        .output()
        .context("failed to run lsof (fallback)")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    struct RawFallback {
        name: String,
        pid: u32,
        user: String,
        protocol: Protocol,
        local_addr: SocketAddr,
        remote_addr: Option<SocketAddr>,
        state: ConnectionState,
    }

    let mut raw: Vec<RawFallback> = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let name = parts[0].to_string();
        let pid: u32 = match parts[1].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let user = parts[2].to_string();
        let proto_str = parts[7];
        let addr_str = parts[8];
        let state_str = parts.get(9).copied().unwrap_or("");

        let protocol = match proto_str.to_lowercase().as_str() {
            s if s.starts_with("tcp") => Protocol::Tcp,
            s if s.starts_with("udp") => Protocol::Udp,
            _ => continue,
        };

        let (local_str, remote_str) = if let Some(pos) = addr_str.find("->") {
            (&addr_str[..pos], Some(&addr_str[pos + 2..]))
        } else {
            (addr_str, None)
        };

        let local_addr = match parse_addr(local_str) {
            Some(a) => a,
            None => continue,
        };
        let remote_addr = remote_str.and_then(parse_addr);
        let state = parse_state(state_str);

        raw.push(RawFallback {
            name,
            pid,
            user,
            protocol,
            local_addr,
            remote_addr,
            state,
        });
    }

    let pids: Vec<u32> = raw
        .iter()
        .map(|r| r.pid)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let ps_info = batch_ps_info(&pids);
    let ppids: Vec<u32> = ps_info
        .values()
        .filter_map(|(_, ppid, _, _)| *ppid)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let parent_names = batch_parent_names(&ppids);

    let entries = raw
        .into_iter()
        .map(|r| {
            let process = build_process_info(r.pid, &r.name, Some(r.user), &ps_info, &parent_names);
            PortEntry {
                protocol: r.protocol,
                local_addr: r.local_addr,
                remote_addr: r.remote_addr,
                state: r.state,
                process,
            }
        })
        .collect();

    Ok(entries)
}

#[allow(clippy::too_many_arguments)]
fn parse_connection_line(
    addr_str: &str,
    pid: u32,
    name: &str,
    protocol: Protocol,
    pre_state: ConnectionState,
    cache: &mut HashMap<u32, ProcessInfo>,
    ps_info: &PsInfo,
    parent_names: &HashMap<u32, String>,
) -> Option<PortEntry> {
    let (local_str, remote_str, state_str) = if let Some(arrow_pos) = addr_str.find("->") {
        let local = &addr_str[..arrow_pos];
        let rest = &addr_str[arrow_pos + 2..];
        if let Some(paren) = rest.find(" (") {
            let remote = &rest[..paren];
            let state = rest[paren + 2..].trim_end_matches(')');
            (local, Some(remote), state)
        } else {
            (local, Some(rest), "")
        }
    } else if let Some(paren) = addr_str.find(" (") {
        let local = &addr_str[..paren];
        let state = addr_str[paren + 2..].trim_end_matches(')');
        (local, None, state)
    } else {
        (addr_str, None, "")
    };

    let local_addr = parse_addr(local_str)?;
    let remote_addr = remote_str.and_then(parse_addr);

    // Use pre-parsed state from lsof T-field; fall back to addr_str parenthesized state
    let state = if pre_state != ConnectionState::Unknown {
        pre_state
    } else {
        parse_state(state_str)
    };

    let process = cache
        .entry(pid)
        .or_insert_with(|| build_process_info(pid, name, None, ps_info, parent_names))
        .clone();

    Some(PortEntry {
        protocol,
        local_addr,
        remote_addr,
        state,
        process,
    })
}

fn parse_addr(s: &str) -> Option<SocketAddr> {
    let s = s.trim();
    if s.is_empty() || s == "*:*" {
        return None;
    }

    let s = s.replace('*', "0.0.0.0");

    if let Ok(addr) = s.parse::<SocketAddr>() {
        return Some(addr);
    }

    if let Some(colon_pos) = s.rfind(':') {
        let host = &s[..colon_pos];
        let port: u16 = s[colon_pos + 1..].parse().ok()?;
        let host = if host.is_empty() { "0.0.0.0" } else { host };

        if let Ok(ip) = host.parse() {
            return Some(SocketAddr::new(ip, port));
        }

        if host.starts_with('[') && host.ends_with(']') {
            if let Ok(ip) = host[1..host.len() - 1].parse() {
                return Some(SocketAddr::new(ip, port));
            }
        }
    }

    None
}

fn parse_state(s: &str) -> ConnectionState {
    let s = s.trim().trim_start_matches('(').trim_end_matches(')');
    match s.to_uppercase().as_str() {
        "LISTEN" => ConnectionState::Listen,
        "ESTABLISHED" => ConnectionState::Established,
        "TIME_WAIT" => ConnectionState::TimeWait,
        "CLOSE_WAIT" => ConnectionState::CloseWait,
        "SYN_SENT" => ConnectionState::SynSent,
        "SYN_RECV" | "SYN_RECEIVED" => ConnectionState::SynRecv,
        "FIN_WAIT1" | "FIN_WAIT_1" => ConnectionState::FinWait1,
        "FIN_WAIT2" | "FIN_WAIT_2" => ConnectionState::FinWait2,
        "CLOSING" => ConnectionState::Closing,
        "LAST_ACK" => ConnectionState::LastAck,
        "CLOSED" => ConnectionState::Closed,
        _ => ConnectionState::Unknown,
    }
}

fn batch_ps_info(pids: &[u32]) -> PsInfo {
    let mut result = HashMap::new();
    if pids.is_empty() {
        return result;
    }

    let pid_list: Vec<String> = pids.iter().map(|p| p.to_string()).collect();
    let pid_arg = pid_list.join(",");

    let output = Command::new("ps")
        .args(["-p", &pid_arg, "-o", "pid=,user=,ppid=,comm=,command="])
        .output()
        .ok();

    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(5, char::is_whitespace).collect();
            if parts.len() < 4 {
                continue;
            }
            let pid: u32 = match parts[0].trim().parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let user = Some(parts[1].trim().to_string()).filter(|s| !s.is_empty());
            let ppid: Option<u32> = parts[2].trim().parse().ok();
            let comm = Some(parts[3].trim().to_string()).filter(|s| !s.is_empty());
            let command = parts
                .get(4)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            result.insert(pid, (user, ppid, command, comm));
        }
    }

    result
}

fn batch_parent_names(ppids: &[u32]) -> HashMap<u32, String> {
    let mut result = HashMap::new();
    if ppids.is_empty() {
        return result;
    }

    let pid_list: Vec<String> = ppids.iter().map(|p| p.to_string()).collect();
    let pid_arg = pid_list.join(",");

    let output = Command::new("ps")
        .args(["-p", &pid_arg, "-o", "pid=,comm="])
        .output()
        .ok();

    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.trim().splitn(2, char::is_whitespace).collect();
            if parts.len() == 2 {
                if let Ok(pid) = parts[0].trim().parse::<u32>() {
                    let name = parts[1].trim().to_string();
                    if !name.is_empty() {
                        result.insert(pid, name);
                    }
                }
            }
        }
    }

    result
}

fn build_process_info(
    pid: u32,
    name: &str,
    user: Option<String>,
    ps_info: &PsInfo,
    parent_names: &HashMap<u32, String>,
) -> ProcessInfo {
    let (ps_user, ppid, cmdline, comm) = ps_info.get(&pid).cloned().unwrap_or_default();

    let path = comm.as_ref().map(PathBuf::from).filter(|p| p.exists());

    ProcessInfo {
        pid,
        name: name.to_string(),
        path,
        cmdline,
        user: user.or(ps_user),
        parent_pid: ppid,
        parent_name: ppid.and_then(|pp| parent_names.get(&pp).cloned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn parse_addr_ipv4_with_port() {
        let addr = parse_addr("127.0.0.1:8080").unwrap();
        assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn parse_addr_wildcard() {
        let addr = parse_addr("*:443").unwrap();
        assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
        assert_eq!(addr.port(), 443);
    }

    #[test]
    fn parse_addr_star_star_returns_none() {
        assert!(parse_addr("*:*").is_none());
    }

    #[test]
    fn parse_addr_empty_returns_none() {
        assert!(parse_addr("").is_none());
    }

    #[test]
    fn parse_addr_ipv6_loopback() {
        let addr = parse_addr("[::1]:8080").unwrap();
        assert_eq!(addr.ip(), IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn parse_state_listen() {
        assert_eq!(parse_state("LISTEN"), ConnectionState::Listen);
    }

    #[test]
    fn parse_state_with_parens() {
        assert_eq!(parse_state("(LISTEN)"), ConnectionState::Listen);
    }

    #[test]
    fn parse_state_case_insensitive() {
        assert_eq!(parse_state("listen"), ConnectionState::Listen);
    }

    #[test]
    fn parse_state_unknown() {
        assert_eq!(parse_state(""), ConnectionState::Unknown);
        assert_eq!(parse_state("BOGUS"), ConnectionState::Unknown);
    }

    fn empty_ps() -> (PsInfo, HashMap<u32, String>) {
        (HashMap::new(), HashMap::new())
    }

    #[test]
    fn parse_connection_line_with_pre_state() {
        let mut cache = HashMap::new();
        let (ps, pn) = empty_ps();
        // State from lsof T-field takes priority
        let entry = parse_connection_line(
            "127.0.0.1:80->10.0.0.1:54321",
            1234,
            "nginx",
            Protocol::Tcp,
            ConnectionState::Established,
            &mut cache,
            &ps,
            &pn,
        )
        .unwrap();
        assert_eq!(entry.local_addr.port(), 80);
        assert_eq!(entry.remote_addr.unwrap().port(), 54321);
        assert_eq!(entry.state, ConnectionState::Established);
        assert_eq!(entry.protocol, Protocol::Tcp);
    }

    #[test]
    fn parse_connection_line_fallback_to_addr_state() {
        let mut cache = HashMap::new();
        let (ps, pn) = empty_ps();
        // When pre_state is Unknown, fall back to parenthesized state in addr
        let entry = parse_connection_line(
            "127.0.0.1:80->10.0.0.1:54321 (ESTABLISHED)",
            1234,
            "nginx",
            Protocol::Tcp,
            ConnectionState::Unknown,
            &mut cache,
            &ps,
            &pn,
        )
        .unwrap();
        assert_eq!(entry.state, ConnectionState::Established);
    }

    #[test]
    fn parse_connection_line_udp_protocol() {
        let mut cache = HashMap::new();
        let (ps, pn) = empty_ps();
        let entry = parse_connection_line(
            "*:5353",
            1234,
            "mDNSResponder",
            Protocol::Udp,
            ConnectionState::Unknown,
            &mut cache,
            &ps,
            &pn,
        )
        .unwrap();
        assert_eq!(entry.protocol, Protocol::Udp);
    }

    #[test]
    fn parse_connection_line_listen_from_t_field() {
        let mut cache = HashMap::new();
        let (ps, pn) = empty_ps();
        // Real lsof -F output: n field has no state, state comes from T field
        let entry = parse_connection_line(
            "*:443",
            1234,
            "nginx",
            Protocol::Tcp,
            ConnectionState::Listen,
            &mut cache,
            &ps,
            &pn,
        )
        .unwrap();
        assert_eq!(entry.local_addr.port(), 443);
        assert!(entry.remote_addr.is_none());
        assert_eq!(entry.state, ConnectionState::Listen);
    }

    #[test]
    fn parse_connection_line_caches_process_info() {
        let mut cache = HashMap::new();
        let (ps, pn) = empty_ps();
        let _ = parse_connection_line(
            "127.0.0.1:80",
            42,
            "test",
            Protocol::Tcp,
            ConnectionState::Listen,
            &mut cache,
            &ps,
            &pn,
        );
        assert!(cache.contains_key(&42));
        let _ = parse_connection_line(
            "127.0.0.1:81",
            42,
            "test",
            Protocol::Tcp,
            ConnectionState::Listen,
            &mut cache,
            &ps,
            &pn,
        );
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn parse_connection_line_invalid_addr_returns_none() {
        let mut cache = HashMap::new();
        let (ps, pn) = empty_ps();
        assert!(parse_connection_line(
            "not_valid",
            1,
            "x",
            Protocol::Tcp,
            ConnectionState::Unknown,
            &mut cache,
            &ps,
            &pn
        )
        .is_none());
    }
}
