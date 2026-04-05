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

    view_chart: "Chart",
    view_topology: "Topology",
    view_process: "Process",
    view_namespaces: "Namespaces",

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
  /            search / filter (! = suspicious only)
  Esc          back to table / clear filter
  r            refresh
  s            enter sudo password

  Navigation:
  j/k Up/Down  move selection
  g/G          jump to start / end

  Bottom panel (Table mode):
  Enter/d      show/hide detail panel
  1/2/3        Tree / Network / Connection tab
  Left/Right   switch detail tab
  h/l          switch detail tab

  Fullscreen views:
  4            Chart (connections per process)
  5            Topology (process -> port -> remote)
  6            Process detail (info, files, env)
  7            Namespaces (Linux only)

  Actions:
  K/Del        kill process
  c            copy line to clipboard
  p            copy PID to clipboard
  b            block remote IP (firewall)
  t            attach/detach strace
  F            SSH port forward (tunnel)

  Table:
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

    hint_back: "back",
    hint_details: "details",
    hint_views: "views",
    hint_sort: "sort",
    hint_copy: "copy",
    hint_block: "block IP",
    hint_trace: "trace",
    hint_navigate: "navigate",
    hint_tabs: "tabs",

    forward_prompt_title: " SSH Forward ",
    forward_host_label: " host:port → ",
    forward_confirm_hint: " [Enter] create  [Esc] cancel",
    hint_forward: "forward",

    lang_switched: "language switched",

    help_title: " Help (any key to close) ",
};
