# Changelog

All notable changes to this project are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [0.3.0] - 2026-09-09

### Added

- **SSH monitoring** via systemd journal (`[ssh]` section, `ssh` / `sshd` unit)
- **Instant SSH ban** on `Invalid user` and `Failed password for invalid user` — no threshold
- Sliding window for `Failed password` on existing usernames (`threshold`, `window_secs`)
- **`install.sh`** — one-line installer (`curl | sudo bash`)
  - Auto-detects Caddy / NGINX access logs and builds `[[sites]]`
  - Downloads pre-built Linux binary from GitHub Release (amd64 / arm64)
  - Configures SSH journal monitoring when `ssh` / `sshd` unit is present
  - Whitelists admin IP from `$SSH_CONNECTION`
  - Idempotent nftables merge via `/etc/nftables.d/caddyban.nft` (backup + `nft -c`)
  - Installs and starts systemd service
- GitHub Actions **release** workflow — builds and attaches Linux binaries on tag push

### Changed

- Default user-agent: `CaddyBan/0.3`
- systemd unit description updated

## [0.2.0] - 2026-09-09

### Added

- Optional SSH brute-force detection via systemd journal (`[ssh]` section)
- `journal_tailer` — follows `journalctl -f` for configured unit (`ssh` / `sshd`)
- `ssh_parser` — detects `Failed password` and `Invalid user` from OpenSSH logs
- Separate sliding window for SSH (`threshold`, `window_secs`) sharing the same nftables set

### Changed

- `BanService` refactored: shared hit counter for web and SSH offenders
- Default crawl user-agent updated to `CaddyBan/0.2`

## [0.1.0] - 2026-08-26

### Added

- Multi-site configuration via `[[sites]]` (separate `base_url`, `log_path`, catalog per site)
- Legacy config support: `[site]` + `[logs].path` still works
- Real-time access log tailing (Caddy JSON, NGINX combined)
- HTTP site crawler on startup and on schedule (`[crawl].interval_secs`)
- Sliding-window ban logic (`threshold`, `window_secs`)
- nftables ban via `nft add element` with configurable timeout
- IP/CIDR whitelist
- Dry-run mode (`[ban].dry_run`)
- CLI: `run` (default), `crawl`
- systemd unit with `SupplementaryGroups=caddy` for Caddy log permissions
- nftables example integrated into `inet filter`
- GitHub Actions CI (fmt, clippy, test, release build)
- Documentation: README (EN/RU), GitHub publish guide, CONTRIBUTING, Source Available LICENSE

### Notes

- Log tailer reads **new lines only** from the end of the file (not historical entries)
- Default `threshold = 5` reduces false positives from single accidental 404s
- IPv4 nft set by default; extend rules for IPv6 if needed

[0.3.0]: https://github.com/Normal66/CaddyBan/releases/tag/v0.3.0
[0.2.0]: https://github.com/Normal66/CaddyBan/releases/tag/v0.2.0
[0.1.0]: https://github.com/Normal66/CaddyBan/releases/tag/v0.1.0
