//! Context-aware action menu, opened with `Space` over the selected entry.
//!
//! Builds a list of [`ActionItem`]s relevant to the current view and the
//! capabilities of the selected entry (e.g. `BlockIp` and `Forward` only
//! appear when the entry has a remote address).

use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::i18n;
use prt_core::model::{ActionItem, ViewMode};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

#[derive(Debug, Clone)]
pub struct ActionMenu {
    pub items: Vec<ActionMenuEntry>,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ActionMenuEntry {
    pub item: ActionItem,
    pub enabled: bool,
    pub reason: Option<&'static str>,
}

impl ActionMenu {
    pub fn for_app(app: &App) -> Option<Self> {
        let entry = app.selected_entry()?;
        let has_remote = entry.entry.remote_addr.is_some();

        let items = match app.view_mode {
            ViewMode::Connections => {
                let s = i18n::strings();
                let mut v = vec![
                    enabled(ActionItem::Kill),
                    enabled(ActionItem::Copy),
                    enabled(ActionItem::CopyPid),
                ];
                v.push(if has_remote {
                    enabled(ActionItem::BlockIp)
                } else {
                    disabled(ActionItem::BlockIp, s.action_unavailable_no_remote)
                });
                v.push(if has_remote {
                    enabled(ActionItem::Forward)
                } else {
                    disabled(ActionItem::Forward, s.action_unavailable_no_remote)
                });
                v.push(enabled(ActionItem::Trace));
                v
            }
            ViewMode::Processes => {
                vec![
                    enabled(ActionItem::Kill),
                    enabled(ActionItem::Copy),
                    enabled(ActionItem::CopyPid),
                    enabled(ActionItem::Trace),
                ]
            }
            ViewMode::Ssh => return None,
        };

        Some(Self { items, selected: 0 })
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.items.len().saturating_sub(1);
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }
}

pub fn open(app: &mut App) {
    if app.action_menu.is_some() {
        return;
    }
    if let Some(menu) = ActionMenu::for_app(app) {
        app.action_menu = Some(menu);
    }
}

/// Returns true when the key was consumed by the menu (incl. closing it).
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    let Some(menu) = app.action_menu.as_mut() else {
        return false;
    };

    match key.code {
        KeyCode::Esc => {
            app.action_menu = None;
        }
        KeyCode::Up | KeyCode::Char('k') => menu.move_up(),
        KeyCode::Down | KeyCode::Char('j') => menu.move_down(),
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let idx = (c as u8 - b'0') as usize;
            if idx >= 1 && idx <= menu.items.len() {
                menu.selected = idx - 1;
                let entry = menu.items[menu.selected];
                if !entry.enabled {
                    return true;
                }
                app.action_menu = None;
                execute(app, entry.item);
            }
        }
        KeyCode::Enter => {
            let entry = menu.items[menu.selected];
            if !entry.enabled {
                return true;
            }
            app.action_menu = None;
            execute(app, entry.item);
        }
        _ => {}
    }
    true
}

fn execute(app: &mut App, item: ActionItem) {
    match item {
        ActionItem::Kill => {
            if let Some(entry) = app.selected_entry() {
                let pid = entry.entry.process.pid;
                let name = entry.entry.process.name.clone();
                app.confirm_kill = Some((pid, name));
            }
        }
        ActionItem::Copy => {
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
        ActionItem::CopyPid => {
            if let Some(entry) = app.selected_entry() {
                let text = entry.entry.process.pid.to_string();
                app.copy_to_clipboard(&text);
            }
        }
        ActionItem::BlockIp => app.initiate_block(),
        ActionItem::Trace => app.toggle_tracer(),
        ActionItem::Forward => {
            app.forward_prompt = true;
            app.forward_input.clear();
        }
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let Some(menu) = &app.action_menu else { return };

    let s = i18n::strings();
    let area = f.area();
    let width: u16 = 36;
    let height: u16 = (menu.items.len() as u16) + 2;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect {
        x,
        y,
        width,
        height,
    };

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", s.action_menu_title))
        .title_alignment(Alignment::Left)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines: Vec<Line> = menu
        .items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if !entry.enabled {
                Style::default().fg(Color::DarkGray)
            } else if i == menu.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            let label = match entry.reason {
                Some(reason) => format!(" {} ({reason}) ", action_label(entry.item)),
                None => format!(" {} ", action_label(entry.item)),
            };
            Line::from(vec![
                Span::styled(format!(" {} ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(label, style),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}

fn enabled(item: ActionItem) -> ActionMenuEntry {
    ActionMenuEntry {
        item,
        enabled: true,
        reason: None,
    }
}

fn disabled(item: ActionItem, reason: &'static str) -> ActionMenuEntry {
    ActionMenuEntry {
        item,
        enabled: false,
        reason: Some(reason),
    }
}

fn action_label(item: ActionItem) -> &'static str {
    let s = i18n::strings();
    match item {
        ActionItem::Kill => s.action_kill,
        ActionItem::Copy => s.action_copy,
        ActionItem::CopyPid => s.action_copy_pid,
        ActionItem::BlockIp => s.action_block,
        ActionItem::Trace => s.action_trace,
        ActionItem::Forward => s.action_forward,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_up_wraps_to_last() {
        let mut m = ActionMenu {
            items: vec![
                enabled(ActionItem::Kill),
                enabled(ActionItem::Copy),
                enabled(ActionItem::CopyPid),
            ],
            selected: 0,
        };
        m.move_up();
        assert_eq!(m.selected, 2);
    }

    #[test]
    fn move_down_wraps_to_first() {
        let mut m = ActionMenu {
            items: vec![enabled(ActionItem::Kill), enabled(ActionItem::Copy)],
            selected: 1,
        };
        m.move_down();
        assert_eq!(m.selected, 0);
    }
}
