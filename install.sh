#!/usr/bin/env bash
# Bouncer one-line installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Normal66/Bouncer/main/install.sh | sudo bash
set -euo pipefail

REPO="${BOUNCER_REPO:-Normal66/Bouncer}"
VERSION="${BOUNCER_VERSION:-}"
INSTALL_BIN="${BOUNCER_BIN:-/usr/local/bin/bouncer}"
CONFIG_DIR="${BOUNCER_CONFIG_DIR:-/etc/bouncer}"
CONFIG_FILE="${CONFIG_DIR}/config.toml"
SERVICE_NAME="${BOUNCER_SERVICE:-bouncer}"
NFT_TABLE="${BOUNCER_NFT_TABLE:-inet filter}"
NFT_SET="${BOUNCER_NFT_SET:-blocked_ips}"
NFT_TIMEOUT="${BOUNCER_NFT_TIMEOUT:-1h}"

DRY_RUN=false
SKIP_NFT=false
ENABLE_SSH=true
ENABLE_MAIL=true
EXTRA_WHITELIST=()
MANUAL_SITES=()

log() { printf '[bouncer] %s\n' "$*"; }
warn() { printf '[bouncer] WARN: %s\n' "$*" >&2; }
die() { printf '[bouncer] ERROR: %s\n' "$*" >&2; exit 1; }

usage() {
	cat <<'EOF'
Bouncer installer

  curl -fsSL https://raw.githubusercontent.com/Normal66/Bouncer/main/install.sh | sudo bash

Options:
  --version TAG       Release tag (default: latest GitHub release)
  --dry-run           Write config with ban.dry_run = true
  --skip-nft          Do not modify nftables
  --no-ssh            Disable SSH journal monitoring
  --no-mail           Disable Postfix/Dovecot journal monitoring
  --whitelist IP      Add IP to whitelist (repeatable)
  --site URL --log PATH   Manual site (repeat pair)
  -h, --help          Show help

Environment:
  BOUNCER_REPO, BOUNCER_VERSION, BOUNCER_BIN, BOUNCER_CONFIG_DIR
EOF
}

parse_args() {
	while [[ $# -gt 0 ]]; do
		case "$1" in
		--version)
			VERSION="$2"
			shift 2
			;;
		--dry-run)
			DRY_RUN=true
			shift
			;;
		--skip-nft)
			SKIP_NFT=true
			shift
			;;
		--no-ssh)
			ENABLE_SSH=false
			shift
			;;
		--no-mail)
			ENABLE_MAIL=false
			shift
			;;
		--whitelist)
			EXTRA_WHITELIST+=("$2")
			shift 2
			;;
		--site)
			[[ $# -ge 4 && "$3" == "--log" ]] || die "usage: --site URL --log PATH"
			MANUAL_SITES+=("$2|$4")
			shift 4
			;;
		-h | --help)
			usage
			exit 0
			;;
		*)
			die "unknown option: $1"
			;;
		esac
	done
}

require_root() {
	[[ "${EUID:-$(id -u)}" -eq 0 ]] || die "run as root (sudo bash)"
}

detect_arch() {
	local machine
	machine="$(uname -m)"
	case "$machine" in
	x86_64 | amd64) echo "amd64" ;;
	aarch64 | arm64) echo "arm64" ;;
	*) die "unsupported architecture: $machine (need x86_64 or aarch64)" ;;
	esac
}

resolve_version() {
	if [[ -n "$VERSION" ]]; then
		return
	fi
	VERSION="$(
		curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
			grep -m1 '"tag_name"' | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/'
	)"
	[[ -n "$VERSION" ]] || die "failed to resolve latest release version"
	log "latest release: $VERSION"
}

download_binary() {
	local arch asset url tmp
	arch="$(detect_arch)"
	asset="bouncer-linux-${arch}"
	url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
	tmp="$(mktemp)"
	log "downloading ${url}"
	curl -fsSL "$url" -o "$tmp"
	chmod 755 "$tmp"
	install -m 755 "$tmp" "$INSTALL_BIN"
	rm -f "$tmp"
	log "installed ${INSTALL_BIN}"
}

admin_ip_from_ssh() {
	if [[ -n "${SSH_CONNECTION:-}" ]]; then
		echo "${SSH_CONNECTION%% *}"
	fi
}

detect_ssh_unit() {
	if systemctl list-unit-files --type=service --no-pager 2>/dev/null | grep -q '^ssh\.service'; then
		if systemctl is-active --quiet ssh 2>/dev/null ||
			systemctl is-enabled --quiet ssh 2>/dev/null; then
			echo "ssh"
			return
		fi
	fi
	if systemctl list-unit-files --type=service --no-pager 2>/dev/null | grep -q '^sshd\.service'; then
		echo "sshd"
		return
	fi
	echo ""
}

