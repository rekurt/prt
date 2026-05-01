use super::Strings;

pub static STRINGS: Strings = Strings {
    app_name: "PRT",

    connections: "connections",
    no_root_warning: "[no root — some processes hidden]",
    sudo_ok: "[sudo OK]",
    filter_label: "filter:",
    search_mode: "[SEARCH]",

    detail_panel_title: "Details",
    detail_panel_tree_header: "Process tree:",
    no_selected_process: " no process selected",

    section_connections: "Connections",
    section_processes: "Processes",
    section_ssh: "SSH",

    view_topology: "Topology",
    view_process: "Process",

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
  Enter/d      show/hide details panel

  Fullscreen views:
  5            Topology (process -> port -> remote)
  6            Process detail (info, files, env)

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
    hint_sort: "sort",
    hint_copy: "copy",
    hint_navigate: "navigate",
    hint_section_next: "section",
    hint_subtab: "tab",
    hint_action_menu: "actions",
    hint_edit_tunnel: "edit",

    forward_prompt_title: " SSH Forward ",
    forward_host_label: " host:port → ",
    forward_confirm_hint: " [Enter] create  [Esc] cancel",

    view_ssh_hosts: "SSH Hosts",
    view_tunnels: "Tunnels",

    ssh_col_alias: "Alias",
    ssh_col_target: "Target",
    ssh_col_source: "Source",
    ssh_hosts_empty:
        "  No SSH hosts found. Add entries to ~/.ssh/config or [[ssh_hosts]] in config.toml.",
    ssh_hosts_reloaded: "SSH hosts reloaded",

    tunnel_col_name: "Name",
    tunnel_col_kind: "Kind",
    tunnel_col_local: "Local",
    tunnel_col_remote: "Remote",
    tunnel_col_host: "Host",
    tunnel_col_status: "Status",
    tunnel_status_alive: "alive",
    tunnel_status_dead: "dead",
    tunnels_empty: "  No active tunnels. Press [n] to create one.",
    tunnels_saved: "tunnels saved to config",
    tunnel_killed: "tunnel killed",
    tunnel_restarted: "tunnel restarted",
    tunnel_create_failed: "tunnel create failed",
    tunnel_kind_local: "local",
    tunnel_kind_dynamic: "dynamic",

    tunnel_form_title: " New SSH Tunnel ",
    tunnel_form_kind: " Kind:        ",
    tunnel_form_local_port: " Local port:  ",
    tunnel_form_remote_host: " Remote host: ",
    tunnel_form_remote_port: " Remote port: ",
    tunnel_form_host_alias: " SSH host:    ",
    tunnel_form_hint: " [Tab] next  [\u{2190}\u{2192}] kind  [Enter] create  [Esc] cancel",
    tunnel_form_invalid: "invalid tunnel form",

    hint_new_tunnel: "new",
    hint_kill_tunnel: "kill",
    hint_restart_tunnel: "restart",
    hint_save_tunnels: "save",
    hint_reload: "reload",
    hint_open_tunnel: "tunnel",

    lang_switched: "language switched",

    help_title: " Help (any key to close) ",
};
