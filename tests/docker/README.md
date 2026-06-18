# Docker Test Helpers

Place Docker helper images for integration tests here. Keep fixtures deterministic and avoid committing generated images or logs.

Container integration coverage should validate the shipped image surface:

- `/usr/local/bin/oxisentinel` remains the default entrypoint.
- `/usr/local/bin/oxisentinelctl` is available inside the image.
- Parser checks use `docker exec -i oxisentinel oxisentinelctl parse --source auto --input -`.
- One-shot checks use `docker run --rm -i --entrypoint /usr/local/bin/oxisentinelctl <image> parse --source auto --input -`.

Journald scenarios must detect Docker journald log-driver support and `journalctl` before running. Skip explicitly when those host capabilities are unavailable.

Smoke scripts:

- `tests/docker/parse-smoke.sh` builds the image, starts a named analyzer container, and verifies `docker exec -i ... oxisentinelctl parse`.
- `tests/docker/journald-smoke.sh` builds the image, runs the container with the journald log driver, and skips when Docker, journald, or visible journal records are unavailable.
