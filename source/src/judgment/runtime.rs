use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::AnalyzerConfig;
use crate::condition::{ConditionEngine, ConditionError, ConditionEvaluation, ConditionSummary};
use crate::parser::NormalizedLogRecord;

use super::config::{JudgmentActionConfig, JudgmentHandlerConfig, JudgmentMode};

#[derive(Clone)]
pub struct JudgmentRuntime {
  snapshot: Arc<RwLock<Arc<JudgmentSnapshot>>>,
  decisions: Arc<Mutex<VecDeque<JudgmentDecision>>>,
  next_sequence: Arc<Mutex<u64>>,
}

impl JudgmentRuntime {
  pub fn from_config(config: &AnalyzerConfig) -> Result<Self, JudgmentError> {
    let snapshot = JudgmentSnapshot::compile(0, config)?;
    Ok(Self {
      decisions: Arc::new(Mutex::new(VecDeque::new())),
      snapshot: Arc::new(RwLock::new(Arc::new(snapshot))),
      next_sequence: Arc::new(Mutex::new(0)),
    })
  }

  pub fn check_config(config: &AnalyzerConfig) -> Result<JudgmentStatus, JudgmentError> {
    let snapshot = JudgmentSnapshot::compile(0, config)?;
    Ok(snapshot.status())
  }

  pub fn status(&self) -> JudgmentStatus {
    self.current().status()
  }

  pub fn replace_from_config(
    &self,
    config: &AnalyzerConfig,
    if_match: Option<&str>,
    mode: PreconditionMode,
  ) -> Result<JudgmentStatus, JudgmentError> {
    let current = self.current();
    check_if_match(current.generation, if_match, mode)?;
    let snapshot = JudgmentSnapshot::compile(current.generation + 1, config)?;
    let status = snapshot.status();
    *self
      .snapshot
      .write()
      .expect("judgment snapshot lock poisoned") = Arc::new(snapshot);
    Ok(status)
  }

  pub fn process_record(&self, record: &NormalizedLogRecord) -> Vec<JudgmentDecision> {
    let snapshot = self.current();
    let decisions = snapshot.process_record(record, self);
    for decision in &decisions {
      self.remember_decision(decision.clone(), snapshot.max_decisions);
    }
    decisions
  }

  pub fn recent_decisions(&self) -> Vec<JudgmentDecision> {
    self
      .decisions
      .lock()
      .expect("judgment decisions lock poisoned")
      .iter()
      .cloned()
      .collect()
  }

  fn current(&self) -> Arc<JudgmentSnapshot> {
    self
      .snapshot
      .read()
      .expect("judgment snapshot lock poisoned")
      .clone()
  }

  fn remember_decision(&self, decision: JudgmentDecision, max_decisions: usize) {
    if max_decisions == 0 {
      return;
    }
    let mut decisions = self
      .decisions
      .lock()
      .expect("judgment decisions lock poisoned");
    while decisions.len() >= max_decisions {
      decisions.pop_front();
    }
    decisions.push_back(decision);
  }

  fn next_decision_id(&self) -> String {
    let mut sequence = self
      .next_sequence
      .lock()
      .expect("judgment sequence lock poisoned");
    *sequence += 1;
    format!("decision-{}", *sequence)
  }
}

#[derive(Clone)]
struct JudgmentSnapshot {
  generation: i64,
  enabled: bool,
  mode: JudgmentMode,
  conditions: ConditionEngine,
  condition_summaries: Vec<ConditionSummary>,
  handlers: Vec<JudgmentHandlerConfig>,
  max_decisions: usize,
}

impl JudgmentSnapshot {
  fn compile(generation: i64, config: &AnalyzerConfig) -> Result<Self, JudgmentError> {
    let conditions = ConditionEngine::compile(config.condition(), config.base_dir())?;
    let mut handlers = config.judgment().handlers.clone();
    validate_handlers(&handlers, &conditions.summaries())?;
    handlers.sort_by(|left, right| {
      left
        .priority
        .cmp(&right.priority)
        .then_with(|| left.name.cmp(&right.name))
    });
    Ok(Self {
      generation,
      enabled: config.judgment().enabled,
      mode: config.judgment().mode,
      condition_summaries: conditions.summaries(),
      conditions,
      handlers,
      max_decisions: config.judgment().max_decisions,
    })
  }

