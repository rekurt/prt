//! Fullscreen SSH tunnels manager.

use crate::app::App;
use crate::forward::TunnelStatus;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::core::scanner::format_uptime;
use prt_core::core::ssh_tunnel::TunnelKind;
use prt_core::i18n;
use prt_core::model::{ConnectionState, TICK_RATE};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::Duration;

/// Grace period after (re)start before a missing listener is reported. The scan
/// backing `has_local_listener` only refreshes every `TICK_RATE`, and a tunnel
/// needs a tick to go `Starting -> Alive` plus another for the scan to observe
/// its `LISTEN` socket, so we'd otherwise flash a bogus "no listener".
const LISTENER_GRACE: Duration = TICK_RATE.saturating_mul(2);

/// True if `ssh_pid` owns a `LISTEN` socket on `local_port` in the latest scan
/// — confirms an `Alive` tunnel actually opened its own socket. Read-only:
/// reuses the data prt already scanned, opens no new connections.
///
/// The PID match matters: OpenSSH defaults to `ExitOnForwardFailure no`, so on
/// a local-port conflict the `ssh` child keeps running while *another* process
/// owns the port. Matching `LISTEN + port` alone would then mask the bind
/// failure as healthy; requiring the listener's PID to be our `ssh` child
/// avoids that false green.
fn has_local_listener(app: &App, local_port: u16, ssh_pid: u32) -> bool {
    app.session.entries.iter().any(|e| {
        e.entry.state == ConnectionState::Listen
            && e.entry.local_addr.port() == local_port
            && e.entry.process.pid == ssh_pid
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
            //
            // This signal is advisory and view-only: the tunnel stays `Alive`,
            // so the auto-reconnect loop never acts on a "no listener" tunnel.
            //
            // Guard against false positives: the scan refreshes only every
            // `TICK_RATE` (and not at all while auto-refresh is paused), so a
            // freshly (re)started tunnel hasn't been observed yet. Within the
            // grace window — or whenever the scan is frozen — trust the `Alive`
            // status instead of crying "no listener".
            let (status, color) = match t.last_status {
                TunnelStatus::Alive => {
                    let scan_can_confirm = !app.auto_refresh_paused && t.uptime() >= LISTENER_GRACE;
                    if !scan_can_confirm || has_local_listener(app, t.spec.local_port, t.pid()) {
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
                _ => format_uptime(t.uptime()),
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
