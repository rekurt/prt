<div align="center">

# prt

**Мониторинг сетевых портов в реальном времени прямо в терминале**

<br>

<img src="docs/prt.gif" alt="prt демо" width="720">

<br>
<br>

[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh.md)

</div>

---

## Что такое prt?

`prt` показывает, какие процессы занимают сетевые порты — в реальном времени, прямо в терминале. Интерактивный аналог `lsof -i` / `ss -tlnp` с подсветкой, фильтрацией и деревом процессов.

## Возможности

| Функция | Описание |
|---------|----------|
| **Живая таблица** | Порты, протоколы, состояния, PID, процессы, пользователи. Обновление каждые 2с |
| **Отслеживание изменений** | Новые соединения — зелёным, закрытые — красным на 5с |
| **Дерево процессов** | Цепочка родителей (launchd → nginx → worker) |
| **Вкладки деталей** | Дерево / Сеть / Соединение — переключение `1` `2` `3` |
| **Поиск и фильтр** | По порту, имени процесса, PID, протоколу, состоянию, пользователю |
| **Сортировка** | По любой колонке, по возрастанию/убыванию |
| **Завершение процесса** | Выбрать → `K` → подтвердить `y` (SIGTERM) или `f` (SIGKILL) |
| **Sudo** | Нажать `s`, ввести пароль — увидеть все процессы системы |
| **Буфер обмена** | Копировать строку (`c`) или PID (`p`) |
| **Экспорт** | `prt --export json` или `prt --export csv` |
| **Мультиязычность** | Английский, русский, китайский. Автоопределение, переключение `L` |

## Установка

```bash
cargo install prt
```

<details>
<summary><b>Сборка из исходников</b></summary>

```bash
git clone https://github.com/rekurt/prt.git
cd prt
make install    # или: cargo install --path crates/prt
```

**Требования:** Rust 1.75+ · macOS 10.15+ или Linux с `/proc` · `lsof` (macOS — предустановлен)

</details>

## Использование

```bash
prt                     # запустить TUI
prt --lang ru           # русский интерфейс
prt --lang zh           # китайский интерфейс
prt --export json       # экспорт в JSON
prt --export csv        # экспорт в CSV
PRT_LANG=ru prt         # язык через переменную окружения
sudo prt                # запуск от root (все процессы)
```

## Горячие клавиши

| Клавиша | Действие | | Клавиша | Действие |
|---------|----------|-|---------|----------|
| `q` | Выход | | `K` / `Del` | Завершить процесс |
| `?` | Справка | | `c` | Копировать строку |
| `/` | Поиск | | `p` | Копировать PID |
| `Esc` | Сбросить фильтр | | `Tab` | След. колонка сортировки |
| `r` | Обновить | | `Shift+Tab` | Направление сортировки |
| `s` | Sudo пароль | | `L` | Переключить язык |
| `j`/`k` `↑`/`↓` | Навигация | | `1` `2` `3` | Вкладки деталей |
| `g` / `G` | В начало / конец | | `Enter` / `d` | Показать/скрыть детали |

## Настройка языка

Приоритет определения:

1. Флаг `--lang en|ru|zh` (высший приоритет)
2. Переменная окружения `PRT_LANG`
3. Автоопределение по системной локали
4. Английский (по умолчанию)

Клавиша `L` в TUI переключает язык без перезапуска.

## Архитектура

```
crates/
├── prt-core/                  # Основная библиотека
│   ├── model.rs               # PortEntry, TrackedEntry, SortState, перечисления
│   ├── core/
│   │   ├── scanner.rs         # сканирование → diff → сортировка → фильтр → экспорт
│   │   ├── killer.rs          # SIGTERM / SIGKILL
│   │   └── session.rs         # цикл обновления
│   ├── i18n/                  # EN / RU / ZH, переключение через AtomicU8
│   └── platform/
│       ├── macos.rs           # lsof + пакетный ps (2 вызова за цикл)
│       └── linux.rs           # /proc через procfs
└── prt/                       # TUI-бинарник (ratatui + crossterm + clap)
```

**Поток данных:**

```
platform::scan_ports() → Session::refresh()
    → diff_entries()        New / Unchanged / Gone
    → retain()              удаление Gone через 5с
    → sort_entries()        текущая SortState
    → filter_indices()      поисковый запрос
    → UI рендерит
```

| Платформа | Метод | Производительность |
|-----------|-------|--------------------|
| **macOS** | `lsof -F` структурированный вывод | 2 вызова `ps` за цикл |
| **Linux** | `/proc/net/` через `procfs` | без подпроцессов |

## Разработка

```bash
make check          # fmt + clippy + тесты (79 тестов)
make bench          # бенчмарки Criterion
make doc-open       # сгенерировать и открыть документацию
make test-verbose   # тесты с выводом stdout
```

Подробнее в [CONTRIBUTING.md](CONTRIBUTING.md).

## Лицензия

[MIT](LICENSE)
