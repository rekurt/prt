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

## 为什么选择 prt？

传统工具 `lsof`、`ss` 和 `netstat` 只能给出静态快照，读到时已经过时。`prt` 提供**实时自动刷新的终端界面**，带变化追踪：

- **实时查看连接变化** — 绿色 = 新连接，红色 = 正在关闭
- **即时发现端口冲突** — 不再需要反复猜测 `lsof -i :8080`
- **自动检测可疑连接** — 异常连接自动标记 `[!]`
- **一键封锁恶意 IP** — 直接在 TUI 中通过防火墙封锁
- **容器集成** — 查看 Docker/Podman 容器占用的端口
- **即时系统调用追踪** — 无需离开 TUI 即可使用 strace/dtruss
- **带宽监控** — 标题栏实时显示系统级吞吐量
- **告警规则** — 端口开启或连接数超限时收到通知

## prt vs lsof vs ss vs netstat

| 功能 | `prt` | `lsof -i` | `ss -tlnp` | `netstat -tlnp` |
|------|:-----:|:---------:|:----------:|:---------------:|
| 实时自动刷新 | **是** | 否 | 否 | 否 |
| 变化追踪（新建/关闭） | **是** | 否 | 否 | 否 |
| 彩色输出 | **是** | 否 | 否 | 否 |
| 交互式过滤 | **是** | 否 | 否 | 否 |
| 进程树 | **是** | 否 | 否 | 否 |
| 已知端口名称（170+） | **是** | 部分 | 部分 | 部分 |
| 可疑连接检测 | **是** | 否 | 否 | 否 |
| Docker/Podman 容器 | **是** | 否 | 否 | 否 |
| 带宽监控 | **是** | 否 | 否 | 否 |
| IP 封锁（防火墙） | **是** | 否 | 否 | 否 |
| Strace/dtruss | **是** | 否 | 否 | 否 |
| SSH 隧道 | **是** | 否 | 否 | 否 |
| 告警规则（TOML 配置） | **是** | 否 | 否 | 否 |
| 导出 JSON/CSV | **是** | 否 | 否 | 否 |
| NDJSON 流式输出 | **是** | 否 | 否 | 否 |
| 多语言（EN/RU/ZH） | **是** | 否 | 否 | 否 |
| macOS + Linux | **是** | macOS/Linux | Linux | Linux |
| 单文件二进制 | **是** | 系统自带 | 系统自带 | 系统自带 |

## 功能特性

### 实时表格与变化追踪

主界面以可排序、可过滤的表格显示所有活跃的网络连接。列包括：端口、服务名、协议、状态、PID、进程名、用户。新连接以**绿色**高亮；关闭的连接以**红色**淡出5秒后消失。每2秒自动刷新。

### 已知端口数据库

`Service` 列将常见端口号映射为可读名称 — http (80)、ssh (22)、postgres (5432) 等约170个。可在 `~/.config/prt/config.toml` 中覆盖或扩展：

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

### 分区

`Tab` / `Shift+Tab` 在三个顶级分区之间循环切换。当前分区在顶部高亮显示。

| 分区 | 默认内容 | 子标签 (`[` / `]`) |
|------|----------|--------------------|
| **连接** | 端口表 + 底部详情面板 (`Enter` / `d` 切换) | — |
| **进程** | 选中条目的进程详情 (CWD、CPU %、RSS、打开的文件、env、所有连接、进程树) | 详情 ⇄ 拓扑 |
| **SSH** | 已保存的主机和活跃隧道集中在同一处 | 主机 ⇄ 隧道 |

连接表下方的 **详情** 面板是单一统一视图，包含 bind 类型、网络接口、远程地址、状态、cmdline、相关端口和进程树 — 不需要切换标签。

进程分区的 **拓扑** 子标签为整个工作集绘制 ASCII 树
`进程 → :本地端口 → 远程`。

可滚动的视图都支持 `j`/`k` 和 `g`/`G`。

### 操作菜单 (`Space`)

对选中条目的几乎所有操作都通过一个上下文相关的弹出菜单（按 `Space` 打开）完成：

- **终止进程**（也可直接按 `K`）
- **复制行**（也可直接按 `c`） / **复制 PID**
- **封锁远程 IP** — `iptables -A INPUT -s <IP> -j DROP` (Linux) /
  `pfctl -t prt_blocked -T add <IP>` (macOS)。状态栏显示撤销命令，需要 sudo。
