use std::process::ExitCode;

use oxisentinel::{AnalyzerConfig, JudgmentRuntime, RuntimeRole, admin, describe_runtime};

fn main() -> ExitCode {
  let config = match load_config() {
    Ok(config) => config,
    Err(error) => {
      eprintln!("{error}");
      return ExitCode::from(2);
    }
  };
  println!("{}", describe_runtime(RuntimeRole::Daemon, &config));

  let runtime = match JudgmentRuntime::from_config(&config) {
    Ok(runtime) => runtime,
    Err(error) => {
      eprintln!("failed to initialize judgment runtime: {error}");
      return ExitCode::from(2);
    }
  };

  if let Err(error) = admin::serve(config, runtime) {
    eprintln!("admin listener failed: {error}");
    return ExitCode::from(1);
  }

  ExitCode::SUCCESS
}

fn load_config() -> Result<AnalyzerConfig, String> {
  let mut args = std::env::args().skip(1);
  match args.next().as_deref() {
    None => Ok(AnalyzerConfig::default()),
    Some("--config") => {
      let path = args
        .next()
        .ok_or_else(|| "--config requires a path".to_owned())?;
      AnalyzerConfig::load(path).map_err(|error| error.to_string())
    }
    Some("--help") | Some("-h") => {
      println!("usage:\n  oxisentinel [--config PATH]");
      std::process::exit(0);
    }
    Some(other) => Err(format!("unknown argument: {other}")),
  }
}
