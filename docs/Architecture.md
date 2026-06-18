# Architecture

OxiSentinel analyzes program logs and access logs emitted by OxiBelt's WAF and dynamic policy system. It is distributed as a CLI and as a daemon-like Docker container for continuous collection and analysis.

Supported collection lanes should stay explicit:

- `docker logs` for containerized OxiBelt deployments.
- `journalctl` for systemd-managed OxiBelt deployments.
- Interprogram OpenAPI access for structured runtime data.
- Access-log files or streams where operators expose them.

## Repository Lanes

- `source` contains the Rust analyzer crate, daemon entrypoint, and CLI entrypoint.
- `fuzz` contains cargo-fuzz targets for parser and normalization boundaries.
- `tests` contains integration tests, scripts, Docker helpers, and fixtures.
- `deploy` contains container, Helm, and observability assets.

The root workspace files should stay focused on orchestration and shared policy.
