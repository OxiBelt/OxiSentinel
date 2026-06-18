# OxiSentinel

OxiSentinel is a container-only analyzer for OxiBelt program logs, access logs, WAF events, and dynamic policy signals.

The repository root owns shared tooling and workspace configuration. Product code lives in `source`, fuzz targets live in `fuzz`, operational assets live under `deploy`, and integration coverage lives under `tests`.

OxiSentinel is expected to collect analyzer inputs from sources such as `docker logs`, `journalctl`, interprogram OpenAPI access, and access-log files or streams when available. The runtime image includes both the long-running analyzer and the control utility:

- `/usr/local/bin/oxisentinel`: default image entrypoint for the analyzer runtime.
- `/usr/local/bin/oxisentinelctl`: control and diagnostic utility.

Operators should run the analyzer container with the stable name `oxisentinel` unless their deployment system owns naming.

```sh
docker exec -it oxisentinel oxisentinelctl health
```

The internal parser normalizes Docker JSON log records, Docker journald records, Linux journal JSON, OxiBelt Admin API response records, and OxiBelt, Authelia, Ory, VoidAuth, and Vaultwarden log input. Parser internals are intentionally not exposed as a supported operator command yet.

## Workspace Layout

- `source`: Rust analyzer library, daemon entrypoint, parser modules, and control entrypoint.
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
