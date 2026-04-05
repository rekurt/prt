<div align="center">

# prt

**终端实时网络端口监控工具**

<br>

<img src="docs/prt.gif" alt="prt 演示" width="720">

<br>
<br>

[![Crates.io](https://img.shields.io/crates/v/prt.svg)](https://crates.io/crates/prt)
[![Downloads](https://img.shields.io/crates/d/prt.svg)](https://crates.io/crates/prt)
[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![docs.rs](https://docs.rs/prt-core/badge.svg)](https://docs.rs/prt-core)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh.md)

</div>

---

## 什么是 prt？

`prt` 实时显示哪些进程占用了您机器上的网络端口。它是 `lsof -i` / `ss -tlnp` 的交互式替代品，支持颜色高亮、过滤和进程树。

## 功能特性

### 实时表格与变化追踪

主界面以可排序、可过滤的表格显示所有活跃的网络连接。列包括：端口、服务名、协议、状态、PID、进程名、用户。新连接以**绿色**高亮；关闭的连接以**红色**淡出5秒后消失。每2秒自动刷新。

### 已知端口数据库

`Service` 列将常见端口号映射为可读名称 — http (80)、ssh (22)、postgres (5432) 等约200个。可在 `~/.config/prt/config.toml` 中覆盖或扩展：

```toml
[known_ports]
3000 = "my-app"
9090 = "prometheus"
```

### 连接老化追踪

每个连接记录其 `first_seen` 时间戳。ESTABLISHED 连接超过1小时显示黄色，超过24小时显示红色。CLOSE_WAIT 始终显示红色，因为它们可能表示资源泄漏。

### 可疑连接检测

连接会被扫描异常情况并标记 `[!]`：

- **非 root 使用特权端口** — 非 root 进程监听端口 < 1024
- **脚本语言占用敏感端口** — Python、Perl、Ruby 或 Node.js 监听端口 22、80 或 443
- **Root 外连高端口** — root 进程建立到远程端口 > 1024 的连接

按 `/` 然后输入 `!` 可仅显示可疑条目。

### 容器感知

如果 Docker 或 Podman 正在运行，`Container` 列显示每个进程所属的容器名称。无容器时该列自动隐藏以节省空间。通过批量 `docker ps` + `docker inspect` 调用解析，超时时间为2秒。

### 带宽估算

标题栏显示系统全局网络吞吐量：`▼ 1.2 MB/s ▲ 340 KB/s`。Linux 上读取 `/proc/net/dev`，macOS 上读取 `netstat -ib`。速率按刷新周期间的差值计算。

### 进程树

按 `Enter` 或 `d` 打开详情面板，然后按 `1` 查看选中进程的完整父进程链（如 `launchd → nginx → worker`）。

### 详情面板标签

底部面板（`Enter`/`d` 切换）有三个标签：

| 标签 | 按键 | 内容 |
|------|------|------|
| **进程树** | `1` | 父进程链 |
| **网络** | `2` | 网络接口详情、IP 地址、MTU |
| **连接** | `3` | 选中 PID 的所有连接 |

### 全屏视图

四个专用视图，通过 `4`-`7` 键访问：

| 视图 | 按键 | 说明 |
|------|------|------|
| **图表** | `4` | 水平柱状图 — 每个进程的连接数 |
| **拓扑** | `5` | ASCII 网络拓扑：进程 → 本地端口 → 远程主机 |
| **进程详情** | `6` | 完整信息：CWD、CPU %、RSS、打开的文件、环境变量、所有连接、网络接口、进程树 |
| **命名空间** | `7` | 按网络命名空间分组（仅 Linux）。显示 `/run/netns/` 中的命名空间名称或原始 inode 编号 |

所有全屏视图支持 `j`/`k` 和 `g`/`G` 滚动。按 `Esc` 返回表格。

### 防火墙快速封锁

在有远程地址的连接上按 `b` 封锁该 IP。确认对话框显示将执行的确切命令：

- **Linux:** `iptables -A INPUT -s <IP> -j DROP`
- **macOS:** `pfctl -t prt_blocked -T add <IP>`

封锁后状态栏显示撤销命令。需要 sudo 权限。

### Strace / Dtruss 附加

按 `t` 将系统调用跟踪器附加到选中进程。详情面板分割显示网络相关系统调用的实时流：

- **Linux:** `strace -p <PID> -e trace=network -f`
- **macOS:** `dtruss -p <PID>`（需要禁用 SIP 或 root）

再次按 `t` 分离。跟踪器进程在退出时自动终止。

### SSH 端口转发

按 `F`（Shift+F）为选中端口创建 SSH 隧道。对话框提示输入远程主机：

```
localhost:5432 →
主机:端口 → user@server.io:5432█
```

隧道通过 `ssh -N -L <local>:localhost:<remote> <host>` 创建。活跃隧道显示在标题栏（`⇄ localhost:5432 → server:22`）。隧道每次刷新检查健康状态，退出时通过 `Drop` 自动终止。

### 告警规则

在 `~/.config/prt/config.toml` 中定义规则：

```toml
[[alerts]]
port = 22
action = "bell"        # 新 SSH 连接时响铃

[[alerts]]
process = "python"
state = "LISTEN"
action = "highlight"   # 黄色高亮行

[[alerts]]
connections_gt = 100
action = "bell"        # 进程超过100个连接时告警
```

告警仅在新条目出现时触发（不是每次刷新）。条件：`port`、`process`、`state`、`connections_gt`。动作：`bell`、`highlight`。

### NDJSON 流式输出

```bash
prt --json | jq '.process.name'
```

每次刷新周期为每个连接输出一个 JSON 对象到 stdout。正确处理 SIGPIPE（管道到 `head` 不会 panic）。不初始化 TUI — 可安全用于脚本和管道。

### Watch 模式

```bash
prt watch 3000 8080 5432
```

简洁的非 TUI 显示，展示特定端口的 UP/DOWN 状态。状态变化时发出 BEL (`\x07`) 信号。连接终端时支持 ANSI 颜色，管道时为纯文本。

```
:3000 ● UP   nginx (1234)   since 14:32:05
:8080 ○ DOWN                 since 14:35:12
:5432 ● UP   postgres (567)  since 14:32:05
```

### 导出

```bash
prt --export json    # 所有连接的 JSON 快照
prt --export csv     # CSV 快照
```

### 多语言界面

英语、俄语、中文。语言优先级：

1. `--lang en|ru|zh` 命令行参数（最高优先级）
2. `PRT_LANG` 环境变量
3. 系统语言自动检测
4. 英语（默认）

在 TUI 中按 `L` 实时切换语言，无需重启。

## 安装

```bash
cargo install prt
```

<details>
<summary><b>从源码编译</b></summary>

```bash
git clone https://github.com/rekurt/prt.git
cd prt
make install    # 或: cargo install --path crates/prt
```

**系统要求:** Rust 1.75+ · macOS 10.15+ 或 Linux（需要 `/proc`）· `lsof`（macOS 已预装）

</details>

## 使用方法

```bash
prt                     # 启动 TUI
prt --lang zh           # 中文界面
prt --export json       # 导出 JSON 快照
prt --export csv        # 导出 CSV 快照
prt --json              # NDJSON 流式输出
prt watch 80 443 5432   # 简洁端口监控模式
sudo prt                # 以 root 运行（查看所有进程）
```

## 快捷键

**导航:**

| 按键 | 操作 |
|------|------|
| `j`/`k` `↑`/`↓` | 移动选择 / 滚动 |
| `g` / `G` | 跳到顶部 / 底部 |
| `/` | 搜索过滤（`!` = 仅可疑） |
| `Esc` | 返回表格 / 清除过滤 |
| `q` | 退出 |

**底部面板（表格模式）:**

| 按键 | 操作 |
|------|------|
| `Enter` / `d` | 显示/隐藏详情面板 |
| `1` `2` `3` | 进程树 / 网络 / 连接 |
| `←`/`→` `h`/`l` | 切换标签 |

**全屏视图:**

| 按键 | 操作 |
|------|------|
| `4` | 图表 — 每进程连接数 |
| `5` | 拓扑 — 进程 → 端口 → 远程 |
| `6` | 进程详情 — 信息、文件、环境变量 |
| `7` | 命名空间（仅 Linux） |

**操作:**

| 按键 | 操作 |
|------|------|
| `K` / `Del` | 终止进程 |
| `c` | 复制行到剪贴板 |
| `p` | 复制 PID 到剪贴板 |
| `b` | 封锁远程 IP（防火墙） |
| `t` | 附加/分离 strace |
| `F` | SSH端口转发 (隧道) |
| `r` | 刷新 |
| `s` | Sudo 密码 |
| `Tab` | 下一排序列 |
| `Shift+Tab` | 反转排序方向 |
| `L` | 切换语言 |
| `?` | 帮助 |

## 配置

创建 `~/.config/prt/config.toml`:

```toml
# 自定义端口名
[known_ports]
3000 = "my-app"
9090 = "prometheus"

# 告警规则
[[alerts]]
port = 22
action = "bell"

[[alerts]]
process = "python"
state = "LISTEN"
action = "highlight"

[[alerts]]
connections_gt = 100
action = "bell"
```

## 开发

```bash
cargo build --workspace          # 构建
cargo test --workspace           # 测试（188个测试）
cargo clippy --workspace         # 代码检查
cargo fmt --all -- --check       # 格式检查
cargo bench -p prt-core          # 基准测试
```

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

[MIT](LICENSE)
