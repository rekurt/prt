use crate::app::App;
use prt_core::core::scanner;
use prt_core::i18n;
use prt_core::model::{DetailTab, EntryStatus, SortColumn};
use ratatui::prelude::*;
use ratatui::widgets::*;

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
    } else if app.show_details {
        let detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(chunks[1]);
        draw_table(f, app, detail_chunks[0]);
        draw_detail_panel(f, app, detail_chunks[1]);
    } else {
        draw_table(f, app, chunks[1]);
    }

    if app.sudo_prompt {
        draw_sudo_prompt(f, app);
    }

    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let mut parts = vec![
        Span::styled(
            format!(" {} ", s.app_name),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        Span::raw(format!(
            " {} ",
            s.fmt_connections(app.filtered_indices.len())
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

    f.render_widget(Line::from(parts), area);
}

fn sort_indicator(app: &App, col: SortColumn) -> &'static str {
    if app.session.sort.column == col {
        if app.session.sort.ascending {
            " ▲"
        } else {
            " ▼"
        }
    } else {
        ""
    }
}

fn draw_table(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from(format!("Port{}", sort_indicator(app, SortColumn::Port))),
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
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .map(|&i| {
            let e = &app.session.entries[i];
            let style = match e.status {
                EntryStatus::New => Style::default().fg(Color::Green),
                EntryStatus::Gone => Style::default().fg(Color::Red).add_modifier(Modifier::DIM),
                EntryStatus::Unchanged => Style::default(),
            };

            Row::new(vec![
                Cell::from(e.entry.local_port().to_string()),
                Cell::from(e.entry.protocol.to_string()),
                Cell::from(e.entry.state.to_string()),
                Cell::from(e.entry.process.pid.to_string()),
                Cell::from(e.entry.process.name.clone()),
                Cell::from(e.entry.process.user.as_deref().unwrap_or("-").to_string()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(7),
        Constraint::Length(5),
        Constraint::Length(13),
        Constraint::Length(8),
        Constraint::Fill(1),
        Constraint::Length(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_detail_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    draw_tab_bar(f, app, chunks[0]);

    let block = Block::default().borders(Borders::ALL & !Borders::TOP);
    let inner = block.inner(chunks[1]);
    f.render_widget(block, chunks[1]);

    match app.detail_tab {
        DetailTab::Tree => draw_tab_tree(f, app, inner),
        DetailTab::Interface => draw_tab_interface(f, app, inner),
        DetailTab::Connection => draw_tab_connection(f, app, inner),
    }
}

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let tabs = vec![
        ("1", s.tab_tree, DetailTab::Tree),
        ("2", s.tab_network, DetailTab::Interface),
        ("3", s.tab_connection, DetailTab::Connection),
    ];

    let mut spans = vec![Span::raw("┌")];
    for (key, label, tab) in &tabs {
        let active = *tab == app.detail_tab;
        if active {
            spans.push(Span::styled(
                format!(" {key}:{label} "),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {key}:{label} "),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
    spans.push(Span::styled(
        "─".repeat(
            area.width
                .saturating_sub(spans.iter().map(|s| s.width() as u16).sum::<u16>())
                as usize,
        ),
        Style::default().fg(Color::DarkGray),
    ));

    f.render_widget(Line::from(spans), area);
}

fn draw_tab_tree(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let entry = match app.selected_entry() {
        Some(e) => e,
        None => {
            f.render_widget(Paragraph::new(s.no_selected_process), area);
            return;
        }
    };

    let tree_lines = scanner::build_process_tree(&app.session.entries, entry.entry.process.pid);
    let lines: Vec<Line> = tree_lines
        .iter()
        .map(|l| Line::from(format!("  {l}")))
        .collect();

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_tab_interface(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let entry = match app.selected_entry() {
        Some(e) => e,
        None => {
            f.render_widget(Paragraph::new(s.no_selected_process), area);
            return;
        }
    };

    let e = &entry.entry;
    let iface = scanner::resolve_interface(&e.local_addr);
    let ip_version = if e.local_addr.is_ipv4() {
        "IPv4"
    } else {
        "IPv6"
    };
    let bind_type = if e.local_addr.ip().is_loopback() {
        s.iface_localhost_only
    } else if e.local_addr.ip().is_unspecified() {
        s.iface_all_interfaces
    } else {
        s.iface_specific
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(s.iface_address, Style::default().fg(Color::Cyan)),
            Span::raw(e.local_addr.to_string()),
        ]),
        Line::from(vec![
            Span::styled(s.iface_interface, Style::default().fg(Color::Cyan)),
            Span::raw(iface),
        ]),
        Line::from(vec![
            Span::styled(s.iface_protocol, Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} / {}", e.protocol, ip_version)),
        ]),
        Line::from(vec![
            Span::styled(s.iface_bind, Style::default().fg(Color::Cyan)),
            Span::raw(bind_type),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), area);
}

fn draw_tab_connection(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();
    let entry = match app.selected_entry() {
        Some(e) => e,
        None => {
            f.render_widget(Paragraph::new(s.no_selected_process), area);
            return;
        }
    };

    let e = &entry.entry;
    let p = &e.process;

    let mut lines = vec![
        Line::from(vec![
            Span::styled(s.conn_local, Style::default().fg(Color::Cyan)),
            Span::raw(e.local_addr.to_string()),
        ]),
        Line::from(vec![
            Span::styled(s.conn_remote, Style::default().fg(Color::Cyan)),
            Span::raw(
                e.remote_addr
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "-".into()),
            ),
        ]),
        Line::from(vec![
            Span::styled(s.conn_state, Style::default().fg(Color::Cyan)),
            Span::raw(e.state.to_string()),
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
            let arrow = c.remote_addr.map(|a| format!(" → {a}")).unwrap_or_default();
            lines.push(Line::from(format!(
                "  :{} {} {}{}",
                c.local_port(),
                c.protocol,
                c.state,
                arrow,
            )));
        }
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

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
            Span::styled("█", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            s.sudo_confirm_hint,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let s = i18n::strings();
    let block = Block::default().borders(Borders::ALL).title(s.help_title);
    let paragraph = Paragraph::new(s.help_text)
        .block(block)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let s = i18n::strings();

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

    let line = Line::from(vec![
        Span::styled(" ? ", Style::default().fg(Color::Black).bg(Color::DarkGray)),
        Span::raw(format!(" {} ", s.hint_help)),
        Span::styled(" / ", Style::default().fg(Color::Black).bg(Color::DarkGray)),
        Span::raw(format!(" {} ", s.hint_search)),
        Span::styled(" K ", Style::default().fg(Color::Black).bg(Color::DarkGray)),
        Span::raw(format!(" {} ", s.hint_kill)),
        Span::styled(" s ", Style::default().fg(Color::Black).bg(Color::DarkGray)),
        Span::raw(format!(" {} ", s.hint_sudo)),
        Span::styled(" L ", Style::default().fg(Color::Black).bg(Color::DarkGray)),
        Span::raw(format!(" {} ", s.hint_lang)),
        Span::styled(" q ", Style::default().fg(Color::Black).bg(Color::DarkGray)),
        Span::raw(format!(" {} ", s.hint_quit)),
        Span::styled(
            format!(" {} ", i18n::lang().label()),
            Style::default().fg(Color::Black).bg(Color::Magenta),
        ),
    ]);
    f.render_widget(line, area);
}
