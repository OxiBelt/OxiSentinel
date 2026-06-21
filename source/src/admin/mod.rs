use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use crate::AnalyzerConfig;
use crate::judgment::{JudgmentError, JudgmentErrorKind, JudgmentRuntime, PreconditionMode};

pub fn serve(config: AnalyzerConfig, runtime: JudgmentRuntime) -> std::io::Result<()> {
  let listener = TcpListener::bind(config.bind_addr())?;
  let runtime = Arc::new(runtime);
  for stream in listener.incoming() {
    let stream = stream?;
    let runtime = Arc::clone(&runtime);
    std::thread::spawn(move || {
      let _ = handle_connection(stream, &runtime);
    });
  }
  Ok(())
}

fn handle_connection(
  mut stream: TcpStream,
  runtime: &JudgmentRuntime,
) -> Result<(), std::io::Error> {
  stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
  let response = match read_request(&mut stream) {
    Ok(request) => route_request(request, runtime),
    Err(error) => json_error(400, "bad_request", &error),
  };
  stream.write_all(response.as_bytes())?;
  stream.flush()
}

fn route_request(request: HttpRequest, runtime: &JudgmentRuntime) -> String {
  match (request.method.as_str(), request.path.as_str()) {
    ("GET", "/admin/v1/judgment/status") => json_response(200, &runtime.status()),
    ("GET", "/admin/v1/judgment/decisions") => json_response(200, &runtime.recent_decisions()),
    ("POST", "/admin/v1/judgment/check") => match config_from_body(&request.body) {
      Ok(config) => match JudgmentRuntime::check_config(&config) {
        Ok(status) => json_response(200, &status),
        Err(error) => judgment_error_response(error),
      },
      Err(error) => json_error(400, "bad_request", &error),
    },
    ("POST", "/admin/v1/judgment/apply") => {
      apply_config(request, runtime, PreconditionMode::Optional)
    }
    ("POST", "/admin/v1/judgment/import") => {
      apply_config(request, runtime, PreconditionMode::Required)
    }
    ("GET" | "POST", _) => json_error(404, "not_found", "unknown Admin API endpoint"),
    _ => json_error(
      405,
      "method_not_allowed",
      "method is not allowed for this endpoint",
    ),
  }
}

fn apply_config(request: HttpRequest, runtime: &JudgmentRuntime, mode: PreconditionMode) -> String {
  match config_from_body(&request.body) {
    Ok(config) => {
      let if_match = request.header("if-match");
      match runtime.replace_from_config(&config, if_match, mode) {
        Ok(status) => json_response(200, &status),
        Err(error) => judgment_error_response(error),
      }
    }
    Err(error) => json_error(400, "bad_request", &error),
  }
}

fn config_from_body(body: &str) -> Result<AnalyzerConfig, String> {
  let base_dir = std::env::current_dir().map_err(|error| error.to_string())?;
  AnalyzerConfig::from_toml_str(body, base_dir).map_err(|error| error.to_string())
}

fn judgment_error_response(error: JudgmentError) -> String {
  match error.kind() {
    JudgmentErrorKind::InvalidConfig => json_error(400, "bad_request", &error.to_string()),
    JudgmentErrorKind::PreconditionFailed => json_error_with_expected(
      412,
      "precondition_failed",
      &error.to_string(),
      error.expected_etag(),
    ),
    JudgmentErrorKind::PreconditionRequired => json_error_with_expected(
      428,
      "precondition_required",
      &error.to_string(),
      error.expected_etag(),
    ),
  }
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
  let mut data = Vec::new();
  let mut buffer = [0_u8; 4096];
  let mut content_length = None;
  loop {
    let read = stream
      .read(&mut buffer)
      .map_err(|error| error.to_string())?;
    if read == 0 {
      break;
    }
    data.extend_from_slice(&buffer[..read]);
    if content_length.is_none()
      && let Some(header_end) = find_header_end(&data)
    {
      let header = std::str::from_utf8(&data[..header_end])
        .map_err(|error| format!("request headers must be UTF-8: {error}"))?;
      content_length = Some(parse_content_length(header)?);
    }
    if let (Some(header_end), Some(length)) = (find_header_end(&data), content_length)
      && data.len() >= header_end + 4 + length
    {
      break;
    }
  }
  parse_request(&data)
}

