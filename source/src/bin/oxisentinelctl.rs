use std::process::ExitCode;

use oxisentinel::{AnalyzerConfig, RuntimeRole, describe_runtime, health_report};

fn main() -> ExitCode {
  let mut args = std::env::args().skip(1);

  match args.next().as_deref() {
    None | Some("health") => {
      print_health();
      ExitCode::SUCCESS
    }
    Some("--help") | Some("-h") => {
      print_usage();
      ExitCode::SUCCESS
    }
    Some(command) => {
      eprintln!("unknown command: {command}");
      print_usage();
      ExitCode::from(2)
    }
  }
}

fn print_health() {
  let config = AnalyzerConfig::default();
  let report = health_report();

  println!("{}", describe_runtime(RuntimeRole::Control, &config));
  println!("health: {}", report.detail());
}

fn print_usage() {
  eprintln!("usage:\n  oxisentinelctl health");
}
