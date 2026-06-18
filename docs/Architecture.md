# Architecture

OxiSentinel uses a split workspace so service code, UI code, deployment assets, and verification assets can evolve independently.

## Repository Lanes

- `source` contains the Rust service crate and command-line binaries.
- `ui` contains browser-facing TypeScript packages.
- `fuzz` contains cargo-fuzz targets that exercise stable Rust interfaces.
- `tests` contains integration tests, scripts, Docker helpers, and fixtures.
- `deploy` contains Helm and observability assets.

The root workspace files should stay focused on orchestration and shared policy.
