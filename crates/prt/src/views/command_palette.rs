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
    english_alias: &'static str,
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
        english_alias: "refresh",
        action: CommandAction::Refresh,
    },
    Command {
        english_alias: "pause",
        action: CommandAction::TogglePause,
    },
    Command {
        english_alias: "clear filter",
        action: CommandAction::ClearFilter,
    },
    Command {
        english_alias: "connections",
        action: CommandAction::Connections,
    },
    Command {
        english_alias: "processes",
        action: CommandAction::Processes,
    },
    Command {
        english_alias: "ssh",
        action: CommandAction::Ssh,
    },
    Command {
        english_alias: "tunnels",
        action: CommandAction::Tunnels,
    },
    Command {
        english_alias: "kill",
        action: CommandAction::Kill,
    },
    Command {
        english_alias: "copy pid",
        action: CommandAction::CopyPid,
    },
    Command {
        english_alias: "trace",
        action: CommandAction::Trace,
    },
    Command {
        english_alias: "block",
        action: CommandAction::Block,
    },
];

impl Command {
    fn label(self, s: &'static i18n::Strings) -> &'static str {
        match self.action {
            CommandAction::Refresh => s.command_refresh,
            CommandAction::TogglePause => s.command_pause,
            CommandAction::ClearFilter => s.command_clear_filter,
            CommandAction::Connections => s.section_connections,
            CommandAction::Processes => s.section_processes,
            CommandAction::Ssh => s.section_ssh,
            CommandAction::Tunnels => s.view_tunnels,
            CommandAction::Kill => s.action_kill,
            CommandAction::CopyPid => s.action_copy_pid,
            CommandAction::Trace => s.action_trace,
            CommandAction::Block => s.action_block,
        }
    }
}

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
            let count = matching_commands(&palette.input, i18n::strings()).len();
            if count > 0 {
                palette.selected = (palette.selected + 1).min(count - 1);
            }
        }
        KeyCode::Char(c) if palette.input.len() < 128 => {
            palette.input.push(c);
            palette.selected = 0;
        }
        KeyCode::Enter => {
            let input = palette.input.clone();
            let selected = palette.selected;
            let command = matching_commands(&input, i18n::strings())
                .get(selected)
                .copied();
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

    let matches = matching_commands(&palette.input, s);
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
                format!(" {}", command.label(s)),
                style,
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn matching_commands(input: &str, s: &'static i18n::Strings) -> Vec<Command> {
    let needle = input.trim().to_lowercase();
    COMMANDS
        .iter()
        .copied()
        .filter(|command| {
            needle.is_empty()
                || command.label(s).to_lowercase().contains(&needle)
                || command.english_alias.contains(&needle)
        })
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
    use std::sync::Mutex;

    static LANG_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn matching_commands_filters_by_substring() {
        let _guard = LANG_TEST_LOCK.lock().unwrap();
        i18n::set_lang(i18n::Lang::En);
        let s = i18n::strings();
        let labels: Vec<_> = matching_commands("tun", s)
            .into_iter()
            .map(|command| command.label(s))
            .collect();
        assert_eq!(labels, vec!["Tunnels"]);
    }

    #[test]
    fn command_labels_and_search_follow_active_language() {
        let _guard = LANG_TEST_LOCK.lock().unwrap();
        i18n::set_lang(i18n::Lang::Ru);
        let s = i18n::strings();
        let labels: Vec<_> = matching_commands("тун", s)
            .into_iter()
            .map(|command| command.label(s))
            .collect();
        assert_eq!(labels, vec!["Туннели"]);

        let labels: Vec<_> = matching_commands("очистить", s)
            .into_iter()
            .map(|command| command.label(s))
            .collect();
        assert_eq!(labels, vec!["Очистить фильтр"]);

        i18n::set_lang(i18n::Lang::Zh);
        let s = i18n::strings();
        let labels: Vec<_> = matching_commands("刷新", s)
            .into_iter()
            .map(|command| command.label(s))
            .collect();
        assert_eq!(labels, vec!["刷新"]);

        i18n::set_lang(i18n::Lang::En);
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
