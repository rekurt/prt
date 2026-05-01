//! Fullscreen SSH tunnels manager.

use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::core::ssh_tunnel::TunnelKind;
use prt_core::i18n;
use ratatui::prelude::*;
use ratatui::widgets::*;

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
            // Status read is fallible without &mut; we render "alive" by default
            // since the cleanup loop in app.rs:382 prunes dead tunnels each tick.
            let status = s.tunnel_status_alive;

            Row::new(vec![
                Cell::from(t.spec.name.clone().unwrap_or_else(|| "-".into())),
                Cell::from(kind_label),
                Cell::from(local),
                Cell::from(remote),
                Cell::from(t.spec.host_alias.clone()),
                Cell::from(status).style(Style::default().fg(Color::Green)),
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
        KeyCode::Char('K') | KeyCode::Delete => {
            app.kill_selected_tunnel();
            true
        }
        KeyCode::Char('r') => {
            app.restart_selected_tunnel();
            true
        }
        KeyCode::Char('s') => {
            app.save_tunnels();
            true
        }
        _ => false,
    }
}
