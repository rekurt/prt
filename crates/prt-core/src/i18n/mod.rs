//! Internationalization with runtime language switching.
//!
//! Supports English, Russian, and Chinese. Language is stored in an
//! [`AtomicU8`] for lock-free reads — every frame calls [`strings()`]
//! to get the current string table, so switching is instantaneous.
//!
//! # Language resolution priority
//!
//! 1. `--lang` CLI flag
//! 2. `PRT_LANG` environment variable
//! 3. System locale (via `sys-locale`)
//! 4. English (default)
//!
//! # Adding a new language
//!
//! 1. Create `xx.rs` with `pub static STRINGS: Strings = Strings { ... }`
//! 2. Add variant to [`Lang`] enum
//! 3. Update [`strings()`], [`Lang::next()`], [`Lang::label()`], `Lang::from_u8()`
//! 4. Compile — any missing `Strings` fields will be caught at compile time

pub mod en;
pub mod ru;
pub mod zh;

use std::sync::atomic::{AtomicU8, Ordering};

/// Supported UI language.
///
/// Stored as `u8` in an `AtomicU8` for lock-free runtime switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En = 0,
    Ru = 1,
    Zh = 2,
}

impl Lang {
    /// Cycle to next language: En → Ru → Zh → En
    pub fn next(self) -> Self {
        match self {
            Self::En => Self::Ru,
            Self::Ru => Self::Zh,
            Self::Zh => Self::En,
        }
    }

    /// Short display name for status bar
    pub fn label(self) -> &'static str {
        match self {
            Self::En => "EN",
            Self::Ru => "RU",
            Self::Zh => "ZH",
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Ru,
            2 => Self::Zh,
            _ => Self::En,
        }
    }
}

static LANG: AtomicU8 = AtomicU8::new(0); // 0 = En

/// Set the active UI language. Thread-safe, lock-free.
pub fn set_lang(lang: Lang) {
    LANG.store(lang as u8, Ordering::Relaxed);
}

/// Get the current UI language.
pub fn lang() -> Lang {
    Lang::from_u8(LANG.load(Ordering::Relaxed))
}

/// Get the string table for the current language.
/// Called on every frame — must be fast (just an atomic load + match).
pub fn strings() -> &'static Strings {
    match lang() {
        Lang::En => &en::STRINGS,
        Lang::Ru => &ru::STRINGS,
        Lang::Zh => &zh::STRINGS,
    }
}

/// Detect language from environment: PRT_LANG env, then system locale, fallback En.
pub fn detect_locale() -> Lang {
    if let Ok(val) = std::env::var("PRT_LANG") {
        return parse_lang(&val);
    }

    if let Some(locale) = sys_locale::get_locale() {
        let lower = locale.to_lowercase();
        if lower.starts_with("ru") {
            return Lang::Ru;
        }
        if lower.starts_with("zh") {
            return Lang::Zh;
        }
    }

    Lang::En
}

/// Parse a language string (e.g. "ru", "chinese") into a [`Lang`] variant.
/// Unknown strings default to English.
pub fn parse_lang(s: &str) -> Lang {
    match s.to_lowercase().as_str() {
        "ru" | "russian" => Lang::Ru,
        "zh" | "cn" | "chinese" => Lang::Zh,
        _ => Lang::En,
    }
}

/// All localizable UI strings for one language.
///
/// Each language module (`en`, `ru`, `zh`) provides a `static STRINGS: Strings`.
/// Adding a field here forces all language files to be updated — compile-time
/// completeness check.
pub struct Strings {
    pub app_name: &'static str,

    // Header
    pub connections: &'static str,
    pub no_root_warning: &'static str,
    pub sudo_ok: &'static str,
    pub filter_label: &'static str,
    pub search_mode: &'static str,

    // Detail tabs
    pub tab_tree: &'static str,
    pub tab_network: &'static str,
    pub tab_connection: &'static str,
    pub no_selected_process: &'static str,

