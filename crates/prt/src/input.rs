use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::i18n;
use prt_core::model::{DetailTab, SortColumn, ViewMode};

pub fn handle_key(app: &mut App, key: KeyEvent) {
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

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('/') => app.filter_mode = true,
        KeyCode::Esc => {
            // Esc: return to Table from fullscreen views, or clear filter
            if app.view_mode != ViewMode::Table {
                app.view_mode = ViewMode::Table;
                app.scroll_offset = 0;
            } else if !app.filter.is_empty() {
                app.filter.clear();
                app.update_filtered();
            }
        }
        KeyCode::Char('r') => {
            app.refresh();
            let s = i18n::strings();
            app.set_status(s.refreshed.into());
        }
        KeyCode::Char('s') => {
            if !app.session.is_root {
                app.sudo_prompt = true;
                app.sudo_password.clear();
            }
        }

        // Navigation (works in all view modes)
        KeyCode::Up | KeyCode::Char('k') => {
            if app.view_mode == ViewMode::Table || app.view_mode == ViewMode::ProcessDetail {
                app.selected = app.selected.saturating_sub(1);
            }
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.view_mode == ViewMode::Table || app.view_mode == ViewMode::ProcessDetail {
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
            app.scroll_offset = u16::MAX; // will be clamped by rendering
        }

        // Toggle bottom detail panel (Table mode only)
        KeyCode::Enter | KeyCode::Char('d') => {
            if app.view_mode == ViewMode::Table {
                app.show_details = !app.show_details;
            }
        }

        // Keys 1-3: bottom panel tabs (in Table mode)
        KeyCode::Char('1') => {
            if app.view_mode == ViewMode::Table {
                app.detail_tab = DetailTab::Tree;
                app.show_details = true;
            }
        }
        KeyCode::Char('2') => {
            if app.view_mode == ViewMode::Table {
                app.detail_tab = DetailTab::Interface;
                app.show_details = true;
            }
        }
        KeyCode::Char('3') => {
            if app.view_mode == ViewMode::Table {
                app.detail_tab = DetailTab::Connection;
                app.show_details = true;
            }
        }

        // Keys 4-7: toggle fullscreen views (press again = back to Table)
        KeyCode::Char('4') => {
            app.scroll_offset = 0;
            app.view_mode = if app.view_mode == ViewMode::Chart {
                ViewMode::Table
            } else {
                ViewMode::Chart
            };
        }
        KeyCode::Char('5') => {
            app.scroll_offset = 0;
            app.view_mode = if app.view_mode == ViewMode::Topology {
                ViewMode::Table
            } else {
                ViewMode::Topology
            };
        }
        KeyCode::Char('6') => {
            app.scroll_offset = 0;
            app.view_mode = if app.view_mode == ViewMode::ProcessDetail {
                ViewMode::Table
            } else {
                ViewMode::ProcessDetail
            };
        }
        KeyCode::Char('7') => {
            app.scroll_offset = 0;
            app.view_mode = if app.view_mode == ViewMode::Namespaces {
                ViewMode::Table
            } else {
                ViewMode::Namespaces
            };
        }

        // Left/Right: switch bottom panel tabs in Table mode
        KeyCode::Right | KeyCode::Char('l') => {
            if app.view_mode == ViewMode::Table && app.show_details {
                app.detail_tab = app.detail_tab.next();
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.view_mode == ViewMode::Table && app.show_details {
                app.detail_tab = app.detail_tab.prev();
            }
        }

        // Kill
        KeyCode::Char('K') | KeyCode::Delete => {
            if let Some(entry) = app.selected_entry() {
                let pid = entry.entry.process.pid;
                let name = entry.entry.process.name.clone();
                app.confirm_kill = Some((pid, name));
            }
        }

        // Copy
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
        KeyCode::Char('p') => {
            if let Some(entry) = app.selected_entry() {
                let text = entry.entry.process.pid.to_string();
                app.copy_to_clipboard(&text);
            }
        }

        // Firewall block
        KeyCode::Char('b') => {
            app.initiate_block();
        }

        // Strace attach/detach
        KeyCode::Char('t') => {
            app.toggle_tracer();
        }

        // Sort (Table mode)
        KeyCode::Tab => {
            if app.view_mode == ViewMode::Table {
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
        }
        KeyCode::BackTab => {
            if app.view_mode == ViewMode::Table {
                app.session.sort.ascending = !app.session.sort.ascending;
                app.refresh();
            }
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