  fn status(&self) -> JudgmentStatus {
    JudgmentStatus {
      generation: self.generation,
      etag: judgment_etag(self.generation),
      enabled: self.enabled,
      mode: self.mode,
      conditions: self.condition_summaries.clone(),
      handlers: self
        .handlers
        .iter()
        .map(|handler| JudgmentHandlerStatus {
          name: handler.name.clone(),
          condition: handler.condition.clone(),
          priority: handler.priority,
          actions: handler.actions.len(),
        })
        .collect(),
    }
  }

  fn process_record(
    &self,
    record: &NormalizedLogRecord,
    runtime: &JudgmentRuntime,
  ) -> Vec<JudgmentDecision> {
    if !self.enabled {
      return Vec::new();
    }
    let evaluations = self.conditions.evaluate(record);
    let mut decisions = Vec::new();
    for evaluation in evaluations {
      if let Some(error) = evaluation.error.as_ref() {
        decisions.push(error_decision(runtime, self, record, &evaluation, error));
        continue;
      }
      if !evaluation.matched {
        continue;
      }
      for handler in self
        .handlers
        .iter()
        .filter(|handler| handler.condition == evaluation.condition.name)
      {
        let mut actions = handler.actions.iter().collect::<Vec<_>>();
        actions.sort_by_key(|action| action.priority());
        if actions.is_empty() {
          decisions.push(matched_decision(
            runtime,
            self,
            record,
            &evaluation,
            handler,
            None,
          ));
          continue;
        }
        for action in actions {
          decisions.push(matched_decision(
            runtime,
            self,
            record,
            &evaluation,
            handler,
            Some(action),
          ));
        }
      }
    }
    decisions
  }
}

#[derive(Clone, Debug, Serialize)]
pub struct JudgmentStatus {
  pub generation: i64,
  pub etag: String,
  pub enabled: bool,
  pub mode: JudgmentMode,
  pub conditions: Vec<ConditionSummary>,
  pub handlers: Vec<JudgmentHandlerStatus>,
}

