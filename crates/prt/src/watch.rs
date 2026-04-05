//! Watch mode: `prt watch 3000 8080` — compact UP/DOWN monitor.
//!
//! Non-TUI loop that prints colored status for watched ports.
//! BEL character on state transitions (UP↔DOWN).

use anyhow::Result;
use prt_core::core::scanner;
use prt_core::model::TICK_RATE;
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};

/// Current state of a watched port.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PortState {
    up: bool,
    process_name: Option<String>,
    pid: Option<u32>,
}

/// Run the watch loop for the given ports. Never returns unless
/// interrupted or a write error occurs.
pub fn run_watch(ports: Vec<u16>) -> Result<()> {
    let is_tty = io::stdout().is_terminal();
    let mut stdout = io::stdout().lock();
    let mut prev_states: HashMap<u16, bool> = HashMap::new();

    loop {
        let entries = scanner::scan()?;

        // Clear screen if TTY
        if is_tty {
            write!(stdout, "\x1b[2J\x1b[H")?;
        }

        let mut any_changed = false;

        for &port in &ports {
            let matching: Vec<_> = entries
                .iter()
                .filter(|e| e.local_addr.port() == port)
                .collect();

            let state = if let Some(entry) = matching.first() {
                PortState {
                    up: true,
                    process_name: Some(entry.process.name.clone()),
                    pid: Some(entry.process.pid),
                }
            } else {
                PortState {
                    up: false,
                    process_name: None,
                    pid: None,
                }
            };

            // Detect transitions
            let prev_up = prev_states.get(&port).copied();
            if prev_up.is_some() && prev_up != Some(state.up) {
                any_changed = true;
            }
            prev_states.insert(port, state.up);

            // Format output
            if is_tty {
                if state.up {
                    let name = state.process_name.as_deref().unwrap_or("?");
                    let pid = state.pid.unwrap_or(0);
                    writeln!(
                        stdout,
                        "\x1b[32m:{port:<5} \u{25cf} UP   \x1b[0m {name} ({pid})"
                    )?;
                } else {
                    writeln!(stdout, "\x1b[31m:{port:<5} \u{25cb} DOWN\x1b[0m")?;
                }
            } else {
                // Plain text for piped output
                if state.up {
                    let name = state.process_name.as_deref().unwrap_or("?");
                    let pid = state.pid.unwrap_or(0);
                    writeln!(stdout, ":{port} UP {name} ({pid})")?;
                } else {
                    writeln!(stdout, ":{port} DOWN")?;
                }
            }
        }

        // BEL on state change
        if any_changed && is_tty {
            write!(stdout, "\x07")?;
        }

        stdout.flush()?;
        std::thread::sleep(TICK_RATE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_state_equality() {
        let a = PortState {
            up: true,
            process_name: Some("nginx".into()),
            pid: Some(123),
        };
        let b = PortState {
            up: true,
            process_name: Some("nginx".into()),
            pid: Some(123),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn port_state_up_vs_down() {
        let up = PortState {
            up: true,
            process_name: Some("nginx".into()),
            pid: Some(1),
        };
        let down = PortState {
            up: false,
            process_name: None,
            pid: None,
        };
        assert_ne!(up, down);
    }

    #[test]
    fn detect_state_changes() {
        let mut prev: HashMap<u16, bool> = HashMap::new();
        prev.insert(80, true);
        prev.insert(443, true);

        // Port 80 stays up, port 443 goes down
        let current_states = [(80u16, true), (443u16, false)];

        let mut changed = false;
        for &(port, up) in &current_states {
            let prev_up = prev.get(&port).copied();
            if prev_up.is_some() && prev_up != Some(up) {
                changed = true;
            }
        }
        assert!(changed, "should detect 443 went down");
    }

    #[test]
    fn no_change_detected_on_first_run() {
        let prev: HashMap<u16, bool> = HashMap::new();
        let current_states = [(80u16, true)];

        let mut changed = false;
        for &(port, up) in &current_states {
            let prev_up = prev.get(&port).copied();
            if prev_up.is_some() && prev_up != Some(up) {
                changed = true;
            }
        }
        assert!(!changed, "first run should not trigger bell");
    }
}
