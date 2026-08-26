# CaddyBan — Publishing to GitHub (copy-paste)

Step-by-step guide: from local project to repository, tag, and release.

---

## Prerequisites

- [github.com](https://github.com) account
- Git
- [GitHub CLI](https://cli.github.com/) (`gh`) — optional but convenient

Replace in all commands:

| Placeholder | Replace with |
|-------------|--------------|
| `YOUR_USER` | your GitHub username |
| `YOUR_NAME` | your name for git commits |
| `your@email.com` | your email for git commits |

---

## Step 1. Create a GitHub repository

### Option A — via website

1. [github.com/new](https://github.com/new)
2. **Repository name:** `caddyban`
3. **Description:**
   ```
   Ban IPs probing non-existent pages via Caddy/NGINX logs and nftables (Rust)
   ```
4. **Public** or **Private** (Private recommended if you want to limit visibility)
5. **Do NOT** check «Add README / .gitignore / license» — already in the project
6. **Create repository**

### Option B — via CLI

```bash
gh auth login
gh repo create YOUR_USER/caddyban --public --description "Ban IPs probing non-existent pages via Caddy/NGINX logs and nftables (Rust)"
```

---

## Step 2. Initialize local git

```bash
cd /path/to/CaddyBan

git init
git branch -M main
git status
```

Ensure `.gitignore` excludes `target/`, secrets, and server-specific configs.

---

## Step 3. First commit

```bash
git add .
git commit -m "Initial release v0.1.0: multi-site log monitor with nftables banning"
```

---

## Step 4. Add remote and push

```bash
git remote add origin https://github.com/YOUR_USER/caddyban.git
git push -u origin main
```

Use a Personal Access Token when prompted (Settings → Developer settings → Tokens).

---

## Step 5. Repository settings on GitHub

Open `https://github.com/YOUR_USER/caddyban/settings`

### About

**Description:**
```
Real-time Caddy/NGINX access log monitor. Bans IPs probing non-existent pages via nftables.
```

**Topics:**

```
rust
nftables
caddy
nginx
web-security
firewall
access-log
bot-detection
intrusion-prevention
linux
```

### License

No standard «Source Available» template on GitHub — choose **Other** or leave unset.
Full text is in [LICENSE](../LICENSE).

### Features

- Enable **Issues**
- Enable **Discussions** — optional

---

## Step 6. Version tag

[Semantic Versioning](https://semver.org/): `vMAJOR.MINOR.PATCH`

| Tag | When |
|-----|------|
| `v0.1.0` | first public release |
| `v0.1.1` | bugfix only |
| `v0.2.0` | new features |
| `v1.0.0` | stable production |

```bash
git tag -a v0.1.0 -m "v0.1.0: initial release"
git push origin v0.1.0
```

---

## Step 7. GitHub Release

### Via website

1. **Releases → Create a new release**
2. **Tag:** `v0.1.0`
3. **Title:** `v0.1.0 — Initial release`
4. Paste release notes below

### Via CLI

```bash
gh release create v0.1.0 --title "v0.1.0 — Initial release" --notes-file CHANGELOG.md
```

### Release notes (copy-paste)

```markdown
## CaddyBan v0.1.0

First public release.

### Features
- Multi-site support: `[[sites]]` with separate crawl catalog and log file per site
- Real-time tail of Caddy JSON and NGINX combined access logs
- Site crawler (startup + scheduled re-crawl)
- Sliding window ban: configurable `threshold` and `window_secs`
- nftables integration via `nft add element` with timeout
- Whitelist IPs/CIDRs
- Dry-run mode for testing
- systemd unit included

### Requirements
- Linux, nftables, Rust 1.85+ (to build)
- Caddy or NGINX with file access logs

### Quick install
See [README.md](README.md) or [README.ru.md](README.ru.md).

### License
Source Available — see [LICENSE](LICENSE). Contributions via PR to this repository only.
```

---

## Step 8. CI

`.github/workflows/ci.yml` runs on push/PR: fmt, clippy, test, release build.

Check the **Actions** tab after pushing.

---

## Step 9. README badges (optional)

```markdown
[![CI](https://github.com/YOUR_USER/caddyban/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USER/caddyban/actions/workflows/ci.yml)
```

---

## Step 10. Future releases

```bash
git add .
git commit -m "fix: describe your change"
git push

git tag -a v0.1.1 -m "v0.1.1: bugfix description"
git push origin v0.1.1
gh release create v0.1.1 --title "v0.1.1" --notes "..."
```

---

## Pre-publish checklist

- [ ] `cargo test` passes locally
- [ ] `config.example.toml` has no secrets or real IPs
- [ ] README placeholders replaced with your GitHub username
- [ ] LICENSE and CONTRIBUTING.md present
- [ ] `.gitignore` excludes `target/`
- [ ] Dry-run tested on server before production

---

## Related docs

| File | Purpose |
|------|---------|
| [README.md](../README.md) | project docs (EN) |
| [README.ru.md](../README.ru.md) | project docs (RU) |
| [GITHUB.ru.md](GITHUB.ru.md) | this guide (RU) |
| [CHANGELOG.md](../CHANGELOG.md) | version history |
| [LICENSE](../LICENSE) | Source Available |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | PR rules |