    // View mode labels (fullscreen views)
    pub view_chart: &'static str,
    pub view_topology: &'static str,
    pub view_process: &'static str,
    pub view_namespaces: &'static str,

    // Tree view
    pub process_not_found: &'static str,

    // Interface tab
    pub iface_address: &'static str,
    pub iface_interface: &'static str,
    pub iface_protocol: &'static str,
    pub iface_bind: &'static str,
    pub iface_localhost_only: &'static str,
    pub iface_all_interfaces: &'static str,
    pub iface_specific: &'static str,
    pub iface_loopback: &'static str,
    pub iface_all: &'static str,

    // Connection tab
    pub conn_local: &'static str,
    pub conn_remote: &'static str,
    pub conn_state: &'static str,
    pub conn_process: &'static str,
    pub conn_cmdline: &'static str,

    // Actions
    pub help_text: &'static str,
    pub kill_cancel: &'static str,
    pub copied: &'static str,
    pub refreshed: &'static str,
    pub clipboard_unavailable: &'static str,
    pub scan_error: &'static str,
    pub cancelled: &'static str,
    pub lang_switched: &'static str,

    // Sudo
    pub sudo_prompt_title: &'static str,
    pub sudo_password_label: &'static str,
    pub sudo_confirm_hint: &'static str,
    pub sudo_failed: &'static str,
    pub sudo_wrong_password: &'static str,
    pub sudo_elevated: &'static str,

    // Footer hints — common
    pub hint_help: &'static str,
    pub hint_search: &'static str,
    pub hint_kill: &'static str,
    pub hint_sudo: &'static str,
    pub hint_quit: &'static str,
    pub hint_lang: &'static str,

    // Footer hints — context-specific
    pub hint_back: &'static str,
    pub hint_details: &'static str,
    pub hint_views: &'static str,
    pub hint_sort: &'static str,
    pub hint_copy: &'static str,
    pub hint_block: &'static str,
    pub hint_trace: &'static str,
    pub hint_navigate: &'static str,
    pub hint_tabs: &'static str,

    // Forward dialog
    pub forward_prompt_title: &'static str,
    pub forward_host_label: &'static str,
    pub forward_confirm_hint: &'static str,
    pub hint_forward: &'static str,

    // Help overlay
    pub help_title: &'static str,
}

impl Strings {
    pub fn fmt_connections(&self, n: usize) -> String {
        format!("{n} {}", self.connections)
    }

    pub fn fmt_kill_confirm(&self, name: &str, pid: u32) -> String {
        match lang() {
            Lang::En => format!("Kill {name} (pid {pid})?"),
            Lang::Ru => format!("Завершить {name} (pid {pid})?"),
            Lang::Zh => format!("终止 {name} (pid {pid})?"),
        }
    }

    pub fn fmt_kill_sent(&self, sig: &str, name: &str, pid: u32) -> String {
        match lang() {
            Lang::En => format!("sent {sig} to {name} (pid {pid})"),
            Lang::Ru => format!("отправлен {sig} → {name} (pid {pid})"),
            Lang::Zh => format!("已发送 {sig} → {name} (pid {pid})"),
        }
    }

    pub fn fmt_kill_failed(&self, err: &str) -> String {
        match lang() {
            Lang::En => format!("kill failed: {err}"),
            Lang::Ru => format!("ошибка завершения: {err}"),
            Lang::Zh => format!("终止失败: {err}"),
        }
    }

    pub fn fmt_scan_error(&self, err: &str) -> String {
        format!("{}: {err}", self.scan_error)
    }

    pub fn fmt_all_ports(&self, n: usize) -> String {
        match lang() {
            Lang::En => format!("--- All ports of process ({n}) ---"),
            Lang::Ru => format!("--- Все порты процесса ({n}) ---"),
            Lang::Zh => format!("--- 进程所有端口 ({n}) ---"),
        }
    }