- **跟踪系统调用** — `strace -p <PID> -e trace=network -f` (Linux) 或
  `dtruss -p <PID>` (macOS，需要禁用 SIP 或 root)。再次执行可分离。
- **SSH 转发** — 打开隧道表单，可选择本地端口、远程目标和主机别名。

菜单只显示对当前条目有效的操作 — 没有远程地址时不会显示「封锁」和「转发」。

### SSH 分区

`SSH` 集中了两个子标签：

- **主机** — 来自 `~/.ssh/config` 和 `~/.config/prt/config.toml` 的 `[[ssh_hosts]]` 的只读列表。按 `Enter` 打开预填了别名的隧道表单。
- **隧道** — 带实时状态的活跃隧道：🟢 活跃，🟡 启动中，🔴 失败（失败的隧道会保留在列表中直到处理）。
  按键: `n` 新建 · `e` 编辑 · `K` 终止 · `r` 重启 · `s` 保存到配置。

隧道表单支持**实时验证**（输入时不正确的字段变红）、**编辑模式**（`Enter` 替换现有隧道），并**防止意外关闭** — 在非空表单上按 `Esc` 需要在 1.5 秒内再次按 `Esc` 才会丢弃。

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

**全局:**

| 按键 | 操作 |
|------|------|
| `?` | 帮助 (cheat sheet) |
| `q` | 退出 |
| `Tab` / `Shift+Tab` | 下一个 / 上一个分区 (连接 \| 进程 \| SSH) |
| `Space` | 操作菜单（终止 / 复制 / 封锁 / 跟踪 / 转发） |
| `/` | 搜索 / 过滤（`!` = 仅可疑） |
| `Esc` | 关闭模态框 · 再按一次清除过滤 |
| `r` | 刷新 |
| `s` | Sudo 密码 |
| `L` | 切换语言 |
| `j`/`k` `↑`/`↓` `g`/`G` | 移动 / 滚动 / 跳到顶部 / 底部 |

**直接快捷键（任意分区）:**

| 按键 | 操作 |
|------|------|
| `K` / `Del` | 终止选中进程 |
| `c` | 复制行到剪贴板 |

**连接分区:**

| 按键 | 操作 |
|------|------|
| `Enter` / `d` | 切换底部详情面板 |
| `o` / `O` | 下一排序列 / 反转方向 |

**进程分区:**

| 按键 | 操作 |
|------|------|
| `[` / `]` | 切换子标签 (详情 \| 拓扑) |

**SSH 分区:**

| 按键 | 操作 |
|------|------|
| `[` / `]` | 切换子标签 (主机 \| 隧道) |
| 主机: `Enter` | 从所选主机创建新隧道 |
| 主机: `r` | 重新加载 `~/.ssh/config` 与 prt 配置 |
| 隧道: `n` | 打开新建隧道表单 |
| 隧道: `e` | 编辑选中隧道（保存时 kill + 重启） |
| 隧道: `K` | 终止选中隧道 |
| 隧道: `r` | 重启选中隧道 |
| 隧道: `s` | 保存活跃隧道到配置 |

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

## 常见问题

### 如何查看所有进程？某些端口显示 "unknown"。

使用 `sudo prt` 运行 — 没有 root 权限时，操作系统会隐藏其他用户的进程 PID。

### prt 支持 Windows 吗？

目前不支持。`prt` 当前支持 **macOS**（10.15+）和 **Linux**（需要 `/proc`）。Windows 支持正在 issue tracker 中跟踪。

### prt 和 `htop` / `btop` 有什么区别？

`htop`/`btop` 是通用进程监控器。`prt` 专注于**网络连接和端口** — 显示哪个进程使用哪个端口、追踪连接生命周期、检测异常，并提供网络特定操作（防火墙封锁、strace、SSH 隧道）。

### prt 可以用在脚本和管道中吗？

可以！使用 `prt --json` 进行 NDJSON 流式输出，`prt --export json|csv` 获取快照，或 `prt watch <端口>` 进行简单的 UP/DOWN 监控。

### prt 在生产环境中安全吗？

`prt` 默认是**只读诊断工具**。破坏性操作（终止进程、封锁 IP、附加 strace）始终需要明确确认。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=rekurt/prt&type=Date)](https://star-history.com/#rekurt/prt&Date)

## 许可证

[MIT](LICENSE)

---

<div align="center">

**如果 `prt` 对您有帮助，请在 GitHub 上点个 Star！**

[![GitHub stars](https://img.shields.io/github/stars/rekurt/prt?style=social)](https://github.com/rekurt/prt)

</div>
