use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{BufRead, Write};
use std::str::FromStr;

mod json;

use json::{JsonObject, JsonValue, parse_json_object, push_json_pair, push_json_string};

const NORMALIZED_SCHEMA: &str = "oxisentinel.log.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseSource {
  Auto,
  DockerLogs,
  DockerJournald,
  LinuxJournal,
  OxiBelt,
  Authelia,
  Ory,
  VoidAuth,
  Vaultwarden,
}

impl ParseSource {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Auto => "auto",
      Self::DockerLogs => "docker_logs",
      Self::DockerJournald => "docker_journald",
      Self::LinuxJournal => "linux_journal",
      Self::OxiBelt => "oxibelt",
      Self::Authelia => "authelia",
      Self::Ory => "ory",
      Self::VoidAuth => "voidauth",
      Self::Vaultwarden => "vaultwarden",
    }
  }

  pub const fn choices() -> &'static [&'static str] {
    &[
      "auto",
      "docker-logs",
      "docker_logs",
      "docker-journald",
      "docker_journald",
      "linux-journal",
      "linux_journal",
      "oxibelt",
      "authelia",
      "ory",
      "voidauth",
      "vaultwarden",
    ]
  }
}

impl FromStr for ParseSource {
  type Err = ParseError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    match value {
      "auto" => Ok(Self::Auto),
      "docker-logs" | "docker_logs" => Ok(Self::DockerLogs),
      "docker-journald" | "docker_journald" => Ok(Self::DockerJournald),
      "linux-journal" | "linux_journal" => Ok(Self::LinuxJournal),
      "oxibelt" => Ok(Self::OxiBelt),
      "authelia" => Ok(Self::Authelia),
      "ory" => Ok(Self::Ory),
      "voidauth" => Ok(Self::VoidAuth),
      "vaultwarden" => Ok(Self::Vaultwarden),
      other => Err(ParseError::UnknownSource(other.to_owned())),
    }
  }
}

#[derive(Debug)]
pub enum ParseError {
  Io(std::io::Error),
  Json { line: usize, reason: String },
  UnknownSource(String),
}

impl fmt::Display for ParseError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io(error) => write!(formatter, "I/O error: {error}"),
      Self::Json { line, reason } => {
        write!(formatter, "failed to parse JSON on line {line}: {reason}")
      }
      Self::UnknownSource(source) => write!(formatter, "unknown parse source: {source}"),
    }
  }
}

impl std::error::Error for ParseError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Io(error) => Some(error),
      Self::Json { .. } | Self::UnknownSource(_) => None,
    }
  }
}

