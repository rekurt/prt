//! Connection count history for sparkline rendering.
//!
//! Tracks per-process connection counts over time using a ring buffer.
//! The TUI renders the last N samples as a sparkline: `▁▂▃▅▇`.

use std::collections::{HashMap, VecDeque};

/// Sparkline block characters (8 levels, Unicode block elements).
const SPARK_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Maximum number of samples to retain per process.
const MAX_SAMPLES: usize = 150; // 5min / 2s tick

/// Number of recent samples to display in the sparkline.
const DISPLAY_SAMPLES: usize = 10;

/// Tracks connection counts per process over time.
#[derive(Debug, Default)]
pub struct ConnectionHistory {
    /// (port, pid) → ring buffer of connection counts per tick.
    data: HashMap<(u16, u32), VecDeque<u16>>,
}

impl ConnectionHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new sample: count of connections per (port, pid).
    /// Call once per refresh cycle with aggregated counts.
    pub fn record(&mut self, counts: &HashMap<(u16, u32), u16>) {
        // Add new samples
        for (&key, &count) in counts {
            let buf = self.data.entry(key).or_default();
            buf.push_back(count);
            if buf.len() > MAX_SAMPLES {
                buf.pop_front();
            }
        }

        // Remove keys not seen this cycle (process exited)
        self.data.retain(|k, _| counts.contains_key(k));
    }

    /// Render a sparkline string for a given (port, pid).
    /// Returns empty string if no history.
    pub fn sparkline(&self, port: u16, pid: u32) -> String {
        render_sparkline(self.data.get(&(port, pid)))
    }
}

/// Render a sparkline from a buffer of values.
fn render_sparkline(buf: Option<&VecDeque<u16>>) -> String {
    let buf = match buf {
        Some(b) if !b.is_empty() => b,
        _ => return String::new(),
    };

    // Take last DISPLAY_SAMPLES values
    let values: Vec<u16> = buf.iter().rev().take(DISPLAY_SAMPLES).copied().collect();
    let values: Vec<u16> = values.into_iter().rev().collect();

    let max = *values.iter().max().unwrap_or(&0);
    if max == 0 {
        return " ".repeat(values.len());
    }

    values
        .iter()
        .map(|&v| {
            let idx = ((v as f64 / max as f64) * 7.0).round() as usize;
            SPARK_CHARS[idx.min(7)]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_history_is_empty() {
        let h = ConnectionHistory::new();
        assert_eq!(h.sparkline(80, 1), "");
    }

    #[test]
    fn record_and_sparkline() {
        let mut h = ConnectionHistory::new();
        for count in [1, 2, 3, 4, 5] {
            let mut counts = HashMap::new();
            counts.insert((80u16, 1u32), count);
            h.record(&counts);
        }
        let s = h.sparkline(80, 1);
        assert_eq!(s.chars().count(), 5);
        // First char should be lowest, last should be highest
        assert_eq!(s.chars().last().unwrap(), '█');
    }

    #[test]
    fn sparkline_capped_at_display_samples() {
        let mut h = ConnectionHistory::new();
        for i in 0..20 {
            let mut counts = HashMap::new();
            counts.insert((80u16, 1u32), i);
            h.record(&counts);
        }
        let s = h.sparkline(80, 1);
        assert_eq!(s.chars().count(), DISPLAY_SAMPLES);
    }

    #[test]
    fn history_capped_at_max_samples() {
        let mut h = ConnectionHistory::new();
        for i in 0..200 {
            let mut counts = HashMap::new();
            counts.insert((80u16, 1u32), i as u16);
            h.record(&counts);
        }
        assert!(h.data[&(80, 1)].len() <= MAX_SAMPLES);
    }

    #[test]
    fn exited_process_is_removed() {
        let mut h = ConnectionHistory::new();
        let mut counts = HashMap::new();
        counts.insert((80u16, 1u32), 1);
        h.record(&counts);
        assert!(!h.data.is_empty());

        // Next cycle: PID 1 is gone
        h.record(&HashMap::new());
        assert!(h.data.is_empty());
    }

    #[test]
    fn all_zeros_renders_spaces() {
        let mut h = ConnectionHistory::new();
        for _ in 0..5 {
            let mut counts = HashMap::new();
            counts.insert((80u16, 1u32), 0);
            h.record(&counts);
        }
        let s = h.sparkline(80, 1);
        assert!(s.chars().all(|c| c == ' '));
    }

    #[test]
    fn constant_value_renders_max_bars() {
        let mut h = ConnectionHistory::new();
        for _ in 0..5 {
            let mut counts = HashMap::new();
            counts.insert((80u16, 1u32), 10);
            h.record(&counts);
        }
        let s = h.sparkline(80, 1);
        assert!(s.chars().all(|c| c == '█'));
    }

    #[test]
    fn render_sparkline_empty() {
        assert_eq!(render_sparkline(None), "");
        let empty = VecDeque::new();
        assert_eq!(render_sparkline(Some(&empty)), "");
    }
}
