//! Multi-field popup form for creating an SSH tunnel.

use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};
use prt_core::core::ssh_tunnel::{SshTunnelSpec, TunnelKind};
use prt_core::i18n;
use ratatui::prelude::*;
use ratatui::widgets::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelFormField {
    Kind,
    LocalPort,
    RemoteHost,
    RemotePort,
    HostAlias,
}

#[derive(Debug, Clone)]
pub struct TunnelFormState {
    pub kind: TunnelKind,
    pub local_port: String,
    pub remote_host: String,
    pub remote_port: String,
    pub host_alias: String,
    pub focused: TunnelFormField,
    /// When set, submit replaces the tunnel at this index instead of creating a new one.
    pub editing_idx: Option<usize>,
}

impl TunnelFormState {
    pub fn new(prefill_alias: Option<String>) -> Self {
        Self {
            kind: TunnelKind::Local,
            local_port: String::new(),
            remote_host: "127.0.0.1".into(),
            remote_port: String::new(),
            host_alias: prefill_alias.unwrap_or_default(),
            focused: TunnelFormField::Kind,
            editing_idx: None,
        }
    }

    /// Open the form in edit-mode for an existing tunnel.
    pub fn edit(spec: &SshTunnelSpec, idx: usize) -> Self {
        Self {
            kind: spec.kind,
            local_port: spec.local_port.to_string(),
            remote_host: spec
                .remote_host
                .clone()
                .unwrap_or_else(|| "127.0.0.1".into()),
            remote_port: spec
                .remote_port
                .map(|p| p.to_string())
                .unwrap_or_default(),
            host_alias: spec.host_alias.clone(),
            focused: TunnelFormField::LocalPort,
            editing_idx: Some(idx),
        }
    }

