use oxisentinel::{RuntimeRole, ServiceConfig, describe_runtime, health_report};

fn main() {
  let config = ServiceConfig::default();
  let report = health_report();

  println!("{}", describe_runtime(RuntimeRole::Control, &config));
  println!("health: {}", report.detail());
}
