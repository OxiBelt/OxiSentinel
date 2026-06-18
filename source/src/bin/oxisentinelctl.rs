use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::process::ExitCode;
use std::str::FromStr;

use oxisentinel::{
  AnalyzerConfig, ParseSource, RuntimeRole, describe_runtime, health_report, parse_reader,
};

fn main() -> ExitCode {
  let mut args = std::env::args().skip(1);

  match args.next().as_deref() {
    None | Some("health") => {
      print_health();
      ExitCode::SUCCESS
    }
    Some("parse") => match parse_command(args.collect()) {
      Ok(()) => ExitCode::SUCCESS,
      Err(error) if error == HELP_REQUESTED => ExitCode::SUCCESS,
      Err(error) => {
        eprintln!("{error}");
        ExitCode::from(2)
      }
    },
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

const HELP_REQUESTED: &str = "help requested";

fn print_health() {
  let config = AnalyzerConfig::default();
  let report = health_report();

  println!("{}", describe_runtime(RuntimeRole::Control, &config));
  println!("health: {}", report.detail());
}

fn parse_command(args: Vec<String>) -> Result<(), String> {
  let options = ParseOptions::from_args(args)?;
  let stdout = io::stdout();
  let mut writer = BufWriter::new(stdout.lock());

  if options.input == "-" {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    parse_reader(reader, &mut writer, options.source).map_err(|error| error.to_string())?;
  } else {
    let input = File::open(&options.input)
      .map_err(|error| format!("failed to open input {}: {error}", options.input))?;
    parse_reader(BufReader::new(input), &mut writer, options.source)
      .map_err(|error| error.to_string())?;
  }

  Ok(())
}

#[derive(Debug, Eq, PartialEq)]
struct ParseOptions {
  source: ParseSource,
  input: String,
}

impl ParseOptions {
  fn from_args(args: Vec<String>) -> Result<Self, String> {
    let mut source = ParseSource::Auto;
    let mut input = "-".to_owned();
    let mut index = 0;

    while index < args.len() {
      match args[index].as_str() {
        "--source" => {
          index += 1;
          let value = args
            .get(index)
            .ok_or_else(|| "missing value for --source".to_owned())?;
          source = ParseSource::from_str(value).map_err(|error| {
            format!(
              "{error}; expected one of: {}",
              ParseSource::choices().join(", ")
            )
          })?;
        }
        "--input" => {
          index += 1;
          input = args
            .get(index)
            .ok_or_else(|| "missing value for --input".to_owned())?
            .clone();
        }
        "--help" | "-h" => {
          print_usage();
          return Err(HELP_REQUESTED.to_owned());
        }
        other => {
          return Err(format!("unknown parse option: {other}"));
        }
      }

      index += 1;
    }

    Ok(Self { source, input })
  }
}

fn print_usage() {
  eprintln!(
    "usage:\n  oxisentinelctl health\n  oxisentinelctl parse [--source SOURCE] [--input PATH|-]\n\nsources: {}",
    ParseSource::choices().join(", ")
  );
}

#[cfg(test)]
mod tests {
  use super::{ParseOptions, ParseSource};

  #[test]
  fn parses_default_parse_options() {
    let options = ParseOptions::from_args(Vec::new()).expect("options parse");

    assert_eq!(
      options,
      ParseOptions {
        source: ParseSource::Auto,
        input: "-".to_owned(),
      }
    );
  }

  #[test]
  fn parses_source_and_input_options() {
    let options = ParseOptions::from_args(vec![
      "--source".to_owned(),
      "docker-journald".to_owned(),
      "--input".to_owned(),
      "/tmp/log.json".to_owned(),
    ])
    .expect("options parse");

    assert_eq!(options.source, ParseSource::DockerJournald);
    assert_eq!(options.input, "/tmp/log.json");
  }
}
