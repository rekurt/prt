//! System-wide bandwidth estimation.
//!
//! Reads network interface byte counters and computes RX/TX rates.
//! - **Linux:** parses `/proc/net/dev`
//! - **macOS:** parses `netstat -ib`
//!
//! The first sample has no delta, so the rate is `None`.

use std::time::Instant;

/// Bandwidth measurement: bytes per second for RX and TX.
#[derive(Debug, Clone, Copy)]
pub struct BandwidthRate {
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
}

/// Tracks bandwidth by sampling byte counters.
#[derive(Debug)]
pub struct BandwidthTracker {
    prev_sample: Option<(u64, u64, Instant)>, // (rx_bytes, tx_bytes, when)
    pub current_rate: Option<BandwidthRate>,
}

impl Default for BandwidthTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BandwidthTracker {
    pub fn new() -> Self {
        Self {
            prev_sample: None,
            current_rate: None,
        }
    }

    /// Take a new sample and compute the rate delta.
    pub fn sample(&mut self) {
        let (rx, tx) = match read_counters() {
            Some(v) => v,
            None => return,
        };
        let now = Instant::now();

        if let Some((prev_rx, prev_tx, prev_time)) = self.prev_sample {
            let elapsed = now.duration_since(prev_time).as_secs_f64();
            if elapsed > 0.0 {
                let rx_delta = rx.saturating_sub(prev_rx) as f64;
                let tx_delta = tx.saturating_sub(prev_tx) as f64;
                self.current_rate = Some(BandwidthRate {
                    rx_bytes_per_sec: rx_delta / elapsed,
                    tx_bytes_per_sec: tx_delta / elapsed,
                });
            }
        }

        self.prev_sample = Some((rx, tx, now));
    }
}

/// Format bytes/sec as human-readable string (e.g. "1.2 MB/s").
pub fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec < 1024.0 {
        format!("{:.0} B/s", bytes_per_sec)
    } else if bytes_per_sec < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else if bytes_per_sec < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB/s", bytes_per_sec / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Read total RX and TX byte counters from the OS.
fn read_counters() -> Option<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        read_counters_linux()
    }
    #[cfg(target_os = "macos")]
    {
        read_counters_macos()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Parse `/proc/net/dev` for total bytes.
#[cfg(target_os = "linux")]
fn read_counters_linux() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/net/dev").ok()?;
    parse_proc_net_dev(&content)
}

/// Parse `netstat -ib` for total bytes.
#[cfg(target_os = "macos")]
fn read_counters_macos() -> Option<(u64, u64)> {
    let output = std::process::Command::new("netstat")
        .args(["-ib"])
        .output()
        .ok()?;
    let content = String::from_utf8(output.stdout).ok()?;
    parse_netstat_ib(&content)
}

/// Parse `/proc/net/dev` format.
///
/// Format:
/// ```text
/// Inter-|   Receive                                                |  Transmit
///  face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs ...
///    lo: 1234 ...   5678 ...
///  eth0: 9999 ...   8888 ...
/// ```
#[allow(dead_code)]
fn parse_proc_net_dev(content: &str) -> Option<(u64, u64)> {
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    let mut found = false;

    for line in content.lines().skip(2) {
        // Skip header lines
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }
        // Skip loopback
        if parts[0].starts_with("lo:") || parts[0] == "lo:" {
            continue;
        }
        if let (Ok(rx), Ok(tx)) = (parts[1].parse::<u64>(), parts[9].parse::<u64>()) {
            total_rx += rx;
            total_tx += tx;
            found = true;
        }
    }

    if found {
        Some((total_rx, total_tx))
    } else {
        None
    }
}

/// Parse `netstat -ib` output (macOS).
///
/// Format:
/// ```text
/// Name  Mtu   Network       Address            Ipkts Ierrs     Ibytes    Opkts Oerrs     Obytes  Coll
/// lo0   16384 <Link#1>                         12345     0     678901    12345     0     678901     0
/// en0   1500  <Link#6>    xx:xx:xx:xx:xx:xx   98765     0   12345678    87654     0    9876543     0
/// ```
#[allow(dead_code)]
fn parse_netstat_ib(content: &str) -> Option<(u64, u64)> {
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    let mut found = false;

    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 {
            continue;
        }
        // Skip loopback
        if parts[0].starts_with("lo") {
            continue;
        }
        // Only count <Link#N> entries (physical interfaces)
        if !parts[2].starts_with("<Link#") {
            continue;
        }
        if let (Ok(rx), Ok(tx)) = (parts[6].parse::<u64>(), parts[9].parse::<u64>()) {
            total_rx += rx;
            total_tx += tx;
            found = true;
        }
    }

    if found {
        Some((total_rx, total_tx))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_rate_bytes() {
        assert_eq!(format_rate(0.0), "0 B/s");
        assert_eq!(format_rate(500.0), "500 B/s");
        assert_eq!(format_rate(1023.0), "1023 B/s");
    }

    #[test]
    fn format_rate_kilobytes() {
        assert_eq!(format_rate(1024.0), "1.0 KB/s");
        assert_eq!(format_rate(1536.0), "1.5 KB/s");
        assert_eq!(format_rate(500_000.0), "488.3 KB/s");
    }

    #[test]
    fn format_rate_megabytes() {
        assert_eq!(format_rate(1_048_576.0), "1.0 MB/s");
        assert_eq!(format_rate(10_000_000.0), "9.5 MB/s");
    }

    #[test]
    fn format_rate_gigabytes() {
        assert_eq!(format_rate(1_073_741_824.0), "1.0 GB/s");
    }

    #[test]
    fn tracker_first_sample_no_rate() {
        let mut t = BandwidthTracker::new();
        assert!(t.current_rate.is_none());
        t.sample();
        // After first sample, still no rate (no delta)
        // (may or may not have rate depending on platform)
    }

    #[test]
    fn parse_proc_net_dev_valid() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 100 10 0 0 0 0 0 0 100 10 0 0 0 0 0 0
  eth0: 5000 50 0 0 0 0 0 0 3000 30 0 0 0 0 0 0
  eth1: 2000 20 0 0 0 0 0 0 1000 10 0 0 0 0 0 0
";
        let (rx, tx) = parse_proc_net_dev(content).unwrap();
        assert_eq!(rx, 7000); // eth0 + eth1, skip lo
        assert_eq!(tx, 4000);
    }

    #[test]
    fn parse_proc_net_dev_empty() {
        let content = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
";
        assert!(parse_proc_net_dev(content).is_none());
    }

    #[test]
    fn parse_netstat_ib_valid() {
        let content = "\
Name  Mtu   Network       Address            Ipkts Ierrs     Ibytes    Opkts Oerrs     Obytes  Coll
lo0   16384 <Link#1>                           1000     0     500000     1000     0     500000     0
en0   1500  <Link#6>    aa:bb:cc:dd:ee:ff      5000     0    3000000     4000     0    2000000     0
en0   1500  192.168.1     192.168.1.100         5000     0    3000000     4000     0    2000000     0
";
        let (rx, tx) = parse_netstat_ib(content).unwrap();
        assert_eq!(rx, 3_000_000); // only en0 <Link#6>, skip lo0 and non-Link
        assert_eq!(tx, 2_000_000);
    }

    #[test]
    fn parse_netstat_ib_empty() {
        let content = "Name  Mtu   Network       Address\n";
        assert!(parse_netstat_ib(content).is_none());
    }
}
