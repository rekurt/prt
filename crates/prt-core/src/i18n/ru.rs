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
  5            Топология (процесс → порт → удалённый)
  6            Детали процесса (инфо, файлы, env)
  7            Namespaces (только Linux)

  Действия:
  K/Del        завершить процесс
  c            копировать строку в буфер
  p            копировать PID в буфер
  b            блокировать IP (firewall)
  t            подключить/отключить strace
  F            SSH проброс порта (туннель)

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

    forward_prompt_title: " SSH-туннель ",
    forward_host_label: " хост:порт → ",
    forward_confirm_hint: " [Enter] создать  [Esc] отмена",
    hint_forward: "туннель",

    view_ssh_hosts: "SSH хосты",
    view_tunnels: "Туннели",

    ssh_col_alias: "Алиас",
    ssh_col_target: "Адрес",
    ssh_col_source: "Источник",
    ssh_hosts_empty:
        "  SSH-хостов нет. Добавьте записи в ~/.ssh/config или [[ssh_hosts]] в config.toml.",
    ssh_hosts_reloaded: "SSH-хосты перечитаны",

    tunnel_col_name: "Имя",
    tunnel_col_kind: "Тип",
    tunnel_col_local: "Локальн.",
    tunnel_col_remote: "Удалённ.",
    tunnel_col_host: "Хост",
    tunnel_col_status: "Статус",
    tunnel_status_alive: "активен",
    tunnel_status_dead: "мёртв",
    tunnels_empty: "  Активных туннелей нет. Нажмите [n] чтобы создать.",
    tunnels_saved: "туннели сохранены в конфиг",
    tunnel_killed: "туннель убит",
    tunnel_restarted: "туннель перезапущен",
    tunnel_create_failed: "не удалось создать туннель",
    tunnel_kind_local: "локальный",
    tunnel_kind_dynamic: "динамический",

    tunnel_form_title: " Новый SSH-туннель ",
    tunnel_form_kind: " Тип:         ",
    tunnel_form_local_port: " Локальн. порт: ",
    tunnel_form_remote_host: " Удалён. хост: ",
    tunnel_form_remote_port: " Удалён. порт: ",
    tunnel_form_host_alias: " SSH-хост:    ",
    tunnel_form_hint: " [Tab] след.  [\u{2190}\u{2192}] тип  [Enter] создать  [Esc] отмена",
    tunnel_form_invalid: "неверные поля туннеля",

    hint_ssh_hosts: "SSH хосты",
    hint_tunnels: "туннели",
    hint_new_tunnel: "новый",
    hint_kill_tunnel: "убить",
    hint_restart_tunnel: "рестарт",
    hint_save_tunnels: "сохр.",
    hint_reload: "обновить",
    hint_open_tunnel: "туннель",

    lang_switched: "язык переключён",

    help_title: " Справка (любая клавиша — закрыть) ",
};
