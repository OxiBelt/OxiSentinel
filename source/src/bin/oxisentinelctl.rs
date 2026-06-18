use oxisentinel::{AnalyzerConfig, RuntimeRole, describe_runtime, health_report};

fn main() {
  let config = AnalyzerConfig::default();
  let report = health_report();

  println!("{}", describe_runtime(RuntimeRole::Control, &config));
  println!("health: {}", report.detail());
}