fn parse_request(data: &[u8]) -> Result<HttpRequest, String> {
  let header_end =
    find_header_end(data).ok_or_else(|| "missing HTTP header terminator".to_owned())?;
  let header = std::str::from_utf8(&data[..header_end])
    .map_err(|error| format!("request headers must be UTF-8: {error}"))?;
  let mut lines = header.split("\r\n");
  let request_line = lines
    .next()
    .ok_or_else(|| "missing request line".to_owned())?;
  let mut parts = request_line.split_whitespace();
  let method = parts
    .next()
    .ok_or_else(|| "missing HTTP method".to_owned())?;
  let path = parts.next().ok_or_else(|| "missing HTTP path".to_owned())?;
  let mut headers = BTreeMap::new();
  for line in lines {
    let Some((name, value)) = line.split_once(':') else {
      continue;
    };
    headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
  }
  let content_length = parse_content_length(header)?;
  let body_start = header_end + 4;
  let body_end = body_start.saturating_add(content_length).min(data.len());
  let body = std::str::from_utf8(&data[body_start..body_end])
    .map_err(|error| format!("request body must be UTF-8: {error}"))?
    .to_owned();
  Ok(HttpRequest {
    method: method.to_owned(),
    path: path.split('?').next().unwrap_or(path).to_owned(),
    headers,
    body,
  })
}

fn parse_content_length(header: &str) -> Result<usize, String> {
  for line in header.split("\r\n").skip(1) {
    let Some((name, value)) = line.split_once(':') else {
      continue;
    };
    if name.trim().eq_ignore_ascii_case("content-length") {
      return value
        .trim()
        .parse()
        .map_err(|error| format!("invalid Content-Length: {error}"));
    }
  }
  Ok(0)
}

fn find_header_end(data: &[u8]) -> Option<usize> {
  data.windows(4).position(|window| window == b"\r\n\r\n")
}

#[derive(Debug)]
struct HttpRequest {
  method: String,
  path: String,
  headers: BTreeMap<String, String>,
  body: String,
}

impl HttpRequest {
  fn header(&self, name: &str) -> Option<&str> {
    self
      .headers
      .get(&name.to_ascii_lowercase())
      .map(String::as_str)
  }
}

fn json_response(status: u16, value: &impl Serialize) -> String {
  let body = serde_json::to_string(value).expect("Admin response serializes");
  http_response(status, "application/json", &body)
}

fn json_error(status: u16, code: &str, message: &str) -> String {
  json_response(
    status,
    &ErrorEnvelope {
      error: ErrorBody {
        code,
        message,
        details: BTreeMap::new(),
      },
    },
  )
}

fn json_error_with_expected(
  status: u16,
  code: &str,
  message: &str,
  expected: Option<&str>,
) -> String {
  let mut details = BTreeMap::new();
  details.insert("header", "If-Match".to_owned());
  if let Some(expected) = expected {
    details.insert("expected", expected.to_owned());
  }
  json_response(
    status,
    &ErrorEnvelope {
      error: ErrorBody {
        code,
        message,
        details,
      },
    },
  )
}

fn http_response(status: u16, content_type: &str, body: &str) -> String {
  let reason = match status {
    200 => "OK",
    400 => "Bad Request",
    404 => "Not Found",
    405 => "Method Not Allowed",
    412 => "Precondition Failed",
    428 => "Precondition Required",
    _ => "OK",
  };
  format!(
    "HTTP/1.1 {status} {reason}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
    body.len()
  )
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
  error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
  code: &'a str,
  message: &'a str,
  #[serde(skip_serializing_if = "BTreeMap::is_empty")]
  details: BTreeMap<&'a str, String>,
}
