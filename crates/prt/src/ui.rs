use crate::app::App;
use prt_core::core::{bandwidth, process_detail, scanner};
use prt_core::i18n;
use prt_core::model::{
    ConnectionState, EntryStatus, ProcessesTab, SortColumn, SshTab, TrackedEntry, ViewMode,
};
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::{Duration, Instant};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);

    if app.show_help {
        draw_help(f, chunks[1]);
    } else {
        match app.view_mode {
            ViewMode::Connections => draw_table_view(f, app, chunks[1]),
            ViewMode::Processes => draw_processes_view(f, app, chunks[1]),
            ViewMode::Ssh => draw_ssh_view(f, app, chunks[1]),
        }
    }

    // Overlay dialogs
    if app.sudo_prompt {
        draw_sudo_prompt(f, app);
    }
    if app.confirm_block.is_some() {
        draw_block_confirm(f, app);
    }
    if app.forward_prompt {
        draw_forward_prompt(f, app);
    }
    if app.tunnel_form.is_some() {
        crate::views::tunnel_form::draw(f, app);
    }
    if app.action_menu.is_some() {
        crate::views::action_menu::draw(f, app);
    }
    if app.command_palette.is_some() {
        crate::views::command_palette::draw(f, app);
    }

    draw_footer(f, app, chunks[2]);
}

// ── Table view (default mode) ────────────────────────────────────

/// Table mode: port table + optional bottom detail panel + optional tracer.
fn draw_table_view(f: &mut Frame, app: &App, area: Rect) {
    if app.show_details {
        if app.tracer.is_some() {
            // Split: table 40%, details 30%, tracer 30%
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                ])
                .split(area);
            draw_table(f, app, chunks[0]);
            draw_detail_panel(f, app, chunks[1]);
            draw_tracer_panel(f, app, chunks[2]);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);
            draw_table(f, app, chunks[0]);
            draw_detail_panel(f, app, chunks[1]);
        }
    } else {
        draw_table(f, app, area);
    }
}

fn draw_processes_view(f: &mut Frame, app: &App, area: Rect) {
    match app.processes_tab {
        ProcessesTab::Detail => draw_process_detail_fullscreen(f, app, area),
        ProcessesTab::Topology => draw_topology_fullscreen(f, app, area),
    }
}

fn draw_ssh_view(f: &mut Frame, app: &App, area: Rect) {
    match app.ssh_tab {
        SshTab::Hosts => crate::views::ssh_hosts::draw(f, app, area),
        SshTab::Tunnels => crate::views::tunnels::draw(f, app, area),
    }
}

