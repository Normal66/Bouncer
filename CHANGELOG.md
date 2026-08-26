# Changelog

All notable changes to this project are documented in this file.

Format based on [Keep a Changelog](https://keepachangelog.com/).
Versioning follows [Semantic Versioning](https://semver.org/).

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

[0.1.0]: https://github.com/YOUR_USER/caddyban/releases/tag/v0.1.0
