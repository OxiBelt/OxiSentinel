# Contributing

OxiSentinel is a CLI and daemon-like Docker container analyzer for OxiBelt program logs, access logs, WAF events, and dynamic policy signals.

Treat changes to log collection, access-log schemas, parsing, normalization, WAF interpretation, dynamic policy analysis, redaction, retention, reporting, OpenAPI access, and CI as review-sensitive. Use root-relative paths in documentation, scripts, issues, and pull request notes.

## Repository Layout

Generated and local-only directories such as `target/`, `source/target/`, `fuzz/artifacts/`, `fuzz/corpus/`, and `tests/.tmp/` are not source contributions and should not be committed.

| Path | Purpose | Change here when |
| --- | --- | --- |
| `source/` | Main Rust analyzer crate. | You are changing analyzer runtime, CLI, daemon, parsing, reporting, configuration, or diagnostics behavior. |
| `source/src/bin/` | Command entrypoints. | You are changing `oxisentinel` daemon startup or `oxisentinelctl` CLI behavior. |
| `source/src/config/` | Typed configuration modules. | You are adding or changing analyzer configuration syntax, defaults, validation, or compatibility. |
| `source/config/` | Example runtime configuration. | User-visible examples need to stay valid. |
| `source/ops/` | Container image and runtime packaging assets. | Docker image layout or runtime packaging changes. |
| `fuzz/` | Fuzz targets for parser and normalization boundaries. | Input handling, parsers, or normalization behavior changes. |
| `deploy/` | Helm and observability assets for daemon-style deployments. | Container deployment, metrics, dashboards, Prometheus, or collector starter assets change. |
| `tests/rust/` | Rust integration tests and repository-level checks. | Behavior changes need regression coverage. |
| `tests/docker/` | Docker-only fixtures and helper images. | Containerized collection or integration scenarios need fixtures. |
| `tests/fixtures/` | Deterministic test fixtures. | Tests need stable sample logs, access-log records, or expected reports. |
| `docs/` | Architecture, configuration, and operations guidance. | User-visible behavior, syntax, compatibility, or operations guidance changes. |
| `.github/workflows/` | GitHub Actions workflows. | CI job structure, matrices, or required checks change. |

## Contribution Workflow

1. Identify the affected area before editing: collectors, parsers, normalization, WAF analysis, dynamic policy analysis, reporting, redaction, configuration, CLI, daemon runtime, Docker packaging, tests, deploy assets, or documentation.
2. Make the smallest reasonable change for the behavior being changed.
3. Add or update tests when analyzer behavior, parsing, reporting, configuration, runtime, Docker, or CI behavior changes.
4. Update documentation when behavior, configuration syntax, commands, operations guidance, or CI workflows change.
5. Run the relevant checks and mention any checks that could not be run.
6. Verify that generated logs, reports, databases, and temporary test data are not committed.

Use workspace-level Rust commands from the repository root:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-features --locked
```

For deploy or docs changes, validate the changed manifest, example, or command with the smallest relevant check.

## Commit Messages

Use Conventional Commits:

```text
<type>(<scope>): <subject>
```

- `type` must be one of `feat`, `fix`, `chore`, `docs`, `ci`, `refactor`, `security`, `tests`, or `perf`.
- `scope` is the area or responsibility touched, such as `collector`, `docker_logs`, `journald`, `openapi`, `access_log`, `waf`, `dynamic_policy`, `redaction`, `reporting`, `config`, `workflows`, or `docs`.
- `subject` is a short imperative summary. Use a present-tense verb.
- In commit titles and detailed descriptions, wrap paths, commands, configuration keys, log field names, function names, variable names, type names, module names, and literal values in Markdown code spans.

Valid examples:

```text
feat(docker_logs): add `docker logs` collector scaffold
fix(access_log): reject records without `request_id`
security(redaction): mask `Authorization` values in reports
ci(workflows): pin `actions/checkout` to a commit SHA
```

Avoid past-tense subjects such as `added parser tests` or unformatted identifiers such as `update AnalyzerConfig`.

## Rust Module Organization

Do not place unrelated functionality in an existing Rust source file only because the file already exists. Add a responsibility-focused module when a new feature has a distinct concern.

Keep module boundaries explicit:

- Docker log collection belongs in collector-focused modules.
- `journalctl` collection belongs in journald-focused modules.
- Interprogram OpenAPI access belongs in OpenAPI client modules.
- Access-log parsing belongs in parser-focused modules.
- WAF event interpretation belongs in WAF analysis modules.
- Dynamic policy interpretation belongs in dynamic policy analysis modules.
- Redaction and privacy behavior belongs in redaction-focused modules.
- Report rendering belongs in reporting modules.
- Configuration parsing and validation belongs in configuration modules.

Treat 750 lines as the review threshold for Rust source files under `source/src/`. Files above that threshold should be split into smaller responsibility-focused modules unless there is a documented reason to keep the implementation together.

When adding a new Rust file or module, choose a responsibility-focused name, add tests for new behavior, update technical documentation when behavior is user-visible, and avoid generic utility modules unless the shared responsibility is clear.

## Test Data And Privacy

OxiSentinel handles operational log data. Do not commit real customer logs, private operational logs, API tokens, credentials, generated reports, local databases, temporary configs, raw `docker logs` captures, raw journal exports, or private OpenAPI responses.

Tests may use synthetic logs and fixtures. Keep those fixtures minimal, deterministic, and free of real identifiers. Prefer generated temporary directories over fixed paths inside the repository, and clean up test-created files where practical.

## Docker And Deploy Changes

Docker-based tests and deploy assets should be reproducible locally and in GitHub Actions. Avoid hidden host dependencies, avoid local-only absolute paths, keep image and chart behavior deterministic, and document any required external services.

OxiSentinel does not rely on host tuning features as part of its scaffold. Collection should be modeled through analyzer inputs such as `docker logs`, `journalctl`, interprogram OpenAPI access, and access-log files or streams.

## GitHub Actions

Reusable GitHub Actions must be pinned by commit SHA, with an optional comment naming the source tag. Do not use mutable tags such as `@v4` or `@v6` in workflow `uses:` entries.

Keep CI focused on checks that match the current repository surface. Do not add custom repository-layout scripts unless they check real behavior that cannot be covered by build, test, lint, or manifest validation.

## Pull Request Checklist

- The change is scoped to the affected analyzer area.
- Relevant Rust checks pass, or skipped checks are called out.
- Tests or fixtures cover behavior changes.
- Documentation is updated for user-visible behavior.
- Generated artifacts and private operational data are not committed.
- GitHub Actions use commit SHA pins for reusable actions.
