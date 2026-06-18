# AGENTS.md

## Project Overview

OxiSentinel is a CLI and daemon-like Docker container analyzer for OxiBelt program logs, access logs, WAF events, and dynamic policy signals.

The main Rust analyzer implementation lives under `source/`. Tests live under `tests/`. Technical documentation lives under `docs/`. Container deployment and observability assets live under `deploy/`.

## Contributor Guidance

`CONTRIBUTING.md` is the source of truth for contributor workflow, repository layout, commit-message format, module organization, testing, privacy, and CI requirements.

Use these sections before making or reviewing changes:

- [Repository Layout](CONTRIBUTING.md#repository-layout)
- [Commit Messages](CONTRIBUTING.md#commit-messages)
- [Rust Module Organization](CONTRIBUTING.md#rust-module-organization)
- [Test Data And Privacy](CONTRIBUTING.md#test-data-and-privacy)
- [GitHub Actions](CONTRIBUTING.md#github-actions)

If this file and `CONTRIBUTING.md` diverge on workflow, testing, privacy, or commit-message requirements, follow `CONTRIBUTING.md`.
