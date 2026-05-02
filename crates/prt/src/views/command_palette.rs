use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::i18n;
use prt_core::model::{ProcessesTab, SshTab, ViewMode};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

#[derive(Debug, Clone, Default)]
pub struct CommandPalette {
    pub input: String,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy)]
struct Command {
    label: &'static str,
    action: CommandAction,
}

#[derive(Debug, Clone, Copy)]
enum CommandAction {
    Refresh,
    TogglePause,
    ClearFilter,
    Connections,
    Processes,
    Ssh,
    Tunnels,
    Kill,
    CopyPid,
    Trace,
    Block,
}

const COMMANDS: &[Command] = &[
    Command {
        label: "refresh",
        action: CommandAction::Refresh,
    },
    Command {
        label: "pause",
        action: CommandAction::TogglePause,
    },
    Command {
        label: "clear filter",
        action: CommandAction::ClearFilter,
    },
    Command {
        label: "connections",
        action: CommandAction::Connections,
    },
    Command {
        label: "processes",
        action: CommandAction::Processes,
    },
    Command {
        label: "ssh",
        action: CommandAction::Ssh,
    },
    Command {
        label: "tunnels",
        action: CommandAction::Tunnels,
    },
    Command {
        label: "kill",
        action: CommandAction::Kill,
    },
    Command {
        label: "copy pid",
        action: CommandAction::CopyPid,
    },
    Command {
        label: "trace",
        action: CommandAction::Trace,
    },
    Command {
        label: "block",
        action: CommandAction::Block,
    },
];

pub fn open(app: &mut App) {
    app.command_palette = Some(CommandPalette::default());
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    let Some(palette) = app.command_palette.as_mut() else {
        return false;
    };

    match key.code {
        KeyCode::Esc => app.command_palette = None,
        KeyCode::Backspace => {
            palette.input.pop();
            palette.selected = 0;
        }
        KeyCode::Up => {
            palette.selected = palette.selected.saturating_sub(1);
        }
        KeyCode::Down => {
            let count = matching_commands(&palette.input).len();
            if count > 0 {
                palette.selected = (palette.selected + 1).min(count - 1);
            }
        }
        KeyCode::Char(c) => {
            if palette.input.len() < 128 {
                palette.input.push(c);
                palette.selected = 0;
            }
        }
        KeyCode::Enter => {
            let input = palette.input.clone();
            let selected = palette.selected;
            let command = matching_commands(&input).get(selected).copied();
            app.command_palette = None;
            if let Some(command) = command {
                execute(app, command.action);
            }
        }
        _ => {}
    }
    true
}

pub fn draw(f: &mut Frame, app: &App) {
    let Some(palette) = &app.command_palette else {
        return;
    };
    let s = i18n::strings();
    let area = f.area();
    let width = 48u16.min(area.width.saturating_sub(4));
    let height = 10u16.min(area.height.saturating_sub(2));
    let popup = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );

    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", s.command_palette_title))
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines = vec![Line::from(vec![
        Span::styled(": ", Style::default().fg(Color::Cyan)),
        Span::raw(&palette.input),
        Span::styled("\u{2588}", Style::default().fg(Color::White)),
    ])];

    let matches = matching_commands(&palette.input);
    if matches.is_empty() {
        lines.push(Line::from(Span::styled(
            s.command_palette_empty,
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (idx, command) in matches
            .iter()
            .take(inner.height.saturating_sub(1) as usize)
            .enumerate()
        {
            let style = if idx == palette.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(
                format!(" {}", command.label),
                style,
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn matching_commands(input: &str) -> Vec<Command> {
    let needle = input.trim().to_lowercase();
    COMMANDS
        .iter()
        .copied()
        .filter(|command| needle.is_empty() || command.label.contains(&needle))
        .collect()
}

fn execute(app: &mut App, action: CommandAction) {
    match action {
        CommandAction::Refresh => app.refresh(),
        CommandAction::TogglePause => app.auto_refresh_paused = !app.auto_refresh_paused,
        CommandAction::ClearFilter => {
            app.filter.clear();
            app.update_filtered();
        }
        CommandAction::Connections => app.view_mode = ViewMode::Connections,
        CommandAction::Processes => {
            app.view_mode = ViewMode::Processes;
            app.processes_tab = ProcessesTab::Detail;
        }
        CommandAction::Ssh => app.view_mode = ViewMode::Ssh,
        CommandAction::Tunnels => {
            app.view_mode = ViewMode::Ssh;
            app.ssh_tab = SshTab::Tunnels;
        }
        CommandAction::Kill => {
            if let Some(entry) = app.selected_entry() {
                app.confirm_kill =
                    Some((entry.entry.process.pid, entry.entry.process.name.clone()));
            }
        }
        CommandAction::CopyPid => {
            if let Some(entry) = app.selected_entry() {
                app.copy_to_clipboard(&entry.entry.process.pid.to_string());
            }
        }
        CommandAction::Trace => app.toggle_tracer(),
        CommandAction::Block => app.initiate_block(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_commands_filters_by_substring() {
        let labels: Vec<_> = matching_commands("tun")
            .into_iter()
            .map(|command| command.label)
            .collect();
        assert_eq!(labels, vec!["tunnels"]);
    }

    #[test]
    fn handle_key_allows_typing_j_and_k() {
        let mut app = App::new();
        open(&mut app);

        handle_key(&mut app, KeyEvent::from(KeyCode::Char('k')));
        handle_key(&mut app, KeyEvent::from(KeyCode::Char('j')));

        let palette = app.command_palette.unwrap();
        assert_eq!(palette.input, "kj");
        assert_eq!(palette.selected, 0);
    }
}
