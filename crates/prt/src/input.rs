use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::i18n;
use prt_core::model::{ProcessesTab, SortColumn, SshTab, ViewMode};
use std::time::{Duration, Instant};

/// How long the "press Esc again" prompt stays armed.
const ESC_ARM_WINDOW: Duration = Duration::from_millis(1500);

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Any non-Esc press disarms the cascade.
    if key.code != KeyCode::Esc {
        app.last_esc = None;
    }

    if app.show_help {
        app.show_help = false;
        return;
    }

    if app.sudo_prompt {
        match key.code {
            KeyCode::Esc => {
                app.sudo_prompt = false;
                app.sudo_password.clear();
            }
            KeyCode::Enter => {
                app.try_sudo();
            }
            KeyCode::Backspace => {
                app.sudo_password.pop();
            }
            KeyCode::Char(c) => {
                app.sudo_password.push(c);
            }
            _ => {}
        }
        return;
    }

    if app.confirm_block.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.execute_block();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.confirm_block = None;
                let s = i18n::strings();
                app.set_status(s.cancelled.into());
            }
            _ => {}
        }
        return;
    }

    if app.confirm_kill.is_some() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.kill_selected(false);
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                app.kill_selected(true);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.confirm_kill = None;
                let s = i18n::strings();
                app.set_status(s.cancelled.into());
            }
            _ => {}
        }
        return;
    }

    if app.forward_prompt {
        match key.code {
            KeyCode::Esc => {
                app.forward_prompt = false;
                app.forward_input.clear();
            }
            KeyCode::Enter => {
                app.create_forward();
            }
            KeyCode::Backspace => {
                app.forward_input.pop();
            }
            KeyCode::Char(c) if app.forward_input.len() < 256 => {
                app.forward_input.push(c);
            }
            _ => {}
        }
        return;
    }

    if app.filter_mode {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.filter_mode = false;
            }
            KeyCode::Backspace => {
                app.filter.pop();
                app.update_filtered();
            }
            KeyCode::Char(c) if app.filter.len() < 256 => {
                app.filter.push(c);
                app.update_filtered();
            }
            _ => {}
        }
        return;
    }

    if app.tunnel_form.is_some() && crate::views::tunnel_form::handle_key(app, key) {
        return;
    }

    // Action menu overlay (highest priority after explicit forms/modals).
    if crate::views::action_menu::handle_key(app, key) {
        return;
    }

    // Space opens the action menu when nothing else is active.
    if let KeyCode::Char(' ') = key.code {
        crate::views::action_menu::open(app);
        return;
    }

    // Section navigation: Tab / Shift+Tab cycles top-level sections.
    match key.code {
        KeyCode::Tab => {
            app.view_mode = app.view_mode.next();
            app.scroll_offset = 0;
            return;
        }
        KeyCode::BackTab => {
            app.view_mode = app.view_mode.prev();
            app.scroll_offset = 0;
            return;
        }
        _ => {}
    }

    // Sub-tab cycling: [ / ] inside Processes and SSH.
    match (app.view_mode, key.code) {
        (ViewMode::Processes, KeyCode::Char('[')) => {
            app.processes_tab = app.processes_tab.prev();
            app.scroll_offset = 0;
            return;
        }
        (ViewMode::Processes, KeyCode::Char(']')) => {
            app.processes_tab = app.processes_tab.next();
            app.scroll_offset = 0;
            return;
        }
        (ViewMode::Ssh, KeyCode::Char('[')) => {
            app.ssh_tab = app.ssh_tab.prev();
            return;
        }
        (ViewMode::Ssh, KeyCode::Char(']')) => {
            app.ssh_tab = app.ssh_tab.next();
            return;
        }
        _ => {}
    }

    // Inside SSH section, delegate to the active sub-view's handler so it can
    // claim its own action keys (e.g. `s` = save in Tunnels, not sudo).
    if app.view_mode == ViewMode::Ssh {
        let consumed = match app.ssh_tab {
            SshTab::Hosts => crate::views::ssh_hosts::handle_key(app, key),
            SshTab::Tunnels => crate::views::tunnels::handle_key(app, key),
        };
        if consumed {
            return;
        }
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('/') => app.filter_mode = true,
        KeyCode::Esc => {
            // Cascade: only meaningful action remaining at top-level is clearing
            // a non-empty filter. To prevent accidental loss, require two presses.
            if !app.filter.is_empty() {
                let armed = app
                    .last_esc
                    .map(|t| t.elapsed() < ESC_ARM_WINDOW)
                    .unwrap_or(false);
                if armed {
                    app.filter.clear();
                    app.update_filtered();
                    app.last_esc = None;
                } else {
                    app.last_esc = Some(Instant::now());
                    let s = i18n::strings();
                    app.set_status(s.esc_again_to_clear_filter.into());
                }
            }
        }
        KeyCode::Char('r') => {
            app.refresh();
            let s = i18n::strings();
            app.set_status(s.refreshed.into());
        }
        KeyCode::Char('s') if !app.session.is_root => {
            app.open_sudo_prompt(crate::app::SudoPurpose::Refresh);
        }

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            if app.view_mode == ViewMode::Connections || app.view_mode == ViewMode::Processes {
                app.selected = app.selected.saturating_sub(1);
            }
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.view_mode == ViewMode::Connections || app.view_mode == ViewMode::Processes {
                app.selected = (app.selected + 1).min(app.filtered_indices.len().saturating_sub(1));
            }
            app.scroll_offset = app.scroll_offset.saturating_add(1);
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.selected = 0;
            app.scroll_offset = 0;
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.selected = app.filtered_indices.len().saturating_sub(1);
            app.scroll_offset = u16::MAX;
        }

        // Toggle bottom Details panel (Connections only)
        KeyCode::Enter | KeyCode::Char('d') if app.view_mode == ViewMode::Connections => {
            app.show_details = !app.show_details;
        }

        // Kill (always available, top-level shortcut)
        KeyCode::Char('K') | KeyCode::Delete => {
            if let Some(entry) = app.selected_entry() {
                let pid = entry.entry.process.pid;
                let name = entry.entry.process.name.clone();
                app.confirm_kill = Some((pid, name));
            }
        }

        // Copy (always available, top-level shortcut)
        KeyCode::Char('c') => {
            if let Some(entry) = app.selected_entry() {
                let text = format!(
                    "{}:{} {} (pid {})",
                    entry.entry.local_addr.ip(),
                    entry.entry.local_port(),
                    entry.entry.process.name,
                    entry.entry.process.pid,
                );
                app.copy_to_clipboard(&text);
            }
        }

        // Sort (Connections only): o = next column, O = reverse direction.
        KeyCode::Char('o') if app.view_mode == ViewMode::Connections => {
            let cols = [
                SortColumn::Port,
                SortColumn::Service,
                SortColumn::Protocol,
                SortColumn::State,
                SortColumn::Pid,
                SortColumn::ProcessName,
                SortColumn::User,
            ];
            let idx = cols
                .iter()
                .position(|&c| c == app.session.sort.column)
                .unwrap_or(0);
            let next = (idx + 1) % cols.len();
            app.session.sort.toggle(cols[next]);
            app.refresh();
        }
        KeyCode::Char('O') if app.view_mode == ViewMode::Connections => {
            app.session.sort.ascending = !app.session.sort.ascending;
            app.refresh();
        }

        // Switch to Processes from Connections: ProcessesTab::Detail by default.
        KeyCode::Char('P') => {
            app.view_mode = ViewMode::Processes;
            app.processes_tab = ProcessesTab::Detail;
            app.scroll_offset = 0;
        }

        // Language
        KeyCode::Char('L') => {
            let current = i18n::lang();
            let next = current.next();
            i18n::set_lang(next);
            let s = i18n::strings();
            app.set_status(format!("{} → {}", s.lang_switched, next.label()));
        }
        _ => {}
    }
}