// ── Header ───────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let mut parts = vec![
        Span::styled(
            format!(" {} ", s.app_name),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::raw(format!(
            " {}/{} {} ",
            app.filtered_indices.len(),
            app.session.entries.len(),
            s.connections
        )),
    ];

    if !app.session.is_root {
        parts.push(Span::styled(
            format!(" {} ", s.no_root_warning),
            Style::default().fg(Color::Yellow),
        ));
    } else if app.session.is_elevated {
        parts.push(Span::styled(
            format!(" {} ", s.sudo_ok),
            Style::default().fg(Color::Green),
        ));
    }

    if !app.filter.is_empty() {
        parts.push(Span::styled(
            format!(" {} {} ", s.filter_label, app.filter),
            Style::default().fg(Color::Green),
        ));
    }

    if app.filter_mode {
        parts.push(Span::styled(
            format!(" {} ", s.search_mode),
            Style::default().fg(Color::Black).bg(Color::Green),
        ));
    }

    if app.auto_refresh_paused {
        parts.push(Span::styled(
            " PAUSED ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ));
    }

    if app.tracer.is_some() {
        parts.push(Span::styled(
            " TRACE ",
            Style::default().fg(Color::Black).bg(Color::Magenta),
        ));
    }

    // Bandwidth indicator
    if let Some(rate) = &app.session.bandwidth.current_rate {
        parts.push(Span::styled(
            format!(
                " \u{25bc} {} \u{25b2} {} ",
                bandwidth::format_rate(rate.rx_bytes_per_sec),
                bandwidth::format_rate(rate.tx_bytes_per_sec),
            ),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Active tunnels indicator
    let tunnels = app.forwards.summaries();
    if !tunnels.is_empty() {
        let label = if tunnels.len() == 1 {
            format!(" \u{21c4} {} ", tunnels[0])
        } else {
            format!(" \u{21c4} {} tunnels ", tunnels.len())
        };
        parts.push(Span::styled(label, Style::default().fg(Color::Cyan)));
    }

    // Section breadcrumb: [Connections | Processes | Ssh]  with active highlighted.
    parts.push(Span::raw("  "));
    for &mode in ViewMode::ALL {
        let label = section_label(mode);
        let style = if mode == app.view_mode {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        parts.push(Span::styled(format!(" {label} "), style));
        parts.push(Span::raw(" "));
    }
    // Sub-tab indicator
    let sub = match app.view_mode {
        ViewMode::Connections => None,
        ViewMode::Processes => Some(match app.processes_tab {
            ProcessesTab::Detail => i18n::strings().view_process,
            ProcessesTab::Topology => i18n::strings().view_topology,
        }),
        ViewMode::Ssh => Some(match app.ssh_tab {
            SshTab::Hosts => i18n::strings().view_ssh_hosts,
            SshTab::Tunnels => i18n::strings().view_tunnels,
        }),
    };
    if let Some(sub) = sub {
        parts.push(Span::styled(
            format!("[{sub}]"),
            Style::default().fg(Color::Cyan),
        ));
    }

    f.render_widget(Line::from(parts), area);
}

fn section_label(mode: ViewMode) -> &'static str {
    let s = i18n::strings();
    match mode {
        ViewMode::Connections => s.section_connections,
        ViewMode::Processes => s.section_processes,
        ViewMode::Ssh => s.section_ssh,
    }
}

// ── Sort indicator ───────────────────────────────────────────────

fn sort_indicator(app: &App, col: SortColumn) -> &'static str {
    if app.session.sort.column == col {
        if app.session.sort.ascending {
            " \u{25b2}"
        } else {
            " \u{25bc}"
        }
    } else {
        ""
    }
}

// ── Entry style (row coloring) ───────────────────────────────────

/// Compute the row style based on entry status and connection aging.
///
/// Priority: New (green) > Gone (red dim) > CLOSE_WAIT (red) >
/// ESTABLISHED >24h (red) > ESTABLISHED >1h (yellow) > suspicious (magenta) > default.
fn entry_style(e: &TrackedEntry, now: Instant) -> Style {
    match e.status {
        EntryStatus::New => return Style::default().fg(Color::Green),
        EntryStatus::Gone => return Style::default().fg(Color::Red).add_modifier(Modifier::DIM),
        EntryStatus::Unchanged => {}
    }

    if e.entry.state == ConnectionState::CloseWait {
        return Style::default().fg(Color::Red);
    }

    if e.entry.state == ConnectionState::Established {
        if let Some(first) = e.first_seen {
            let age = now.duration_since(first);
            if age > Duration::from_secs(86400) {
                return Style::default().fg(Color::Red);
            }
            if age > Duration::from_secs(3600) {
                return Style::default().fg(Color::Yellow);
            }
        }
    }

    if !e.suspicious.is_empty() {
        return Style::default().fg(Color::Magenta);
    }

    Style::default()
}

// ── Adaptive columns ─────────────────────────────────────────────

fn show_service_column(width: u16) -> bool {
    width > 90
}

fn show_container_column(width: u16, app: &App) -> bool {
    width > 110
        && app
            .session
            .entries
            .iter()
            .any(|e| e.container_name.is_some())
}

fn show_age_column(width: u16) -> bool {
    width > 100
}

fn show_remote_column(width: u16) -> bool {
    width > 120
}

fn format_age(first_seen: Option<Instant>, now: Instant) -> String {
    let Some(first_seen) = first_seen else {
        return "-".into();
    };
    let secs = now.duration_since(first_seen).as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

// ── Port table ───────────────────────────────────────────────────

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let wide = show_service_column(area.width);
    let show_age = show_age_column(area.width);
    let show_remote = show_remote_column(area.width);
    let show_container = show_container_column(area.width, app);

    let mut header_cells = vec![Cell::from(format!(
        "Port{}",
        sort_indicator(app, SortColumn::Port)
    ))];
    if wide {
        header_cells.push(Cell::from(format!(
            "Service{}",
            sort_indicator(app, SortColumn::Service)
        )));
    }
    header_cells.extend([
        Cell::from(format!(
            "Proto{}",
            sort_indicator(app, SortColumn::Protocol)
        )),
        Cell::from(format!("State{}", sort_indicator(app, SortColumn::State))),
        Cell::from(format!("PID{}", sort_indicator(app, SortColumn::Pid))),
        Cell::from(format!(
            "Process{}",
            sort_indicator(app, SortColumn::ProcessName)
        )),
        Cell::from(format!("User{}", sort_indicator(app, SortColumn::User))),
    ]);
    if show_container {
        header_cells.push(Cell::from("Container"));
    }
    if show_age {
        header_cells.push(Cell::from(s.col_age));
    }
    if show_remote {
        header_cells.push(Cell::from(s.col_remote));
    }

    let header = Row::new(header_cells).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let now = Instant::now();
    if app.filtered_indices.is_empty() {
        let message = if app.filter.is_empty() {
            s.no_connections.to_string()
        } else {
            format!("{}: {}", s.no_filter_matches, app.filter)
        };
        f.render_widget(
            Paragraph::new(message).style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .map(|&i| {
            let e = &app.session.entries[i];
            let mut style = entry_style(e, now);
            if app.is_alert_highlighted(i) {
                style = style.bg(Color::DarkGray);
            }

            let mut cells = vec![Cell::from(e.entry.local_port().to_string())];
            if wide {
                cells.push(Cell::from(e.service_name.as_deref().unwrap_or("-")));
            }

            let proc_name = if e.suspicious.is_empty() {
                e.entry.process.name.clone()
            } else {
                format!("[!] {}", e.entry.process.name)
            };

            cells.extend([
                Cell::from(e.entry.protocol.to_string()),
                Cell::from(e.entry.state.to_string()),
                Cell::from(e.entry.process.pid.to_string()),
                Cell::from(proc_name),
                Cell::from(e.entry.process.user.as_deref().unwrap_or("-").to_string()),
            ]);
            if show_container {
                cells.push(Cell::from(
                    e.container_name.as_deref().unwrap_or("-").to_string(),
                ));
            }
            if show_age {
                cells.push(Cell::from(format_age(e.first_seen, now)));
            }
            if show_remote {
                cells.push(Cell::from(
                    e.entry
                        .remote_addr
                        .map(|a| a.to_string())
                        .unwrap_or_else(|| "-".into()),
                ));
            }

            Row::new(cells).style(style)
        })
        .collect();

    let mut widths = vec![Constraint::Length(7)];
    if wide {
        widths.push(Constraint::Length(12));
    }
    widths.extend([
        Constraint::Length(5),
        Constraint::Length(13),
        Constraint::Length(8),
        Constraint::Fill(1),
        Constraint::Length(15),
    ]);
    if show_container {
        widths.push(Constraint::Length(15));
    }
    if show_age {
        widths.push(Constraint::Length(7));
    }
    if show_remote {
        widths.push(Constraint::Length(24));
    }

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("\u{25b6} ");

    let mut state = TableState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}

// ── Bottom detail panel (Table mode only) ────────────────────────

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", s.detail_panel_title))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let entry = match app.selected_entry() {
        Some(e) => e,
        None => {
            f.render_widget(Paragraph::new(s.no_selected_process), inner);
            return;
        }
    };

    let e = &entry.entry;
    let p = &e.process;
    let iface = scanner::resolve_interface(&e.local_addr);
    let bind_type = if e.local_addr.ip().is_loopback() {
        s.iface_localhost_only
    } else if e.local_addr.ip().is_unspecified() {
        s.iface_all_interfaces
    } else {
        s.iface_specific
    };
    let remote = e
        .remote_addr
        .map(|a| a.to_string())
        .unwrap_or_else(|| "-".into());

    let mut lines = vec![
        Line::from(vec![
            Span::styled(s.conn_local, Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}  {}  ({})", e.local_addr, e.protocol, bind_type)),
        ]),
        Line::from(vec![
            Span::styled(s.iface_interface, Style::default().fg(Color::Cyan)),
            Span::raw(iface),
        ]),
        Line::from(vec![
            Span::styled(s.conn_remote, Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}  [{}]", remote, e.state)),
        ]),
        Line::from(vec![
            Span::styled(s.conn_process, Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} (PID {})", p.name, p.pid)),
        ]),
        Line::from(vec![
            Span::styled(s.conn_cmdline, Style::default().fg(Color::Cyan)),
            Span::raw(p.cmdline.as_deref().unwrap_or("-")),
        ]),
    ];

    let conns = scanner::process_connections(&app.session.entries, p.pid);
    if conns.len() > 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            s.fmt_all_ports(conns.len()),
            Style::default().fg(Color::DarkGray),
        )));
        for conn in &conns {
            let c = &conn.entry;
            let arrow = c
                .remote_addr
                .map(|a| format!(" \u{2192} {a}"))
                .unwrap_or_default();
            lines.push(Line::from(format!(
                "  :{} {} {}{}",
                c.local_port(),
                c.protocol,
                c.state,
                arrow,
            )));
        }
    }

    let tree_lines = scanner::build_process_tree(&app.session.entries, p.pid);
    if !tree_lines.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            s.detail_panel_tree_header,
            Style::default().fg(Color::DarkGray),
        )));
        for l in &tree_lines {
            lines.push(Line::from(format!("  {l}")));
        }
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

