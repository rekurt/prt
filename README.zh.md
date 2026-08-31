<div align="center">

# prt

**在终端中实时查看哪些进程正在占用网络端口。**

[![Crates.io](https://img.shields.io/crates/v/prt.svg)](https://crates.io/crates/prt)
[![下载量](https://img.shields.io/crates/d/prt.svg)](https://crates.io/crates/prt)
[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![MIT 许可证](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![API 文档](https://docs.rs/prt-core/badge.svg)](https://docs.rs/prt-core)

[English](README.md) · [Русский](README.ru.md) · [中文](README.zh.md)

</div>

`prt` 是一个完全由键盘操作的终端界面，用于在 macOS 和 Linux 上检查网络连接、查找端口冲突、查看进程详情以及管理 SSH 隧道。它提供实时连接表、过滤、变化追踪、进程拓扑、告警和适合脚本处理的输出格式。

## 演示

<p align="center">
  <img src="docs/prt.gif" alt="prt 动画演示：实时连接、进程详情、网络拓扑、命令面板和上下文操作" width="960">
</p>

13 秒动画会直接在 README 中播放。你也可以[查看静态画面](docs/prt-demo.png)或[阅读演示文字说明](docs/demo-transcript.md)。录制过程可通过 [`docs/demo.tape`](docs/demo.tape) 复现。

## 快速开始

### 环境要求

- 使用 Cargo 安装时需要 Rust 1.75 或更高版本
- macOS 10.15 或更高版本（使用系统自带的 `lsof`）
- Linux，并已挂载 `/proc` 文件系统
- 支持 UTF-8 的终端；窗口越宽，可显示的表格列越多

### 安装并运行

```sh
cargo install prt
prt
```

如果系统隐藏了其他用户的进程，可运行 `sudo prt`。普通查看不需要管理员权限。

从当前源码构建：

```sh
git clone https://github.com/rekurt/prt.git
cd prt
cargo install --path crates/prt
```

## 主要功能

| 任务 | `prt` 的帮助 |
|---|---|
| 查找端口冲突 | 按端口、进程、协议、状态、服务、PID 或用户搜索 |
| 追踪连接变化 | 每两秒刷新，并突出显示新增和关闭的连接 |
| 调查进程 | 查看命令行、父进程树、CPU、内存、打开文件和相关连接 |
| 查看网络拓扑 | 以 `进程 → 本地端口 → 远程端点` 的树形结构显示 |
| 查找可疑监听器 | 使用内置规则标记 `[!]`，并支持单独过滤 |
| 识别容器 | 检测并显示所属 Docker 或 Podman 容器 |
| 管理 SSH 转发 | 读取 SSH config，并创建、编辑、重启或保存隧道 |
| 自动化检查 | 导出单次 JSON/CSV 快照，或持续输出 NDJSON |

`prt` 还可以估算系统网络吞吐量、识别常用端口服务、执行可配置告警，并提供终止进程、防火墙封锁、复制、系统调用追踪和 SSH 转发等上下文操作。

## 命令行模式

```sh
prt                         # 启动交互式 TUI
prt --lang zh               # 使用中文（也支持 en 和 ru）
prt --export json           # 输出一次 JSON 快照并退出
prt --export csv            # 输出一次 CSV 快照并退出
prt --json                  # 持续输出 NDJSON
prt watch 80 443 5432       # 监控指定端口的 UP/DOWN 状态
sudo prt                    # 包含当前用户看不到的进程
```

需要有限快照时使用 `--export`。`--json` 会持续运行，并在每次扫描时为每个连接输出一个对象；当 `head` 等下游命令关闭管道时，它会正常退出。

```sh
# 保存快照以便比较或记录事件。
prt --export json > ports.json

# 从实时流中读取进程名。
prt --json | jq -r '.process.name'

# 仅监控开发端口。
prt watch 3000 5432 8080
```

## 界面指南

使用 `Tab` 和 `Shift+Tab` 在三个顶级页面之间切换：

| 页面 | 用途 | 子标签 |
|---|---|---|
| 连接 | 可排序连接表和可选详情面板 | 无 |
| 进程 | 所选进程详情及网络拓扑 | 详情、拓扑 |
| SSH | SSH config 中的主机和受管理的隧道 | 主机、隧道 |

按 `?` 打开应用内快捷键说明。不想记忆快捷键时，可按 `:` 搜索命令面板。

### 常用快捷键

| 按键 | 操作 |
|---|---|
| `?` | 打开帮助；任意键关闭 |
| `q` | 退出 |
| `Tab` / `Shift+Tab` | 下一个 / 上一个页面 |
| `Space` | 打开上下文操作菜单 |
| `:` | 打开可搜索的命令面板 |
| `/` | 搜索和过滤；连续按两次 `Esc` 清除非空过滤器 |
| `p` | 暂停或继续自动刷新 |
| `r` | 立即刷新 |
| `s` | 输入 sudo 密码以查看更多进程 |
| `L` | 切换界面语言 |
| `j` / `k`、`↑` / `↓` | 移动或滚动 |
| `g` / `G`、`Home` / `End` | 跳到开头 / 结尾 |
| `K` / `Delete` | 请求终止所选进程 |
| `c` | 复制所选连接 |

页面专用快捷键：

| 上下文 | 按键 | 操作 |
|---|---|---|
| 连接 | `Enter` | 在“进程”页面打开所选进程 |
| 连接 | `d` | 显示或隐藏底部详情面板 |
| 连接 | `o` / `O` | 下一个排序列 / 反转排序方向 |
| 进程 | `[` / `]` | 切换详情和拓扑 |
| SSH | `[` / `]` | 切换主机和隧道 |
| SSH 主机 | `Enter` | 为所选主机打开隧道表单 |
| SSH 主机 | `r` | 重新加载 SSH 和 `prt` 配置 |
| SSH 隧道 | `n` / `e` | 新建 / 编辑隧道 |
| SSH 隧道 | `K` / `r` / `s` | 终止 / 重启 / 保存隧道 |

## 搜索与变化追踪

按 `/` 过滤实时表格。普通文本会匹配连接数据；`new`、`gone` 和 `active` 等状态别名可筛选生命周期状态。输入 `!` 或 `suspicious` 只显示被可疑连接检测器标记的条目。

新增条目显示为绿色；关闭的条目显示为暗红色，并保留五秒。连接状态和持续时间有文字显示，但 new/gone 的区别目前仍依赖颜色。

## 配置

可选配置文件位于 `~/.config/prt/config.toml`。文件不存在时使用默认值；解析失败时会报告错误并继续使用默认值。

```toml
# 添加或覆盖端口服务名。
[known_ports]
3000 = "frontend"
5432 = "postgres"

# 新 SSH 连接出现时响铃。
[[alerts]]
port = 22
action = "bell"

# 高亮 Python 监听器。
[[alerts]]
process = "python"
state = "LISTEN"
action = "highlight"

# 添加 ~/.ssh/config 之外的主机。
[[ssh_hosts]]
alias = "staging"
hostname = "staging.example.com"
user = "deploy"
port = 22
```

告警条件包括 `port`、`process`、`state` 和 `connections_gt`；操作包括 `bell` 和 `highlight`。铃声只会为新条目触发。

SSH 页面还会读取 `~/.ssh/config`。隧道页面会把已保存隧道写入 `prt` 配置的 `[[ssh_tunnels]]` 部分。

## 安全与权限

扫描和导航是只读操作。会改变系统状态的功能集中在 `Space` 菜单中：

- 终止进程时需要选择 SIGTERM 或 SIGKILL；
- 防火墙封锁会要求确认，并需要相应权限；
- 系统调用追踪在 Linux 上需要 `ptrace` 权限，在 macOS 上需要可用的 `dtruss` 权限；
- SSH 转发会使用隧道表单中显示的参数启动 `ssh` 子进程。

执行前请检查确认信息或表单。漏洞报告方法和支持版本见[安全策略](SECURITY.md)。

## 文档无障碍

- README 开头提供可直接复制的快速开始，链接文字能够说明目标。
- 演示提供静态预览、详细替代文本和文字说明。
- TUI 完全由键盘控制，并包含帮助页面和命令面板。
- 界面和 README 提供英语、俄语和中文版本。
- 连接状态和可疑标记使用文字；new/gone 生命周期提示目前仍依赖颜色。
- JSON、CSV、NDJSON 和 watch 模式为全屏 TUI 提供替代方案。

如果辅助技术无法使用终端界面，`prt --export json` 是最稳定的机器可读替代方式。

## 架构与开发

仓库是包含两个 crate 的 Rust workspace：

```text
crates/
├── prt-core/   扫描、追踪、过滤、告警、配置、进程信息、
│               容器、国际化和平台适配
└── prt/        CLI、ratatui 界面、输入处理、流模式、watch 模式、
                系统调用追踪和 SSH 隧道管理
```

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

库 API 见 [docs.rs 上的 `prt-core` 文档](https://docs.rs/prt-core)，参与贡献请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

本项目使用 [MIT 许可证](LICENSE)。

如果 `prt` 对你有帮助，欢迎[在 GitHub 上给项目加星](https://github.com/rekurt/prt)。
