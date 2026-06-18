use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::provider::string_field;
use super::{NormalizedLogRecord, ParseSource};

pub(crate) fn infer_application_source(
  requested_source: &ParseSource,
  object: Option<&Map<String, Value>>,
  payload: &str,
) -> ParseSource {
  if !requested_source.is_auto() {
    return requested_source.clone();
  }

  let mut haystack = String::new();
  if let Some(object) = object
    && let Some(service) = string_field(object, APPLICATION_SOURCE_FIELDS)
  {
    haystack.push_str(&service);
    haystack.push(' ');
  }
  haystack.push_str(payload);

  let haystack = haystack.to_ascii_lowercase();

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

pub(crate) fn source_for_unstructured(requested_source: &ParseSource) -> ParseSource {
  match requested_source {
    ParseSource::Auto => ParseSource::DockerLogs,
    other => other.clone(),
  }
}

pub(crate) fn parse_vaultwarden_text(
  payload: &str,
  requested_source: &ParseSource,
) -> Option<NormalizedLogRecord> {
  let first = bracketed_prefix(payload)?;
  let rest = first.remainder.trim_start();
  let second = bracketed_prefix(rest)?;
  let rest = second.remainder.trim_start();
  let third = bracketed_prefix(rest)?;

  if !is_level_token(third.value) {
    return None;
  }

  let inferred_source = if requested_source.is_auto() {
    ParseSource::Vaultwarden
  } else {
    requested_source.clone()
  };
  let mut record = NormalizedLogRecord::new(&inferred_source, third.remainder.trim());
  record.timestamp = Some(first.value.to_owned());
  record.service = Some(second.value.to_owned());
  record.level = Some(normalize_level(third.value.to_owned()));
  Some(record)
}

pub(crate) fn parse_logfmt_text(
  payload: &str,
  source: &ParseSource,
) -> Option<NormalizedLogRecord> {
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

pub(crate) fn parse_plain_text(payload: &str, source: &ParseSource) -> NormalizedLogRecord {
  let (timestamp, rest) = split_rfc3339_prefix(payload);
  let (level, message) = split_level_prefix(rest);

  let mut record = NormalizedLogRecord::new(source, message.trim());
  record.timestamp = timestamp.map(str::to_owned);
  record.level = level.map(|value| normalize_level(value.to_owned()));
  record
}

pub(crate) fn normalize_level(level: String) -> String {
  match level.trim().to_ascii_lowercase().as_str() {
    "warn" => "warning".to_owned(),
    "err" => "error".to_owned(),
    "crit" => "critical".to_owned(),
    other => other.to_owned(),
  }
}

pub(crate) fn clean_message(message: &str) -> String {
  message.trim_end_matches(['\r', '\n']).to_owned()
}

pub(crate) fn clean_container_name(name: String) -> String {
  name.trim_start_matches('/').to_owned()
}

pub(crate) const APPLICATION_MESSAGE_FIELDS: &[&str] =
  &["message", "msg", "log", "event", "error", "description"];

pub(crate) const APPLICATION_TIMESTAMP_FIELDS: &[&str] =
  &["timestamp", "time", "ts", "@timestamp", "date"];

pub(crate) const APPLICATION_LEVEL_FIELDS: &[&str] = &["level", "lvl", "severity", "status"];

pub(crate) const APPLICATION_SOURCE_FIELDS: &[&str] = &[
  "service",
  "service_name",
  "program",
  "app",
  "application",
  "logger",
  "target",
  "component",
];

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
