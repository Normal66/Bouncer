# CaddyBan — публикация на GitHub (copy-paste)

Пошаговая инструкция: от локального проекта до репозитория, тега и релиза.

---

## Что понадобится

- Аккаунт [github.com](https://github.com)
- Git (Windows: [Git for Windows](https://git-scm.com/download/win))
- [GitHub CLI](https://cli.github.com/) (`gh`) — опционально, но удобно

Замените во всех командах:

| Плейсхолдер | На что заменить |
|-------------|-----------------|
| `YOUR_USER` | ваш логин GitHub |
| `YOUR_NAME` | ваше имя для git commit |
| `your@email.com` | ваш email для git commit |

---

## Шаг 1. Создать репозиторий на GitHub

### Вариант A — через сайт

1. [github.com/new](https://github.com/new)
2. **Repository name:** `caddyban`
3. **Description:**
   ```
   Ban IPs probing non-existent pages via Caddy/NGINX logs and nftables (Rust)
   ```
4. **Public** или **Private** (для Source Available лучше **Private**, если не хотите fork)
5. **НЕ** ставьте галочки «Add README / .gitignore / license» — всё уже в проекте
6. **Create repository**

### Вариант B — через CLI

```bash
gh auth login
gh repo create YOUR_USER/caddyban --public --description "Ban IPs probing non-existent pages via Caddy/NGINX logs and nftables (Rust)"
```

---

## Шаг 2. Подготовить локальный git

В PowerShell (Windows) или bash (Linux):

```bash
cd F:\Develop\CaddyBan

git init
git branch -M main
```

Проверьте `.gitignore` — в репозиторий **не** должны попасть:

- `target/`
- `.env`, секреты, локальные конфиги серверов

```bash
git status
```

---

## Шаг 3. Первый коммит

```bash
git add .
git commit -m "Initial release v0.1.0: multi-site log monitor with nftables banning"
```

---

## Шаг 4. Привязать remote и push

```bash
git remote add origin https://github.com/YOUR_USER/caddyban.git
git push -u origin main
```

При запросе авторизации — Personal Access Token (Settings → Developer settings → Tokens).

---

## Шаг 5. Настройки репозитория на GitHub

Откройте `https://github.com/YOUR_USER/caddyban/settings`

### About (справа на главной / Settings → General)

**Description (EN):**
```
Real-time Caddy/NGINX access log monitor. Bans IPs probing non-existent pages via nftables.
```

**Website:** (опционально) URL вашего сайта или docs

**Topics** — вставьте через пробел или запятую:

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

GitHub не имеет шаблона Source Available → выберите **Other** или не указывайте шаблон.
Текст лицензии уже в файле [LICENSE](../LICENSE).

### Features (рекомендуется)

- [x] **Issues** — баги и предложения
- [ ] **Discussions** — по желанию
- [ ] **Wiki** — не нужно, всё в README/docs

### Security

Settings → **Private vulnerability reporting** — включите, если репозиторий public.

---

## Шаг 6. Тег версии

Используем [Semantic Versioning](https://semver.org/): `vMAJOR.MINOR.PATCH`

| Тег | Когда |
|-----|-------|
| `v0.1.0` | первый публичный релиз |
| `v0.1.1` | bugfix без новых фич |
| `v0.2.0` | новая функциональность |
| `v1.0.0` | стабильный production-ready |

```bash
git tag -a v0.1.0 -m "v0.1.0: initial release"
git push origin v0.1.0
```

Проверка:

```bash
git tag -l
```

---

## Шаг 7. GitHub Release

### Через сайт

1. **Releases → Create a new release**
2. **Choose a tag:** `v0.1.0`
3. **Release title:** `v0.1.0 — Initial release`
4. **Description** — скопируйте блок ниже

### Через CLI

```bash
gh release create v0.1.0 --title "v0.1.0 — Initial release" --notes-file CHANGELOG.md
```

### Текст release notes (copy-paste)

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

5. **Publish release**

---

## Шаг 8. CI (уже в репозитории)

Файл `.github/workflows/ci.yml` запускается на push/PR:

- `cargo fmt --check`
- `cargo clippy`
- `cargo test`
- `cargo build --release`

После push проверьте вкладку **Actions** на GitHub.

---

## Шаг 9. Что написать в README на GitHub

GitHub показывает `README.md` на главной. Русская версия — [README.ru.md](../README.ru.md).

В начале README уже есть:
- описание проекта
- quick start
- ссылка на русскую документацию

Добавьте в README бейджи (опционально, после создания repo):

```markdown
[![CI](https://github.com/YOUR_USER/caddyban/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USER/caddyban/actions/workflows/ci.yml)
```

---

## Шаг 10. CONTRIBUTING и Issues

Файл [CONTRIBUTING.md](../CONTRIBUTING.md) уже описывает правила:

- PR только в upstream
- без публичных long-lived fork для independent development

Шаблон Issue (создайте `.github/ISSUE_TEMPLATE/bug_report.md` при желании):

```markdown
---
name: Bug report
about: Report a problem
---

**Version:** v0.1.0
**OS / distro:**
**Caddy or NGINX version:**

**config.toml** (redact secrets):

**What happened:**

**Expected:**

**Logs:** `journalctl -u caddyban -n 50`
```

---

## Шаг 11. Обновления и новые релизы

```bash
# правки в коде
git add .
git commit -m "fix: log tailer permission hint in docs"
git push

# новая версия
git tag -a v0.1.1 -m "v0.1.1: bugfix description"
git push origin v0.1.1
gh release create v0.1.1 --title "v0.1.1" --notes "Bug fixes: ..."
```

---

## Чеклист перед публикацией

- [ ] `cargo test` проходит локально
- [ ] `config.example.toml` без секретов и реальных IP
- [ ] В README `YOUR_USER` / `your-org` заменены на ваш GitHub
- [ ] LICENSE и CONTRIBUTING.md на месте
- [ ] `.gitignore` исключает `target/`
- [ ] Проверен dry-run на сервере перед `dry_run = false`

---

## Связанные документы

| Файл | Назначение |
|------|------------|
| [README.ru.md](../README.ru.md) | документация проекта (RU) |
| [README.md](../README.md) | документация проекта (EN) |
| [CHANGELOG.md](../CHANGELOG.md) | история версий |
| [LICENSE](../LICENSE) | Source Available |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | правила PR |

---

## English version

See [GITHUB.en.md](GITHUB.en.md).
