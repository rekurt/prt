use super::Strings;

pub static STRINGS: Strings = Strings {
    app_name: "PRT",

    connections: "connections",
    no_root_warning: "[no root — some processes hidden]",
    sudo_ok: "[sudo OK]",
    filter_label: "filter:",
    search_mode: "[SEARCH]",

    tab_tree: "Tree",
    tab_network: "Network",
    tab_connection: "Connection",
    no_selected_process: " no process selected",

    process_not_found: "process not found",

    iface_address: "  Address:    ",
    iface_interface: "  Interface:  ",
    iface_protocol: "  Protocol:   ",
    iface_bind: "  Bind:       ",
    iface_localhost_only: "localhost only",
    iface_all_interfaces: "all interfaces",
    iface_specific: "specific address",
    iface_loopback: "lo0 (loopback)",
    iface_all: "* (all interfaces)",

    conn_local: "  Local:      ",
    conn_remote: "  Remote:     ",
    conn_state: "  State:      ",
    conn_process: "  Process:    ",
    conn_cmdline: "  Cmdline:    ",

    help_text: r#"
  Keys:
  q            quit
  ?            this help
  /            search / filter
  Esc          clear filter
  r            refresh
  s            enter sudo password
  j/k Up/Down  navigation
  g/G          jump to start / end
  Enter/d      show/hide details
  1/2/3        switch tabs
  Left/Right   switch tabs
  K/Del        kill process
  c            copy line
  p            copy PID
  Tab          next sort column
  Shift+Tab    reverse sort direction
  L            switch language
"#,

    kill_cancel: "[y] SIGTERM  [f] SIGKILL  [n/Esc] cancel",
    copied: "copied to clipboard",
    refreshed: "refreshed",
    clipboard_unavailable: "clipboard unavailable",
    scan_error: "scan error",
    cancelled: "cancelled",

    sudo_prompt_title: " Enter sudo password ",
    sudo_password_label: " Password: ",
    sudo_confirm_hint: " [Enter] confirm  [Esc] cancel",
    sudo_failed: "sudo failed",
    sudo_wrong_password: "wrong password",
    sudo_elevated: "sudo OK — showing all processes",

    hint_help: "help",
    hint_search: "search",
    hint_kill: "kill",
    hint_sudo: "sudo",
    hint_quit: "quit",
    hint_lang: "lang",

    lang_switched: "language switched",

    help_title: " Help (any key to close) ",
};
