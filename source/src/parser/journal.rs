use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::provider::{
  LogSourceProvider, ParseContext, SourceDescriptor, SourceLane, copy_selected_attributes,
  has_any_key, string_field,
};
use super::text::{clean_container_name, clean_message, normalize_level};
use super::{NormalizedLogRecord, ParseSource};

const JOURNAL_DESCRIPTORS: &[SourceDescriptor] = &[
  SourceDescriptor {
    canonical: "docker_journald",
    aliases: &["docker-journald", "docker_journald"],
    lane: SourceLane::Journal,
  },
  SourceDescriptor {
    canonical: "linux_journal",
    aliases: &["linux-journal", "linux_journal"],
    lane: SourceLane::Journal,
  },
];

pub(crate) struct JournalProvider;

impl LogSourceProvider for JournalProvider {
  fn descriptors(&self) -> &'static [SourceDescriptor] {
    JOURNAL_DESCRIPTORS
  }

  fn normalize_json(
    &self,
    object: &Map<String, Value>,
    context: &ParseContext<'_>,
  ) -> Option<Vec<NormalizedLogRecord>> {
    if !(object.contains_key("MESSAGE")
      || object.contains_key("__REALTIME_TIMESTAMP")
      || matches!(
        context.requested_source,
        ParseSource::DockerJournald | ParseSource::LinuxJournal
      ))
    {
      return None;
    }

    Some(vec![normalize_journal_json(
      object,
      context.requested_source,
    )])
  }
}

fn normalize_journal_json(
  object: &Map<String, Value>,
  requested_source: &ParseSource,
) -> NormalizedLogRecord {
  let source = if !requested_source.is_auto() {
    requested_source.clone()
  } else if has_any_key(object, &["CONTAINER_ID", "CONTAINER_NAME", "CONTAINER_TAG"]) {
    ParseSource::DockerJournald
  } else {
    ParseSource::LinuxJournal
  };

  let message = string_field(object, &["MESSAGE"]).unwrap_or_default();
  let mut record = NormalizedLogRecord::new(&source, clean_message(&message));

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

fn journal_priority(object: &Map<String, Value>) -> Option<String> {
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
