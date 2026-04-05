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

| 功能 | 说明 |
|------|------|
| **实时表格** | 端口、服务名、协议、状态、PID、进程、用户。每2秒自动刷新 |
| **变化追踪** | 新连接绿色高亮，关闭的连接红色淡出5秒 |
| **可疑检测** | `[!]` 标记非root使用特权端口、脚本语言占用敏感端口等 |
| **进程树** | 查看完整父进程链（如 launchd → nginx → worker） |
| **详情面板** | 进程树 / 网络 / 连接 — 用 `1` `2` `3` 切换 |
| **全屏视图** | 图表 (`4`)、拓扑 (`5`)、进程详情 (`6`)、命名空间 (`7`) |
| **搜索过滤** | 按端口、服务名、进程名、PID、协议、状态、用户。`!` = 仅可疑 |
| **排序** | 按任意列排序，支持升序/降序 |
| **终止进程** | 选择进程 → `K` → 确认 `y`（SIGTERM）或 `f`（SIGKILL） |
| **封锁IP** | `b` → 通过 iptables/pfctl 封锁远程IP，带撤销命令 |
| **Strace** | `t` → 分屏面板中实时系统调用跟踪 |
| **Sudo 提权** | 按 `s` 输入密码，查看所有系统进程 |
| **剪贴板** | 复制整行（`c`）或仅 PID（`p`） |
| **容器感知** | 显示 Docker/Podman 容器名（无容器时自动隐藏） |
| **带宽** | 标题栏显示系统 RX/TX 速率 |
| **导出** | `prt --export json`、`prt --export csv`、`prt --json`（NDJSON 流） |
| **Watch 模式** | `prt watch 3000 8080` — 简洁的 UP/DOWN 监控 |
| **告警规则** | TOML 配置支持按端口、进程或连接数触发铃声/高亮 |
| **多语言** | 英语、俄语、中文。自动检测语言环境，按 `L` 切换 |
| **配置文件** | `~/.config/prt/config.toml` — 自定义端口名、告警规则 |

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
```

## 语言设置

优先级：

1. `--lang en|ru|zh` 命令行参数（最高优先级）
2. `PRT_LANG` 环境变量
3. 系统语言自动检测
4. 英语（默认）

在 TUI 中按 `L` 实时切换语言，无需重启。

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
