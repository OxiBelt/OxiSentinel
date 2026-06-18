use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::{NormalizedLogRecord, ParseError, ParseSource};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceLane {
  Transport,
  Journal,
  OpenApi,
  Application,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SourceDescriptor {
  pub canonical: &'static str,
  pub aliases: &'static [&'static str],
  pub lane: SourceLane,
}

impl SourceDescriptor {
  fn matches(self, value: &str) -> bool {
    self.canonical == value || self.aliases.contains(&value)
  }
}

pub(crate) struct ParseContext<'a> {
  pub requested_source: &'a ParseSource,
  pub registry: &'a SourceRegistry<'a>,
}

pub(crate) trait LogSourceProvider: Sync {
  fn descriptors(&self) -> &'static [SourceDescriptor];

  fn normalize_json(
    &self,
    _object: &Map<String, Value>,
    _context: &ParseContext<'_>,
  ) -> Option<Vec<NormalizedLogRecord>> {
    None
  }

  fn normalize_text(
    &self,
    _payload: &str,
    _context: &ParseContext<'_>,
  ) -> Option<Vec<NormalizedLogRecord>> {
    None
  }
}

#[derive(Clone, Copy)]
pub(crate) struct SourceRegistry<'a> {
  providers: &'a [&'a dyn LogSourceProvider],
}

impl<'a> SourceRegistry<'a> {
  pub(crate) const fn new(providers: &'a [&'a dyn LogSourceProvider]) -> Self {
    Self { providers }
  }

  pub(crate) fn choices(&self) -> Vec<&'static str> {
    let mut choices = vec!["auto"];
    for provider in self.providers {
      for descriptor in provider.descriptors() {
        for alias in descriptor.aliases {
          choices.push(alias);
        }
      }
    }
    choices
  }

  pub(crate) fn parse_source(&self, value: &str) -> Option<ParseSource> {
    for provider in self.providers {
      for descriptor in provider.descriptors() {
        if descriptor.matches(value) {
          return Some(ParseSource::from_canonical(descriptor.canonical));
        }
      }
    }

    None
  }

  pub(crate) fn parse_line_records(
    &self,
    line: &str,
    requested_source: &ParseSource,
    line_number: usize,
  ) -> Result<Vec<NormalizedLogRecord>, ParseError> {
    let payload = line.trim_end_matches(['\r', '\n']);

    if let Some(object) = json_object(payload, line_number)? {
      return Ok(self.normalize_json_object(&object, requested_source));
    }

    Ok(self.normalize_text_payload(payload, requested_source))
  }

  pub(crate) fn normalize_payload_records(
    &self,
    payload: &str,
    requested_source: &ParseSource,
  ) -> Vec<NormalizedLogRecord> {
    let payload = payload.trim_end_matches(['\r', '\n']);

    if let Ok(Value::Object(object)) = serde_json::from_str::<Value>(payload.trim_start()) {
      return self.normalize_json_object(&object, requested_source);
    }

    self.normalize_text_payload(payload, requested_source)
  }

  fn normalize_json_object(
    &self,
    object: &Map<String, Value>,
    requested_source: &ParseSource,
  ) -> Vec<NormalizedLogRecord> {
    let context = ParseContext {
      requested_source,
      registry: self,
    };

    if !requested_source.is_auto() {
      for provider in self.providers_for_lane(SourceLane::Transport) {
        if let Some(records) = provider.normalize_json(object, &context) {
          return records;
        }
      }
      for provider in self.providers_for_lane(SourceLane::Journal) {
        if let Some(records) = provider.normalize_json(object, &context) {
          return records;
        }
      }
      if let Some(provider) = self.provider_for_source(requested_source)
        && let Some(records) = provider.normalize_json(object, &context)
      {
        return records;
      }
    }

    for provider in self.providers {
      if let Some(records) = provider.normalize_json(object, &context) {
        return records;
      }
    }

    Vec::new()
  }

  fn normalize_text_payload(
    &self,
    payload: &str,
    requested_source: &ParseSource,
  ) -> Vec<NormalizedLogRecord> {
    let context = ParseContext {
      requested_source,
      registry: self,
    };

    if !requested_source.is_auto()
      && let Some(provider) = self.provider_for_source(requested_source)
      && let Some(records) = provider.normalize_text(payload, &context)
    {
      return records;
    }

    for provider in self.providers {
      if let Some(records) = provider.normalize_text(payload, &context) {
        return records;
      }
    }

    Vec::new()
  }

  fn providers_for_lane(
    &self,
    lane: SourceLane,
  ) -> impl Iterator<Item = &'a dyn LogSourceProvider> + '_ {
    self.providers.iter().copied().filter(move |provider| {
      provider
        .descriptors()
        .iter()
        .any(|descriptor| descriptor.lane == lane)
    })
  }

  fn provider_for_source(&self, source: &ParseSource) -> Option<&'a dyn LogSourceProvider> {
    self.providers.iter().copied().find(|provider| {
      provider
        .descriptors()
        .iter()
        .any(|descriptor| descriptor.canonical == source.as_str())
    })
  }
}

fn json_object(
  payload: &str,
  line_number: usize,
) -> Result<Option<Map<String, Value>>, ParseError> {
  let trimmed = payload.trim_start();

  if !trimmed.starts_with('{') {
    return Ok(None);
  }

  match serde_json::from_str::<Value>(trimmed) {
    Ok(Value::Object(object)) => Ok(Some(object)),
    Ok(_) => Err(ParseError::Json {
      line: line_number,
      reason: "top-level JSON value must be an object".to_owned(),
    }),
    Err(error) => Err(ParseError::Json {
      line: line_number,
      reason: error.to_string(),
    }),
  }
}

pub(crate) fn has_any_key(object: &Map<String, Value>, keys: &[&str]) -> bool {
  keys.iter().any(|key| object.contains_key(*key))
}

pub(crate) fn string_field(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
  keys
    .iter()
    .find_map(|key| object.get(*key).and_then(value_to_attribute))
    .filter(|value| !value.is_empty())
}

pub(crate) fn value_to_attribute(value: &Value) -> Option<String> {
  match value {
    Value::Null => None,
    Value::Bool(value) => Some(value.to_string()),
    Value::Number(value) => Some(value.to_string()),
    Value::String(value) => Some(value.clone()),
    Value::Array(_) | Value::Object(_) => Some(value.to_string()),
  }
}

pub(crate) fn copy_selected_attributes(
  object: &Map<String, Value>,
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
