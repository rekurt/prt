//! Strace/dtruss attachment for live syscall monitoring.
//!
//! Spawns `strace` (Linux) or `dtruss` (macOS) in a background thread
//! and streams output via `mpsc::channel`. The TUI polls for new lines.

use std::collections::VecDeque;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// Maximum number of strace output lines to buffer.
const MAX_LINES: usize = 1000;

/// An active strace/dtruss session.
pub struct StraceSession {
    pid: u32,
    child: Child,
    rx: mpsc::Receiver<String>,
    pub lines: VecDeque<String>,
}

impl StraceSession {
    /// Attach strace/dtruss to a running process.
    pub fn attach(pid: u32) -> Result<Self, String> {
        let (cmd, args) = if cfg!(target_os = "linux") {
            (
                "strace",
                vec![
                    "-p".to_string(),
                    pid.to_string(),
                    "-e".to_string(),
                    "trace=network".to_string(),
                    "-f".to_string(),
                ],
            )
        } else if cfg!(target_os = "macos") {
            ("dtruss", vec!["-p".to_string(), pid.to_string()])
        } else {
            return Err("unsupported platform".into());
        };

        let mut child = Command::new(cmd)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to start {cmd}: {e}"))?;

        let (tx, rx) = mpsc::channel();

        // Read stderr in background thread
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "failed to capture stderr".to_string())?;
        thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break; // receiver dropped
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            pid,
            child,
            rx,
            lines: VecDeque::with_capacity(MAX_LINES),
        })
    }

    /// Poll for new lines from the background thread.
    pub fn poll(&mut self) {
        while let Ok(line) = self.rx.try_recv() {
            self.lines.push_back(line);
            if self.lines.len() > MAX_LINES {
                self.lines.pop_front();
            }
        }
    }

    /// Check if the tracer process is still alive.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// The PID being traced.
    pub fn traced_pid(&self) -> u32 {
        self.pid
    }

    /// Detach: kill the tracer process.
    pub fn detach(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for StraceSession {
    fn drop(&mut self) {
        self.detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_lines_constant() {
        assert_eq!(MAX_LINES, 1000);
    }

    #[test]
    fn deque_respects_capacity() {
        let mut buf = VecDeque::with_capacity(MAX_LINES);
        for i in 0..1500 {
            buf.push_back(format!("line {i}"));
            if buf.len() > MAX_LINES {
                buf.pop_front();
            }
        }
        assert_eq!(buf.len(), MAX_LINES);
    }
}
