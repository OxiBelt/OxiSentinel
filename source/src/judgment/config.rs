use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct JudgmentConfig {
  pub enabled: bool,
  pub mode: JudgmentMode,
  pub handlers: Vec<JudgmentHandlerConfig>,
  pub max_decisions: usize,
}

impl Default for JudgmentConfig {
  fn default() -> Self {
    Self {
      enabled: false,
      mode: JudgmentMode::Monitor,
      handlers: Vec::new(),
      max_decisions: 256,
    }
  }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentMode {
  #[default]
  Monitor,
  Enforce,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct JudgmentHandlerConfig {
  pub name: String,
  pub condition: String,
  pub priority: i64,
  pub actions: Vec<JudgmentActionConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JudgmentActionConfig {
  EmitDecision {
    #[serde(default)]
    priority: i64,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    message: Option<String>,
  },
  EmitCallbackIntent {
    #[serde(default)]
    priority: i64,
    target: String,
    operation: String,
    #[serde(default)]
    payload: BTreeMap<String, String>,
    #[serde(default)]
    dedupe_key: Option<String>,
  },
}

impl JudgmentActionConfig {
  pub(crate) fn priority(&self) -> i64 {
    match self {
      Self::EmitDecision { priority, .. } | Self::EmitCallbackIntent { priority, .. } => *priority,
    }
  }
}
