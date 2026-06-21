use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use online_dsl_forge::{
  Analyzer, EvalLimits, ExpressionDialect, ExpressionFunctionMode, MapRuntime,
  RuntimePatternSetConfig, RuntimePatternSets, RuntimeSchema, SecurityProfile, Value,
  VerifiedProgram, evaluate_verified, parse_expression,
};
use serde::Deserialize;

use crate::parser::NormalizedLogRecord;

use super::config::{
  ConditionConfig, ConditionMerge, ConditionPatternSetConfig, ConditionPatternSetKind,
  ConditionRuleConfig, ConditionRuleGroupConfig,
};
use super::error::ConditionError;
use super::methods::condition_registry;

#[derive(Clone)]
pub struct ConditionEngine {
  rules: Vec<CompiledCondition>,
  registry: online_dsl_forge::DynamicRegistry,
  limits: EvalLimits,
}

impl ConditionEngine {
  pub fn compile(config: &ConditionConfig, base_dir: &Path) -> Result<Self, ConditionError> {
    if !config.enabled {
      return Ok(Self {
        rules: Vec::new(),
        registry: condition_registry(empty_pattern_sets()?),
        limits: EvalLimits::default(),
      });
    }

    let pattern_sets = compile_pattern_sets(&config.pattern_sets)?;
    let registry = condition_registry(pattern_sets);
    let mut compiler = ConditionCompiler::new(config, base_dir)?;
    let rules = compiler.compile_rules()?;
    Ok(Self {
      rules,
      registry,
      limits: EvalLimits::default(),
    })
  }

  pub fn evaluate(&self, record: &NormalizedLogRecord) -> Vec<ConditionEvaluation> {
    let runtime = runtime_for_record(record, self.registry.clone());
    self
      .rules
      .iter()
      .map(|rule| {
        let value = evaluate_verified(&rule.program, &runtime, self.limits);
        match value {
          Ok(Value::Bool(true)) => ConditionEvaluation {
            condition: rule.summary.clone(),
            matched: true,
            error: None,
          },
          Ok(Value::Bool(false)) => ConditionEvaluation {
            condition: rule.summary.clone(),
            matched: false,
            error: None,
          },
          Ok(other) => ConditionEvaluation {
            condition: rule.summary.clone(),
            matched: false,
            error: Some(format!(
              "condition evaluated to {}, expected bool",
              other.type_name()
            )),
          },
          Err(error) => ConditionEvaluation {
            condition: rule.summary.clone(),
            matched: false,
            error: Some(error.to_string()),
          },
        }
      })
      .collect()
  }

  pub fn summaries(&self) -> Vec<ConditionSummary> {
    self.rules.iter().map(|rule| rule.summary.clone()).collect()
  }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConditionSummary {
  pub name: String,
  pub id: Option<String>,
  pub tags: Vec<String>,
  pub priority: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct ConditionEvaluation {
  pub condition: ConditionSummary,
  pub matched: bool,
  pub error: Option<String>,
}

#[derive(Clone)]
struct CompiledCondition {
  summary: ConditionSummary,
  program: VerifiedProgram,
}

struct ConditionCompiler<'a> {
  config: &'a ConditionConfig,
  condition_dir: PathBuf,
  global_groups: BTreeMap<String, ConditionRuleGroupConfig>,
}

impl<'a> ConditionCompiler<'a> {
  fn new(config: &'a ConditionConfig, base_dir: &Path) -> Result<Self, ConditionError> {
    let condition_dir = safe_join(base_dir, &config.condition_dir)?;
    let mut global_groups = BTreeMap::new();
    for group in &config.rule_groups {
      validate_group(group)?;
      if global_groups
        .insert(group.name.clone(), group.clone())
        .is_some()
      {
        return Err(ConditionError::new(format!(
          "duplicate condition rule group {}",
          group.name
        )));
      }
    }
    Ok(Self {
      config,
      condition_dir,
      global_groups,
    })
  }

