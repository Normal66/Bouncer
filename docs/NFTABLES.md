# nftables integration

Bouncer does **not** create firewall rules at runtime. It only runs:

```bash
nft add element inet filter blocked_ips { 203.0.113.10 timeout 1h }
```

You must define the **set** and a **drop rule** in `/etc/nftables.conf` (or includes merged into the same table).

## Rule order (critical)

The drop rule must match **before** any `accept` for ports you want to protect (typically `:22`, `:80`, `:443`):

```nft
chain input {
    iif "lo" accept
    ct state invalid drop
    ct state established,related accept
    ip saddr @blocked_ips drop          # ← here
    tcp dport 22 ... accept
    tcp dport { 80, 443 } accept
}
```

Verify:

```bash
nft -a list chain inet filter input
```

The `@blocked_ips` rule handle must be **lower** than your SSH/HTTP accept rules.

## Wrong pattern: include with extra `chain input`

This fragment **does not work** if included after your main rules:

```nft
# /etc/nftables.d/bouncer.nft — BAD if this is the only drop rule
table inet filter {
    chain input {
        ip saddr @blocked_ips drop
    }
}
```

nftables **appends** rules from a second `chain input` block to the end of the chain — after your `accept` rules. Banned IPs still reach sshd and the web server.

Use [examples/nftables/bouncer.nft](../examples/nftables/bouncer.nft) (**set only**) and put `ip saddr @blocked_ips drop` in the main config early in `input`.

## install.sh behavior

1. `nft add set` / `nft insert rule … drop` for immediate effect (insert = top of chain).
2. Writes set-only `/etc/nftables.d/bouncer.nft`.
3. If `/etc/nftables.conf` has no `@blocked_ips`, inserts `ip saddr @blocked_ips drop` after `established,related accept` via `sed`.
4. Adds `include "/etc/nftables.d/bouncer.nft"` only if the set is not already defined inline.

## Files in this repo

| File | Purpose |
|------|---------|
| [examples/nftables/setup.nft](../examples/nftables/setup.nft) | Minimal full firewall example with correct order |
| [examples/nftables/bouncer.nft](../examples/nftables/bouncer.nft) | Set-only fragment for `include` |

## Troubleshooting

| Symptom | Check |
|---------|--------|
| IP in set, still hits sshd | `nft -a list chain inet filter input` — drop after accept? |
| Set missing after reboot | Set not in persisted config; merge set into `/etc/nftables.conf` |
| `nft add element` errors | Set does not exist — apply config or run `install.sh` |

See also [README.md](../README.md#nftables-rule-order) (EN) and [README.ru.md](../README.ru.md) (RU).