mail_unit_available() {
	local unit="$1"
	if systemctl list-unit-files --type=service --no-pager 2>/dev/null | grep -q "^${unit}\.service"; then
		if systemctl is-active --quiet "$unit" 2>/dev/null ||
			systemctl is-enabled --quiet "$unit" 2>/dev/null; then
			return 0
		fi
	fi
	return 1
}

detect_mail_units() {
	local unit
	for unit in postfix dovecot; do
		if mail_unit_available "$unit"; then
			echo "$unit"
		fi
	done
}

log_format_for_file() {
	local file="$1"
	if [[ -f "$file" ]]; then
		local first
		first="$(head -n1 "$file" 2>/dev/null || true)"
		if [[ "$first" == "{"* ]]; then
			echo "caddy"
			return
		fi
	fi
	echo "nginx"
}

site_name_from_domain() {
	local domain="$1"
	echo "${domain//./-}" | tr '[:upper:]' '[:lower:]'
}

domain_from_log_path() {
	local base domain
	base="$(basename "$1")"
	domain="${base%%.access.log}"
	domain="${domain%%.log}"
	if [[ "$domain" == *.* ]]; then
		echo "$domain"
	fi
}

detect_caddy_sites() {
	local file domain name
	shopt -s nullglob
	for file in /var/log/caddy/*.access.log /var/log/caddy/*.log; do
		[[ -f "$file" ]] || continue
		domain="$(domain_from_log_path "$file")"
		[[ -n "$domain" ]] || continue
		name="$(site_name_from_domain "$domain")"
		echo "caddy|${name}|https://${domain}|${file}"
	done
	shopt -u nullglob
}

detect_nginx_sites() {
	local file domain name
	if command -v nginx >/dev/null 2>&1; then
		while IFS= read -r file; do
			[[ -f "$file" ]] || continue
			domain="$(domain_from_log_path "$file")"
			[[ -n "$domain" ]] || continue
			name="$(site_name_from_domain "$domain")"
			echo "nginx|${name}|https://${domain}|${file}"
		done < <(
			nginx -T 2>/dev/null |
				grep -E '^[[:space:]]*access_log[[:space:]]+' |
				grep -v syslog |
				awk '{print $2}' |
				tr -d ';' |
				sort -u
		)
	fi
	shopt -s nullglob
	for file in /var/log/nginx/*access*.log; do
		[[ -f "$file" ]] || continue
		domain="$(domain_from_log_path "$file")"
		[[ -n "$domain" ]] || continue
		name="$(site_name_from_domain "$domain")"
		echo "nginx|${name}|https://${domain}|${file}"
	done
	shopt -u nullglob
}

collect_sites() {
	local -a sites=()
	local entry kind name url log seen=""
	local key

	if ((${#MANUAL_SITES[@]} > 0)); then
		for entry in "${MANUAL_SITES[@]}"; do
			url="${entry%%|*}"
			log="${entry#*|}"
			name="$(site_name_from_domain "$(echo "$url" | sed -E 's#^https?://([^/]+).*#\1#')")"
			kind="$(log_format_for_file "$log")"
			sites+=("${kind}|${name}|${url}|${log}")
		done
	else
		while IFS= read -r line; do
			[[ -n "$line" ]] && sites+=("$line")
		done < <(detect_caddy_sites)
		while IFS= read -r line; do
			[[ -n "$line" ]] || continue
			key="${line##*|}"
			if [[ "$seen" != *"|${key}|"* ]]; then
				sites+=("$line")
				seen="${seen}|${key}|"
			fi
		done < <(detect_nginx_sites)
	fi

	if ((${#sites[@]} == 0)); then
		die "no sites detected; use --site URL --log PATH or configure Caddy/NGINX logs first"
	fi

	printf '%s\n' "${sites[@]}"
}

primary_log_format() {
	local sites=("$@")
	local first="${sites[0]%%|*}"
	if [[ "$first" == "caddy" ]]; then
		echo "caddy"
	else
		echo "nginx"
	fi
}

write_config() {
	local log_format="$1"
	shift
	local -a sites=("$@")

	local ssh_unit ssh_enabled mail_enabled dry_run admin_ip
	local -a mail_units=()
	ssh_unit="$(detect_ssh_unit)"
	if [[ "$ENABLE_SSH" == true && -n "$ssh_unit" ]]; then
		ssh_enabled=true
	else
		ssh_enabled=false
		[[ "$ENABLE_SSH" == true && -z "$ssh_unit" ]] && warn "SSH service unit not found; disabling [ssh]"
	fi

	while IFS= read -r unit; do
		[[ -n "$unit" ]] && mail_units+=("$unit")
	done < <(detect_mail_units)
	if [[ "$ENABLE_MAIL" == true && ${#mail_units[@]} -gt 0 ]]; then
		mail_enabled=true
	else
		mail_enabled=false
		[[ "$ENABLE_MAIL" == true && ${#mail_units[@]} -eq 0 ]] &&
			warn "Postfix/Dovecot not found; disabling [mail]"
	fi

	if [[ "$DRY_RUN" == true ]]; then
		dry_run=true
	else
		dry_run=false
	fi

	mkdir -p "$CONFIG_DIR"
	if [[ -f "$CONFIG_FILE" ]]; then
		cp -a "$CONFIG_FILE" "${CONFIG_FILE}.bak.$(date +%Y%m%d%H%M%S)"
		log "backed up existing config"
	fi

	{
		echo "# Generated by Bouncer install.sh on $(date -u +%Y-%m-%dT%H:%M:%SZ)"
		echo
		echo "[crawl]"
		echo "interval_secs = 86400"
		echo "timeout_secs = 15"
		echo "max_pages = 200"
		echo "max_depth = 4"
		echo
		echo "[logs]"
		echo "format = \"${log_format}\""
		echo "poll_interval_ms = 300"
		echo
		local entry kind name url log
		for entry in "${sites[@]}"; do
			IFS='|' read -r kind name url log <<<"$entry"
			echo
			echo "[[sites]]"
			echo "name = \"${name}\""
			echo "base_url = \"${url}\""
			echo "log_path = \"${log}\""
			echo 'extra_paths = ["/robots.txt", "/favicon.ico"]'
		done
		echo
		echo "[ban]"
		echo "threshold = 3"
		echo "window_secs = 60"
		echo "ban_duration_secs = 3600"
		echo "dry_run = ${dry_run}"
		echo
		echo "[web.probes]"
		echo "enabled = true"
		echo 'paths = ['
		echo '  "/.env",'
		echo '  "/.git/config",'
		echo '  "/.git/HEAD",'
		echo '  "/.aws/credentials",'
		echo '  "/wp-login.php",'
		echo '  "/wp-admin",'
		echo '  "/xmlrpc.php",'
		echo '  "/phpmyadmin",'
		echo '  "/pma",'
		echo '  "/admin/config.php",'
		echo '  "/vendor/phpunit/phpunit/src/Util/PHP/eval-stdin.php",'
		echo '  "/config.json",'
		echo '  "/actuator",'
		echo '  "/server-status",'
		echo ']'
		echo
		echo "[web.auth]"
		echo "enabled = true"
		echo 'prefixes = ["/admin", "/login", "/wp-login.php", "/wp-admin", "/api/login", "/api/auth", "/user/login"]'
		echo "threshold = 5"
		echo "window_secs = 60"
		echo
		echo "[ssh]"
		echo "enabled = ${ssh_enabled}"
		if [[ "$ssh_enabled" == true ]]; then
			echo "unit = \"${ssh_unit}\""
		else
			echo 'unit = "ssh"'
		fi
		echo "threshold = 3"
		echo "window_secs = 120"
		echo
		echo "[mail]"
		echo "enabled = ${mail_enabled}"
		if [[ "$mail_enabled" == true ]]; then
			echo -n 'units = ['
			local first=true unit
			for unit in "${mail_units[@]}"; do
				if [[ "$first" == true ]]; then
					echo -n "\"${unit}\""
					first=false
				else
					echo -n ", \"${unit}\""
				fi
			done
			echo "]"
			log "mail monitoring: ${mail_units[*]}"
		else
			echo 'units = []'
		fi
		echo "threshold = 3"
		echo "window_secs = 120"
		echo
		echo "[nft]"
		echo "table = \"${NFT_TABLE}\""
		echo "set = \"${NFT_SET}\""
		echo "timeout = \"${NFT_TIMEOUT}\""
		echo
		echo "[whitelist]"
		echo -n 'ips = ["127.0.0.1", "::1"'
		admin_ip="$(admin_ip_from_ssh)"
		if [[ -n "$admin_ip" ]]; then
			echo -n ", \"${admin_ip}\""
			log "whitelisted admin IP from SSH_CONNECTION: ${admin_ip}"
		fi
		local ip
		for ip in "${EXTRA_WHITELIST[@]}"; do
			echo -n ", \"${ip}\""
		done
		echo "]"
		echo 'cidrs = ["10.0.0.0/8", "192.168.0.0/16"]'
	} >"$CONFIG_FILE"
	log "wrote ${CONFIG_FILE}"
}

nft_set_exists() {
	# shellcheck disable=SC2086
	nft list set ${NFT_TABLE} ${NFT_SET} >/dev/null 2>&1
}

nft_drop_rule_exists() {
	# shellcheck disable=SC2086
	nft list chain ${NFT_TABLE} input 2>/dev/null | grep -q "@${NFT_SET}"
}

setup_nftables() {
	[[ "$SKIP_NFT" == true ]] && {
		warn "skipping nftables setup (--skip-nft)"
		return
	}

	command -v nft >/dev/null 2>&1 || die "nft command not found"

	# shellcheck disable=SC2086
	if ! nft list table ${NFT_TABLE} >/dev/null 2>&1; then
		die "nft table '${NFT_TABLE}' not found; create it first or use --skip-nft"
	fi

	if nft_set_exists && nft_drop_rule_exists; then
		log "nftables set and drop rule already present"
		return
	fi

	if ! nft_set_exists; then
		# shellcheck disable=SC2086
		nft add set ${NFT_TABLE} ${NFT_SET} "{ type ipv4_addr; flags timeout; timeout ${NFT_TIMEOUT}; }"
		log "created nft set ${NFT_SET}"
	fi

	if ! nft_drop_rule_exists; then
		# shellcheck disable=SC2086
		nft insert rule ${NFT_TABLE} input ip saddr @${NFT_SET} drop
		log "added drop rule to input chain"
	fi

	local fragment="/etc/nftables.d/bouncer.nft"
	local main_conf="/etc/nftables.conf"

	if [[ ! -f "$fragment" ]]; then
		mkdir -p /etc/nftables.d
		cat >"$fragment" <<EOF
# Bouncer — managed by install.sh
table inet filter {
	set ${NFT_SET} {
		type ipv4_addr
		flags timeout
		timeout ${NFT_TIMEOUT}
	}

	chain input {
		ip saddr @${NFT_SET} drop
	}
}
EOF
		log "wrote ${fragment}"
	fi

	if [[ -f "$main_conf" ]] && ! grep -q 'bouncer.nft' "$main_conf"; then
		cp -a "$main_conf" "${main_conf}.bak.bouncer.$(date +%Y%m%d%H%M%S)"
		printf '\ninclude "/etc/nftables.d/bouncer.nft"\n' >>"$main_conf"
		nft -c -f "$main_conf" || die "nftables config check failed after adding include"
		log "added include to ${main_conf}"
	fi

	nft_set_exists || die "failed to configure nft set ${NFT_SET}"
	nft_drop_rule_exists || warn "drop rule missing; check input chain manually"
	log "nftables configured"
}

install_systemd() {
	local unit="/etc/systemd/system/${SERVICE_NAME}.service"
	local supp=""
	if getent group caddy >/dev/null 2>&1; then
		supp="SupplementaryGroups=caddy"
	fi

	cat >"$unit" <<EOF
[Unit]
Description=Bouncer - ban probing web pages and SSH brute-force
After=network-online.target nftables.service
Wants=network-online.target

[Service]
Type=simple
User=root
Group=root
${supp}
ExecStart=${INSTALL_BIN} --config ${CONFIG_FILE} run
Restart=on-failure
RestartSec=5
TimeoutStopSec=10
KillMode=mixed
AmbientCapabilities=CAP_NET_ADMIN
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOF

	systemctl daemon-reload
	systemctl enable "$SERVICE_NAME"
	log "systemd unit installed: ${unit}"
}

smoke_test() {
	log "running crawl smoke test"
	if ! "$INSTALL_BIN" --config "$CONFIG_FILE" crawl >/dev/null 2>&1; then
		warn "crawl smoke test failed (site may be unreachable); service will still start"
	fi
}

start_service() {
	systemctl restart "$SERVICE_NAME"
	sleep 2
	if systemctl is-active --quiet "$SERVICE_NAME"; then
		log "service active: ${SERVICE_NAME}"
	else
		die "service failed to start; check: journalctl -u ${SERVICE_NAME} -n 30 --no-pager"
	fi
}

print_summary() {
	cat <<EOF

Bouncer installed successfully.

  Binary:  ${INSTALL_BIN}
  Config:  ${CONFIG_FILE}
  Service: ${SERVICE_NAME}
  Version: ${VERSION}

Useful commands:
  journalctl -u ${SERVICE_NAME} -f
  nft list set ${NFT_TABLE} ${NFT_SET}
  ${INSTALL_BIN} --config ${CONFIG_FILE} crawl

EOF
	if [[ "$DRY_RUN" == true ]]; then
		warn "dry_run=true in config — no IPs will be banned until you set dry_run = false"
	fi
}

main() {
	parse_args "$@"
	require_root
	command -v curl >/dev/null 2>&1 || die "curl is required"
	command -v systemctl >/dev/null 2>&1 || die "systemd is required"

	resolve_version
	download_binary

	local -a sites=()
	while IFS= read -r line; do
		sites+=("$line")
	done < <(collect_sites)

	local log_format
	log_format="$(primary_log_format "${sites[@]}")"
	log "detected ${#sites[@]} site(s), log format: ${log_format}"

	write_config "$log_format" "${sites[@]}"
	setup_nftables
	install_systemd
	smoke_test
	start_service
	print_summary
}

main "$@"
