# OxiSentinel

OxiSentinel is organized as a Rust and TypeScript monorepo.

The repository root owns shared tooling and workspace configuration. Product runtime code lives in `source`, fuzz targets live in `fuzz`, browser-facing TypeScript packages live under `ui`, and operational assets live under `deploy`.

## Workspace Layout

- `source`: Rust service, library, and command-line binaries.
- `fuzz`: cargo-fuzz package for protocol and parser fuzz targets.
- `ui`: TypeScript workspace packages.
- `docs`: architecture, configuration, and operator notes.
- `deploy`: Helm and observability assets.
- `tests`: Rust integration tests, scripts, Docker helpers, and fixtures.
- `kernel-extension`: system-level install and verification assets.

## Local Checks

```sh
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
npm run typecheck
npm run lint
npm run build
```
