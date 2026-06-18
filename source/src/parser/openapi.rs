use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::provider::{
  LogSourceProvider, ParseContext, SourceDescriptor, SourceLane, copy_selected_attributes,
  string_field, value_to_attribute,
};
use super::text::{clean_message, normalize_level};
use super::{NormalizedLogRecord, ParseSource};

const OPENAPI_DESCRIPTORS: &[SourceDescriptor] = &[SourceDescriptor {
  canonical: "openapi",
  aliases: &["openapi", "open-api", "admin-openapi"],
  lane: SourceLane::OpenApi,
}];

const ARRAY_COLLECTION_FIELDS: &[&str] = &[
  "audit",
  "events",
  "operations",
  "ipm_audit",
  "dynamic_policy_audit",
  "dynamic_policy",
  "dynamic_policies",
];

pub(crate) struct OpenApiProvider;

impl LogSourceProvider for OpenApiProvider {
  fn descriptors(&self) -> &'static [SourceDescriptor] {
    OPENAPI_DESCRIPTORS
  }

  fn normalize_json(
    &self,
    object: &Map<String, Value>,
    context: &ParseContext<'_>,
  ) -> Option<Vec<NormalizedLogRecord>> {
    if !matches!(
      context.requested_source,
      ParseSource::OpenApi | ParseSource::Auto
    ) && !looks_like_openapi_record(object)
    {
      return None;
    }

    if let Some(records) = expand_collection_response(object) {
      return Some(records);
    }

    if matches!(context.requested_source, ParseSource::OpenApi) || looks_like_openapi_record(object)
    {
      return Some(vec![normalize_openapi_object(object, None)]);
    }

    None
  }
}

fn expand_collection_response(object: &Map<String, Value>) -> Option<Vec<NormalizedLogRecord>> {
  for field in ARRAY_COLLECTION_FIELDS {
    let Some(Value::Array(values)) = object.get(*field) else {
      continue;
    };

    let mut records = Vec::new();
    for value in values {
      if let Value::Object(item) = value {
        records.push(normalize_openapi_object(item, Some(field)));
      }
    }

    if !records.is_empty() {
      return Some(records);
    }
  }

  None
}

fn normalize_openapi_object(
  object: &Map<String, Value>,
  collection: Option<&str>,
) -> NormalizedLogRecord {
  let message = openapi_message(object);
  let mut record = NormalizedLogRecord::new_source("openapi", message);
  record.timestamp = openapi_timestamp(object);
  record.level = openapi_level(object);
  record.service = string_field(object, &["service"])
    .or_else(|| collection.map(|value| value.trim_end_matches("_audit").to_owned()))
    .or_else(|| Some("admin_api".to_owned()));

  if let Some(collection) = collection {
    record
      .attributes
      .insert("api_collection".to_owned(), collection.to_owned());
  }

  if let Some(operation) = object.get("operation").and_then(Value::as_object) {
    if let Some(kind) = string_field(operation, &["kind"]) {
      record.attributes.insert("operation_kind".to_owned(), kind);
    }
    if let Some(state) = string_field(operation, &["state"]) {
      record
        .attributes
        .insert("operation_state".to_owned(), state);
    }
    if let Some(request_id) = string_field(operation, &["request_id"]) {
      record
        .attributes
        .insert("request_id".to_owned(), request_id);
    }
  }

  let mut consumed = BTreeSet::from([
    "message".to_owned(),
    "summary".to_owned(),
    "event".to_owned(),
    "operation".to_owned(),
    "outcome".to_owned(),
    "action".to_owned(),
    "error".to_owned(),
    "timestamp".to_owned(),
    "created_at".to_owned(),
    "updated_at".to_owned(),
    "time".to_owned(),
    "created_at_unix_ms".to_owned(),
    "timestamp_unix_ms".to_owned(),
    "level".to_owned(),
    "status".to_owned(),
    "service".to_owned(),
  ]);
  copy_selected_attributes(object, &mut record, &mut consumed);

  record
}

fn looks_like_openapi_record(object: &Map<String, Value>) -> bool {
  object.contains_key("request_id")
    && (object.contains_key("actor")
      || object.contains_key("principal")
      || object.contains_key("outcome")
      || object.contains_key("operation"))
    || object.contains_key("event") && object.contains_key("operation")
    || object.contains_key("error") && object.contains_key("request_id")
    || ARRAY_COLLECTION_FIELDS
      .iter()
      .any(|field| object.contains_key(*field))
}

fn openapi_message(object: &Map<String, Value>) -> String {
  if let Some(message) = string_field(object, &["message", "summary", "event"]) {
    return clean_message(&message);
  }

  if let Some(error) = object.get("error") {
    if let Some(error) = error.as_object()
      && let Some(message) = string_field(error, &["message", "code"])
    {
      return clean_message(&message);
    }

    if let Some(message) = value_to_attribute(error) {
      return clean_message(&message);
    }
  }

  if let Some(operation) = object.get("operation").and_then(Value::as_object) {
    let kind = string_field(operation, &["kind"]).unwrap_or_else(|| "operation".to_owned());
    let state = string_field(operation, &["state"]);
    return state.map_or(kind.clone(), |state| format!("{kind} {state}"));
  }

  string_field(object, &["outcome", "action", "operation"]).unwrap_or_default()
}

fn openapi_timestamp(object: &Map<String, Value>) -> Option<String> {
  string_field(object, &["timestamp", "created_at", "updated_at", "time"])
    .or_else(|| string_field(object, &["created_at_unix_ms", "timestamp_unix_ms"]))
}

fn openapi_level(object: &Map<String, Value>) -> Option<String> {
  string_field(object, &["level", "status"])
    .map(normalize_level)
    .or_else(|| {
      string_field(object, &["outcome"]).map(|outcome| {
        match outcome.to_ascii_lowercase().as_str() {
          "allow" | "allowed" | "success" | "succeeded" | "ok" => "info".to_owned(),
          "deny" | "denied" | "failure" | "failed" | "error" => "error".to_owned(),
          _ => normalize_level(outcome),
        }
      })
    })
}