  fn compile_rules(&mut self) -> Result<Vec<CompiledCondition>, ConditionError> {
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut rules = Vec::new();
    for rule in &self.config.rules {
      let loaded = self.load_rule(rule)?;
      validate_rule(&loaded.rule)?;
      if !names.insert(loaded.rule.name.clone()) {
        return Err(ConditionError::new(format!(
          "duplicate condition rule name {}",
          loaded.rule.name
        )));
      }
      if let Some(id) = loaded.rule.id.as_deref().filter(|id| !id.is_empty())
        && !ids.insert(id.to_owned())
      {
        return Err(ConditionError::new(format!(
          "duplicate condition rule id {id}"
        )));
      }
      let expression = self.effective_expression(&loaded)?;
      let program = compile_expression(&expression).map_err(|error| {
        ConditionError::new(format!(
          "invalid condition rule {} expression: {error}",
          loaded.rule.name
        ))
      })?;
      rules.push(CompiledCondition {
        summary: ConditionSummary {
          name: loaded.rule.name,
          id: loaded.rule.id,
          tags: loaded.rule.tags,
          priority: loaded.rule.priority,
        },
        program,
      });
    }
    rules.sort_by(|left, right| {
      left
        .summary
        .priority
        .cmp(&right.summary.priority)
        .then_with(|| left.summary.name.cmp(&right.summary.name))
    });
    Ok(rules)
  }

  fn load_rule(&self, rule: &ConditionRuleConfig) -> Result<LoadedRule, ConditionError> {
    let Some(path) = rule.path.as_ref() else {
      return Ok(LoadedRule {
        rule: rule.clone(),
        local_groups: BTreeMap::new(),
      });
    };
    if rule.when.is_some()
      || rule.merge_condition_as != ConditionMerge::And
      || !rule.groups.is_empty()
    {
      return Err(ConditionError::new(format!(
        "condition rule {} external path cannot be combined with inline when, merge_condition_as, or groups",
        rule.name
      )));
    }
    let path = safe_join(&self.condition_dir, path)?;
    let raw = std::fs::read_to_string(&path).map_err(|error| {
      ConditionError::new(format!(
        "failed to read condition rule file {}: {error}",
        path.display()
      ))
    })?;
    let external: ExternalConditionRuleFile = toml::from_str(&raw).map_err(|error| {
      ConditionError::new(format!(
        "failed to parse condition rule file {}: {error}",
        path.display()
      ))
    })?;
    let mut loaded = rule.clone();
    loaded.path = None;
    loaded.when = external.when;
    loaded.merge_condition_as = external.merge_condition_as;
    loaded.groups = external.groups;
    let mut local_groups = BTreeMap::new();
    for group in external.rule_groups {
      validate_group(&group)?;
      if local_groups
        .insert(group.name.clone(), group.clone())
        .is_some()
      {
        return Err(ConditionError::new(format!(
          "duplicate local condition rule group {} in {}",
          group.name,
          path.display()
        )));
      }
    }
    Ok(LoadedRule {
      rule: loaded,
      local_groups,
    })
  }

  fn effective_expression(&self, loaded: &LoadedRule) -> Result<String, ConditionError> {
    let mut accumulator = ConditionAccumulator::default();
    for group_name in &loaded.rule.groups {
      let group = loaded
        .local_groups
        .get(group_name)
        .or_else(|| self.global_groups.get(group_name))
        .ok_or_else(|| {
          ConditionError::new(format!(
            "condition rule {} references unknown group {}",
            loaded.rule.name, group_name
          ))
        })?;
      append_group(&mut accumulator, group)?;
    }
    if let Some(when) = loaded.rule.when.as_deref() {
      accumulator.push(loaded.rule.merge_condition_as, when)?;
    }
    Ok(accumulator.finish())
  }
}

#[derive(Default)]
struct ConditionAccumulator {
  expression: Option<String>,
  override_seen: bool,
}

impl ConditionAccumulator {
  fn push(&mut self, merge: ConditionMerge, expression: &str) -> Result<(), ConditionError> {
    if expression.trim().is_empty() {
      return Err(ConditionError::new(
        "condition expression must not be empty",
      ));
    }
    if merge == ConditionMerge::Override {
      if self.override_seen {
        return Err(ConditionError::new(
          "condition merge_condition_as override may appear only once",
        ));
      }
      self.override_seen = true;
      self.expression = Some(format!("({expression})"));
      return Ok(());
    }
    let Some(previous) = self.expression.take() else {
      self.expression = Some(format!("({expression})"));
      return Ok(());
    };
    let op = match merge {
      ConditionMerge::And => "&&",
      ConditionMerge::Or => "||",
      ConditionMerge::Override => unreachable!("override handled above"),
    };
    self.expression = Some(format!("({previous}) {op} ({expression})"));
    Ok(())
  }

