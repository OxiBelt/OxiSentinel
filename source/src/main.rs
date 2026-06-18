use oxisentinel::{AnalyzerConfig, RuntimeRole, describe_runtime};

fn main() {
  let config = AnalyzerConfig::default();
  println!("{}", describe_runtime(RuntimeRole::Daemon, &config));
}
