use crate::model::{ConnectionState, PortEntry, ProcessInfo, Protocol};
use anyhow::{Context, Result};
use procfs::net::{TcpNetEntry, TcpState, UdpNetEntry};
use procfs::process::Process;
use std::collections::HashMap;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::process::{Command, Stdio};

pub fn scan() -> Result<Vec<PortEntry>> {
    let mut entries = Vec::new();
    let pid_map = build_inode_pid_map()?;

    if let Ok(tcp) = procfs::net::tcp() {
        for e in tcp {
            if let Some(entry) = tcp_entry_to_port_entry(&e, Protocol::Tcp, &pid_map) {
                entries.push(entry);
            }
        }
    }
    if let Ok(tcp6) = procfs::net::tcp6() {
        for e in tcp6 {
            if let Some(entry) = tcp_entry_to_port_entry(&e, Protocol::Tcp, &pid_map) {
                entries.push(entry);
            }
        }
    }
    if let Ok(udp) = procfs::net::udp() {
        for e in udp {
            if let Some(entry) = udp_entry_to_port_entry(&e, &pid_map) {
                entries.push(entry);
            }
        }
    }
    if let Ok(udp6) = procfs::net::udp6() {
        for e in udp6 {
            if let Some(entry) = udp_entry_to_port_entry(&e, &pid_map) {
                entries.push(entry);
            }
        }
    }

    Ok(entries)
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

pub fn scan_with_sudo(password: &str) -> Result<Vec<PortEntry>> {
    let mut child = Command::new("sudo")
        .args(["-S", "-v"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run sudo")?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "{password}");
    }

    let output = child.wait_with_output().context("sudo validation failed")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("sudo: {}", stderr.trim());
    }

    scan()
}

fn build_inode_pid_map() -> Result<HashMap<u64, u32>> {
    let mut map = HashMap::new();
    if let Ok(procs) = procfs::process::all_processes() {
        for proc_result in procs {
            let Ok(proc) = proc_result else { continue };
            let pid = proc.pid() as u32;
            if let Ok(fds) = proc.fd() {
                for fd_result in fds {
                    let Ok(fd) = fd_result else { continue };
                    if let procfs::process::FDTarget::Socket(inode) = fd.target {
                        map.insert(inode, pid);
                    }
                }
            }
        }
    }
    Ok(map)
}

fn process_info_from_pid(pid: u32) -> ProcessInfo {
    let proc = Process::new(pid as i32);
    let (name, path, cmdline, parent_pid, parent_name, user) = match proc {
        Ok(p) => {
            let name = p.stat().map(|s| s.comm.clone()).unwrap_or_default();
            let path = p.exe().ok();
            let cmdline = p.cmdline().ok().map(|c| c.join(" "));
            let uid = p.uid().ok();
            let user = uid.and_then(|u| {
                uzers::get_user_by_uid(u).map(|user| user.name().to_string_lossy().into_owned())
            });
            let ppid = p.stat().ok().map(|s| s.ppid as u32);
            let pname = ppid.and_then(|pp| {
                Process::new(pp as i32)
                    .ok()
                    .and_then(|p| p.stat().ok().map(|s| s.comm.clone()))
            });
            (name, path, cmdline, ppid, pname, user)
        }
        Err(_) => (String::new(), None, None, None, None, None),
    };

    ProcessInfo {
        pid,
        name,
        path,
        cmdline,
        user,
        parent_pid,
        parent_name,
    }
}

fn tcp_state_to_connection_state(state: TcpState) -> ConnectionState {
    match state {
        TcpState::Established => ConnectionState::Established,
        TcpState::SynSent => ConnectionState::SynSent,
        TcpState::SynRecv => ConnectionState::SynRecv,
        TcpState::FinWait1 => ConnectionState::FinWait1,
        TcpState::FinWait2 => ConnectionState::FinWait2,
        TcpState::TimeWait => ConnectionState::TimeWait,
        TcpState::Close => ConnectionState::Closed,
        TcpState::CloseWait => ConnectionState::CloseWait,
        TcpState::LastAck => ConnectionState::LastAck,
        TcpState::Listen => ConnectionState::Listen,
        TcpState::Closing => ConnectionState::Closing,
        _ => ConnectionState::Unknown,
    }
}

fn tcp_entry_to_port_entry(
    e: &TcpNetEntry,
    proto: Protocol,
    pid_map: &HashMap<u64, u32>,
) -> Option<PortEntry> {
    let pid = pid_map.get(&e.inode).copied()?;
    let local_addr = e.local_address;
    let remote = e.remote_address;
    let remote_addr = if remote.port() == 0
        && (remote.ip() == IpAddr::V4(Ipv4Addr::UNSPECIFIED)
            || remote.ip() == IpAddr::V6(Ipv6Addr::UNSPECIFIED))
    {
        None
    } else {
        Some(remote)
    };

    Some(PortEntry {
        protocol: proto,
        local_addr,
        remote_addr,
        state: tcp_state_to_connection_state(e.state.clone()),
        process: process_info_from_pid(pid),
    })
}

fn udp_entry_to_port_entry(e: &UdpNetEntry, pid_map: &HashMap<u64, u32>) -> Option<PortEntry> {
    let pid = pid_map.get(&e.inode).copied()?;
    let local_addr = e.local_address;
    let remote = e.remote_address;
    let remote_addr = if remote.port() == 0
        && (remote.ip() == IpAddr::V4(Ipv4Addr::UNSPECIFIED)
            || remote.ip() == IpAddr::V6(Ipv6Addr::UNSPECIFIED))
    {
        None
    } else {
        Some(remote)
    };

    Some(PortEntry {
        protocol: Protocol::Udp,
        local_addr,
        remote_addr,
        state: ConnectionState::Unknown,
        process: process_info_from_pid(pid),
    })
}
