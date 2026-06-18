# Contributing

Keep changes scoped to one workspace lane whenever possible.

Before opening a pull request, run the checks that match the files you changed:

- Rust: `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --all-features --locked`.
- TypeScript: `npm run typecheck`, `npm run lint`, and `npm run build`.
- Deploy or docs: validate generated manifests or examples with the smallest relevant command.

Do not commit generated build output, local dependency directories, fuzz artifacts, or temporary test files.
