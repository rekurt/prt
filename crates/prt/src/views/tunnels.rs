//! Fullscreen SSH tunnels manager.

use crate::app::App;
use crate::forward::TunnelStatus;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::core::ssh_tunnel::TunnelKind;
use prt_core::i18n;
use prt_core::model::ConnectionState;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::Duration;

/// Compact uptime: `45s`, `12m`, `3h04m`, `2d05h`.
fn fmt_uptime(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h{:02}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d{:02}h", secs / 86_400, (secs % 86_400) / 3600)
    }
}

/// True if a local listener is currently bound to `local_port` in the latest
/// scan — confirms an `Alive` tunnel actually opened its socket. Read-only:
/// reuses the data prt already scanned, opens no new connections.
fn has_local_listener(app: &App, local_port: u16) -> bool {
    app.session.entries.iter().any(|e| {
        e.entry.state == ConnectionState::Listen && e.entry.local_addr.port() == local_port
    })
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ({}) ", s.view_tunnels, app.forwards.count()))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.forwards.tunnels.is_empty() {
        f.render_widget(
            Paragraph::new(s.tunnels_empty).style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from(s.tunnel_col_name),
        Cell::from(s.tunnel_col_kind),
        Cell::from(s.tunnel_col_local),
        Cell::from(s.tunnel_col_remote),
        Cell::from(s.tunnel_col_host),
        Cell::from(s.tunnel_col_uptime),
        Cell::from(s.tunnel_col_status),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .forwards
        .tunnels
        .iter()
        .map(|t| {
            let kind_label = match t.spec.kind {
                TunnelKind::Local => s.tunnel_kind_local,
                TunnelKind::Dynamic => s.tunnel_kind_dynamic,
            };
            let local = format!("localhost:{}", t.spec.local_port);
            let remote = match t.spec.kind {
                TunnelKind::Local => format!(
                    "{}:{}",
                    t.spec.remote_host.as_deref().unwrap_or("?"),
                    t.spec
                        .remote_port
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "?".into())
                ),
                TunnelKind::Dynamic => "(SOCKS5)".into(),
            };
            // Status, with a listener health check layered on top: an `Alive`
            // ssh child whose local port isn't actually being listened on is a
            // common sign of a broken `-D`/`-L` (e.g. bind failure), so surface
            // it as a yellow warning rather than a misleading green "alive".
            let (status, color) = match t.last_status {
                TunnelStatus::Alive => {
                    if has_local_listener(app, t.spec.local_port) {
                        (s.tunnel_status_alive.to_string(), Color::Green)
                    } else {
                        (s.tunnel_health_no_listener.to_string(), Color::Yellow)
                    }
                }
                TunnelStatus::Starting => (s.tunnel_status_starting.to_string(), Color::Yellow),
                TunnelStatus::Failed => (s.tunnel_status_failed.to_string(), Color::Red),
            };

            // Uptime is only meaningful while the child is running.
            let uptime = match t.last_status {
                TunnelStatus::Failed => "-".to_string(),
                _ => fmt_uptime(t.uptime()),
            };

            Row::new(vec![
                Cell::from(t.spec.name.clone().unwrap_or_else(|| "-".into())),
                Cell::from(kind_label),
                Cell::from(local),
                Cell::from(remote),
                Cell::from(t.spec.host_alias.clone()),
                Cell::from(uptime),
                Cell::from(status).style(Style::default().fg(color)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(16),
        Constraint::Length(10),
        Constraint::Length(20),
        Constraint::Fill(1),
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Length(12),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("\u{25b6} ");

    let mut state = TableState::default();
    state.select(Some(
        app.tunnels_selected
            .min(app.forwards.tunnels.len().saturating_sub(1)),
    ));
    f.render_stateful_widget(table, inner, &mut state);
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.forwards.tunnels.is_empty()
                && app.tunnels_selected + 1 < app.forwards.tunnels.len()
            {
                app.tunnels_selected += 1;
            }
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.tunnels_selected > 0 {
                app.tunnels_selected -= 1;
            }
            true
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.tunnels_selected = 0;
            true
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.tunnels_selected = app.forwards.tunnels.len().saturating_sub(1);
            true
        }
        KeyCode::Char('n') => {
            app.open_tunnel_form(None);
            true
        }
        KeyCode::Char('e') => {
            app.open_tunnel_form_edit(app.tunnels_selected);
            true
        }
        KeyCode::Char('K') | KeyCode::Delete => {
            app.kill_selected_tunnel();
            true
        }
        KeyCode::Char('r') => {
            app.restart_selected_tunnel();
            true
        }
        KeyCode::Char('c') => {
            app.copy_selected_tunnel_command();
            true
        }
        KeyCode::Char('s') => {
            app.save_tunnels();
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_uptime_buckets() {
        assert_eq!(fmt_uptime(Duration::from_secs(0)), "0s");
        assert_eq!(fmt_uptime(Duration::from_secs(45)), "45s");
        assert_eq!(fmt_uptime(Duration::from_secs(59)), "59s");
        assert_eq!(fmt_uptime(Duration::from_secs(60)), "1m");
        assert_eq!(fmt_uptime(Duration::from_secs(3599)), "59m");
        assert_eq!(fmt_uptime(Duration::from_secs(3600)), "1h00m");
        assert_eq!(fmt_uptime(Duration::from_secs(3600 + 4 * 60)), "1h04m");
        assert_eq!(fmt_uptime(Duration::from_secs(86_400)), "1d00h");
        assert_eq!(fmt_uptime(Duration::from_secs(86_400 + 5 * 3600)), "1d05h");
    }
}
