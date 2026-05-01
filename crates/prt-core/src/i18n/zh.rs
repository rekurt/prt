use super::Strings;

pub static STRINGS: Strings = Strings {
    app_name: "PRT",

    connections: "个连接",
    no_root_warning: "[非root — 部分进程已隐藏]",
    sudo_ok: "[sudo 已启用]",
    filter_label: "过滤:",
    search_mode: "[搜索]",

    tab_tree: "进程树",
    tab_network: "网络",
    tab_connection: "连接",
    no_selected_process: " 未选择进程",

    view_topology: "拓扑",
    view_process: "进程详情",
    view_namespaces: "命名空间",

    process_not_found: "未找到进程",

    iface_address: "  地址:       ",
    iface_interface: "  接口:       ",
    iface_protocol: "  协议:       ",
    iface_bind: "  绑定:       ",
    iface_localhost_only: "仅本地",
    iface_all_interfaces: "所有接口",
    iface_specific: "指定地址",
    iface_loopback: "lo0 (回环)",
    iface_all: "* (所有接口)",

    conn_local: "  本地:       ",
    conn_remote: "  远程:       ",
    conn_state: "  状态:       ",
    conn_process: "  进程:       ",
    conn_cmdline: "  命令行:     ",

    help_text: r#"
  快捷键:
  q            退出
  ?            帮助
  /            搜索 / 过滤 (! = 可疑连接)
  Esc          返回表格 / 清除过滤
  r            刷新
  s            输入sudo密码

  导航:
  j/k 上/下    移动选择
  g/G          跳到开头 / 结尾

  底部面板 (表格模式):
  Enter/d      显示/隐藏详情面板
  1/2/3        进程树 / 网络 / 连接
  左/右        切换标签页
  h/l          切换标签页

  全屏模式:
  5            拓扑 (进程 → 端口 → 远程)
  6            进程详情 (信息、文件、环境变量)
  7            命名空间 (仅Linux)

  操作:
  K/Del        终止进程
  c            复制行到剪贴板
  p            复制PID到剪贴板
  b            封锁远程IP (防火墙)
  t            附加/分离 strace
  F            SSH端口转发 (隧道)

  表格:
  Tab          下一排序列
  Shift+Tab    反转排序方向
  L            切换语言
"#,

    kill_cancel: "[y] SIGTERM  [f] SIGKILL  [n/Esc] 取消",
    copied: "已复制",
    refreshed: "已刷新",
    clipboard_unavailable: "剪贴板不可用",
    scan_error: "扫描错误",
    cancelled: "已取消",

    sudo_prompt_title: " 输入sudo密码 ",
    sudo_password_label: " 密码: ",
    sudo_confirm_hint: " [Enter] 确认  [Esc] 取消",
    sudo_failed: "sudo失败",
    sudo_wrong_password: "密码错误",
    sudo_elevated: "sudo已启用 — 显示所有进程",

    hint_help: "帮助",
    hint_search: "搜索",
    hint_kill: "终止",
    hint_sudo: "sudo",
    hint_quit: "退出",
    hint_lang: "语言",

    hint_back: "返回",
    hint_details: "详情",
    hint_views: "视图",
    hint_sort: "排序",
    hint_copy: "复制",
    hint_block: "封锁IP",
    hint_trace: "跟踪",
    hint_navigate: "导航",
    hint_tabs: "标签",

    forward_prompt_title: " SSH隧道 ",
    forward_host_label: " 主机:端口 → ",
    forward_confirm_hint: " [Enter] 创建  [Esc] 取消",
    hint_forward: "转发",

    view_ssh_hosts: "SSH 主机",
    view_tunnels: "隧道",

    ssh_col_alias: "别名",
    ssh_col_target: "目标",
    ssh_col_source: "来源",
    ssh_hosts_empty: "  无 SSH 主机。请在 ~/.ssh/config 或 config.toml 的 [[ssh_hosts]] 添加。",
    ssh_hosts_reloaded: "SSH 主机已重新加载",

    tunnel_col_name: "名称",
    tunnel_col_kind: "类型",
    tunnel_col_local: "本地",
    tunnel_col_remote: "远端",
    tunnel_col_host: "主机",
    tunnel_col_status: "状态",
    tunnel_status_alive: "活跃",
    tunnel_status_dead: "已断",
    tunnels_empty: "  无活跃隧道。按 [n] 创建。",
    tunnels_saved: "隧道已保存到配置",
    tunnel_killed: "隧道已终止",
    tunnel_restarted: "隧道已重启",
    tunnel_create_failed: "创建隧道失败",
    tunnel_kind_local: "本地",
    tunnel_kind_dynamic: "动态",

    tunnel_form_title: " 新建 SSH 隧道 ",
    tunnel_form_kind: " 类型:        ",
    tunnel_form_local_port: " 本地端口:    ",
    tunnel_form_remote_host: " 远端主机:    ",
    tunnel_form_remote_port: " 远端端口:    ",
    tunnel_form_host_alias: " SSH 主机:    ",
    tunnel_form_hint: " [Tab] 下一项  [\u{2190}\u{2192}] 类型  [Enter] 创建  [Esc] 取消",
    tunnel_form_invalid: "隧道字段无效",

    hint_ssh_hosts: "SSH 主机",
    hint_tunnels: "隧道",
    hint_new_tunnel: "新建",
    hint_kill_tunnel: "终止",
    hint_restart_tunnel: "重启",
    hint_save_tunnels: "保存",
    hint_reload: "重载",
    hint_open_tunnel: "隧道",

    lang_switched: "语言已切换",

    help_title: " 帮助 (任意键关闭) ",
};
