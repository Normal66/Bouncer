# CaddyBan

Real-time web access log monitor that bans IPs probing **non-existent pages** via **nftables**.

Works with **Caddy** (JSON access log) and **NGINX** (combined log format). Supports **multiple sites** in one daemon.

**Docs:** [README.ru.md](README.ru.md) · [GitHub publish guide (EN)](docs/GITHUB.en.md) · [GitHub publish guide (RU)](docs/GITHUB.ru.md)

---

## How it works

1. **Path catalog** — on startup CaddyBan crawls each configured site and builds a set of valid URL paths.
2. **Scheduled re-crawl** — catalogs refresh on an interval (default: 24 h).
3. **Log tailing** — each site's access log is tailed **from the end of the file** (new lines only).
4. **Detection** — if an IP gets HTTP **404** for a path **not** in that site's catalog, hits accumulate in a sliding window.
5. **Ban** — when `threshold` is reached within `window_secs`, the IP is added to an **nftables set** with timeout.

```
  [[sites]] ──► Crawler ──► Path catalog ◄── compare ── Log tailer ◄── access.log
                                              │
                                              ▼
                                        Ban service ──► nft add element ...
```

### Why `threshold = 5` (not 1)?

A single 404 can be a typo, broken link, or one bot probe. **Five 404s on unknown paths within ~2 minutes** looks like scanning. Tune in config — see [Ban tuning](#ban-tuning).

### Initial crawl vs scheduled crawl

| Approach | Recommendation |
|----------|----------------|
| **Initial crawl on startup** | **Required** |
| **Scheduled re-crawl** | **Recommended** (default 24 h) |
| **`extra_paths` in config** | **Recommended** for API/SPA routes |

---

## Requirements

- Linux with **nftables**
- root or `CAP_NET_ADMIN`
- Rust **1.85+** (edition 2024) to build
- Caddy or NGINX writing access logs to files readable by the daemon

---

## Quick start (copy-paste)

### 1. Build

```bash
git clone https://github.com/YOUR_USER/caddyban.git
cd caddyban
cargo build --release
sudo install -m 755 target/release/caddyban /usr/local/bin/caddyban
```

### 2. nftables — merge into existing rules

**Back up first:**

```bash
sudo cp -a /etc/nftables.conf /etc/nftables.conf.bak.$(date +%Y%m%d%H%M%S)
```

Add to your `inet filter` table (do **not** replace existing rules):

```nft
set blocked_ips {
    type ipv4_addr
    flags timeout
    timeout 1h
}

# Inside chain input (after established,related and lo):
ip saddr @blocked_ips drop
```

Example full structure — see [examples/nftables/setup.nft](examples/nftables/setup.nft).

Apply and verify:

```bash
sudo nft -c -f /etc/nftables.conf && sudo nft -f /etc/nftables.conf
sudo nft list set inet filter blocked_ips
```

In `config.toml`:

```toml
[nft]
table = "inet filter"
set = "blocked_ips"
timeout = "1h"
```

### 3. Configure

```bash
sudo mkdir -p /etc/caddyban
sudo cp config.example.toml /etc/caddyban/config.toml
sudo nano /etc/caddyban/config.toml
```

**Multi-site example:**

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
name = "site-a"
base_url = "https://example.com"
log_path = "/var/log/caddy/example.com.access.log"
extra_paths = ["/robots.txt", "/favicon.ico"]

[[sites]]
name = "site-b"
base_url = "https://other.example"
log_path = "/var/log/caddy/other.example.access.log"

[ban]
threshold = 5
window_secs = 120
dry_run = true

[nft]
table = "inet filter"
set = "blocked_ips"
timeout = "1h"

[whitelist]
ips = ["127.0.0.1", "::1", "YOUR_ADMIN_IP"]
cidrs = ["10.0.0.0/8", "192.168.0.0/16"]
```

**Caddy** — JSON access log per site:

```caddyfile
log {
    output file /var/log/caddy/example.com.access.log {
        roll_size 100mb
        roll_keep 5
    }
    format json
}
```

**NGINX** — combined format:

```nginx
access_log /var/log/nginx/access.log combined;
```

### 4. Test crawl

```bash
caddyban --config /etc/caddyban/config.toml crawl
# Output: [site-a] /  [site-a] /about.html  ...
```

### 5. Dry run, then production

Keep `dry_run = true`, run manually, watch journal:

```bash
sudo RUST_LOG=caddyban=info caddyban --config /etc/caddyban/config.toml run
```

When satisfied, set `dry_run = false` and use systemd (step 6).

### 6. systemd

```bash
sudo cp systemd/caddyban.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now caddyban
sudo journalctl -u caddyban -f
```

The unit uses `SupplementaryGroups=caddy` so root can read Caddy log files (`640`).

---

## Ban tuning

| Parameter | Default | Meaning |
|-----------|---------|---------|
| `threshold` | `5` | 404 hits on **unknown** paths before ban |
| `window_secs` | `60` | sliding window (seconds) |
| `dry_run` | `false` | log only, no `nft` call |

Examples:

```toml
# Stricter (faster bans)
threshold = 3
window_secs = 60

# Softer (fewer false positives)
threshold = 10
window_secs = 300
```

**Important:** only **new** log lines after daemon start are processed. Historical 404s in the file are ignored.

---

## Configuration reference

| Section | Key | Default | Description |
|---------|-----|---------|-------------|
| `[crawl]` | `interval_secs` | `86400` | Re-crawl all sites |
| `[crawl]` | `timeout_secs` | `10` | HTTP timeout per page |
| `[crawl]` | `max_pages` | `500` | Max pages per site |
| `[crawl]` | `max_depth` | `5` | Link crawl depth |
| `[logs]` | `format` | — | `caddy` or `nginx` |
| `[logs]` | `poll_interval_ms` | `500` | Log poll interval |
| `[[sites]]` | `base_url` | — | Site root for crawl |
| `[[sites]]` | `log_path` | — | Access log file |
| `[[sites]]` | `name` | optional | Label in logs/CLI |
| `[[sites]]` | `extra_paths` | `[]` | Always-valid paths |
| `[ban]` | `threshold` | `5` | 404 count before ban |
| `[ban]` | `window_secs` | `60` | Sliding window |
| `[ban]` | `dry_run` | `false` | Test without nft |
| `[nft]` | `table` | — | e.g. `inet filter` |
| `[nft]` | `set` | — | e.g. `blocked_ips` |
| `[nft]` | `timeout` | `1h` | Ban duration in nft |
| `[whitelist]` | `ips`, `cidrs` | — | Never ban these |

Legacy single-site config (`[site]` + `[logs].path`) is still supported.

---

## CLI

```bash
caddyban --config /etc/caddyban/config.toml run
caddyban --config /etc/caddyban/config.toml crawl
RUST_LOG=caddyban=debug caddyban run
```

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Empty `blocked_ips` set | No new qualifying traffic since start | Normal; wait for scanners or lower `threshold` |
| Empty set, many 404s in file | Tailer reads **from EOF only** | Expected; only new lines count |
| `failed to open` log file | Permissions / systemd sandbox | Use `SupplementaryGroups=caddy` (see unit) |
| Ban not blocking traffic | Drop rule missing in `input` chain | Add `ip saddr @blocked_ips drop` |
| Your IP banned | Admin traffic to probe URLs | Add IP to `[whitelist].ips` |

Check bans:

```bash
sudo nft list set inet filter blocked_ips
sudo journalctl -u caddyban | grep banned
```

---

## Project layout

```
caddyban/
├── src/                  # Rust source
├── config.example.toml   # example config
├── systemd/              # systemd unit
├── examples/nftables/    # nftables snippet
├── docs/GITHUB.en.md     # GitHub publish guide
├── docs/GITHUB.ru.md
├── CHANGELOG.md
├── CONTRIBUTING.md
└── LICENSE
```

---

## Contributing & license

Contributions via **pull request** to the official repository only — [CONTRIBUTING.md](CONTRIBUTING.md).

**Source Available** license — [LICENSE](LICENSE). Forking and redistribution without permission are not allowed.

---

## Publish on GitHub

Step-by-step (tags, releases, topics): **[docs/GITHUB.en.md](docs/GITHUB.en.md)** · **[docs/GITHUB.ru.md](docs/GITHUB.ru.md)**

---

## Русская документация

[README.ru.md](README.ru.md)