// ── Fullscreen: Topology (process → port → remote) ──────────────

fn draw_topology_fullscreen(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", s.view_topology))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.session.entries.is_empty() {
        f.render_widget(Paragraph::new(s.no_selected_process), inner);
        return;
    }

    // Build topology: process → local ports → remote hosts
    let mut lines: Vec<Line> = Vec::new();

    let mut by_process: std::collections::HashMap<String, Vec<&TrackedEntry>> =
        std::collections::HashMap::new();
    for idx in &app.filtered_indices {
        let e = &app.session.entries[*idx];
        if e.status != EntryStatus::Gone {
            by_process
                .entry(e.entry.process.name.clone())
                .or_default()
                .push(e);
        }
    }

    let mut process_names: Vec<_> = by_process.keys().cloned().collect();
    process_names.sort();

    for name in &process_names {
        let entries = &by_process[name];
        lines.push(Line::from(vec![Span::styled(
            format!("  {name}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]));

        let max_entries = 12;
        for (i, entry) in entries.iter().take(max_entries).enumerate() {
            let port = entry.entry.local_port();
            let remote = entry
                .entry
                .remote_addr
                .map(|a| format!(" \u{2192} {a}"))
                .unwrap_or_default();
            let state = &entry.entry.state;
            let is_last = i + 1 >= entries.len().min(max_entries) && entries.len() <= max_entries;
            let connector = if is_last {
                "\u{2514}\u{2500}"
            } else {
                "\u{251c}\u{2500}"
            };
            lines.push(Line::from(format!(
                "    {connector} :{port} {state}{remote}"
            )));
        }
        if entries.len() > max_entries {
            lines.push(Line::from(Span::styled(
                format!(
                    "    \u{2514}\u{2500} ... +{} more",
                    entries.len() - max_entries
                ),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let max_scroll = (lines.len() as u16).saturating_sub(inner.height);
    let scroll = app.scroll_offset.min(max_scroll);
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        inner,
    );
}

// ── Fullscreen: Process Detail ───────────────────────────────────

fn draw_process_detail_fullscreen(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", s.view_process))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let entry = match app.selected_entry() {
        Some(e) => e,
        None => {
            f.render_widget(Paragraph::new(s.no_selected_process), inner);
            return;
        }
    };

    let pid = entry.entry.process.pid;
    let e = &entry.entry;
    let p = &e.process;

    // Split into left (process info) and right (connections + tree)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    // ── Left column: process info + detail ──
    let mut left_lines: Vec<Line> = Vec::new();

    left_lines.push(Line::from(vec![
        Span::styled("  Process:    ", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{} (PID {})", p.name, p.pid),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]));
    left_lines.push(Line::from(vec![
        Span::styled("  User:       ", Style::default().fg(Color::Cyan)),
        Span::raw(p.user.as_deref().unwrap_or("-")),
    ]));
    left_lines.push(Line::from(vec![
        Span::styled("  Cmdline:    ", Style::default().fg(Color::Cyan)),
        Span::raw(p.cmdline.as_deref().unwrap_or("-")),
    ]));
    if let Some(parent_pid) = p.parent_pid {
        left_lines.push(Line::from(vec![
            Span::styled("  Parent:     ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{} (PID {})",
                p.parent_name.as_deref().unwrap_or("?"),
                parent_pid
            )),
        ]));
    }

    // Process detail (cwd, cpu, rss, files, env) from cache
    if let Some((cached_pid, detail)) = &app.detail_cache {
        if *cached_pid == pid {
            left_lines.push(Line::from(""));

            left_lines.push(Line::from(vec![
                Span::styled("  CWD:        ", Style::default().fg(Color::Cyan)),
                Span::raw(
                    detail
                        .cwd
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "-".into()),
                ),
            ]));
            left_lines.push(Line::from(vec![
                Span::styled("  CPU:        ", Style::default().fg(Color::Cyan)),
                Span::raw(
                    detail
                        .cpu_percent
                        .map(|c| format!("{c:.1}%"))
                        .unwrap_or_else(|| "-".into()),
                ),
                Span::raw("    "),
                Span::styled("RSS: ", Style::default().fg(Color::Cyan)),
                Span::raw(
                    detail
                        .rss_kb
                        .map(process_detail::format_rss)
                        .unwrap_or_else(|| "-".into()),
                ),
            ]));

            // Open files
            if !detail.open_files.is_empty() {
                left_lines.push(Line::from(""));
                left_lines.push(Line::from(Span::styled(
                    format!("  Open files ({}):", detail.open_files.len()),
                    Style::default().fg(Color::Cyan),
                )));
                let max_files = (columns[0].height as usize).saturating_sub(left_lines.len() + 1);
                for file in detail.open_files.iter().take(max_files) {
                    let safe_file = process_detail::sanitize_for_terminal(file);
                    left_lines.push(Line::from(format!("    {safe_file}")));
                }
                if detail.open_files.len() > max_files {
                    left_lines.push(Line::from(Span::styled(
                        format!("    ... +{} more", detail.open_files.len() - max_files),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }

            // Environment variables
            if !detail.env_vars.is_empty() && left_lines.len() < columns[0].height as usize {
                left_lines.push(Line::from(""));
                left_lines.push(Line::from(Span::styled(
                    format!("  Environment ({}):", detail.env_vars.len()),
                    Style::default().fg(Color::Cyan),
                )));
                let remaining = (columns[0].height as usize).saturating_sub(left_lines.len());
                for (k, v) in detail.env_vars.iter().take(remaining) {
                    let safe_key = process_detail::sanitize_for_terminal(k);
                    let safe_val = process_detail::sanitize_for_terminal(v);
                    let val_display = if safe_val.len() > 60 {
                        format!("{}...", &safe_val[..57])
                    } else {
                        safe_val
                    };
                    left_lines.push(Line::from(format!("    {safe_key}={val_display}")));
                }
            }
        }
    } else {
        left_lines.push(Line::from(""));
        left_lines.push(Line::from(Span::styled(
            format!("  Loading details for PID {pid}..."),
            Style::default().fg(Color::DarkGray),
        )));
    }

    f.render_widget(Paragraph::new(left_lines), columns[0]);

    // ── Right column: connections + network info ──
    let mut right_lines: Vec<Line> = Vec::new();

    right_lines.push(Line::from(Span::styled(
        "  Connections:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    let conns = scanner::process_connections(&app.session.entries, p.pid);
    if conns.is_empty() {
        right_lines.push(Line::from("    (none)"));
    } else {
        for conn in &conns {
            let c = &conn.entry;
            let arrow = c
                .remote_addr
                .map(|a| format!(" \u{2192} {a}"))
                .unwrap_or_default();
            let state_style = match c.state {
                ConnectionState::Listen => Style::default().fg(Color::Green),
                ConnectionState::Established => Style::default().fg(Color::Yellow),
                ConnectionState::CloseWait => Style::default().fg(Color::Red),
                _ => Style::default().fg(Color::DarkGray),
            };
            right_lines.push(Line::from(vec![
                Span::raw(format!("    :{} ", c.local_port())),
                Span::styled(c.state.to_string(), state_style),
                Span::raw(format!(" {}{}", c.protocol, arrow)),
            ]));
        }
    }

    // Interface info
    right_lines.push(Line::from(""));
    right_lines.push(Line::from(Span::styled(
        "  Network:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    let iface = scanner::resolve_interface(&e.local_addr);
    let bind_type = if e.local_addr.ip().is_loopback() {
        s.iface_localhost_only
    } else if e.local_addr.ip().is_unspecified() {
        s.iface_all_interfaces
    } else {
        s.iface_specific
    };
    right_lines.push(Line::from(vec![
        Span::styled("    Address:    ", Style::default().fg(Color::Cyan)),
        Span::raw(e.local_addr.to_string()),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled("    Interface:  ", Style::default().fg(Color::Cyan)),
        Span::raw(iface),
    ]));
    right_lines.push(Line::from(vec![
        Span::styled("    Bind:       ", Style::default().fg(Color::Cyan)),
        Span::raw(bind_type),
    ]));

    // Process tree
    right_lines.push(Line::from(""));
    right_lines.push(Line::from(Span::styled(
        "  Process tree:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    let tree_lines = scanner::build_process_tree(&app.session.entries, p.pid);
    let remaining = (columns[1].height as usize).saturating_sub(right_lines.len());
    for l in tree_lines.iter().take(remaining) {
        right_lines.push(Line::from(format!("    {l}")));
    }

    f.render_widget(Paragraph::new(right_lines), columns[1]);
}

// ── Tracer panel (strace split) ──────────────────────────────────

fn draw_tracer_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(
            " strace (PID {}) \u{2014} t to detach ",
            app.tracer.as_ref().map(|t| t.traced_pid()).unwrap_or(0)
        ))
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = if let Some(ref tracer) = app.tracer {
        let visible = inner.height as usize;
        tracer
            .lines
            .iter()
            .rev()
            .take(visible)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|l| Line::from(Span::raw(l.as_str())))
            .collect()
    } else {
        vec![Line::from("no tracer active")]
    };

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Overlay: sudo prompt ─────────────────────────────────────────

fn draw_sudo_prompt(f: &mut Frame, app: &App) {
    let s = i18n::strings();
    let area = f.area();
    let w = 42u16.min(area.width.saturating_sub(4));
    let h = 5u16;
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let popup_area = Rect::new(x, y, w, h);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(s.sudo_prompt_title);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let stars = "*".repeat(app.sudo_password.len());
    let lines = vec![
        Line::from(vec![
            Span::styled(s.sudo_password_label, Style::default().fg(Color::Cyan)),
            Span::raw(&stars),
            Span::styled("\u{2588}", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            s.sudo_confirm_hint,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Overlay: firewall block confirm ──────────────────────────────

fn draw_block_confirm(f: &mut Frame, app: &App) {
    if let Some((ip, ref cmd)) = app.confirm_block {
        let area = f.area();
        let w = 60u16.min(area.width.saturating_sub(4));
        let h = 6u16;
        let x = (area.width.saturating_sub(w)) / 2;
        let y = (area.height.saturating_sub(h)) / 2;
        let popup_area = Rect::new(x, y, w, h);

        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" Block IP ");

        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let lines = vec![
            Line::from(vec![
                Span::styled("  IP: ", Style::default().fg(Color::Cyan)),
                Span::raw(ip.to_string()),
            ]),
            Line::from(vec![
                Span::styled("  Cmd: ", Style::default().fg(Color::Cyan)),
                Span::raw(cmd.as_str()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  [y] block  [n/Esc] cancel",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        f.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Overlay: SSH forward prompt ──────────────────────────────────

fn draw_forward_prompt(f: &mut Frame, app: &App) {
    let s = i18n::strings();
    let area = f.area();
    let local_port = app
        .selected_entry()
        .map(|e| e.entry.local_port())
        .unwrap_or(0);

    let w = 50u16.min(area.width.saturating_sub(4));
    let h = 6u16;
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let popup_area = Rect::new(x, y, w, h);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(s.forward_prompt_title);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let tunnel_count = app.forwards.count();
    let status = if tunnel_count > 0 {
        format!("  ({tunnel_count} active)")
    } else {
        String::new()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("  localhost:{local_port} →"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(&status),
        ]),
        Line::from(vec![
            Span::styled(s.forward_host_label, Style::default().fg(Color::Cyan)),
            Span::raw(&app.forward_input),
            Span::styled("\u{2588}", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            s.forward_confirm_hint,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Help overlay ─────────────────────────────────────────────────

fn draw_help(f: &mut Frame, area: Rect) {
    let s = i18n::strings();
    let block = Block::default().borders(Borders::ALL).title(s.help_title);
    let paragraph = Paragraph::new(s.help_text)
        .block(block)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(paragraph, area);
}

// ── Footer ───────────────────────────────────────────────────────

/// Helper: push a `key label` pair into the hints vec.
fn hint(hints: &mut Vec<Span<'static>>, key: &'static str, label: &str) {
    hints.push(Span::styled(
        format!(" {key} "),
        Style::default().fg(Color::Black).bg(Color::DarkGray),
    ));
    hints.push(Span::raw(format!(" {label} ")));
}

/// Accent-colored hint (for language badge, etc.).
fn hint_accent(hints: &mut Vec<Span<'static>>, key: &str, color: Color) {
    hints.push(Span::styled(
        format!(" {key} "),
        Style::default().fg(Color::Black).bg(color),
    ));
}

fn hint_cost(key: &str, label: &str) -> usize {
    key.chars().count() + label.chars().count() + 4
}

fn push_budgeted_hints(
    hints: &mut Vec<Span<'static>>,
    items: &[(&'static str, &'static str)],
    max_width: u16,
    more_label: &'static str,
) {
    let mut used = 0usize;
    let more_cost = hint_cost("?", more_label);
    for (idx, (key, label)) in items.iter().enumerate() {
        let cost = hint_cost(key, label);
        let has_more = idx + 1 < items.len();
        let reserved = if has_more { more_cost } else { 0 };
        if used + cost + reserved > max_width as usize {
            hint(hints, "?", more_label);
            return;
        }
        hint(hints, key, label);
        used += cost;
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();

    // Kill confirmation — takes over the entire footer
    if let Some((pid, name)) = &app.confirm_kill {
        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", s.fmt_kill_confirm(name, *pid)),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(s.kill_cancel),
        ]);
        f.render_widget(line, area);
        return;
    }

    // Status message (temporary, 4 seconds)
    if let Some((msg, t)) = &app.status_msg {
        if t.elapsed().as_secs() < 4 {
            f.render_widget(
                Line::from(Span::styled(
                    format!(" {msg}"),
                    Style::default().fg(Color::Yellow),
                )),
                area,
            );
            return;
        }
    }
    if app.filter_mode {
        let line = Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::styled(
                format!(" {} ", s.hint_filter_examples),
                Style::default().fg(Color::Green),
            ),
        ]);
        f.render_widget(line, area);
        return;
    }

    let mut items: Vec<(&'static str, &'static str)> = vec![
        ("?", s.hint_help),
        ("Tab", s.hint_section_next),
        ("/", s.hint_search),
        ("Space", s.hint_action_menu),
        (
            "p",
            if app.auto_refresh_paused {
                s.hint_resume
            } else {
                s.hint_pause
            },
        ),
    ];

    if app.view_mode == ViewMode::Connections && (app.filter_mode || !app.filter.is_empty()) {
        let filter_hint = vec![
            Span::styled(" / ", Style::default().fg(Color::Black).bg(Color::Green)),
            Span::raw(" filter "),
            Span::styled("status:new", Style::default().fg(Color::Green)),
            Span::raw(" · "),
            Span::styled("status:gone", Style::default().fg(Color::Red)),
            Span::raw(" · "),
            Span::styled("risk:high", Style::default().fg(Color::Magenta)),
            Span::raw(" · "),
            Span::styled("proc:nginx", Style::default().fg(Color::Cyan)),
            Span::raw(" · Esc Esc clear"),
        ];
        f.render_widget(Line::from(filter_hint), area);
        return;
    }

    match app.view_mode {
        ViewMode::Connections => {
            if !app.show_details {
                items.push(("d", s.hint_details));
            }
            items.push(("Enter", s.view_process));
            items.push(("K", s.hint_kill));
            items.push(("c", s.hint_copy));
            items.push(("o/O", s.hint_sort));
        }
        ViewMode::Processes => {
            items.push(("[ ]", s.hint_subtab));
            items.push(("K", s.hint_kill));
            items.push(("c", s.hint_copy));
        }
        ViewMode::Ssh => {
            items.push(("[ ]", s.hint_subtab));
            match app.ssh_tab {
                SshTab::Hosts => {
                    items.push(("Enter", s.hint_open_tunnel));
                    items.push(("r", s.hint_reload));
                }
                SshTab::Tunnels => {
                    items.push(("n", s.hint_new_tunnel));
                    items.push(("e", s.hint_edit_tunnel));
                    items.push(("K", s.hint_kill_tunnel));
                    items.push(("r", s.hint_restart_tunnel));
                    items.push(("s", s.hint_save_tunnels));
                }
            }
        }
    }

    if !app.session.is_root && !app.session.is_elevated && app.view_mode == ViewMode::Connections {
        items.push(("s", s.hint_sudo));
    }
    items.push(("q", s.hint_quit));

    let mut hints: Vec<Span> = Vec::new();
    let lang_width = i18n::lang().label().chars().count() + 4;
    let max_hint_width = area.width.saturating_sub(lang_width as u16);
    push_budgeted_hints(&mut hints, &items, max_hint_width, s.more);
    hints.push(Span::raw(" "));
    hint_accent(&mut hints, i18n::lang().label(), Color::Magenta);

    f.render_widget(Line::from(hints), area);
}
