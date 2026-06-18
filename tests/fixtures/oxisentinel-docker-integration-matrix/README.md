# OxiSentinel Docker Integration Matrix

Future Docker integration scenarios should live under this directory, grouped by behavior under test.

Required internal parser fixture scenarios:

- Docker JSON log driver input.
- Docker journald driver input when Docker, journald, and `journalctl` are available.
- Linux journal JSON input.
- OxiBelt Admin API response input.
- OxiBelt, Authelia, Ory, VoidAuth, and Vaultwarden application input.

Multi-arch image scenarios should cover artifact suffixes and target validation for `linux/amd64` with `x86-64-v2`, `x86-64-v3`, and `x86-64-v4`, `linux/arm64` generic, and `linux/riscv64` using `riscv64gc-unknown-linux-musl`.
