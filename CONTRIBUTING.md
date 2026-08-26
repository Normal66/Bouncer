# Contributing

Thank you for your interest in CaddyBan.

## How to contribute

1. **Do not maintain long-lived public forks** for independent development.
2. Open an **issue** first for non-trivial changes.
3. Send a **pull request** to the official upstream repository only.
4. Keep changes focused and minimal (KISS).

## Pull requests

- One logical change per PR when possible.
- Include a short description and test notes.
- Run locally before submitting:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## License of contributions

By submitting a pull request, you confirm that you have the right to submit
the work and you grant the maintainers permission to include it under the
project license (see [LICENSE](LICENSE)).