  fn finish(self) -> String {
    self.expression.unwrap_or_else(|| "true".to_owned())
  }
}

struct LoadedRule {
  rule: ConditionRuleConfig,
  local_groups: BTreeMap<String, ConditionRuleGroupConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ExternalConditionRuleFile {
  when: Option<String>,
  merge_condition_as: ConditionMerge,
  groups: Vec<String>,
  rule_groups: Vec<ConditionRuleGroupConfig>,
}

fn append_group(
  accumulator: &mut ConditionAccumulator,
  group: &ConditionRuleGroupConfig,
) -> Result<(), ConditionError> {
  if let Some(when) = group.when.as_deref() {
    accumulator.push(group.merge_condition_as, when)?;
  }
  for condition in &group.conditions {
    accumulator.push(condition.merge_condition_as, &condition.when)?;
  }
  Ok(())
}

fn validate_group(group: &ConditionRuleGroupConfig) -> Result<(), ConditionError> {
  if group.name.trim().is_empty() {
    return Err(ConditionError::new(
      "condition rule group name must not be empty",
    ));
  }
  if group.when.as_deref().is_none_or(str::is_empty) && group.conditions.is_empty() {
    return Err(ConditionError::new(format!(
      "condition rule group {} must declare when or conditions",
      group.name
    )));
  }
  Ok(())
}

fn validate_rule(rule: &ConditionRuleConfig) -> Result<(), ConditionError> {
  if rule.name.trim().is_empty() {
    return Err(ConditionError::new("condition rule name must not be empty"));
  }
  if rule.priority < 0 {
    return Err(ConditionError::new(format!(
      "condition rule {} priority must not be negative",
      rule.name
    )));
  }
  if rule.path.is_none() && rule.when.is_none() && rule.groups.is_empty() {
    return Err(ConditionError::new(format!(
      "condition rule {} must declare when, groups, or path",
      rule.name
    )));
  }
  Ok(())
}

fn compile_pattern_sets(
  configs: &[ConditionPatternSetConfig],
) -> Result<RuntimePatternSets, ConditionError> {
  let configs = configs.iter().map(|config| match config.kind {
    ConditionPatternSetKind::Contains => {
      RuntimePatternSetConfig::contains(config.name.clone(), config.patterns.clone())
    }
    ConditionPatternSetKind::Regex => {
      RuntimePatternSetConfig::regex(config.name.clone(), config.patterns.clone())
    }
  });
  RuntimePatternSets::compile(configs)
    .map_err(|error| ConditionError::new(format!("invalid condition pattern set: {error}")))
}

fn empty_pattern_sets() -> Result<RuntimePatternSets, ConditionError> {
  RuntimePatternSets::compile(Vec::<RuntimePatternSetConfig>::new())
    .map_err(|error| ConditionError::new(error.to_string()))
}

fn compile_expression(expression: &str) -> Result<VerifiedProgram, String> {
  let ast = parse_expression(expression).map_err(|error| error.to_string())?;
  let mut schema = RuntimeSchema::oxirule_waf();
  schema.add_variable("Log");
  Analyzer::new(SecurityProfile::oxirule_waf_request())
    .with_dialect(ExpressionDialect::OxiRuleV1)
    .with_expression_function_mode(ExpressionFunctionMode::CallFrame)
    .analyze(&ast, &schema)
    .map_err(|error| error.to_string())
}

fn runtime_for_record(
  record: &NormalizedLogRecord,
  registry: online_dsl_forge::DynamicRegistry,
) -> MapRuntime {
  let mut variables = BTreeMap::new();
  variables.insert("Log".to_owned(), log_value(record));
  variables.insert("Context".to_owned(), context_value(record));
  variables.insert("Request".to_owned(), request_value(record));
  variables.insert("DynamicPolicy".to_owned(), dynamic_policy_value(record));
  MapRuntime::new(variables, registry)
}

fn log_value(record: &NormalizedLogRecord) -> Value {
  let mut log = BTreeMap::new();
  log.insert("Schema".to_owned(), Value::String(record.schema.to_owned()));
  log.insert("Source".to_owned(), Value::String(record.source.clone()));
  log.insert("Timestamp".to_owned(), option_string(&record.timestamp));
  log.insert("Level".to_owned(), option_string(&record.level));
  log.insert("Service".to_owned(), option_string(&record.service));
  log.insert("Message".to_owned(), Value::String(record.message.clone()));
  log.insert(
    "Attributes".to_owned(),
    Value::Object(
      record
        .attributes
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect(),
    ),
  );
  Value::Object(log)
}

fn context_value(record: &NormalizedLogRecord) -> Value {
  let mut context = BTreeMap::new();
  context.insert("Source".to_owned(), Value::String(record.source.clone()));
  context.insert(
    "Service".to_owned(),
    Value::String(record.service.clone().unwrap_or_default()),
  );
  context.insert("RuleTags".to_owned(), Value::Array(Vec::new()));
  Value::Object(context)
}

fn request_value(record: &NormalizedLogRecord) -> Value {
  let mut http = BTreeMap::new();
  http.insert(
    "Method".to_owned(),
    Value::String(attribute(
      record,
      &["method", "request_method", "http_method"],
    )),
  );
  http.insert(
    "Path".to_owned(),
    Value::String(attribute(
      record,
      &["path", "request_path", "http_path", "uri"],
    )),
  );
  http.insert(
    "Query".to_owned(),
    Value::String(attribute(record, &["query", "request_query", "http_query"])),
  );
  http.insert("Headers".to_owned(), Value::Object(BTreeMap::new()));
  let mut body = BTreeMap::new();
  body.insert(
    "Size".to_owned(),
    Value::Int(
      attribute(record, &["body_size", "request_body_size"])
        .parse()
        .unwrap_or(0),
    ),
  );
  body.insert(
    "Text".to_owned(),
    Value::String(attribute(record, &["body", "request_body"])),
  );
  body.insert("Bytes".to_owned(), Value::String(String::new()));
  body.insert("IsTruncated".to_owned(), Value::Bool(false));
  http.insert("Body".to_owned(), Value::Object(body));

  let mut client = BTreeMap::new();
  client.insert(
    "Ip".to_owned(),
    Value::String(attribute(record, &["client_ip", "remote_addr", "ip"])),
  );

  let mut request = BTreeMap::new();
  request.insert("Http".to_owned(), Value::Object(http));
  request.insert("Client".to_owned(), Value::Object(client));
  request.insert("Tags".to_owned(), Value::Array(Vec::new()));
  Value::Object(request)
}

fn dynamic_policy_value(record: &NormalizedLogRecord) -> Value {
  let mut policy = BTreeMap::new();
  for (target, keys) in [
    ("Source", &["source", "policy_source"][..]),
    ("Name", &["name", "policy_name"][..]),
    ("Action", &["action", "policy_action"][..]),
    ("Outcome", &["outcome", "status"][..]),
    ("Subject", &["subject", "policy_subject"][..]),
  ] {
    policy.insert(target.to_owned(), Value::String(attribute(record, keys)));
  }
  Value::Object(policy)
}

fn attribute(record: &NormalizedLogRecord, keys: &[&str]) -> String {
  keys
    .iter()
    .find_map(|key| record.attributes.get(*key).cloned())
    .unwrap_or_default()
}

fn option_string(value: &Option<String>) -> Value {
  value
    .as_ref()
    .map(|value| Value::String(value.clone()))
    .unwrap_or(Value::Null)
}

fn safe_join(base_dir: &Path, path: &Path) -> Result<PathBuf, ConditionError> {
  if path.is_absolute() {
    return Err(ConditionError::new(format!(
      "condition path {} must be relative",
      path.display()
    )));
  }
  for component in path.components() {
    if matches!(
      component,
      Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_)
    ) {
      return Err(ConditionError::new(format!(
        "condition path {} must not contain . or .. components",
        path.display()
      )));
    }
  }
  Ok(base_dir.join(path))
}
