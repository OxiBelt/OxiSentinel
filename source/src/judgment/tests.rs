use crate::parser::{ParseSource, parse_line};
use crate::{AnalyzerConfig, JudgmentRuntime};

#[test]
fn judgment_emits_callback_intent_for_matching_condition() {
  let config = AnalyzerConfig::from_toml_str(
    r#"
[condition]
enabled = true

[[condition.rules]]
name = "deny-log"
when = "Log.Message.contains('denied')"

[judgment]
enabled = true

[[judgment.handlers]]
name = "dynamic-policy-intent"
condition = "deny-log"

[[judgment.handlers.actions]]
type = "emit_callback_intent"
target = "oxibelt_dynamic_policy"
operation = "apply"
dedupe_key = "deny-log"

[judgment.handlers.actions.payload]
source = "oxisentinel"
name = "deny-log"
"#,
    ".",
  )
  .expect("config parses");
  let runtime = JudgmentRuntime::from_config(&config).expect("runtime compiles");
  let record = parse_line(
    r#"{"service":"oxibelt","level":"warn","message":"policy denied"}"#,
    ParseSource::Auto,
  )
  .expect("record parses")
  .expect("record normalizes");

  let decisions = runtime.process_record(&record);

  assert_eq!(decisions.len(), 1);
  assert_eq!(decisions[0].outcome, "callback_intent");
  assert_eq!(
    decisions[0]
      .callback_intent
      .as_ref()
      .map(|intent| intent.target.as_str()),
    Some("oxibelt_dynamic_policy")
  );
}