    pub fn fmt_sudo_error(&self, err: &str) -> String {
        match lang() {
            Lang::En => format!("sudo: {err}"),
            Lang::Ru => format!("sudo ошибка: {err}"),
            Lang::Zh => format!("sudo 错误: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_next_cycles_all() {
        let cases = [
            (Lang::En, Lang::Ru),
            (Lang::Ru, Lang::Zh),
            (Lang::Zh, Lang::En),
        ];
        for (from, expected) in cases {
            assert_eq!(from.next(), expected, "{:?}.next()", from);
        }
    }

    #[test]
    fn lang_next_full_cycle() {
        let start = Lang::En;
        let after_3 = start.next().next().next();
        assert_eq!(after_3, start);
    }

    #[test]
    fn lang_label() {
        let cases = [(Lang::En, "EN"), (Lang::Ru, "RU"), (Lang::Zh, "ZH")];
        for (lang, expected) in cases {
            assert_eq!(lang.label(), expected);
        }
    }

    #[test]
    fn lang_from_u8_table() {
        let cases = [
            (0, Lang::En),
            (1, Lang::Ru),
            (2, Lang::Zh),
            (99, Lang::En),
            (255, Lang::En),
        ];
        for (val, expected) in cases {
            assert_eq!(Lang::from_u8(val), expected, "from_u8({val})");
        }
    }

    #[test]
    fn parse_lang_table() {
        let cases = [
            ("en", Lang::En),
            ("ru", Lang::Ru),
            ("russian", Lang::Ru),
            ("zh", Lang::Zh),
            ("cn", Lang::Zh),
            ("chinese", Lang::Zh),
            ("EN", Lang::En),
            ("RU", Lang::Ru),
            ("ZH", Lang::Zh),
            ("unknown", Lang::En),
            ("", Lang::En),
            ("fr", Lang::En),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_lang(input), expected, "parse_lang({input:?})");
        }
    }

    #[test]
    fn set_and_get_lang() {
        set_lang(Lang::Ru);
        assert_eq!(lang(), Lang::Ru);
        set_lang(Lang::Zh);
        assert_eq!(lang(), Lang::Zh);
        set_lang(Lang::En);
        assert_eq!(lang(), Lang::En);
    }

    #[test]
    fn strings_returns_correct_lang() {
        set_lang(Lang::En);
        assert_eq!(strings().app_name, "PRT");
        set_lang(Lang::Ru);
        assert_eq!(strings().app_name, "PRT");
        // Verify a lang-specific field
        assert_eq!(strings().hint_quit, "выход");
        set_lang(Lang::En);
        assert_eq!(strings().hint_quit, "quit");
    }

    #[test]
    fn strings_all_languages_have_non_empty_fields() {
        for l in [Lang::En, Lang::Ru, Lang::Zh] {
            set_lang(l);
            let s = strings();
            assert!(!s.app_name.is_empty(), "{:?} app_name empty", l);
            assert!(!s.connections.is_empty(), "{:?} connections empty", l);
            assert!(!s.help_text.is_empty(), "{:?} help_text empty", l);
            assert!(!s.hint_help.is_empty(), "{:?} hint_help empty", l);
            assert!(!s.hint_lang.is_empty(), "{:?} hint_lang empty", l);
            assert!(!s.lang_switched.is_empty(), "{:?} lang_switched empty", l);
        }
        set_lang(Lang::En); // restore
    }

    #[test]
    fn fmt_connections_contains_count() {
        set_lang(Lang::En);
        let s = strings();
        assert!(s.fmt_connections(42).contains("42"));
    }

    #[test]
    fn fmt_kill_confirm_contains_name_and_pid() {
        for l in [Lang::En, Lang::Ru, Lang::Zh] {
            set_lang(l);
            let s = strings();
            let msg = s.fmt_kill_confirm("nginx", 1234);
            assert!(msg.contains("nginx"), "{:?}: {msg}", l);
            assert!(msg.contains("1234"), "{:?}: {msg}", l);
        }
        set_lang(Lang::En);
    }
}
