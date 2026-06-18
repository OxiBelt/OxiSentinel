# Architecture

OxiSentinel analyzes program logs and access logs emitted by OxiBelt's WAF and dynamic policy system. It is distributed as a Docker image only. The image contains the analyzer runtime and the `oxisentinelctl` control utility; host installation of `oxisentinelctl` is not part of the supported model.

Supported collection lanes should stay explicit:

- `docker logs` for containerized OxiBelt deployments.
- `journalctl` for systemd-managed OxiBelt deployments.
- Interprogram OpenAPI access for structured runtime data.
- Access-log files or streams where operators expose them.

## Container Runtime

The runtime image installs two binaries:

- `/usr/local/bin/oxisentinel`: the image entrypoint and long-running analyzer process.
- `/usr/local/bin/oxisentinelctl`: control, diagnostics, and parser utility executed inside a running analyzer container or as a one-shot utility container.

Use `--name oxisentinel` for the running analyzer container when possible. Interactive control commands should use:

```sh
docker exec -it oxisentinel oxisentinelctl health
```

Scripted stdin parsing should use:

```sh
docker exec -i oxisentinel oxisentinelctl parse --source auto --input -
```

One-shot parsing without a long-running container should use:

```sh
docker run --rm -i --entrypoint /usr/local/bin/oxisentinelctl oxisentinel:latest parse --source auto --input -
```

`oxisentinelctl parse` is the primary parser entrypoint. It normalizes Docker JSON log driver records, Docker journald records, Linux journal JSON, OxiBelt, Authelia, Ory, VoidAuth, and Vaultwarden input to NDJSON records using the `oxisentinel.log.v1` schema.

## Image Targets

The image build contract supports:

- `linux/amd64` with `x86-64-v2`, `x86-64-v3`, or `x86-64-v4` target CPU levels.
- `linux/arm64` generic builds using `aarch64-unknown-linux-musl`.
- `linux/riscv64` generic builds using `riscv64gc-unknown-linux-musl`.

Use `source/ops/build-image.sh` to validate platform and CPU combinations before invoking `docker buildx`.

## Repository Lanes

- `source` contains the Rust analyzer crate, daemon entrypoint, parser modules, image target metadata, and control entrypoint.
- `fuzz` contains cargo-fuzz targets for parser and normalization boundaries.
- `tests` contains integration tests, scripts, Docker helpers, and fixtures.
- `deploy` contains container, Helm, and observability assets.

The root workspace files should stay focused on orchestration and shared policy.
