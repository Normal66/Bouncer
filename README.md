# Bouncer

[![CI](https://github.com/Normal66/Bouncer/actions/workflows/ci.yml/badge.svg)](https://github.com/Normal66/Bouncer/actions/workflows/ci.yml)

> **One curl. Zero tolerance for scanners and SSH probes.**  
> Your server's bouncer — drop intruders at the door via **nftables**.

**Bouncer** bans malicious IPs before they reach your apps:

- **Web** — probes for non-existent pages (404 on unknown paths)
- **SSH** — brute-force attempts (`Failed password`, `Invalid user` in systemd journal)

Works with **Caddy** (JSON access log) and **NGINX** (combined log format). Supports **multiple sites** in one daemon.

**Docs:** [README.ru.md](README.ru.md) · [nftables](docs/NFTABLES.md) · [GitHub publish (EN)](docs/GITHUB.en.md) · [GitHub publish (RU)](docs/GITHUB.ru.md)

---

## How it works

1. **Path catalog** — on startup Bouncer crawls each configured site and builds a set of valid URL paths.
2. **Scheduled re-crawl** — catalogs refresh on an interval (default: 24 h).
3. **Log tailing** — each site's access log is tailed **from the end of the file** (new lines only).
4. **Detection** — if an IP gets HTTP **404** for a path **not** in that site's catalog, hits accumulate in a sliding window.
5. **Ban** — when `threshold` is reached within `window_secs`, the IP is added to an **nftables set** with timeout (blocks **all** incoming traffic from that IP).
6. **SSH** (optional) — follows `journalctl -f -u ssh` / `sshd`:
   - `Invalid user … from IP` → **instant ban** (first attempt)
   - `Failed password for invalid user …` → **ignored** (duplicate line from sshd)
   - `Failed password for root …` → sliding window (`threshold` / `window_secs`)

```
  [[sites]] ──► Crawler ──► Path catalog ◄── compare ── Log tailer ◄── access.log
                                              │
  [ssh] journal ──► ssh_parser ───────────────┤
                                              ▼
                                        Ban service ──► nft add element ...
```

### Why `threshold = 5` (not 1)?

A single 404 can be a typo, broken link, or one bot probe. **Five 404s on unknown paths within ~2 minutes** looks like scanning. Tune in config — see [Ban tuning](#ban-tuning).

### SSH: instant vs sliding window

| Event in journal | Action |
|------------------|--------|
| `Invalid user lee from 203.0.113.10 …` | Ban immediately (one log line per IP) |
| `Failed password for invalid user …` | Ignored (duplicate of `Invalid user` on same attempt) |
| `Failed password for root from …` | Wait for `threshold` (default 3) within `window_secs` |

There is no legitimate reason to try random usernames on a production server.

### Initial crawl vs scheduled crawl

| Approach | Recommendation |
|----------|----------------|
| **Initial crawl on startup** | **Required** |
| **Scheduled re-crawl** | **Recommended** (default 24 h) |
| **`extra_paths` in config** | **Recommended** for API/SPA routes |

### Why Bouncer (not fail2ban)?

| | fail2ban | Bouncer |
|---|----------|---------|
| Install | jails, filters, Python | `curl \| bash` |
| Web scanning | manual filters | auto crawl + 404 detection |
| SSH `Invalid user` | needs custom filter | **instant ban** built-in |
| Firewall | iptables / scripts | **nftables native** |
| Binary | Python stack | single Rust binary |

---

## Requirements

- Linux with **nftables**
- root or `CAP_NET_ADMIN`
- Rust **1.85+** (edition 2024) to build
- Caddy or NGINX writing access logs to files readable by the daemon

**GitHub Release binaries** (current CI) are **static musl** builds (`cross`, `*-unknown-linux-musl`) with no glibc version requirement — suitable for **Debian 12**, Ubuntu, and other Linux amd64/arm64 hosts. Older releases before the musl switch may show `GLIBC_… not found`; upgrade the release or build on the server ([Manual install](#manual-install)).

---

## Quick start (copy-paste)

### One-line install (recommended)

On a Linux server with **Caddy** or **NGINX** and **nftables** (`inet filter` table):

```bash
curl -fsSL https://raw.githubusercontent.com/Normal66/Bouncer/main/install.sh | sudo bash
```

The script will:

1. Download the release binary (`linux-amd64` or `arm64`)
2. Detect access log files and generate `/etc/bouncer/config.toml`
3. Enable SSH monitoring if `ssh` / `sshd` is present
4. Whitelist your IP from `$SSH_CONNECTION`
5. Configure nftables: `blocked_ips` set + `ip saddr @blocked_ips drop` **early** in `input` (see [nftables rule order](#nftables-rule-order))
6. Install systemd unit and start the service

**Options:**

```bash
# Pin version, test without banning:
curl -fsSL .../install.sh | sudo bash -s -- --version v1.0.0 --dry-run

# Manual site when auto-detect finds nothing:
curl -fsSL .../install.sh | sudo bash -s -- \
  --site https://example.com --log /var/log/caddy/example.com.access.log

# nftables already configured:
curl -fsSL .../install.sh | sudo bash -s -- --skip-nft
```

After install, set `dry_run = false` in `/etc/bouncer/config.toml` and run `systemctl restart bouncer` when ready.

If `install.sh` fails after downloading the binary, or `bouncer` will not start, see [Troubleshooting](#troubleshooting).

---

### Manual install

#### 1. Build

```bash
git clone https://github.com/Normal66/Bouncer.git
cd Bouncer
cargo build --release
sudo install -m 755 target/release/bouncer /usr/local/bin/bouncer
```

#### 2. nftables — merge into existing rules

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

# Inside chain input — right after established,related (before accept :22/:443):
ip saddr @blocked_ips drop
```

Example full structure — [examples/nftables/setup.nft](examples/nftables/setup.nft).  
Set-only fragment for `include` — [examples/nftables/bouncer.nft](examples/nftables/bouncer.nft) (drop rule stays in main `input` chain).

### nftables rule order

Bouncer runtime only runs `nft add element … blocked_ips { IP }`. **Firewall drop must exist in config** and match **before** port-specific `accept` rules:

```bash
sudo nft -a list chain inet filter input
# expect: ip saddr @blocked_ips drop  (handle lower than accept :22 / :443)
```

Putting `drop` in a separate `include` file as a second `chain input { … }` block appends it at the **end** — banned IPs still reach SSH and HTTP. `install.sh` inserts the drop line after `established,related accept` in `/etc/nftables.conf` and writes a set-only `/etc/nftables.d/bouncer.nft`.

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

#### 3. Configure

```bash
sudo mkdir -p /etc/bouncer
sudo cp config.example.toml /etc/bouncer/config.toml
sudo nano /etc/bouncer/config.toml
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

[ssh]
enabled = true
unit = "ssh"       # use "sshd" on RHEL/Fedora
threshold = 3
window_secs = 120

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

#### 4. Test crawl

```bash
bouncer --config /etc/bouncer/config.toml crawl
# Output: [site-a] /  [site-a] /about.html  ...
```

#### 5. Dry run, then production

Keep `dry_run = true`, run manually, watch journal:

```bash
sudo RUST_LOG=bouncer=info bouncer --config /etc/bouncer/config.toml run
```

When satisfied, set `dry_run = false` and use systemd (step 6).

#### 6. systemd

```bash
sudo cp systemd/bouncer.service /etc/systemd/system/bouncer.service
sudo systemctl daemon-reload
sudo systemctl enable --now bouncer
sudo journalctl -u bouncer -f
```

The unit uses `SupplementaryGroups=caddy` so root can read Caddy log files (`640`). If journal still shows **access denied** when tailing logs (often after rotation or tight directory permissions), set filesystem ACL — see [Troubleshooting](#troubleshooting).

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
| `[ssh]` | `enabled` | `false` | Monitor SSH journal |
| `[ssh]` | `unit` | `ssh` | systemd unit (`ssh` or `sshd`) |
| `[ssh]` | `threshold` | `3` | Failed **password** attempts before ban (valid usernames only) |
| `[ssh]` | `window_secs` | `120` | SSH sliding window for failed passwords |
| — | — | — | `Invalid user` → **instant ban** (no threshold) |
| `[nft]` | `table` | — | e.g. `inet filter` |
| `[nft]` | `set` | — | e.g. `blocked_ips` |
| `[nft]` | `timeout` | `1h` | Ban duration in nft |
| `[whitelist]` | `ips`, `cidrs` | — | Never ban these |

Legacy single-site config (`[site]` + `[logs].path`) is still supported.

---

## CLI

```bash
bouncer --config /etc/bouncer/config.toml run
bouncer --config /etc/bouncer/config.toml crawl
RUST_LOG=bouncer=debug bouncer run
```

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Empty `blocked_ips` set | No new qualifying traffic since start | Normal; wait for scanners or lower `threshold` |
| Empty set, many 404s in file | Tailer reads **from EOF only** | Expected; only new lines count |
| `failed to open` / `access denied` log file | Directory permissions or new file after rotation | `SupplementaryGroups=caddy` (see unit); ACL below if needed |
| Ban not blocking traffic | Drop rule after accept `:22/:443` | Add `ip saddr @blocked_ips drop` right after `established,related accept` |
| Repeated `IP banned` for same IP | New sshd/journal lines after ban | Normal since v1.1.1: only **first** ban is logged; check `nft list set` |
| Your IP banned | Admin traffic or SSH typos | Add IP to `[whitelist].ips` |
| SSH ban not triggering | Too few attempts after start | Lower `[ssh].threshold`; check `journalctl -u ssh` |
| SSH unit not found | Wrong unit name on distro | Debian/Ubuntu: `ssh`; RHEL/Fedora: `sshd` |
| `install.sh` fails on download | No GitHub Release yet | Push tag `v*` or pass `--version` |
| Binary won't run, `GLIBC_… not found` | Older release (dynamic glibc) | Install latest GitHub Release (musl) or `cargo build --release` on the server |
| `no sites detected` | Non-standard log paths | `--site URL --log PATH` |

Check bans:

```bash
sudo nft list set inet filter blocked_ips
sudo journalctl -u bouncer | grep banned
```

**Log directory ACL** (same for any `log_path` in `config.toml`; use the log file’s parent directory):

```bash
LOG_DIR="/var/log/caddy"
sudo setfacl -R -m u:root:rx "$LOG_DIR"
sudo setfacl -R -m u:root:r "$LOG_DIR"/*
sudo setfacl -R -d -m u:root:r "$LOG_DIR"
sudo setfacl -d -m u:root:rx "$LOG_DIR"
```

Default ACL keeps **new** files readable after Caddy/NGINX log rotation (bouncer runs as `User=root`).

---

## Project layout

```
Bouncer/
├── install.sh            # one-line installer (curl | bash)
├── src/                  # Rust source
├── config.example.toml   # example config
├── systemd/              # systemd unit
├── examples/nftables/    # setup.nft (full example), bouncer.nft (set-only)
├── docs/NFTABLES.md      # firewall set + rule order
├── docs/GITHUB.en.md     # GitHub publish guide
├── docs/GITHUB.ru.md
├── CHANGELOG.md
├── CONTRIBUTING.md
└── LICENSE
```

---

## Migrating from CaddyBan

If you ran the old **CaddyBan** on a server:

```bash
systemctl stop caddyban
cp /etc/caddyban/config.toml /etc/bouncer/config.toml   # paths still valid
curl -fsSL https://raw.githubusercontent.com/Normal66/Bouncer/main/install.sh | sudo bash -s -- --skip-nft
```

Or copy config manually, install `bouncer` binary, use `systemd/bouncer.service`.

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