impl From<std::io::Error> for ParseError {
  fn from(error: std::io::Error) -> Self {
    Self::Io(error)
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedLogRecord {
  pub schema: &'static str,
  pub source: &'static str,
  pub timestamp: Option<String>,
  pub level: Option<String>,
  pub service: Option<String>,
  pub message: String,
  pub attributes: BTreeMap<String, String>,
}

impl NormalizedLogRecord {
  fn new(source: ParseSource, message: impl Into<String>) -> Self {
    Self {
      schema: NORMALIZED_SCHEMA,
      source: source.as_str(),
      timestamp: None,
      level: None,
      service: None,
      message: message.into(),
      attributes: BTreeMap::new(),
    }
  }

  pub fn to_ndjson_line(&self) -> String {
    let mut output = String::new();

    output.push('{');
    push_json_pair(&mut output, "schema", self.schema, false);
    push_json_pair(&mut output, "source", self.source, true);

    if let Some(timestamp) = &self.timestamp {
      push_json_pair(&mut output, "timestamp", timestamp, true);
    }
    if let Some(level) = &self.level {
      push_json_pair(&mut output, "level", level, true);
    }
    if let Some(service) = &self.service {
      push_json_pair(&mut output, "service", service, true);
    }

    push_json_pair(&mut output, "message", &self.message, true);

    if !self.attributes.is_empty() {
      output.push_str(",\"attributes\":{");
      for (index, (key, value)) in self.attributes.iter().enumerate() {
        if index > 0 {
          output.push(',');
        }
        push_json_string(&mut output, key);
        output.push(':');
        push_json_string(&mut output, value);
      }
      output.push('}');
    }

    output.push('}');
    output
  }
}

pub fn parse_line(
  line: &str,
  requested_source: ParseSource,
) -> Result<Option<NormalizedLogRecord>, ParseError> {
  let line = line.trim_end_matches(['\r', '\n']);

  if line.trim().is_empty() {
    return Ok(None);
  }

  if let Some(record) = parse_json_line(line, requested_source, 1)? {
    return Ok(Some(record));
  }

  Ok(Some(parse_text_line(line, requested_source)))
}

pub fn parse_reader(
  reader: impl BufRead,
  writer: &mut impl Write,
  requested_source: ParseSource,
) -> Result<usize, ParseError> {
  let mut count = 0;

  for (index, line) in reader.lines().enumerate() {
    let line = line?;
    let line_number = index + 1;

    let record = match parse_json_line(&line, requested_source, line_number)? {
      Some(record) => Some(record),
      None if line.trim().is_empty() => None,
      None => Some(parse_text_line(&line, requested_source)),
    };

    if let Some(record) = record {
      writer.write_all(record.to_ndjson_line().as_bytes())?;
      writer.write_all(b"\n")?;
      count += 1;
    }
  }

  Ok(count)
}

fn parse_json_line(
  line: &str,
  requested_source: ParseSource,
  line_number: usize,
) -> Result<Option<NormalizedLogRecord>, ParseError> {
  let trimmed = line.trim_start();

  if !trimmed.starts_with('{') {
    return Ok(None);
  }

  let object = parse_json_object(trimmed).map_err(|reason| ParseError::Json {
    line: line_number,
    reason,
  })?;

  if object.contains_key("log")
    && (requested_source == ParseSource::DockerLogs
      || (object.contains_key("stream") && object.contains_key("time")))
  {
    return Ok(Some(normalize_docker_json(&object, requested_source)));
  }

  if object.contains_key("MESSAGE") || object.contains_key("__REALTIME_TIMESTAMP") {
    return Ok(Some(normalize_journal_json(&object, requested_source)));
  }

  Ok(Some(normalize_application_json(&object, requested_source)))
}

fn normalize_docker_json(
  object: &JsonObject,
  requested_source: ParseSource,
) -> NormalizedLogRecord {
  let payload = string_field(object, &["log"]).unwrap_or_default();
  let application_source = explicit_or_inferred_source(requested_source, object, &payload);
  let mut record = parse_payload_for_source(&payload, application_source);

  record.source = ParseSource::DockerLogs.as_str();
  record.timestamp = string_field(object, &["time"]).or(record.timestamp);
  record.attributes.insert(
    "input_source".to_owned(),
    ParseSource::DockerLogs.as_str().to_owned(),
  );

  let mut consumed = BTreeSet::from(["log".to_owned(), "time".to_owned(), "stream".to_owned()]);
  copy_selected_attributes(object, &mut record, &mut consumed);

  if let Some(stream) = string_field(object, &["stream"]) {
    record.attributes.insert("stream".to_owned(), stream);
  }

  record
}

fn normalize_journal_json(
  object: &JsonObject,
  requested_source: ParseSource,
) -> NormalizedLogRecord {
  let source = if requested_source != ParseSource::Auto {
    requested_source
  } else if has_any_key(object, &["CONTAINER_ID", "CONTAINER_NAME", "CONTAINER_TAG"]) {
    ParseSource::DockerJournald
  } else {
    ParseSource::LinuxJournal
  };

  let message = string_field(object, &["MESSAGE"]).unwrap_or_default();
  let mut record = NormalizedLogRecord::new(source, clean_message(&message));

  record.timestamp = string_field(
    object,
    &["__REALTIME_TIMESTAMP", "_SOURCE_REALTIME_TIMESTAMP"],
  );
  record.level = journal_priority(object)
    .or_else(|| string_field(object, &["PRIORITY"]))
    .map(normalize_level);
  record.service = string_field(
    object,
    &[
      "CONTAINER_NAME",
      "CONTAINER_TAG",
      "SYSLOG_IDENTIFIER",
      "_SYSTEMD_UNIT",
      "_COMM",
    ],
  )
  .map(clean_container_name);

  let mut consumed = BTreeSet::from([
    "MESSAGE".to_owned(),
    "__REALTIME_TIMESTAMP".to_owned(),
    "_SOURCE_REALTIME_TIMESTAMP".to_owned(),
    "PRIORITY".to_owned(),
    "CONTAINER_NAME".to_owned(),
    "CONTAINER_TAG".to_owned(),
    "SYSLOG_IDENTIFIER".to_owned(),
    "_SYSTEMD_UNIT".to_owned(),
    "_COMM".to_owned(),
  ]);

  copy_selected_attributes(object, &mut record, &mut consumed);
  record
}

fn normalize_application_json(
  object: &JsonObject,
  requested_source: ParseSource,
) -> NormalizedLogRecord {
  let source = explicit_or_inferred_source(requested_source, object, "");
  let message = string_field(
    object,
    &["message", "msg", "log", "event", "error", "description"],
  )
  .unwrap_or_default();
  let mut record = NormalizedLogRecord::new(source, clean_message(&message));

  record.timestamp = string_field(object, &["timestamp", "time", "ts", "@timestamp", "date"]);
  record.level = string_field(object, &["level", "lvl", "severity", "status"]).map(normalize_level);
  record.service = string_field(
    object,
    &[
      "service",
      "service_name",
      "program",
      "app",
      "application",
      "logger",
      "target",
      "component",
    ],
  );

  let mut consumed = BTreeSet::from([
    "message".to_owned(),
    "msg".to_owned(),
    "log".to_owned(),
    "event".to_owned(),
    "error".to_owned(),
    "description".to_owned(),
    "timestamp".to_owned(),
    "time".to_owned(),
    "ts".to_owned(),
    "@timestamp".to_owned(),
    "date".to_owned(),
    "level".to_owned(),
    "lvl".to_owned(),
    "severity".to_owned(),
    "status".to_owned(),
    "service".to_owned(),
    "service_name".to_owned(),
    "program".to_owned(),
    "app".to_owned(),
    "application".to_owned(),
    "logger".to_owned(),
    "target".to_owned(),
    "component".to_owned(),
  ]);

  copy_selected_attributes(object, &mut record, &mut consumed);
  record
}

fn parse_text_line(line: &str, requested_source: ParseSource) -> NormalizedLogRecord {
  parse_payload_for_source(line, requested_source)
}

fn parse_payload_for_source(payload: &str, source: ParseSource) -> NormalizedLogRecord {
  let payload = payload.trim_end_matches(['\r', '\n']);

  if let Some(record) = parse_json_payload(payload, source) {
    return record;
  }

  if let Some(record) = parse_vaultwarden_text(payload, source) {
    return record;
  }

  let fallback_source = source_for_unstructured(source);

  if let Some(record) = parse_logfmt_text(payload, fallback_source) {
    return record;
  }

  parse_plain_text(payload, fallback_source)
}

fn parse_json_payload(payload: &str, source: ParseSource) -> Option<NormalizedLogRecord> {
  if !payload.trim_start().starts_with('{') {
    return None;
  }

  let object = parse_json_object(payload).ok()?;
  Some(normalize_application_json(&object, source))
}

fn parse_vaultwarden_text(payload: &str, source: ParseSource) -> Option<NormalizedLogRecord> {
  let first = bracketed_prefix(payload)?;
  let rest = first.remainder.trim_start();
  let second = bracketed_prefix(rest)?;
  let rest = second.remainder.trim_start();
  let third = bracketed_prefix(rest)?;

  if !is_level_token(third.value) {
    return None;
  }

  let inferred_source = if source == ParseSource::Auto {
    ParseSource::Vaultwarden
  } else {
    source
  };
  let mut record = NormalizedLogRecord::new(inferred_source, third.remainder.trim());
  record.timestamp = Some(first.value.to_owned());
  record.service = Some(second.value.to_owned());
  record.level = Some(normalize_level(third.value.to_owned()));
  Some(record)
}

fn parse_logfmt_text(payload: &str, source: ParseSource) -> Option<NormalizedLogRecord> {
  let fields = parse_logfmt(payload);

  if !(fields.contains_key("msg")
    || fields.contains_key("message")
    || fields.contains_key("level")
    || fields.contains_key("time"))
  {
    return None;
  }

  let mut record = NormalizedLogRecord::new(
    source,
    fields
      .get("msg")
      .or_else(|| fields.get("message"))
      .cloned()
      .unwrap_or_else(|| payload.to_owned()),
  );
  record.timestamp = fields
    .get("time")
    .or_else(|| fields.get("timestamp"))
    .or_else(|| fields.get("ts"))
    .cloned();
  record.level = fields
    .get("level")
    .or_else(|| fields.get("lvl"))
    .cloned()
    .map(normalize_level);
  record.service = fields
    .get("service")
    .or_else(|| fields.get("service_name"))
    .or_else(|| fields.get("app"))
    .cloned();

  for (key, value) in fields {
    if matches!(
      key.as_str(),
      "msg"
        | "message"
        | "time"
        | "timestamp"
        | "ts"
        | "level"
        | "lvl"
        | "service"
        | "service_name"
        | "app"
    ) {
      continue;
    }
    record.attributes.insert(key, value);
  }

  Some(record)
}

fn parse_plain_text(payload: &str, source: ParseSource) -> NormalizedLogRecord {
  let (timestamp, rest) = split_rfc3339_prefix(payload);
  let (level, message) = split_level_prefix(rest);

  let mut record = NormalizedLogRecord::new(source, message.trim());
  record.timestamp = timestamp.map(str::to_owned);
  record.level = level.map(|value| normalize_level(value.to_owned()));
  record
}

fn explicit_or_inferred_source(
  requested_source: ParseSource,
  object: &JsonObject,
  payload: &str,
) -> ParseSource {
  if requested_source != ParseSource::Auto {
    return requested_source;
  }

  let haystack = [
    string_field(
      object,
      &[
        "service",
        "service_name",
        "program",
        "app",
        "application",
        "logger",
        "target",
        "component",
      ],
    )
    .unwrap_or_default(),
    payload.to_owned(),
  ]
  .join(" ")
  .to_ascii_lowercase();

  if haystack.contains("oxibelt") {
    ParseSource::OxiBelt
  } else if haystack.contains("authelia") {
    ParseSource::Authelia
  } else if haystack.contains("ory") || haystack.contains("kratos") || haystack.contains("hydra") {
    ParseSource::Ory
  } else if haystack.contains("voidauth") {
    ParseSource::VoidAuth
  } else if haystack.contains("vaultwarden") || haystack.contains("bitwarden") {
    ParseSource::Vaultwarden
  } else {
    ParseSource::DockerLogs
  }
}

fn source_for_unstructured(requested_source: ParseSource) -> ParseSource {
  match requested_source {
    ParseSource::Auto => ParseSource::DockerLogs,
    other => other,
  }
}

fn copy_selected_attributes(
  object: &JsonObject,
  record: &mut NormalizedLogRecord,
  consumed: &mut BTreeSet<String>,
) {
  for (key, value) in object {
    if consumed.contains(key) {
      continue;
    }

    if let Some(string_value) = value_to_attribute(value) {
      record.attributes.insert(key.clone(), string_value);
    }
  }
}

fn has_any_key(object: &JsonObject, keys: &[&str]) -> bool {
  keys.iter().any(|key| object.contains_key(*key))
}

fn string_field(object: &JsonObject, keys: &[&str]) -> Option<String> {
  keys
    .iter()
    .find_map(|key| object.get(*key).and_then(value_to_attribute))
    .filter(|value| !value.is_empty())
}

fn value_to_attribute(value: &JsonValue) -> Option<String> {
  match value {
    JsonValue::Null => None,
    JsonValue::Bool(value) => Some(value.to_string()),
    JsonValue::Number(value) => Some(value.clone()),
    JsonValue::String(value) => Some(value.clone()),
    JsonValue::Array(_) | JsonValue::Object(_) => Some(value.to_compact_json()),
  }
}

fn journal_priority(object: &JsonObject) -> Option<String> {
  match string_field(object, &["PRIORITY"])?.as_str() {
    "0" => Some("emergency".to_owned()),
    "1" => Some("alert".to_owned()),
    "2" => Some("critical".to_owned()),
    "3" => Some("error".to_owned()),
    "4" => Some("warning".to_owned()),
    "5" => Some("notice".to_owned()),
    "6" => Some("info".to_owned()),
    "7" => Some("debug".to_owned()),
    _ => None,
  }
}

fn normalize_level(level: String) -> String {
  match level.trim().to_ascii_lowercase().as_str() {
    "warn" => "warning".to_owned(),
    "err" => "error".to_owned(),
    "crit" => "critical".to_owned(),
    other => other.to_owned(),
  }
}

fn clean_message(message: &str) -> String {
  message.trim_end_matches(['\r', '\n']).to_owned()
}

fn clean_container_name(name: String) -> String {
  name.trim_start_matches('/').to_owned()
}

struct BracketedPrefix<'a> {
  value: &'a str,
  remainder: &'a str,
}

fn bracketed_prefix(value: &str) -> Option<BracketedPrefix<'_>> {
  let value = value.strip_prefix('[')?;
  let closing = value.find(']')?;

