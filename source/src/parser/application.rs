use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::provider::{
  LogSourceProvider, ParseContext, SourceDescriptor, SourceLane, copy_selected_attributes,
  string_field,
};
use super::text::{
  APPLICATION_LEVEL_FIELDS, APPLICATION_MESSAGE_FIELDS, APPLICATION_SOURCE_FIELDS,
  APPLICATION_TIMESTAMP_FIELDS, clean_message, infer_application_source, normalize_level,
  parse_logfmt_text, parse_plain_text, parse_vaultwarden_text, source_for_unstructured,
};
use super::{NormalizedLogRecord, ParseSource};

const APPLICATION_DESCRIPTORS: &[SourceDescriptor] = &[
  SourceDescriptor {
    canonical: "oxibelt",
    aliases: &["oxibelt"],
    lane: SourceLane::Application,
  },
  SourceDescriptor {
    canonical: "authelia",
    aliases: &["authelia"],
    lane: SourceLane::Application,
  },
  SourceDescriptor {
    canonical: "ory",
    aliases: &["ory"],
    lane: SourceLane::Application,
  },
  SourceDescriptor {
    canonical: "voidauth",
    aliases: &["voidauth"],
    lane: SourceLane::Application,
  },
  SourceDescriptor {
    canonical: "vaultwarden",
    aliases: &["vaultwarden"],
    lane: SourceLane::Application,
  },
];

pub(crate) struct ApplicationProvider;

impl LogSourceProvider for ApplicationProvider {
  fn descriptors(&self) -> &'static [SourceDescriptor] {
    APPLICATION_DESCRIPTORS
  }

  fn normalize_json(
    &self,
    object: &Map<String, Value>,
    context: &ParseContext<'_>,
  ) -> Option<Vec<NormalizedLogRecord>> {
    Some(vec![normalize_application_json(
      object,
      context.requested_source,
    )])
  }

  fn normalize_text(
    &self,
    payload: &str,
    context: &ParseContext<'_>,
  ) -> Option<Vec<NormalizedLogRecord>> {
    Some(vec![normalize_application_text(
      payload,
      context.requested_source,
    )])
  }
}

fn normalize_application_json(
  object: &Map<String, Value>,
  requested_source: &ParseSource,
) -> NormalizedLogRecord {
  let source = infer_application_source(requested_source, Some(object), "");
  let message = string_field(object, APPLICATION_MESSAGE_FIELDS).unwrap_or_default();
  let mut record = NormalizedLogRecord::new(&source, clean_message(&message));

  record.timestamp = string_field(object, APPLICATION_TIMESTAMP_FIELDS);
  record.level = string_field(object, APPLICATION_LEVEL_FIELDS).map(normalize_level);
  record.service = string_field(object, APPLICATION_SOURCE_FIELDS);

  let mut consumed = BTreeSet::new();
  consumed.extend(
    APPLICATION_MESSAGE_FIELDS
      .iter()
      .map(|key| (*key).to_owned()),
  );
  consumed.extend(
    APPLICATION_TIMESTAMP_FIELDS
      .iter()
      .map(|key| (*key).to_owned()),
  );
  consumed.extend(APPLICATION_LEVEL_FIELDS.iter().map(|key| (*key).to_owned()));
  consumed.extend(
    APPLICATION_SOURCE_FIELDS
      .iter()
      .map(|key| (*key).to_owned()),
  );

  copy_selected_attributes(object, &mut record, &mut consumed);
  record
}

fn normalize_application_text(
  payload: &str,
  requested_source: &ParseSource,
) -> NormalizedLogRecord {
  let payload = payload.trim_end_matches(['\r', '\n']);

  if let Some(record) = parse_vaultwarden_text(payload, requested_source) {
    return record;
  }

  let fallback_source = source_for_unstructured(requested_source);

  if let Some(record) = parse_logfmt_text(payload, &fallback_source) {
    return record;
  }

  parse_plain_text(payload, &fallback_source)
}