#[derive(Clone, Debug, Serialize)]
pub struct JudgmentHandlerStatus {
  pub name: String,
  pub condition: String,
  pub priority: i64,
  pub actions: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct JudgmentDecision {
  pub id: String,
  pub generation: i64,
  pub created_at_unix_ms: u128,
  pub condition: String,
  pub handler: Option<String>,
  pub outcome: String,
  pub mode: JudgmentMode,
  pub severity: Option<String>,
  pub message: Option<String>,
  pub callback_intent: Option<JudgmentCallbackIntent>,
  pub record_source: String,
  pub record_message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JudgmentCallbackIntent {
  pub target: String,
  pub operation: String,
  pub payload: std::collections::BTreeMap<String, String>,
  pub dedupe_key: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PreconditionMode {
  Required,
  Optional,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JudgmentError {
  message: String,
  kind: JudgmentErrorKind,
  expected_etag: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum JudgmentErrorKind {
  InvalidConfig,
  PreconditionRequired,
  PreconditionFailed,
}

impl JudgmentError {
  fn invalid(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
      kind: JudgmentErrorKind::InvalidConfig,
      expected_etag: None,
    }
  }

  fn precondition(kind: JudgmentErrorKind, expected_etag: String) -> Self {
    let message = match kind {
      JudgmentErrorKind::PreconditionRequired => "If-Match is required",
      JudgmentErrorKind::PreconditionFailed => {
        "If-Match does not match the active judgment generation"
      }
      JudgmentErrorKind::InvalidConfig => "invalid judgment configuration",
    };
    Self {
      message: message.to_owned(),
      kind,
      expected_etag: Some(expected_etag),
    }
  }

  pub fn kind(&self) -> JudgmentErrorKind {
    self.kind
  }

  pub fn expected_etag(&self) -> Option<&str> {
    self.expected_etag.as_deref()
  }
}

impl std::fmt::Display for JudgmentError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl std::error::Error for JudgmentError {}

impl From<ConditionError> for JudgmentError {
  fn from(error: ConditionError) -> Self {
    Self::invalid(error.to_string())
  }
}

pub fn judgment_etag(generation: i64) -> String {
  format!("\"oxisentinel-judgment-{generation}\"")
}

fn check_if_match(
  generation: i64,
  if_match: Option<&str>,
  mode: PreconditionMode,
) -> Result<(), JudgmentError> {
  let expected = judgment_etag(generation);
  match if_match {
    Some(value) if value == expected => Ok(()),
    Some(_) => Err(JudgmentError::precondition(
      JudgmentErrorKind::PreconditionFailed,
      expected,
    )),
    None if mode == PreconditionMode::Required => Err(JudgmentError::precondition(
      JudgmentErrorKind::PreconditionRequired,
      expected,
    )),
    None => Ok(()),
  }
}

fn validate_handlers(
  handlers: &[JudgmentHandlerConfig],
  conditions: &[ConditionSummary],
) -> Result<(), JudgmentError> {
  let mut names = std::collections::BTreeSet::new();
  let condition_names = conditions
    .iter()
    .map(|condition| condition.name.as_str())
    .collect::<std::collections::BTreeSet<_>>();
  for handler in handlers {
    if handler.name.trim().is_empty() {
      return Err(JudgmentError::invalid(
        "judgment handler name must not be empty",
      ));
    }
    if handler.condition.trim().is_empty() {
      return Err(JudgmentError::invalid(format!(
        "judgment handler {} condition must not be empty",
        handler.name
      )));
    }
    if !condition_names.contains(handler.condition.as_str()) {
      return Err(JudgmentError::invalid(format!(
        "judgment handler {} references unknown condition {}",
        handler.name, handler.condition
      )));
    }
    if handler.priority < 0 {
      return Err(JudgmentError::invalid(format!(
        "judgment handler {} priority must not be negative",
        handler.name
      )));
    }
    if !names.insert(handler.name.as_str()) {
      return Err(JudgmentError::invalid(format!(
        "duplicate judgment handler name {}",
        handler.name
      )));
    }
    for action in &handler.actions {
      if action.priority() < 0 {
        return Err(JudgmentError::invalid(format!(
          "judgment handler {} action priority must not be negative",
          handler.name
        )));
      }
    }
  }
  Ok(())
}

fn error_decision(
  runtime: &JudgmentRuntime,
  snapshot: &JudgmentSnapshot,
  record: &NormalizedLogRecord,
  evaluation: &ConditionEvaluation,
  error: &str,
) -> JudgmentDecision {
  JudgmentDecision {
    id: runtime.next_decision_id(),
    generation: snapshot.generation,
    created_at_unix_ms: now_unix_ms(),
    condition: evaluation.condition.name.clone(),
    handler: None,
    outcome: "error".to_owned(),
    mode: snapshot.mode,
    severity: Some("error".to_owned()),
    message: Some(error.to_owned()),
    callback_intent: None,
    record_source: record.source.clone(),
    record_message: record.message.clone(),
  }
}

fn matched_decision(
  runtime: &JudgmentRuntime,
  snapshot: &JudgmentSnapshot,
  record: &NormalizedLogRecord,
  evaluation: &ConditionEvaluation,
  handler: &JudgmentHandlerConfig,
  action: Option<&JudgmentActionConfig>,
) -> JudgmentDecision {
  let mut decision = JudgmentDecision {
    id: runtime.next_decision_id(),
    generation: snapshot.generation,
    created_at_unix_ms: now_unix_ms(),
    condition: evaluation.condition.name.clone(),
    handler: Some(handler.name.clone()),
    outcome: "matched".to_owned(),
    mode: snapshot.mode,
    severity: None,
    message: None,
    callback_intent: None,
    record_source: record.source.clone(),
    record_message: record.message.clone(),
  };
  match action {
    Some(JudgmentActionConfig::EmitDecision {
      severity, message, ..
    }) => {
      decision.severity = severity.clone();
      decision.message = message.clone();
    }
    Some(JudgmentActionConfig::EmitCallbackIntent {
      target,
      operation,
      payload,
      dedupe_key,
      ..
    }) => {
      decision.outcome = "callback_intent".to_owned();
      decision.callback_intent = Some(JudgmentCallbackIntent {
        target: target.clone(),
        operation: operation.clone(),
        payload: payload.clone(),
        dedupe_key: dedupe_key.clone(),
      });
    }
    None => {}
  }
  decision
}

fn now_unix_ms() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_millis())
    .unwrap_or_default()
}
