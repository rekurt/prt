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

    view_chart: "图表",
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
  4            图表 (每进程连接数)
  5            拓扑 (进程 → 端口 → 远程)
  6            进程详情 (信息、文件、环境变量)
  7            命名空间 (仅Linux)

  操作:
  K/Del        终止进程
  c            复制行到剪贴板
  p            复制PID到剪贴板
  b            封锁远程IP (防火墙)
  t            附加/分离 strace

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

    lang_switched: "语言已切换",

    help_title: " 帮助 (任意键关闭) ",
};