    /// Returns Some(error) if the field as currently typed is invalid.
    /// Used for inline (per-field) validation feedback.
    pub fn validate_field(&self, field: TunnelFormField) -> Option<&'static str> {
        let s = i18n::strings();
        match field {
            TunnelFormField::Kind => None,
            TunnelFormField::LocalPort => {
                if self.local_port.trim().is_empty() {
                    return Some(s.tunnel_form_field_required);
                }
                self.local_port
                    .trim()
                    .parse::<u16>()
                    .err()
                    .map(|_| "1..=65535")
            }
            TunnelFormField::RemoteHost => {
                if self.kind == TunnelKind::Dynamic {
                    return None;
                }
                if self.remote_host.trim().is_empty() {
                    Some(s.tunnel_form_field_required)
                } else {
                    None
                }
            }
            TunnelFormField::RemotePort => {
                if self.kind == TunnelKind::Dynamic {
                    return None;
                }
                if self.remote_port.trim().is_empty() {
                    return Some(s.tunnel_form_field_required);
                }
                self.remote_port
                    .trim()
                    .parse::<u16>()
                    .err()
                    .map(|_| "1..=65535")
            }
            TunnelFormField::HostAlias => {
                if self.host_alias.trim().is_empty() {
                    Some(s.tunnel_form_field_required)
                } else {
                    None
                }
            }
        }
    }

    /// Has the user entered any data that would be lost on Esc?
    pub fn is_dirty(&self) -> bool {
        !self.local_port.is_empty()
            || !self.remote_port.is_empty()
            || !self.host_alias.is_empty()
            || self.remote_host != "127.0.0.1"
    }

    pub fn next_field(&mut self) {
        self.focused = match self.focused {
            TunnelFormField::Kind => TunnelFormField::LocalPort,
            TunnelFormField::LocalPort => match self.kind {
                TunnelKind::Local => TunnelFormField::RemoteHost,
                TunnelKind::Dynamic => TunnelFormField::HostAlias,
            },
            TunnelFormField::RemoteHost => TunnelFormField::RemotePort,
            TunnelFormField::RemotePort => TunnelFormField::HostAlias,
            TunnelFormField::HostAlias => TunnelFormField::Kind,
        };
    }

    pub fn prev_field(&mut self) {
        self.focused = match self.focused {
            TunnelFormField::Kind => TunnelFormField::HostAlias,
            TunnelFormField::LocalPort => TunnelFormField::Kind,
            TunnelFormField::RemoteHost => TunnelFormField::LocalPort,
            TunnelFormField::RemotePort => TunnelFormField::RemoteHost,
            TunnelFormField::HostAlias => match self.kind {
                TunnelKind::Local => TunnelFormField::RemotePort,
                TunnelKind::Dynamic => TunnelFormField::LocalPort,
            },
        };
    }

    pub fn toggle_kind(&mut self) {
        self.kind = match self.kind {
            TunnelKind::Local => TunnelKind::Dynamic,
            TunnelKind::Dynamic => TunnelKind::Local,
        };
        // If we just made remote_* fields irrelevant, jump focus forward.
        if self.kind == TunnelKind::Dynamic
            && matches!(
                self.focused,
                TunnelFormField::RemoteHost | TunnelFormField::RemotePort
            )
        {
            self.focused = TunnelFormField::HostAlias;
        }
    }

    pub fn build_spec(&self) -> Result<SshTunnelSpec, String> {
        let local_port: u16 = self
            .local_port
            .trim()
            .parse()
            .map_err(|_| "local_port must be a number 1..=65535".to_string())?;
        let host_alias = self.host_alias.trim();
        if host_alias.is_empty() {
            return Err("ssh host (alias) is required".into());
        }

        let (remote_host, remote_port) = match self.kind {
            TunnelKind::Local => {
                let h = self.remote_host.trim();
                if h.is_empty() {
                    return Err("remote host required for local tunnel".into());
                }
                let p: u16 = self
                    .remote_port
                    .trim()
                    .parse()
                    .map_err(|_| "remote_port must be a number 1..=65535".to_string())?;
                (Some(h.to_string()), Some(p))
            }
            TunnelKind::Dynamic => (None, None),
        };

        let spec = SshTunnelSpec {
            name: None,
            kind: self.kind,
            local_port,
            remote_host,
            remote_port,
            host_alias: host_alias.to_string(),
        };
        spec.validate()?;
        Ok(spec)
    }

    fn current_text_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            TunnelFormField::Kind => None,
            TunnelFormField::LocalPort => Some(&mut self.local_port),
            TunnelFormField::RemoteHost => Some(&mut self.remote_host),
            TunnelFormField::RemotePort => Some(&mut self.remote_port),
            TunnelFormField::HostAlias => Some(&mut self.host_alias),
        }
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let form = match &app.tunnel_form {
        Some(s) => s,
        None => return,
    };
    let s = i18n::strings();
    let area = f.area();
    let w = 64u16.min(area.width.saturating_sub(4));
    let h = 12u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let popup_area = Rect::new(x, y, w, h);

    f.render_widget(Clear, popup_area);

    let title = if form.editing_idx.is_some() {
        s.tunnel_form_edit_title
    } else {
        s.tunnel_form_title
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let kind_value = match form.kind {
        TunnelKind::Local => s.tunnel_kind_local,
        TunnelKind::Dynamic => s.tunnel_kind_dynamic,
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(field_line(
        s.tunnel_form_kind,
        &format!("\u{25c0} {kind_value} \u{25b6}"),
        form.focused == TunnelFormField::Kind,
        false,
        None,
    ));
    lines.push(field_line(
        s.tunnel_form_local_port,
        &form.local_port,
        form.focused == TunnelFormField::LocalPort,
        true,
        form.validate_field(TunnelFormField::LocalPort),
    ));

    let dim_remote = form.kind == TunnelKind::Dynamic;
    lines.push(field_line_dim(
        s.tunnel_form_remote_host,
        &form.remote_host,
        form.focused == TunnelFormField::RemoteHost,
        true,
        dim_remote,
        if dim_remote {
            None
        } else {
            form.validate_field(TunnelFormField::RemoteHost)
        },
    ));
    lines.push(field_line_dim(
        s.tunnel_form_remote_port,
        &form.remote_port,
        form.focused == TunnelFormField::RemotePort,
        true,
        dim_remote,
        if dim_remote {
            None
        } else {
            form.validate_field(TunnelFormField::RemotePort)
        },
    ));
    lines.push(field_line(
        s.tunnel_form_host_alias,
        &form.host_alias,
        form.focused == TunnelFormField::HostAlias,
        true,
        form.validate_field(TunnelFormField::HostAlias),
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        s.tunnel_form_hint,
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(Paragraph::new(lines), inner);
}

fn field_line(
    label: &'static str,
    value: &str,
    focused: bool,
    cursor: bool,
    error: Option<&str>,
) -> Line<'static> {
    field_line_dim(label, value, focused, cursor, false, error)
}

fn field_line_dim(
    label: &'static str,
    value: &str,
    focused: bool,
    cursor: bool,
    dim: bool,
    error: Option<&str>,
) -> Line<'static> {
    let has_error = error.is_some();
    let label_style = if dim {
        Style::default().fg(Color::DarkGray)
    } else if has_error {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let value_style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(if has_error { Color::Red } else { Color::Cyan })
            .add_modifier(Modifier::BOLD)
    } else if dim {
        Style::default().fg(Color::DarkGray)
    } else if has_error {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
    };
    let mut spans = vec![
        Span::styled(label.to_string(), label_style),
        Span::styled(value.to_string(), value_style),
    ];
    if cursor && focused {
        spans.push(Span::styled(
            "\u{2588}".to_string(),
            Style::default().fg(Color::White),
        ));
    }
    if let Some(err) = error {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("\u{2717} {err}"),
            Style::default().fg(Color::Red),
        ));
    }
    Line::from(spans)
}

