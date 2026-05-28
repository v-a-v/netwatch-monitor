# NetWatch Monitor

Красивый TUI (Terminal User Interface) мониторинг ping для нескольких серверов. Написан на Rust с использованием Ratatui.

## Возможности

- 🌐 Мониторинг нескольких серверов одновременно
- 📊 История пингов в виде heatmap (справа налево)
- 📈 Статистика: min/avg/max latency, packet loss %, TTL/hops
- 🎨 Цветовая индикация: latency и packet loss
- ⌨️ Управление с клавиатуры
- ⚙️ Конфигурация через TOML файл
- 🕐 Текущее время в заголовке
- 🌍 Определение внешнего IP с city/country информацией
- 🔍 Детальный режим просмотра ping (нажми Enter на сервере)
- 📱 Адаптивный интерфейс под размер терминала

## Установка

### Требования

- Rust (1.70+)
- `ping` утилита в PATH (обычно есть в Linux/macOS)

### Быстрая установка

```bash
# 1. Клонируйте репозиторий
git clone git@github.com:v-a-v/netwatch-monitor.git
cd netwatch-monitor

# 2. Соберите бинарник
./build.sh

# 3. Установите в систему
./install.sh
```

После установки запускайте команду `netwatch-monitor` из любого места.

### Ручная установка

```bash
cargo build --release

# Установка в ~/.local/bin (пользовательская)
cp target/release/netwatch-monitor ~/.local/bin/
chmod +x ~/.local/bin/netwatch-monitor

# Или установка в /usr/local/bin (системная, требует sudo)
sudo install -Dm755 target/release/netwatch-monitor /usr/local/bin/netwatch-monitor
```

## Конфигурация

### Поиск конфигурации

Программа ищет `config.toml` в следующих местах (по приоритету):

1. `./config.toml` (текущая директория)
2. `~/.config/netwatch/config.toml`
3. `/etc/netwatch/config.toml`
4. Встроенные значения по умолчанию

### Полный пример config.toml

```toml
# Интервал пинга в секундах
interval = 2

# Размер истории (количество сэмплов на сервер)
history_size = 120

# Внешний IP мониторинг
[external_ip]
# Эндпоинт для получения внешнего IP
endpoint = "https://ifconfig.io/ip"

# Whois информация автоматически берется с ipwho.is
# Этот параметр не используется, оставлен для совместимости
whois_endpoint = ""

# Как часто проверять внешний IP (секунды)
check_interval_sec = 300

# Список серверов для мониторинга
[[servers]]
name = "Gateway"
host = "192.168.1.1"
timeout_ms = 1000

[[servers]]
name = "Google DNS"
host = "8.8.8.8"
timeout_ms = 1000

[[servers]]
name = "Cloudflare DNS"
host = "1.1.1.1"
timeout_ms = 1000

[[servers]]
name = "GitHub"
host = "github.com"
timeout_ms = 2000
```

### Параметры сервера

| Параметр | Описание | По умолчанию |
|----------|----------|--------------|
| `name` | Отображаемое имя сервера | - |
| `host` | IP адрес или доменное имя | - |
| `timeout_ms` | Таймаут ping в миллисекундах | 1000 |

## Управление

### В списке серверов

| Клавиша | Действие |
|---------|----------|
| `↑` / `k` | Выбрать сервер выше |
| `↓` / `j` | Выбрать сервер ниже |
| `Home` | Перейти к первому серверу |
| `End` | Перейти к последнему серверу |
| `Enter` | Открыть детальный просмотр ping |
| `q` | Выход |

### В детальном режиме

| Клавиша | Действие |
|---------|----------|
| `Esc` | Вернуться к списку |
| `q` | Выход |

## Визуализация

### Heatmap истории

```
█ - latency < 50ms  (отлично)
▓ - latency < 100ms (хорошо)
▒ - latency < 200ms (нормально)
░ - latency > 200ms (медленно)
✗ - ping failed     (ошибка)
```

### Цветовая индикация

**Latency (Avg ms):**
- 🟢 Зеленый: < 50ms
- 🟡 Желтый: < 100ms
- 🟠 LightRed: < 200ms
- 🔴 Красный: > 200ms

**Status (Packet Loss):**
- 🟢 Зеленый: 0% потерь
- 🟠 Желтый: < 20% потерь
- 🔴 Красный: > 20% потерь или DNS ошибка
- ⚪ Серый: Нет данных

## Пример экрана

```
┌───────────────────────────────────────────────────────────────────────────┐
│ 🌐 NetWatch  │  13:45:21  │  🌍 91.108.4.12 (Moscow, RU)                  │
├───────────────────────────────────────────────────────────────────────────┤
│ Server            Host               Avg (ms) Hop Status      History     │
│ ▶ Google DNS      8.8.8.8            12.3     58  🟢 0.0% ████▓▓▒░       │
│   Cloudflare      1.1.1.1            15.7     52  🟢 0.0% ████▓▓▒░       │
│   Yandex DNS      77.88.8.8          8.2      48  🟢 0.0% ████████       │
│   GitHub          github.com         89.3     42  🟢 0.0% ▓▓▒▒░░         │
│   Bad Domain      nonexistent.ru     0.0     --   🚫 DNS resolution... ✗✗│
├───────────────────────────────────────────────────────────────────────────┤
│ Min: 8.20ms  Avg: 12.30ms  Max: 25.40ms                                   │
│ Packet Loss: 0.0%  Success: 60/60                                         │
│ Legend: █ <50ms ▓ <100ms ▒ <200ms ░ >200ms ✗ fail                        │
├───────────────────────────────────────────────────────────────────────────┤
│ ↑/↓: Select | Enter: Detail | q: Quit                                     │
└───────────────────────────────────────────────────────────────────────────┘
```

## Скрипты

- `build.sh` — сборка release бинарника
- `install.sh` — установка в систему

## Contributing

Вклад в проект приветствуется!

### Как внести вклад

1. **Fork** проекта
2. Создай ветку: `git checkout -b feature/amazing-feature`
3. Сделай **commit**: `git commit -m 'feat: добавлена amazing feature'`
4. **Push**: `git push origin feature/amazing-feature`
5. Открой **Pull Request**

### Разработка

```bash
# Сборка в режиме разработки
cargo build

# Запуск с логированием
RUST_LOG=netwatch_monitor=debug cargo run

# Форматирование кода
cargo fmt

# Проверка кода
cargo clippy

# Запуск тестов
cargo test
```

## License

MIT
