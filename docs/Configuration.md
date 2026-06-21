# Configuration

Runtime configuration examples live in `source/config`.

OxiSentinel loads typed TOML configuration from the path passed to
`oxisentinel --config PATH`. Without `--config`, the analyzer keeps the built-in
defaults for the analyzer identity and Admin API bind address.

OxiSentinel is configured and operated through its container image. Do not install `oxisentinelctl` on the host for supported deployments; execute it through the analyzer container instead:

```sh
docker exec -it oxisentinel oxisentinelctl health
```

## Condition And Judgment

`[condition]` declares side-effect-free expressions over normalized log records.
The evaluator uses `online-dsl-forge` in OxiRule-compatible mode. Every
condition can read a complete `Log` object:

- `Log.Schema`, `Log.Source`, `Log.Timestamp`, `Log.Level`, `Log.Service`,
  `Log.Message`
- `Log.Attributes`, containing normalized string attributes

OxiRule-style `Context`, `Request`, and `DynamicPolicy` roots are also
available on a best-effort basis from normalized attributes. Missing or invalid
runtime data fails closed for that condition and records a judgment error.

Rules may reference inline `when` expressions, `groups`, or an external `path`.
External condition files resolve under `condition.condition_dir`; absolute paths
and `.` or `..` path components are rejected.

`[judgment]` maps matched conditions to typed decision events. V1 judgment
actions are local decisions and callback intents:

```toml
[[judgment.handlers.actions]]
type = "emit_callback_intent"
target = "oxibelt_dynamic_policy"
operation = "apply"
dedupe_key = "example"
```

The analyzer records callback intents but does not directly mutate OxiBelt from
condition evaluation. Dispatching those intents to OxiBelt WAF or dynamic-policy
Admin APIs is a separate handler concern.

## Admin API And Control

The daemon exposes a small Admin surface on `[analyzer].bind_addr`:

- `GET /admin/v1/judgment/status`
- `POST /admin/v1/judgment/check`
- `POST /admin/v1/judgment/apply`
- `POST /admin/v1/judgment/import`
- `GET /admin/v1/judgment/decisions`

`status` returns the active generation and ETag in the form
`"oxisentinel-judgment-<generation>"`. `import` requires `If-Match` and returns
`428` when the header is missing or `412` when it is stale. `apply` treats
`If-Match` as optional but enforces it when supplied.

The matching control commands are:

```sh
docker exec -it oxisentinel oxisentinelctl judgment status
docker exec -it oxisentinel oxisentinelctl judgment check --config /etc/oxisentinel/oxisentinel.toml
docker exec -it oxisentinel oxisentinelctl judgment apply --config /etc/oxisentinel/oxisentinel.toml
```

Configuration should model analyzer behavior directly: collection sources,
access-log parsing, WAF event interpretation, dynamic policy analysis,
condition processing, judgment handling, redaction, retention, reporting, and
diagnostics.