  Some(BracketedPrefix {
    value: &value[..closing],
    remainder: &value[(closing + 1)..],
  })
}

fn parse_logfmt(payload: &str) -> BTreeMap<String, String> {
  let mut fields = BTreeMap::new();
  let bytes = payload.as_bytes();
  let mut index = 0;

  while index < bytes.len() {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
      index += 1;
    }

    let key_start = index;
    while index < bytes.len() && bytes[index] != b'=' && !bytes[index].is_ascii_whitespace() {
      index += 1;
    }

    if index >= bytes.len() || bytes[index] != b'=' || key_start == index {
      break;
    }

    let key = &payload[key_start..index];
    index += 1;

    let value = if index < bytes.len() && bytes[index] == b'"' {
      index += 1;
      let value_start = index;
      let mut escaped = false;

      while index < bytes.len() {
        if escaped {
          escaped = false;
        } else if bytes[index] == b'\\' {
          escaped = true;
        } else if bytes[index] == b'"' {
          break;
        }
        index += 1;
      }

      let value = payload[value_start..index].replace("\\\"", "\"");
      if index < bytes.len() {
        index += 1;
      }
      value
    } else {
      let value_start = index;
      while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
        index += 1;
      }
      payload[value_start..index].to_owned()
    };

    fields.insert(key.to_owned(), value);
  }

  fields
}

