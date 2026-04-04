<div align="center">

# prt

**终端实时网络端口监控工具**

<br>

<img src="docs/prt.gif" alt="prt 演示" width="720">

<br>
<br>

[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh.md)

</div>

---

## 什么是 prt？

`prt` 实时显示哪些进程占用了您机器上的网络端口。它是 `lsof -i` / `ss -tlnp` 的交互式替代品，支持颜色高亮、过滤和进程树。

## 功能特性

| 功能 | 说明 |
|------|------|
| **实时表格** | 端口、协议、状态、PID、进程、用户。每2秒自动刷新 |
| **变化追踪** | 新连接绿色高亮，关闭的连接红色淡出5秒 |
| **进程树** | 查看完整父进程链（如 launchd → nginx → worker） |
| **详情标签** | 进程树 / 网络 / 连接 — 用 `1` `2` `3` 切换 |
| **搜索过滤** | 按端口、进程名、PID、协议、状态、用户搜索 |
| **排序** | 按任意列排序，支持升序/降序 |
| **终止进程** | 选择进程 → `K` → 确认 `y`（SIGTERM）或 `f`（SIGKILL） |
| **Sudo 提权** | 按 `s` 输入密码，查看所有系统进程 |
| **剪贴板** | 复制整行（`c`）或仅 PID（`p`） |
| **导出** | `prt --export json` 或 `prt --export csv` |
| **多语言** | 英语、俄语、中文。自动检测语言环境，按 `L` 切换 |

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
prt --lang ru           # 俄语界面
prt --export json       # 导出 JSON 快照
prt --export csv        # 导出 CSV 快照
PRT_LANG=zh prt         # 通过环境变量设置语言
sudo prt                # 以 root 运行（查看所有进程）
```

## 快捷键

| 按键 | 操作 | | 按键 | 操作 |
|------|------|-|------|------|
| `q` | 退出 | | `K` / `Del` | 终止进程 |
| `?` | 帮助 | | `c` | 复制行 |
| `/` | 搜索 | | `p` | 复制 PID |
| `Esc` | 清除过滤 | | `Tab` | 下一排序列 |
| `r` | 刷新 | | `Shift+Tab` | 反转排序 |
| `s` | Sudo 密码 | | `L` | 切换语言 |
| `j`/`k` `↑`/`↓` | 导航 | | `1` `2` `3` | 详情标签 |
| `g` / `G` | 顶部 / 底部 | | `Enter` / `d` | 显示/隐藏详情 |

## 语言设置

优先级：

1. `--lang en|ru|zh` 命令行参数（最高优先级）
2. `PRT_LANG` 环境变量
3. 系统语言自动检测
4. 英语（默认）

在 TUI 中按 `L` 实时切换语言，无需重启。

## 架构

```
crates/
├── prt-core/                  # 核心库（平台无关）
│   ├── model.rs               # PortEntry, TrackedEntry, SortState, 枚举
│   ├── core/
│   │   ├── scanner.rs         # 扫描 → 差异 → 排序 → 过滤 → 导出
│   │   ├── killer.rs          # SIGTERM / SIGKILL
│   │   └── session.rs         # 刷新周期状态管理
│   ├── i18n/                  # EN / RU / ZH，基于 AtomicU8 的运行时切换
│   └── platform/
│       ├── macos.rs           # lsof + 批量 ps（每周期2次调用）
│       └── linux.rs           # 通过 procfs 读取 /proc
└── prt/                       # TUI 可执行文件（ratatui + crossterm + clap）
```

**数据流:**

```
platform::scan_ports() → Session::refresh()
    → diff_entries()        New / Unchanged / Gone
    → retain()              5秒后移除 Gone
    → sort_entries()        当前 SortState
    → filter_indices()      用户搜索查询
    → UI 渲染
```

| 平台 | 方法 | 性能 |
|------|------|------|
| **macOS** | `lsof -F` 结构化输出 | 每周期仅2次 `ps` 调用 |
| **Linux** | `/proc/net/` via `procfs` | 无子进程开销 |

## 开发

```bash
make check          # fmt + clippy + 测试（79个测试）
make bench          # Criterion 基准测试
make doc-open       # 生成并打开文档
make test-verbose   # 带输出的测试
```

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

[MIT](LICENSE)