/// Returns true if the key was consumed.
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    let form = match &mut app.tunnel_form {
        Some(s) => s,
        None => return false,
    };
    match key.code {
        KeyCode::Esc => {
            // If the form has unsaved input, require a second Esc within the
            // arm window before discarding (mirrors the filter cascade).
            if form.is_dirty() {
                let armed = app
                    .last_esc
                    .map(|t| t.elapsed() < std::time::Duration::from_millis(1500))
                    .unwrap_or(false);
                if armed {
                    app.tunnel_form = None;
                    app.last_esc = None;
                } else {
                    app.last_esc = Some(std::time::Instant::now());
                    let s = i18n::strings();
                    app.set_status(s.esc_again_to_discard_form.into());
                }
            } else {
                app.tunnel_form = None;
            }
            true
        }
        KeyCode::Enter => {
            let editing_idx = form.editing_idx;
            let result = form.build_spec();
            match result {
                Ok(spec) => {
                    app.tunnel_form = None;
                    match editing_idx {
                        Some(idx) => app.replace_tunnel(idx, spec),
                        None => app.create_tunnel(spec),
                    }
                }
                Err(e) => {
                    app.set_status(format!("{}: {e}", i18n::strings().tunnel_form_invalid));
                }
            }
            true
        }
        KeyCode::Tab => {
            form.next_field();
            true
        }
        KeyCode::BackTab => {
            form.prev_field();
            true
        }
        KeyCode::Left => {
            if form.focused == TunnelFormField::Kind {
                form.toggle_kind();
            }
            true
        }
        KeyCode::Right => {
            if form.focused == TunnelFormField::Kind {
                form.toggle_kind();
            }
            true
        }
        KeyCode::Backspace => {
            if let Some(field) = form.current_text_mut() {
                field.pop();
            }
            true
        }
        KeyCode::Char(' ') if form.focused == TunnelFormField::Kind => {
            form.toggle_kind();
            true
        }
        KeyCode::Char(c) => {
            if let Some(field) = form.current_text_mut() {
                if field.len() < 256 {
                    field.push(c);
                }
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_spec_validates_local() {
        let mut f = TunnelFormState::new(Some("prod".into()));
        f.local_port = "5433".into();
        f.remote_host = "127.0.0.1".into();
        f.remote_port = "5432".into();
        let spec = f.build_spec().unwrap();
        assert_eq!(spec.local_port, 5433);
        assert_eq!(spec.host_alias, "prod");
        assert_eq!(spec.kind, TunnelKind::Local);
    }

    #[test]
    fn build_spec_dynamic_skips_remote() {
        let mut f = TunnelFormState::new(Some("prod".into()));
        f.kind = TunnelKind::Dynamic;
        f.local_port = "1080".into();
        let spec = f.build_spec().unwrap();
        assert_eq!(spec.kind, TunnelKind::Dynamic);
        assert!(spec.remote_host.is_none());
    }

    #[test]
    fn build_spec_rejects_missing_alias() {
        let mut f = TunnelFormState::new(None);
        f.local_port = "1".into();
        f.remote_host = "h".into();
        f.remote_port = "1".into();
        assert!(f.build_spec().is_err());
    }

    #[test]
    fn build_spec_rejects_bad_port() {
        let mut f = TunnelFormState::new(Some("p".into()));
        f.local_port = "abc".into();
        f.remote_host = "h".into();
        f.remote_port = "1".into();
        assert!(f.build_spec().is_err());
    }

    #[test]
    fn next_field_skips_remote_for_dynamic() {
        let mut f = TunnelFormState::new(None);
        f.kind = TunnelKind::Dynamic;
        f.focused = TunnelFormField::LocalPort;
        f.next_field();
        assert_eq!(f.focused, TunnelFormField::HostAlias);
    }

    #[test]
    fn edit_constructor_fills_all_fields() {
        let spec = SshTunnelSpec {
            name: None,
            kind: TunnelKind::Local,
            local_port: 5433,
            remote_host: Some("db.internal".into()),
            remote_port: Some(5432),
            host_alias: "prod".into(),
        };
        let f = TunnelFormState::edit(&spec, 7);
        assert_eq!(f.local_port, "5433");
        assert_eq!(f.remote_host, "db.internal");
        assert_eq!(f.remote_port, "5432");
        assert_eq!(f.host_alias, "prod");
        assert_eq!(f.editing_idx, Some(7));
    }

    #[test]
    fn validate_field_returns_error_for_empty_required() {
        let f = TunnelFormState::new(None);
        assert!(f.validate_field(TunnelFormField::LocalPort).is_some());
        assert!(f.validate_field(TunnelFormField::HostAlias).is_some());
    }

    #[test]
    fn validate_field_rejects_non_numeric_port() {
        let mut f = TunnelFormState::new(Some("p".into()));
        f.local_port = "abc".into();
        assert!(f.validate_field(TunnelFormField::LocalPort).is_some());
    }

    #[test]
    fn is_dirty_after_typing() {
        let mut f = TunnelFormState::new(None);
        assert!(!f.is_dirty());
        f.local_port = "1".into();
        assert!(f.is_dirty());
    }

    #[test]
    fn toggle_kind_jumps_focus_off_remote() {
        let mut f = TunnelFormState::new(None);
        f.focused = TunnelFormField::RemoteHost;
        f.toggle_kind();
        assert_eq!(f.kind, TunnelKind::Dynamic);
        assert_eq!(f.focused, TunnelFormField::HostAlias);
    }
}
