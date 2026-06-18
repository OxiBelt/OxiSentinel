# Docker Test Helpers

Place Docker helper images for integration tests here. Keep fixtures deterministic and avoid committing generated images or logs.

Container integration coverage should validate the shipped image surface:

- `/usr/local/bin/oxisentinel` remains the default entrypoint.
- `/usr/local/bin/oxisentinelctl` is available inside the image.
- `oxisentinelctl health` works through `docker exec`.
- Parser internals are not exposed as a runtime-image control command.

Future live journald scenarios must detect Docker journald log-driver support and `journalctl` before running. Skip explicitly when those host capabilities are unavailable.

Smoke scripts:

- `tests/docker/control-smoke.sh` builds the image, starts a named analyzer container, verifies `oxisentinelctl health`, and rejects `oxisentinelctl parse`.
- `tests/docker/parser-unit-smoke.sh` builds the `parser-tests` Dockerfile target, which runs Rust parser tests inside Docker without exposing a parser control command.
