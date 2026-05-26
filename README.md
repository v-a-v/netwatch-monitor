# NetWatch Monitor

Красивый TUI (Terminal User Interface) мониторинг ping для нескольких серверов. Написан на Rust с использованием Ratatui.

## Возможности

- 🌐 Мониторинг нескольких серверов одновременно
- 📊 История пингов в виде heatmap (справа налево)
- 📈 Статистика: min/avg/max latency, packet loss %, TTL
- 🎨 Красивый цветовой индикатор статуса
- ⌨️ Управление с клавиатуры
- ⚙️ Конфигурация через TOML файл
- 🕐 Текущее время в заголовке
- 🌍 Определение внешнего IP с whois информацией

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

### Проверка установки

```bash
which netwatch-monitor
netwatch-monitor --help  # или просто запустить
```

## Конфигурация

### По умолчанию

Программа ищет `config.toml` в следующих местах (по приоритету):

1. Текущая директория запуска
2. `~/.config/netwatch/config.toml`
3. `/etc/netwatch/config.toml`

### Пример config.toml

```toml
# Интервал пинга в секундах
interval = 2

# Размер истории (количество сэмплов на сервер)
history_size = 60

# Внешний IP мониторинг
[external_ip]
# Эндпоинт для получения внешнего IP
endpoint = "https://ifconfig.io/ip"

# Whois эндпоинт (добавляет IP в конец URL)
whois_endpoint = "https://ifconfig.io/whois/"

# Интервал проверки внешнего IP (секунды)
check_interval_sec = 300

# Список серверов для мониторинга
[[servers]]
name = "Google DNS"
host = "8.8.8.8"
timeout_ms = 1000

[[servers]]
name = "Cloudflare DNS"
host = "1.1.1.1"
timeout_ms = 1000

[[servers]]
name = "Yandex DNS"
host = "77.88.8.8"
timeout_ms = 1000

[[servers]]
name = "GitHub"
host = "github.com"
timeout_ms = 2000
```

### Создание пользовательской конфигурации

```bash
# Для пользователя
mkdir -p ~/.config/netwatch
cp config.toml ~/.config/netwatch/

# Или для всей системы (требуется root)
sudo mkdir -p /etc/netwatch
sudo cp config.toml /etc/netwatch/
```

## Управление

| Клавиша | Действие |
|---------|----------|
| `↑` / `k` | Выбрать сервер выше |
| `↓` / `j` | Выбрать сервер ниже |
| `Home` | Перейти к первому серверу |
| `End` | Перейти к последнему серверу |
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

### Индикаторы статуса

- 🟢 - 0% packet loss
- 🟡 - < 20% packet loss
- 🟠 - 20-50% packet loss
- 🔴 - > 50% packet loss

## Пример экрана

```
┌───────────────────────────────────────────────────────────────────────────┐
│ 🌐 NetWatch Monitor  │  2026-05-26 13:45:21  │  🌍 91.108.4.12 (RU / Moscow) │
├───────────────────────────────────────────────────────────────────────────┤
│ Server                Host                 Avg (ms) TTL   Status Histo... │
│ ▶ Google DNS          8.8.8.8              12.3     TTL:58  🟢 0.0% ████▓▒│
│   Cloudflare DNS      1.1.1.1              15.7     TTL:52  🟢 0.0% ████▓▓│
│   Yandex DNS          77.88.8.8            8.2      TTL:48  🟢 0.0% ██████│
│   Google HTTPS        google.com           45.1     TTL:54  🟡 5.0% ███▓▒░│
│   GitHub              github.com           89.3     TTL:42  🟢 0.0% ▓▓▒▒░░│
├───────────────────────────────────────────────────────────────────────────┤
│ Min: 8.20ms Avg: 12.30ms Max: 25.40ms                                     │
│ Packet Loss: 0.0% Success: 60/60                                          │
│ Legend: █ <50ms ▓ <100ms ▒ <200ms ░ >200ms ✗ fail                         │
├───────────────────────────────────────────────────────────────────────────┤
│ ↑/↓: Select server | q: Quit                                              │
└───────────────────────────────────────────────────────────────────────────┘
```

## Скрипты

- `build.sh` — сборка release бинарника
- `install.sh` — установка в систему

## Contributing

Вклад в проект приветствуется! Вот как можно помочь:

### Как внести вклад

1. **Fork** проекта
2. Создай ветку для своей фичи: `git checkout -b feature/amazing-feature`
3. Сделай **commit** изменений: `git commit -m 'feat: добавлена amazing feature'`
4. **Push** в ветку: `git push origin feature/amazing-feature`
5. Открой **Pull Request**

### Guidelines

- Следуй существующему стилю кода
- Пиши понятные commit сообщения (conventional commits)
- Добавляй описание изменений в PR
- Проверяй, что проект собирается: `./build.sh`

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
