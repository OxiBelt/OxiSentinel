use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ConditionConfig {
  pub enabled: bool,
  pub condition_dir: PathBuf,
  pub pattern_sets: Vec<ConditionPatternSetConfig>,
  pub rule_groups: Vec<ConditionRuleGroupConfig>,
  pub rules: Vec<ConditionRuleConfig>,
}

impl Default for ConditionConfig {
  fn default() -> Self {
    Self {
      enabled: false,
      condition_dir: PathBuf::from("conditions"),
      pattern_sets: Vec::new(),
      rule_groups: Vec::new(),
      rules: Vec::new(),
    }
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionPatternSetConfig {
  pub name: String,
  pub kind: ConditionPatternSetKind,
  pub patterns: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionPatternSetKind {
  Contains,
  Regex,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ConditionRuleGroupConfig {
  pub name: String,
  pub tags: Vec<String>,
  pub when: Option<String>,
  pub merge_condition_as: ConditionMerge,
  pub conditions: Vec<ConditionFragmentConfig>,
}

impl Default for ConditionRuleGroupConfig {
  fn default() -> Self {
    Self {
      name: String::new(),
      tags: Vec::new(),
      when: None,
      merge_condition_as: ConditionMerge::And,
      conditions: Vec::new(),
    }
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ConditionFragmentConfig {
  pub label: Option<String>,
  pub when: String,
  pub merge_condition_as: ConditionMerge,
}

impl Default for ConditionFragmentConfig {
  fn default() -> Self {
    Self {
      label: None,
      when: String::new(),
      merge_condition_as: ConditionMerge::And,
    }
  }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ConditionRuleConfig {
  pub name: String,
  pub id: Option<String>,
  pub tags: Vec<String>,
  pub priority: i64,
  pub when: Option<String>,
  pub merge_condition_as: ConditionMerge,
  pub path: Option<PathBuf>,
  pub groups: Vec<String>,
}

impl Default for ConditionRuleConfig {
  fn default() -> Self {
    Self {
      name: String::new(),
      id: None,
      tags: Vec::new(),
      priority: 0,
      when: None,
      merge_condition_as: ConditionMerge::And,
      path: None,
      groups: Vec::new(),
    }
  }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionMerge {
  #[default]
  And,
  Or,
  Override,
}
