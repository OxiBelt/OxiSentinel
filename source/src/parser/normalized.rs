use std::collections::BTreeMap;

use serde::Serialize;

use super::{NORMALIZED_SCHEMA, ParseSource};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NormalizedLogRecord {
  pub schema: &'static str,
  pub source: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub level: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub service: Option<String>,
  pub message: String,
  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  pub attributes: BTreeMap<String, String>,
}

impl NormalizedLogRecord {
  pub(crate) fn new(source: &ParseSource, message: impl Into<String>) -> Self {
    Self {
      schema: NORMALIZED_SCHEMA,
      source: source.as_str().to_owned(),
      timestamp: None,
      level: None,
      service: None,
      message: message.into(),
      attributes: BTreeMap::new(),
    }
  }

  pub(crate) fn new_source(source: &str, message: impl Into<String>) -> Self {
    Self {
      schema: NORMALIZED_SCHEMA,
      source: source.to_owned(),
      timestamp: None,
      level: None,
      service: None,
      message: message.into(),
      attributes: BTreeMap::new(),
    }
  }

  pub fn to_ndjson_line(&self) -> String {
    serde_json::to_string(self).expect("normalized log records serialize")
  }
}
