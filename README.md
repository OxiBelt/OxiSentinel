# OxiSentinel

OxiSentinel is a CLI and daemon-like Docker container analyzer for OxiBelt program logs, access logs, WAF events, and dynamic policy signals.

The repository root owns shared tooling and workspace configuration. Product code lives in `source`, fuzz targets live in `fuzz`, operational assets live under `deploy`, and integration coverage lives under `tests`.

OxiSentinel is expected to collect analyzer inputs from sources such as `docker logs`, `journalctl`, interprogram OpenAPI access, and access-log files or streams when available.

## Workspace Layout

- `source`: Rust analyzer library, daemon entrypoint, and CLI entrypoint.
- `fuzz`: cargo-fuzz package for parser and normalization targets.
- `docs`: architecture, configuration, and operations notes.
- `deploy`: Docker and Helm assets for running the analyzer daemon.
- `tests`: Rust integration tests, scripts, Docker helpers, and fixtures.

## Local Checks

```sh
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
```
