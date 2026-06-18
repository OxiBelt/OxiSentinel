# Tests

The test tree is split by responsibility:

- `rust`: integration and contract tests registered from `source/Cargo.toml`.
- `docker`: helper images for future integration tests.
- `fixtures`: deterministic test fixtures.

Rust parser and control-utility tests are part of:

```sh
cargo test --all-features --locked
```

Docker integration tests should build the OxiSentinel image first and exercise the in-container control path:

```sh
tests/docker/control-smoke.sh
tests/docker/parser-unit-smoke.sh
```

Future live journald integration tests should only run when Docker, the journald log driver, and `journalctl` are available. When any prerequisite is missing, tests must skip with a clear message instead of failing for host capability.
