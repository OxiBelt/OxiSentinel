use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::provider::{
  LogSourceProvider, ParseContext, SourceDescriptor, SourceLane, copy_selected_attributes,
  has_any_key, string_field,
};
use super::text::infer_application_source;
use super::{NormalizedLogRecord, ParseSource};

const DOCKER_LOGS_DESCRIPTOR: &[SourceDescriptor] = &[SourceDescriptor {
  canonical: "docker_logs",
  aliases: &["docker-logs", "docker_logs"],
  lane: SourceLane::Transport,
}];

pub(crate) struct DockerLogsProvider;

impl LogSourceProvider for DockerLogsProvider {
  fn descriptors(&self) -> &'static [SourceDescriptor] {
    DOCKER_LOGS_DESCRIPTOR
  }

  fn normalize_json(
    &self,
    object: &Map<String, Value>,
    context: &ParseContext<'_>,
  ) -> Option<Vec<NormalizedLogRecord>> {
    if !object.contains_key("log")
      || !(matches!(context.requested_source, ParseSource::DockerLogs)
        || has_any_key(object, &["stream"]) && has_any_key(object, &["time"]))
    {
      return None;
    }

    let payload = string_field(object, &["log"]).unwrap_or_default();
    let application_source =
      infer_application_source(context.requested_source, Some(object), &payload);
    let mut records = context
      .registry
      .normalize_payload_records(&payload, &application_source);

    if records.is_empty() {
      records.push(NormalizedLogRecord::new(&application_source, ""));
    }

    for record in &mut records {
      record.source = ParseSource::DockerLogs.as_str().to_owned();
      record.timestamp = string_field(object, &["time"]).or_else(|| record.timestamp.take());
      record.attributes.insert(
        "input_source".to_owned(),
        ParseSource::DockerLogs.as_str().to_owned(),
      );

      let mut consumed = BTreeSet::from(["log".to_owned(), "time".to_owned(), "stream".to_owned()]);
      copy_selected_attributes(object, record, &mut consumed);

      if let Some(stream) = string_field(object, &["stream"]) {
        record.attributes.insert("stream".to_owned(), stream);
      }
    }

    Some(records)
  }
}
