use crate::AnalyzerConfig;
use crate::condition::ConditionEngine;
use crate::parser::{ParseSource, parse_line};

#[test]
fn condition_matches_normalized_log_fields() {
  let config = AnalyzerConfig::from_toml_str(
    r#"
[condition]
enabled = true

[[condition.rules]]
name = "oxibelt-error"
priority = 10
when = "Log.Source == 'oxibelt' && Log.Level == 'error'"
"#,
    ".",
  )
  .expect("config parses");
  let engine =
    ConditionEngine::compile(config.condition(), config.base_dir()).expect("conditions compile");
  let record = parse_line(
    r#"{"service":"oxibelt","level":"error","message":"blocked"}"#,
    ParseSource::Auto,
  )
  .expect("record parses")
  .expect("record normalizes");

  let evaluations = engine.evaluate(&record);

  assert_eq!(evaluations.len(), 1);
  assert!(evaluations[0].matched);
}

#[test]
fn condition_groups_merge_fragments() {
  let config = AnalyzerConfig::from_toml_str(
    r#"
[condition]
enabled = true

[[condition.rule_groups]]
name = "bad-level"
when = "Log.Level == 'error'"

[[condition.rules]]
name = "oxibelt-error"
groups = ["bad-level"]
when = "Log.Source == 'oxibelt'"
"#,
    ".",
  )
  .expect("config parses");
  let engine =
    ConditionEngine::compile(config.condition(), config.base_dir()).expect("conditions compile");
  let record = parse_line(
    r#"{"service":"oxibelt","level":"error","message":"blocked"}"#,
    ParseSource::Auto,
  )
  .expect("record parses")
  .expect("record normalizes");

  assert!(engine.evaluate(&record)[0].matched);
}