fn split_rfc3339_prefix(payload: &str) -> (Option<&str>, &str) {
  let Some((timestamp, rest)) = payload.split_once(char::is_whitespace) else {
    return (None, payload);
  };

  if is_rfc3339_like(timestamp) {
    (Some(timestamp), rest.trim_start())
  } else {
    (None, payload)
  }
}

fn split_level_prefix(payload: &str) -> (Option<&str>, &str) {
  let trimmed = payload.trim_start();
  let Some((level, message)) = trimmed.split_once(char::is_whitespace) else {
    return (None, payload);
  };

  let level = level.trim_matches(['[', ']', ':']);

  if is_level_token(level) {
    (Some(level), message)
  } else {
    (None, payload)
  }
}

fn is_rfc3339_like(value: &str) -> bool {
  let bytes = value.as_bytes();

  bytes.len() >= 20
    && bytes.get(4) == Some(&b'-')
    && bytes.get(7) == Some(&b'-')
    && matches!(bytes.get(10), Some(b'T') | Some(b't') | Some(b' '))
    && bytes.get(13) == Some(&b':')
    && bytes.get(16) == Some(&b':')
}

fn is_level_token(value: &str) -> bool {
  matches!(
    value.to_ascii_lowercase().as_str(),
    "trace"
      | "debug"
      | "info"
      | "notice"
      | "warn"
      | "warning"
      | "error"
      | "err"
      | "critical"
      | "crit"
      | "fatal"
  )
}

#[cfg(test)]
mod tests;
