# Bouncer — Publishing to GitHub (copy-paste)

Step-by-step guide: from local project to repository, tag, and release.

---

## Prerequisites

- [github.com](https://github.com) account
- Git
- [GitHub CLI](https://cli.github.com/) (`gh`) — optional but convenient

Repository: **https://github.com/Normal66/Bouncer**

Replace in commands if needed:

| Placeholder | Replace with |
|-------------|--------------|
| `YOUR_NAME` | your name for git commits |
| `your@email.com` | your email for git commits |

---

## Step 1. Create a GitHub repository

### Option A — via website

1. [github.com/new](https://github.com/new)
2. **Repository name:** `bouncer`
3. **Description:**
   ```
   Server bouncer — ban web scanners & SSH brute-forcers via nftables. One curl install.
   ```
4. **Public** or **Private** (Private recommended if you want to limit visibility)
5. **Do NOT** check «Add README / .gitignore / license» — already in the project
6. **Create repository**

### Option B — via CLI

```bash
gh auth login
gh repo create Normal66/Bouncer --public --description "Server bouncer — ban web scanners & SSH brute-forcers via nftables. One curl install."
```

---

## Step 2. Initialize local git

```bash
cd /path/to/Bouncer

git init
git branch -M main
git status
```

Ensure `.gitignore` excludes `target/`, secrets, and server-specific configs.

### Commit author: keep **cursor** out of Contributors

GitHub **Contributors** lists every distinct `user.name` / email in history. Commits made with Cursor’s default identity add **cursor** to that list.

Before any commit, set identity **for this repo only**:

```bash
git config user.name "YOUR_NAME"
git config user.email "your@email.com"
git config --get user.name
git config --get user.email
```

Prefer a GitHub noreply address. In Cursor, confirm commits use your name before push.

If **cursor** is already a contributor, rewrite those commits and update `main` (see **Still seeing cursor** below).

#### Still seeing cursor in Contributors

The graph uses commits on the **default branch** (`main`). One commit authored as `cursor` keeps the avatar until history is rewritten.

**1. Find cursor commits (all refs):**

```bash
git fetch --all --tags --prune
git log --all --format="%an <%ae>" | sort -u
git log --all --regexp-ignore-case --extended-regexp --author='cursor|cursoragent' --oneline
```

**2. Match remote `main`:**

```bash
git log origin/main --regexp-ignore-case --extended-regexp --author='cursor|cursoragent' --oneline
```

If empty locally but the site still shows cursor: confirm repo/branch on **Insights → Contributors**, wait **24–72 h** for cache, or contact GitHub Support.

**3. Many commits — `git filter-repo`:**

```bash
pip install git-filter-repo
git clone https://github.com/Normal66/Bouncer.git bouncer-rewrite && cd bouncer-rewrite

git filter-repo --force --name-callback '
    if name.lower() in (b"cursor", b"cursor agent"):
        return b"YOUR_NAME"
    return name
' --email-callback '
    if b"cursor" in email.lower():
        return b"your@email.com"
    return email
'

git remote add origin https://github.com/Normal66/Bouncer.git
git push --force-with-lease origin main
```

Update tags if needed: `git push --force-with-lease origin --tags`

**4. Latest commit only:** `git commit --amend --author="YOUR_NAME <your@email.com>" --no-edit` then `git push --force-with-lease origin main`.

**5. Feature branches** do not affect Contributors until merged into `main`.

---

## Step 3. First commit

```bash
git add .
git commit -m "Initial release v0.1.0: multi-site log monitor with nftables banning"
```

---

## Step 4. Add remote and push

```bash
git remote add origin https://github.com/Normal66/Bouncer.git
git push -u origin main
```

Use a Personal Access Token when prompted (Settings → Developer settings → Tokens).

---

## Step 5. Repository settings on GitHub

Open `https://github.com/Normal66/Bouncer/settings`

### About

**Description:**
```
Server bouncer — one curl, zero tolerance for scanners and SSH probes. nftables native.
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
| `v1.0.0` | one-line `install.sh` + release binaries |
| `v0.2.0` | SSH brute-force detection |
| `v1.0.0` | stable production |

```bash
git tag -a v1.0.0 -m "v1.0.0: SSH monitoring, instant invalid-user ban, install.sh"
git push origin v1.0.0
```

Pushing a `v*` tag triggers `.github/workflows/release.yml` — builds `bouncer-linux-amd64` and `bouncer-linux-arm64` and attaches `install.sh` to the GitHub Release.

---

## Step 7. GitHub Release

### Via website

1. **Releases → Create a new release**
2. **Tag:** `v1.0.0`
3. **Title:** `v1.0.0 — SSH ban + one-line installer`
4. Paste release notes from [CHANGELOG.md](../CHANGELOG.md) section `[0.3.0]`

Release workflow uploads Linux binaries automatically. Users install with:

```bash
curl -fsSL https://raw.githubusercontent.com/Normal66/Bouncer/main/install.sh | sudo bash
```

### Via CLI

```bash
gh release create v1.0.0 --title "v1.0.0 — SSH ban + one-line installer" --notes-file CHANGELOG.md
```

---

## Step 8. CI

`.github/workflows/ci.yml` runs on push/PR: fmt, clippy, test, release build.

Release workflow (`.github/workflows/release.yml`) uses **cross** to build static **musl** binaries for amd64 and arm64; the job verifies they are not dynamically linked before publishing.

Check the **Actions** tab after pushing.

---

## Step 9. README badges (optional)

```markdown
[![CI](https://github.com/Normal66/Bouncer/actions/workflows/ci.yml/badge.svg)](https://github.com/Normal66/Bouncer/actions/workflows/ci.yml)
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
- [ ] No server-specific configs or real IPs in tracked files
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
