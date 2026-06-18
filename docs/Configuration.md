# Configuration

Runtime configuration examples live in `source/config`.

The initial scaffold defines the default analyzer identity and bind address in Rust code. File-backed configuration should be introduced with a typed parser and tests before production deployment.

Configuration should model analyzer behavior directly: collection sources, access-log parsing, WAF event interpretation, dynamic policy analysis, redaction, retention, reporting, and diagnostics.
