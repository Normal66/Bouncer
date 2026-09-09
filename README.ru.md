# CaddyBan

[![CI](https://github.com/Normal66/CaddyBan/actions/workflows/ci.yml/badge.svg)](https://github.com/Normal66/CaddyBan/actions/workflows/ci.yml)

Мониторинг в реальном времени с автоматическим баном IP через **nftables**:

- **Web** — запросы несуществующих страниц (404 на неизвестные пути)
- **SSH** — брутфорс (`Failed password`, `Invalid user` в systemd journal)

Поддержка **Caddy** (JSON), **NGINX** (combined), **нескольких сайтов** в одном процессе.

**Документация:** [README.md](README.md) · [Публикация на GitHub (RU)](docs/GITHUB.ru.md) · [GitHub publish (EN)](docs/GITHUB.en.md)

---

## Как это работает

1. **Каталог страниц** — для каждого сайта из `[[sites]]` выполняется обход и строится список валидных путей.
2. **Периодический обход** — каталоги обновляются по расписанию (по умолчанию раз в 24 ч).
3. **Чтение логов** — tail каждого `log_path` **с конца файла** (только новые строки).
4. **Детекция** — IP получает **404** на путь **вне каталога** → событие в скользящем окне.
5. **Бан** — при достижении `threshold` за `window_secs` → IP в **nftables set** с таймаутом (блокируется **весь** входящий трафик с IP).

6. **SSH** (опционально) — `journalctl -f -u ssh` / `sshd`:
   - `Invalid user … from IP` → **мгновенный бан** (с первой попытки)
   - `Failed password for root …` → скользящее окно (`threshold` / `window_secs`)

### SSH: мгновенно vs окно

| Событие в journal | Действие |
|-------------------|----------|
| `Invalid user lee from 203.0.113.10 …` | Бан сразу |
| `Failed password for invalid user …` | Бан сразу |
| `Failed password for root from …` | Ждём `threshold` (по умолчанию 3) за `window_secs` |

Случайного перебора несуществующих пользователей на проде не бывает.

### Почему `threshold = 5`, а не 1?

Один 404 — опечатка, битая ссылка или один probe бота. **Пять 404 на неизвестные пути за ~2 минуты** — типичное сканирование. Настраивается в конфиге — см. [Настройка бана](#настройка-бана).

### Обход сайта

| Вариант | Рекомендация |
|---------|--------------|
| **При старте** | **Обязателен** |
| **По расписанию** | **Рекомендуется** (24 ч) |
| **`extra_paths`** | **Рекомендуется** для API/SPA |

---

## Требования

- Linux, **nftables**
- root или `CAP_NET_ADMIN`
- Rust **1.85+**
- Caddy или NGINX с access log в файлах

---

## Быстрый старт (copy-paste)

### Установка одной командой (рекомендуется)

На Linux-сервере с **Caddy** или **NGINX** и **nftables** (таблица `inet filter`):

```bash
curl -fsSL https://raw.githubusercontent.com/Normal66/CaddyBan/main/install.sh | sudo bash
```

Скрипт автоматически:

1. Скачает бинарник из GitHub Release (`linux-amd64` / `arm64`)
2. Найдёт access-логи и создаст `/etc/caddyban/config.toml`
3. Включит мониторинг SSH, если есть unit `ssh` / `sshd`
4. Добавит ваш IP из `$SSH_CONNECTION` в whitelist
5. Настроит nftables (`blocked_ips` + drop в `input`)
6. Установит systemd unit и запустит сервис

**Опции:**

```bash
# Конкретная версия, тест без бана:
curl -fsSL .../install.sh | sudo bash -s -- --version v0.3.0 --dry-run

# Ручной сайт, если auto-detect ничего не нашёл:
curl -fsSL .../install.sh | sudo bash -s -- \
  --site https://example.com --log /var/log/caddy/example.com.access.log

# nftables уже настроен:
curl -fsSL .../install.sh | sudo bash -s -- --skip-nft
```

После проверки установите `dry_run = false` в конфиге и `systemctl restart caddyban`.

---

### Ручная установка

#### 1. Сборка

```bash
git clone https://github.com/Normal66/CaddyBan.git
cd CaddyBan
cargo build --release
sudo install -m 755 target/release/caddyban /usr/local/bin/caddyban
```

#### 2. nftables — влить в существующие правила

**Сначала бэкап:**

```bash
sudo cp -a /etc/nftables.conf /etc/nftables.conf.bak.$(date +%Y%m%d%H%M%S)
```

Добавьте в таблицу `inet filter` (не заменяйте файл целиком):

```nft
set blocked_ips {
    type ipv4_addr
    flags timeout
    timeout 1h
}

# В chain input (после established,related и lo):
ip saddr @blocked_ips drop
```

Пример — [examples/nftables/setup.nft](examples/nftables/setup.nft).

```bash
sudo nft -c -f /etc/nftables.conf && sudo nft -f /etc/nftables.conf
```

В конфиге:

```toml
[nft]
table = "inet filter"
set = "blocked_ips"
timeout = "1h"
```

#### 3. Конфигурация

```bash
sudo mkdir -p /etc/caddyban
sudo cp config.example.toml /etc/caddyban/config.toml
sudo nano /etc/caddyban/config.toml
```

**Multi-site:**

```toml
[crawl]
interval_secs = 86400
timeout_secs = 15
max_pages = 200
max_depth = 4

[logs]
format = "caddy"
poll_interval_ms = 300

[[sites]]
name = "example"
base_url = "https://example.com"
log_path = "/var/log/caddy/example.com.access.log"
extra_paths = ["/robots.txt", "/favicon.ico"]

[[sites]]
name = "other"
base_url = "https://other.example"
log_path = "/var/log/caddy/other.example.access.log"

[ban]
threshold = 5
window_secs = 120
dry_run = true

[ssh]
enabled = true
unit = "ssh"       # на RHEL/Fedora: "sshd"
threshold = 3
window_secs = 120

[whitelist]
ips = ["127.0.0.1", "::1", "ВАШ_IP"]
cidrs = ["10.0.0.0/8", "192.168.0.0/16"]
```

**Caddy** — JSON log:

```caddyfile
log {
    output file /var/log/caddy/example.com.access.log { roll_size 100mb roll_keep 5 }
    format json
}
```

#### 4. Проверка обхода

```bash
caddyban --config /etc/caddyban/config.toml crawl
```

#### 5. Dry-run → production

```bash
sudo RUST_LOG=caddyban=info caddyban --config /etc/caddyban/config.toml run
```

Затем `dry_run = false` и systemd.

#### 6. systemd

```bash
sudo cp systemd/caddyban.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now caddyban
sudo journalctl -u caddyban -f
```

Unit содержит `SupplementaryGroups=caddy` — root читает логи Caddy (`640`).

---

## Настройка бана

| Параметр | По умолчанию | Смысл |
|----------|--------------|-------|
| `threshold` | `5` | сколько 404 на **неизвестные** пути до бана |
| `window_secs` | `60` | окно в секундах |
| `dry_run` | `false` | только лог, без nft |

```toml
# Жёстче
threshold = 3
window_secs = 60

# Мягче
threshold = 10
window_secs = 300
```

**Важно:** обрабатываются только **новые** строки лога после старта демона. Старые 404 в файле не учитываются.

---

## Параметры конфигурации

| Секция | Параметр | По умолчанию | Описание |
|--------|----------|--------------|----------|
| `[crawl]` | `interval_secs` | `86400` | Интервал re-crawl |
| `[crawl]` | `max_pages` | `500` | Лимит страниц |
| `[logs]` | `format` | — | `caddy` / `nginx` |
| `[[sites]]` | `base_url`, `log_path` | — | Сайт и его лог |
| `[[sites]]` | `extra_paths` | `[]` | Всегда валидные пути |
| `[ban]` | `threshold` | `5` | Порог 404 |
| `[ban]` | `window_secs` | `60` | Окно |
| `[ssh]` | `enabled` | `false` | Мониторинг SSH journal |
| `[ssh]` | `unit` | `ssh` | systemd unit |
| `[ssh]` | `threshold` | `3` | Попыток `Failed password` (существующий user) |
| `[ssh]` | `window_secs` | `120` | Окно для failed password |
| — | — | — | `Invalid user` → **мгновенный бан** |
| `[nft]` | `table`, `set` | — | nftables |
| `[whitelist]` | `ips`, `cidrs` | — | Исключения |

Старый формат `[site]` + `[logs].path` поддерживается.

---

## CLI

```bash
caddyban --config /etc/caddyban/config.toml run
caddyban --config /etc/caddyban/config.toml crawl
RUST_LOG=caddyban=debug caddyban run
```

---

## Troubleshooting

| Симптом | Причина | Решение |
|---------|---------|---------|
| Set `blocked_ips` пуст | Нет новых событий после старта | Норма; подождать или снизить `threshold` |
| Много 404 в файле, банов нет | Tail только с EOF | Только новые строки |
| `failed to open` log | Права / sandbox | `SupplementaryGroups=caddy` в unit |
| Бан не блокирует | Нет `drop` в input | Добавить `ip saddr @blocked_ips drop` |
| Забанили себя | Свои probe-запросы или опечатки SSH | IP в `[whitelist].ips` |
| SSH-бан не срабатывает | Мало попыток после старта | Снизить `[ssh].threshold`; проверить `journalctl -u ssh` |
| `install.sh` — ошибка download | Нет GitHub Release | Push тег `v*` или `--version` |
| `no sites detected` | Нестандартные пути логов | `--site URL --log PATH` |

```bash
sudo nft list set inet filter blocked_ips
sudo journalctl -u caddyban | grep banned
```

---

## Структура проекта

```
CaddyBan/
├── install.sh            # установка одной командой (curl | bash)
├── src/
├── config.example.toml
├── systemd/
├── examples/nftables/
├── docs/GITHUB.ru.md     # инструкция GitHub
├── docs/GITHUB.en.md
├── CHANGELOG.md
├── CONTRIBUTING.md
└── LICENSE
```

---

## Contributing и лицензия

Pull request'ы в официальный репозиторий — [CONTRIBUTING.md](CONTRIBUTING.md).

**Source Available** — [LICENSE](LICENSE). Fork и redistribution без разрешения запрещены.

---

## Публикация на GitHub

Пошагово: теги, releases, topics — **[docs/GITHUB.ru.md](docs/GITHUB.ru.md)**

---

## English documentation

[README.md](README.md)
