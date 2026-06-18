use super::{NormalizedLogRecord, ParseSource, parse_line, parse_reader};

#[test]
fn parses_docker_json_log_records() {
  let record = parse_one(
    r#"{"log":"{\"level\":\"INFO\",\"service\":\"oxibelt\",\"msg\":\"allowed\"}\n","stream":"stdout","time":"2026-06-18T10:00:00.000000000Z"}"#,
    ParseSource::Auto,
  );

  assert_eq!(record.source, "docker_logs");
  assert_eq!(
    record.timestamp.as_deref(),
    Some("2026-06-18T10:00:00.000000000Z")
  );
  assert_eq!(record.level.as_deref(), Some("info"));
  assert_eq!(record.service.as_deref(), Some("oxibelt"));
  assert_eq!(record.message, "allowed");
  assert_eq!(
    record.attributes.get("stream").map(String::as_str),
    Some("stdout")
  );
}

#[test]
fn parses_linux_journal_json_records() {
  let record = parse_one(
    r#"{"__REALTIME_TIMESTAMP":"1781786400000000","SYSLOG_IDENTIFIER":"sshd","PRIORITY":"4","MESSAGE":"login refused"}"#,
    ParseSource::Auto,
  );

  assert_eq!(record.source, "linux_journal");
  assert_eq!(record.timestamp.as_deref(), Some("1781786400000000"));
  assert_eq!(record.level.as_deref(), Some("warning"));
  assert_eq!(record.service.as_deref(), Some("sshd"));
  assert_eq!(record.message, "login refused");
}

#[test]
fn parses_docker_journald_records() {
  let record = parse_one(
    r#"{"__REALTIME_TIMESTAMP":"1781786400000000","CONTAINER_NAME":"/oxisentinel","PRIORITY":"6","MESSAGE":"ready"}"#,
    ParseSource::Auto,
  );

  assert_eq!(record.source, "docker_journald");
  assert_eq!(record.service.as_deref(), Some("oxisentinel"));
  assert_eq!(record.level.as_deref(), Some("info"));
  assert_eq!(record.message, "ready");
}

#[test]
fn parses_supported_application_json_sources() {
  let oxibelt = parse_one(
    r#"{"time":"2026-06-18T10:00:00Z","level":"warn","service":"oxibelt","message":"policy denied","request_id":"req-1"}"#,
    ParseSource::Auto,
  );
  let authelia = parse_one(
    r#"{"time":"2026-06-18T10:00:01Z","level":"info","service":"authelia","msg":"Access granted"}"#,
    ParseSource::Auto,
  );
  let ory = parse_one(
    r#"{"time":"2026-06-18T10:00:02Z","level":"error","service_name":"Ory Kratos","msg":"identity lookup failed"}"#,
    ParseSource::Auto,
  );

  assert_eq!(oxibelt.source, "oxibelt");
  assert_eq!(oxibelt.level.as_deref(), Some("warning"));
  assert_eq!(
    oxibelt.attributes.get("request_id").map(String::as_str),
    Some("req-1")
  );
  assert_eq!(authelia.source, "authelia");
  assert_eq!(ory.source, "ory");
}

#[test]
fn explicit_sources_normalize_text_formats() {
  let voidauth = parse_one(
    r#"time="2026-06-18T10:00:03Z" level=info msg="token accepted" subject=user-1"#,
    ParseSource::VoidAuth,
  );
  let vaultwarden = parse_one(
    "[2026-06-18 10:00:04.000][vaultwarden::api][INFO] request complete",
    ParseSource::Auto,
  );

  assert_eq!(voidauth.source, "voidauth");
  assert_eq!(voidauth.message, "token accepted");
  assert_eq!(
    voidauth.attributes.get("subject").map(String::as_str),
    Some("user-1")
  );
  assert_eq!(vaultwarden.source, "vaultwarden");
  assert_eq!(vaultwarden.service.as_deref(), Some("vaultwarden::api"));
  assert_eq!(vaultwarden.message, "request complete");
}

#[test]
fn parse_reader_emits_normalized_ndjson() {
  let mut output = Vec::new();
  let count = parse_reader(
    "2026-06-18T10:00:05Z INFO daemon ready\n\n".as_bytes(),
    &mut output,
    ParseSource::DockerLogs,
  )
  .expect("reader parses");

  let output = String::from_utf8(output).expect("output is utf-8");

  assert_eq!(count, 1);
  assert!(output.contains(r#""schema":"oxisentinel.log.v1""#));
  assert!(output.contains(r#""source":"docker_logs""#));
  assert!(output.contains(r#""message":"daemon ready""#));
}

fn parse_one(line: &str, source: ParseSource) -> NormalizedLogRecord {
  parse_line(line, source)
    .expect("line parses")
    .expect("line normalizes")
}
