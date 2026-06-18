use oxisentinel::{AnalyzerConfig, RuntimeRole, describe_runtime};

fn main() {
  let config = AnalyzerConfig::default();
  println!("{}", describe_runtime(RuntimeRole::Daemon, &config));

  // Keep the container alive until collector and analyzer loops are wired in.
  loop {
    std::thread::park();
  }
}
