use super::Strings;

pub static STRINGS: Strings = Strings {
    app_name: "PRT",

    connections: "соединений",
    no_root_warning: "[нет root — часть процессов скрыта]",
    sudo_ok: "[sudo ✓]",
    filter_label: "фильтр:",
    search_mode: "[ПОИСК]",

    tab_tree: "Дерево",
    tab_network: "Сеть",
    tab_connection: "Соединение",
    no_selected_process: " нет выбранного процесса",

    process_not_found: "процесс не найден",

    iface_address: "  Адрес:      ",
    iface_interface: "  Интерфейс:  ",
    iface_protocol: "  Протокол:   ",
    iface_bind: "  Привязка:   ",
    iface_localhost_only: "только localhost",
    iface_all_interfaces: "все адреса",
    iface_specific: "конкретный адрес",
    iface_loopback: "lo0 (loopback)",
    iface_all: "* (все интерфейсы)",

    conn_local: "  Локальный:  ",
    conn_remote: "  Удалённый:  ",
    conn_state: "  Состояние:  ",
    conn_process: "  Процесс:    ",
    conn_cmdline: "  Cmdline:    ",

    help_text: r#"
  Клавиши:
  q            выход
  ?            эта справка
  /            поиск / фильтр
  Esc          сбросить фильтр
  r            обновить
  s            ввести sudo пароль
  j/k ↑/↓     навигация
  g/G          в начало / в конец
  Enter/d      показать/скрыть детали
  1/2/3        переключение вкладок
  ←/→ h/l      переключение вкладок
  K/Del        завершить процесс
  c            копировать строку
  p            копировать PID
  Tab          следующая колонка сортировки
  Shift+Tab    изменить направление сортировки
  L            переключить язык
"#,

    kill_cancel: "[y] SIGTERM  [f] SIGKILL  [n/Esc] отмена",
    copied: "скопировано",
    refreshed: "обновлено",
    clipboard_unavailable: "буфер недоступен",
    scan_error: "ошибка сканирования",
    cancelled: "отменено",

    sudo_prompt_title: " Введите пароль sudo ",
    sudo_password_label: " Пароль: ",
    sudo_confirm_hint: " [Enter] подтвердить  [Esc] отмена",
    sudo_failed: "sudo не удался",
    sudo_wrong_password: "неверный пароль",
    sudo_elevated: "sudo ✓ — показаны все процессы",

    hint_help: "справка",
    hint_search: "поиск",
    hint_kill: "завершить",
    hint_sudo: "sudo",
    hint_quit: "выход",
    hint_lang: "язык",

    lang_switched: "язык переключён",

    help_title: " Справка (любая клавиша — закрыть) ",
};
