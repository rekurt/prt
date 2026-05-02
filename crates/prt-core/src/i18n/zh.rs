use super::Strings;

pub static STRINGS: Strings = Strings {
    app_name: "PRT",

    connections: "个连接",
    no_root_warning: "[非root — 部分进程已隐藏]",
    sudo_ok: "[sudo 已启用]",
    filter_label: "过滤:",
    search_mode: "[搜索]",

    detail_panel_title: "详情",
    detail_panel_tree_header: "进程树:",
    no_selected_process: " 未选择进程",

    section_connections: "连接",
    section_processes: "进程",
    section_ssh: "SSH",

    view_topology: "拓扑",
    view_process: "进程详情",

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
  全局:
  ?            帮助
  q            退出
  Tab / Sh+Tab 下 / 上 一个分区 (连接 | 进程 | SSH)
  Space        操作菜单 (终止 / 复制 / 封锁 / 跟踪 / 转发)
  :            命令面板
  /            搜索 / 过滤   (Esc 两次清除)
  p            暂停 / 恢复自动刷新
  r            刷新
  s            输入 sudo 密码
  L            切换语言
  K / Del      终止选中的进程
  c            复制行到剪贴板
  j/k g/G      导航 / 跳到开头 | 结尾

  连接 (默认分区):
  Enter        打开进程详情
  d            显示/隐藏底部详情面板
  o / O        下一排序列 / 反转方向

  进程:
  [ / ]        切换子标签 (详情 | 拓扑)

  SSH:
  [ / ]        切换子标签 (主机 | 隧道)
  主机         Enter = 从该主机新建隧道,  r = 重新加载
  隧道         n = 新建,  e = 编辑,  K = 终止,  r = 重启,  s = 保存
"#,

    kill_cancel: "[y] SIGTERM  [f] SIGKILL  [n/Esc] 取消",
    copied: "已复制",
    refreshed: "已刷新",
    clipboard_unavailable: "剪贴板不可用",
    scan_error: "扫描错误",
    cancelled: "已取消",
    paused: "自动刷新已暂停",
    resumed: "自动刷新已恢复",
    no_connections: " 无可见连接",
    no_filter_matches: " 过滤无匹配",
    more: "更多",
    col_age: "时长",
    col_remote: "远程",

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
    hint_sort: "排序",
    hint_copy: "复制",
    hint_navigate: "导航",
    hint_section_next: "分区",
    hint_subtab: "标签",
    hint_action_menu: "操作",
    hint_edit_tunnel: "编辑",
    hint_pause: "暂停",
    hint_resume: "继续",

    action_menu_title: "操作",
    action_kill: "终止进程",
    action_copy: "复制行",
    action_copy_pid: "复制 PID",
    action_block: "封锁远程 IP",
    action_trace: "跟踪系统调用",
    action_forward: "SSH 转发",
    action_unavailable_no_remote: "无远程地址",
    command_palette_title: "命令",
    command_palette_empty: "无命令",

    esc_again_to_clear_filter: "再按 Esc 清除过滤",
    esc_again_to_discard_form: "再按 Esc 放弃更改",

    forward_prompt_title: " SSH隧道 ",
    forward_host_label: " 主机:端口 → ",
    forward_confirm_hint: " [Enter] 创建  [Esc] 取消",

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
    tunnel_status_starting: "启动中",
    tunnel_status_failed: "失败",
    tunnel_form_edit_title: " 编辑 SSH 隧道 ",
    tunnel_form_field_required: "必填",
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

    hint_new_tunnel: "新建",
    hint_kill_tunnel: "终止",
    hint_restart_tunnel: "重启",
    hint_save_tunnels: "保存",
    hint_reload: "重载",
    hint_open_tunnel: "隧道",

    lang_switched: "语言已切换",

    help_title: " 帮助 (任意键关闭) ",
};
