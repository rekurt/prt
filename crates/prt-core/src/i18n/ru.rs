use super::Strings;

pub static STRINGS: Strings = Strings {
    app_name: "PRT",

    connections: "соединений",
    no_root_warning: "[нет root — часть процессов скрыта]",
    sudo_ok: "[sudo ✓]",
    filter_label: "фильтр:",
    search_mode: "[ПОИСК]",

    detail_panel_title: "Детали",
    detail_panel_tree_header: "Дерево процессов:",
    no_selected_process: " нет выбранного процесса",

    section_connections: "Соединения",
    section_processes: "Процессы",
    section_ssh: "SSH",

    view_topology: "Топология",
    view_process: "Процесс",

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
  Общие:
  ?            эта справка
  q            выход
  Tab / Sh+Tab след. / предыд. раздел (Соединения | Процессы | SSH)
  Space        меню действий (Убить / Копир. / Блок / Трасс. / Туннель)
  :            палитра команд
  /            поиск / фильтр   (Esc дважды — стереть)
  p            пауза / продолжить автообновление
  r            обновить
  s            ввести sudo-пароль
  L            сменить язык
  K / Del      убить выделенный процесс
  c            копировать строку в буфер
  j/k g/G      навигация / в начало | в конец

  Соединения (раздел по умолчанию):
  Enter        открыть детали процесса
  d            показать/скрыть панель Деталей
  o / O        след. колонка сорт. / реверс направления

  Процессы:
  [ / ]        переключить вкладку (Детали | Топология)

  SSH:
  [ / ]        переключить вкладку (Хосты | Туннели)
  Хосты        Enter = новый туннель от хоста,  r = перезагрузить
  Туннели      n = новый,  e = правка,  K = убить,  r = рестарт,  s = сохранить
"#,

    kill_cancel: "[y] SIGTERM  [f] SIGKILL  [n/Esc] отмена",
    copied: "скопировано",
    refreshed: "обновлено",
    clipboard_unavailable: "буфер недоступен",
    scan_error: "ошибка сканирования",
    cancelled: "отменено",
    paused: "автообновление на паузе",
    resumed: "автообновление включено",
    no_connections: " нет видимых соединений",
    no_filter_matches: " нет совпадений по фильтру",
    more: "ещё",
    col_age: "Возраст",
    col_remote: "Удалённый",

    sudo_prompt_title: " Введите пароль sudo ",
    sudo_password_label: " Пароль: ",
    sudo_confirm_hint: " [Enter] подтвердить  [Esc] отмена",
    sudo_failed: "sudo не удался",
    sudo_wrong_password: "неверный пароль",
    sudo_elevated: "sudo ✓ — показаны все процессы",

    hint_help: "справка",
    hint_search: "поиск",
    hint_filter_examples: "фильтры: status:new risk:high pid:1234 !",
    hint_kill: "завершить",
    hint_sudo: "sudo",
    hint_quit: "выход",
    hint_lang: "язык",

    hint_back: "назад",
    hint_details: "детали",
    hint_sort: "сорт.",
    hint_copy: "копир.",
    hint_navigate: "навиг.",
    hint_section_next: "раздел",
    hint_subtab: "вкладка",
    hint_action_menu: "действия",
    hint_edit_tunnel: "правка",
    hint_pause: "пауза",
    hint_resume: "продолж.",

    action_menu_title: "Действия",
    action_kill: "Убить процесс",
    action_copy: "Копировать строку",
    action_copy_pid: "Копировать PID",
    action_block: "Блокировать IP",
    action_trace: "Трассировать syscalls",
    action_forward: "SSH-туннель",
    action_unavailable_no_remote: "нет удалённого адреса",
    command_palette_title: "Команда",
    command_palette_empty: "команд нет",

    esc_again_to_clear_filter: "Esc ещё раз — стереть фильтр",
    esc_again_to_discard_form: "Esc ещё раз — отменить изменения",

    forward_prompt_title: " SSH-туннель ",
    forward_host_label: " хост:порт → ",
    forward_confirm_hint: " [Enter] создать  [Esc] отмена",

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
    tunnel_status_starting: "запускается",
    tunnel_status_failed: "сбой",
    tunnel_form_edit_title: " Правка SSH-туннеля ",
    tunnel_form_field_required: "обязательно",
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

    hint_new_tunnel: "новый",
    hint_kill_tunnel: "убить",
    hint_restart_tunnel: "рестарт",
    hint_save_tunnels: "сохр.",
    hint_reload: "обновить",
    hint_open_tunnel: "туннель",

    lang_switched: "язык переключён",

    help_title: " Справка (любая клавиша — закрыть) ",
};
