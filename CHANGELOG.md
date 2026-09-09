# Changelog

All notable changes to this project are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [1.1.0] - 2026-09-09

### Added

- **Instant ban** on known probe paths (`/.env`, `/wp-admin`, …) — no crawl required
- **401/403 auth probing** — threshold ban on configurable path prefixes
- **Mail protection** — Postfix/Dovecot via journald (invalid user → instant, auth failed → threshold)
- `install.sh` auto-detects Postfix/Dovecot and only enables `[mail]` when services exist

## [1.0.0] - 2026-09-09

**Bouncer** — rebrand and first public release under the new name (formerly developed as *CaddyBan*).

### Added

- **One-line install:** `curl | bash` via [install.sh](install.sh)
- **Web protection** — ban IPs probing non-existent pages (Caddy JSON / NGINX combined logs)
- **SSH protection** — systemd journal monitoring (`ssh` / `sshd`)
- **Instant ban** on `Invalid user` and `Failed password for invalid user`
- Sliding window for `Failed password` on existing usernames
- Multi-site support, nftables `blocked_ips` set, whitelist, dry-run mode
- GitHub Actions: CI + release binaries (`bouncer-linux-amd64`, `bouncer-linux-arm64`)

### Install

```bash
curl -fsSL https://raw.githubusercontent.com/Normal66/Bouncer/main/install.sh | sudo bash
```

[1.1.0]: https://github.com/Normal66/Bouncer/releases/tag/v1.1.0
[1.0.0]: https://github.com/Normal66/Bouncer/releases/tag/v1.0.0
