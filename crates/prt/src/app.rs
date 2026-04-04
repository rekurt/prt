use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use prt_core::core::{killer, session::Session};
use prt_core::i18n;
use prt_core::model::{DetailTab, TrackedEntry, TICK_RATE};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Instant;

use crate::input::handle_key;
use crate::ui::draw;

pub struct App {
    pub session: Session,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub filter: String,
    pub filter_mode: bool,
    pub show_help: bool,
    pub show_details: bool,
    pub detail_tab: DetailTab,
    pub confirm_kill: Option<(u32, String)>,
    pub sudo_prompt: bool,
    pub sudo_password: String,
    pub status_msg: Option<(String, Instant)>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            session: Session::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            filter: String::new(),
            filter_mode: false,
            show_help: false,
            show_details: true,
            detail_tab: DetailTab::Tree,
            confirm_kill: None,
            sudo_prompt: false,
            sudo_password: String::new(),
            status_msg: None,
            should_quit: false,
        }
    }

    pub fn refresh(&mut self) {
        if let Err(msg) = self.session.refresh() {
            self.set_status(msg);
        }
        self.update_filtered();
    }

    pub fn update_filtered(&mut self) {
        self.filtered_indices = self.session.filtered_indices(&self.filter);
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn selected_entry(&self) -> Option<&TrackedEntry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.session.entries.get(i))
    }

    pub fn kill_selected(&mut self, force: bool) {
        if let Some(entry) = self.selected_entry() {
            let pid = entry.entry.process.pid;
            let name = entry.entry.process.name.clone();
            let s = i18n::strings();
            match killer::kill_process(pid, force) {
                Ok(()) => {
                    let sig = if force { "SIGKILL" } else { "SIGTERM" };
                    self.set_status(s.fmt_kill_sent(sig, &name, pid));
                    self.confirm_kill = None;
                }
                Err(e) => {
                    self.set_status(s.fmt_kill_failed(&e.to_string()));
                    self.confirm_kill = None;
                }
            }
        }
    }

    pub fn copy_to_clipboard(&mut self, text: &str) {
        let s = i18n::strings();
        match arboard::Clipboard::new().and_then(|mut clip| clip.set_text(text)) {
            Ok(()) => self.set_status(s.copied.into()),
            Err(_) => self.set_status(s.clipboard_unavailable.into()),
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_msg = Some((msg, Instant::now()));
    }

    pub fn try_sudo(&mut self) {
        let password = std::mem::take(&mut self.sudo_password);
        self.sudo_prompt = false;
        let msg = self.session.try_sudo(&password);
        self.set_status(msg);
        self.update_filtered();
    }
}

pub fn run() -> Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    let mut last_tick = Instant::now();

    app.refresh();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.should_quit = true;
                }
                handle_key(&mut app, key);
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            app.refresh();
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
