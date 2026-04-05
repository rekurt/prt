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

    view_chart: "График",
    view_topology: "Топология",
    view_process: "Процесс",
    view_namespaces: "Namespaces",

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
  /            поиск / фильтр (! = подозрительные)
  Esc          назад к таблице / сбросить фильтр
  r            обновить
  s            ввести sudo пароль

  Навигация:
  j/k ↑/↓     перемещение выбора
  g/G          в начало / в конец

  Нижняя панель (режим таблицы):
  Enter/d      показать/скрыть панель деталей
  1/2/3        Дерево / Сеть / Соединение
  ←/→          переключение вкладок
  h/l          переключение вкладок

  Полноэкранные режимы:
  4            График (соединения по процессам)
  5            Топология (процесс → порт → удалённый)
  6            Детали процесса (инфо, файлы, env)
  7            Namespaces (только Linux)

  Действия:
  K/Del        завершить процесс
  c            копировать строку в буфер
  p            копировать PID в буфер
  b            блокировать IP (firewall)
  t            подключить/отключить strace

  Таблица:
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

    hint_back: "назад",
    hint_details: "детали",
    hint_views: "режимы",
    hint_sort: "сорт.",
    hint_copy: "копир.",
    hint_block: "блок. IP",
    hint_trace: "трасс.",
    hint_navigate: "навиг.",
    hint_tabs: "вкладки",

    lang_switched: "язык переключён",

    help_title: " Справка (любая клавиша — закрыть) ",
};
