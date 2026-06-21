mod config;
mod error;
mod methods;
mod runtime;

pub use config::{
  ConditionConfig, ConditionFragmentConfig, ConditionMerge, ConditionPatternSetConfig,
  ConditionPatternSetKind, ConditionRuleConfig, ConditionRuleGroupConfig,
};
pub use error::ConditionError;
pub use runtime::{ConditionEngine, ConditionEvaluation, ConditionSummary};

#[cfg(test)]
mod tests;
