//! Fullscreen list of saved SSH hosts.

use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::core::ssh_config::SshHost;
use prt_core::i18n;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", s.view_ssh_hosts))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.ssh_hosts.is_empty() {
        f.render_widget(
            Paragraph::new(s.ssh_hosts_empty).style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from(s.ssh_col_alias),
        Cell::from(s.ssh_col_target),
        Cell::from(s.ssh_col_source),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .ssh_hosts
        .iter()
        .map(|h: &SshHost| {
            Row::new(vec![
                Cell::from(h.alias.clone()),
                Cell::from(h.target()),
                Cell::from(h.source.label()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(20),
        Constraint::Fill(1),
        Constraint::Length(12),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("\u{25b6} ");

    let mut state = TableState::default();
    state.select(Some(
        app.ssh_hosts_selected
            .min(app.ssh_hosts.len().saturating_sub(1)),
    ));
    f.render_stateful_widget(table, inner, &mut state);
}

/// Returns true if the key was consumed by this view.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.ssh_hosts.is_empty() {
                let max = app.ssh_hosts.len() - 1;
                if app.ssh_hosts_selected < max {
                    app.ssh_hosts_selected += 1;
                }
            }
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.ssh_hosts_selected > 0 {
                app.ssh_hosts_selected -= 1;
            }
            true
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.ssh_hosts_selected = 0;
            true
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.ssh_hosts_selected = app.ssh_hosts.len().saturating_sub(1);
            true
        }
        KeyCode::Enter => {
            let alias = app
                .ssh_hosts
                .get(app.ssh_hosts_selected)
                .map(|h| h.alias.clone());
            app.open_tunnel_form(alias);
            true
        }
        KeyCode::Char('r') => {
            app.reload_ssh_hosts();
            true
        }
        _ => false,
    }
}
