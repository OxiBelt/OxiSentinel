use std::process::ExitCode;

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

use oxisentinel::{AnalyzerConfig, JudgmentRuntime, RuntimeRole, describe_runtime, health_report};

fn main() -> ExitCode {
  let args = std::env::args().skip(1).collect::<Vec<_>>();

  match args.first().map(String::as_str) {
    None | Some("health") => {
      print_health();
      ExitCode::SUCCESS
    }
    Some("judgment") => judgment_command(&args[1..]),
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
  eprintln!(
    "usage:\n  oxisentinelctl health\n  oxisentinelctl judgment status [--addr ADDR]\n  oxisentinelctl judgment decisions [--addr ADDR]\n  oxisentinelctl judgment check --config PATH\n  oxisentinelctl judgment apply --config PATH [--addr ADDR] [--if-match ETAG]\n  oxisentinelctl judgment import --config PATH --if-match ETAG [--addr ADDR]"
  );
}

fn judgment_command(args: &[String]) -> ExitCode {
  match args.first().map(String::as_str) {
    Some("status") => http_get("/admin/v1/judgment/status", addr_arg(&args[1..])),
    Some("decisions") => http_get("/admin/v1/judgment/decisions", addr_arg(&args[1..])),
    Some("check") => {
      let Some(path) = option_value(&args[1..], "--config") else {
        eprintln!("judgment check requires --config PATH");
        return ExitCode::from(2);
      };
      let config = match AnalyzerConfig::load(path) {
        Ok(config) => config,
        Err(error) => {
          eprintln!("{error}");
          return ExitCode::from(1);
        }
      };
      match JudgmentRuntime::check_config(&config) {
        Ok(status) => {
          println!(
            "{}",
            serde_json::to_string(&status).expect("judgment status serializes")
          );
          ExitCode::SUCCESS
        }
        Err(error) => {
          eprintln!("{error}");
          ExitCode::from(1)
        }
      }
    }
    Some("apply") => http_post_config(
      "/admin/v1/judgment/apply",
      option_value(&args[1..], "--config"),
      option_value(&args[1..], "--if-match"),
      addr_arg(&args[1..]),
      false,
    ),
    Some("import") => http_post_config(
      "/admin/v1/judgment/import",
      option_value(&args[1..], "--config"),
      option_value(&args[1..], "--if-match"),
      addr_arg(&args[1..]),
      true,
    ),
    Some("--help") | Some("-h") | None => {
      print_usage();
      ExitCode::SUCCESS
    }
    Some(command) => {
      eprintln!("unknown judgment command: {command}");
      print_usage();
      ExitCode::from(2)
    }
  }
}

fn http_get(path: &str, addr: String) -> ExitCode {
  send_http(
    &addr,
    &format!("GET {path} HTTP/1.1\r\nhost: {addr}\r\nconnection: close\r\n\r\n"),
  )
}

fn http_post_config(
  path: &str,
  config_path: Option<&str>,
  if_match: Option<&str>,
  addr: String,
  require_if_match: bool,
) -> ExitCode {
  let Some(config_path) = config_path else {
    eprintln!("judgment command requires --config PATH");
    return ExitCode::from(2);
  };
  if require_if_match && if_match.is_none() {
    eprintln!("judgment import requires --if-match ETAG");
    return ExitCode::from(2);
  }
  let body = match std::fs::read_to_string(config_path) {
    Ok(body) => body,
    Err(error) => {
      eprintln!("failed to read {config_path}: {error}");
      return ExitCode::from(1);
    }
  };
  let if_match = if_match
    .map(|value| format!("if-match: {value}\r\n"))
    .unwrap_or_default();
  let request = format!(
    "POST {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n{if_match}connection: close\r\n\r\n{body}",
    body.len()
  );
  send_http(&addr, &request)
}

fn send_http(addr: &str, request: &str) -> ExitCode {
  let mut stream = match TcpStream::connect(addr) {
    Ok(stream) => stream,
    Err(error) => {
      eprintln!("failed to connect to {addr}: {error}");
      return ExitCode::from(1);
    }
  };
  if let Err(error) = stream.write_all(request.as_bytes()) {
    eprintln!("failed to write request: {error}");
    return ExitCode::from(1);
  }
  let _ = stream.shutdown(Shutdown::Write);
  let mut response = String::new();
  if let Err(error) = stream.read_to_string(&mut response) {
    eprintln!("failed to read response: {error}");
    return ExitCode::from(1);
  }
  let (head, body) = response.split_once("\r\n\r\n").unwrap_or(("", &response));
  let status = head
    .split_whitespace()
    .nth(1)
    .and_then(|value| value.parse::<u16>().ok())
    .unwrap_or(0);
  if (200..300).contains(&status) {
    println!("{body}");
    ExitCode::SUCCESS
  } else {
    eprintln!("{body}");
    ExitCode::from(1)
  }
}

fn addr_arg(args: &[String]) -> String {
  option_value(args, "--addr")
    .map(str::to_owned)
    .or_else(|| std::env::var("OXISENTINEL_ADMIN_ADDR").ok())
    .unwrap_or_else(|| "127.0.0.1:8080".to_owned())
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
  args
    .windows(2)
    .find(|window| window[0] == name)
    .map(|window| window[1].as_str())
}
