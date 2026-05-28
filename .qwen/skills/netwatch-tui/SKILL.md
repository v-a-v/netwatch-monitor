---
name: netwatch-tui
description: TUI ping monitor для Linux с мониторингом внешнего IP, координатами и детальной диагностикой
source: auto-skill
extracted_at: '2026-05-28'
---

# NetWatch Monitor - TUI Ping Monitor

Красивое TUI (Terminal User Interface) приложение для мониторинга ping нескольких серверов, написанное на Rust с использованием Ratatui.

## Основные возможности

- **Мониторинг нескольких серверов** - одновременный ping нескольких хостов
- **Heatmap истории** - визуализация истории пингов справа налево
- **Статистика** - min/avg/max latency, packet loss %, TTL/hop count
- **Внешний IP с детальной информацией** - автоматическое определение внешнего IP с:
  - Город, регион, страна (с кодом)
  - Географические координаты (широта/долгота)
  - ISP, организация, ASN
  - Часовой пояс
- **DNS error detection** - обнаружение проблем с DNS resolution
- **Детальный режим** - живой вывод ping команды при нажатии Enter на сервере
- **Конфигурация** - TOML конфиг с поиском в стандартных путях

## Установка и запуск

```bash
# Клонирование
git clone git@github.com:v-a-v/netwatch-monitor.git
cd netwatch-monitor

# Сборка
./build.sh  # или cargo build --release

# Установка в систему
./install.sh  # в ~/.local/bin или /usr/local/bin

# Запуск
netwatch-monitor
```

## Конфигурация

Конфиг ищется в порядке приоритета:
1. `./config.toml` (текущая директория)
2. `~/.config/netwatch/config.toml`
3. `/etc/netwatch/config.toml`
4. Built-in defaults

Пример `config.toml`:

```toml
interval = 2
history_size = 60

[external_ip]
endpoint = "https://ifconfig.io/ip"
check_interval_sec = 300

[[servers]]
name = "Google DNS"
host = "8.8.8.8"
timeout_ms = 1000
```

## Управление

### В списке серверов
- `↑/k` - выбрать сервер выше
- `↓/j` - выбрать сервер ниже
- `Home/End` - перейти к первому/последнему
- `Enter` - открыть детальный режим ping
- `q` - выход

### В детальном режиме
- `Esc` - вернуться к списку
- `q` - выход

## Визуализация

### Heatmap
- `█` - latency < 50ms (отлично)
- `▓` - latency < 100ms (хорошо)
- `▒` - latency < 200ms (нормально)
- `░` - latency > 200ms (медленно)
- `✗` - ping failed

### Цветовая индикация latency
- Зеленый - < 50ms
- Желтый - < 100ms
- LightRed - < 200ms
- Красный - > 200ms

### Цветовая индикация status (packet loss)
- 🟢 Зеленый - 0% потерь
- 🟠 Желтый - < 20% потерь
- 🔴 Красный - > 20% потерь или DNS ошибка
- ⚪ Серый - Нет данных

### Заголовок с внешней IP информацией
```
🌐 NetWatch  │  13:45:21  │  🌍 91.108.4.12 (Moscow, RU [RU], AS15169, Google LLC, Europe/Moscow, 55°N 37°E) • 13:45:21
```

## Структура проекта

```
netwatch/
├── src/
│   ├── main.rs           # Основной цикл, TUI setup, обработка ввода
│   ├── config.rs         # Парсинг TOML конфига
│   ├── ping.rs           # Асинхронный ping, continuous ping
│   ├── ui.rs             # Рендеринг TUI
│   └── external_ip.rs    # HTTP запросы для внешнего IP
├── config.toml           # Пример конфигурации
├── build.sh              # Скрипт сборки
├── install.sh            # Скрипт установки
└── README.md             # Документация
```

## Зависимости

- Rust 1.70+
- ping утилита в PATH
- reqwest (HTTP client с rustls TLS)
- ratatui + crossterm (TUI)
- tokio (async runtime)
- serde + toml (конфигурация)
- chrono (время)
- dirs (пути к конфигам)

## GitHub

Репозиторий: https://github.com/v-a-v/netwatch-monitor

## Разработка

```bash
# Сборка в режиме разработки
cargo build

# Запуск с логированием
RUST_LOG=netwatch_monitor=debug cargo run

# Форматирование
cargo fmt

# Linting
cargo clippy

# Тесты
cargo test
```

## Contributing

1. Fork проекта
2. Создать ветку: `git checkout -b feature/amazing-feature`
3. Commit: `git commit -m 'feat: добавлена amazing feature'`
4. Push: `git push origin feature/amazing-feature`
5. Открыть Pull Request
