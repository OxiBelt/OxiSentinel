use std::fmt;
use std::io::{BufRead, Write};
use std::str::FromStr;

mod application;
mod journal;
mod normalized;
mod openapi;
mod provider;
#[cfg(test)]
mod tests;
mod text;
mod transport;

pub use normalized::NormalizedLogRecord;
use provider::{LogSourceProvider, SourceRegistry};

const NORMALIZED_SCHEMA: &str = "oxisentinel.log.v1";

static DOCKER_LOGS_PROVIDER: transport::DockerLogsProvider = transport::DockerLogsProvider;
static JOURNAL_PROVIDER: journal::JournalProvider = journal::JournalProvider;
static OPENAPI_PROVIDER: openapi::OpenApiProvider = openapi::OpenApiProvider;
static APPLICATION_PROVIDER: application::ApplicationProvider = application::ApplicationProvider;

static BUILTIN_PROVIDERS: &[&dyn LogSourceProvider] = &[
  &DOCKER_LOGS_PROVIDER,
  &JOURNAL_PROVIDER,
  &OPENAPI_PROVIDER,
  &APPLICATION_PROVIDER,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseSource {
  Auto,
  DockerLogs,
  DockerJournald,
  LinuxJournal,
  OpenApi,
  OxiBelt,
  Authelia,
  Ory,
  VoidAuth,
  Vaultwarden,
  External(String),
}

impl ParseSource {
  pub fn as_str(&self) -> &str {
    match self {
      Self::Auto => "auto",
      Self::DockerLogs => "docker_logs",
      Self::DockerJournald => "docker_journald",
      Self::LinuxJournal => "linux_journal",
      Self::OpenApi => "openapi",
      Self::OxiBelt => "oxibelt",
      Self::Authelia => "authelia",
      Self::Ory => "ory",
      Self::VoidAuth => "voidauth",
      Self::Vaultwarden => "vaultwarden",
      Self::External(source) => source.as_str(),
    }
  }

  pub fn choices() -> Vec<&'static str> {
    builtin_registry().choices()
  }

  fn from_canonical(value: &str) -> Self {
    match value {
      "docker_logs" => Self::DockerLogs,
      "docker_journald" => Self::DockerJournald,
      "linux_journal" => Self::LinuxJournal,
      "openapi" => Self::OpenApi,
      "oxibelt" => Self::OxiBelt,
      "authelia" => Self::Authelia,
      "ory" => Self::Ory,
      "voidauth" => Self::VoidAuth,
      "vaultwarden" => Self::Vaultwarden,
      other => Self::External(other.to_owned()),
    }
  }

  fn is_auto(&self) -> bool {
    matches!(self, Self::Auto)
  }
}

impl FromStr for ParseSource {
  type Err = ParseError;

  fn from_str(value: &str) -> Result<Self, Self::Err> {
    if value == "auto" {
      return Ok(Self::Auto);
    }

    builtin_registry()
      .parse_source(value)
      .ok_or_else(|| ParseError::UnknownSource(value.to_owned()))
  }
}

#[derive(Debug)]
pub enum ParseError {
  Io(std::io::Error),
  Json { line: usize, reason: String },
  UnknownSource(String),
}

impl fmt::Display for ParseError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Io(error) => write!(formatter, "I/O error: {error}"),
      Self::Json { line, reason } => {
        write!(formatter, "failed to parse JSON on line {line}: {reason}")
      }
      Self::UnknownSource(source) => write!(formatter, "unknown parse source: {source}"),
    }
  }
}

impl std::error::Error for ParseError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Io(error) => Some(error),
      Self::Json { .. } | Self::UnknownSource(_) => None,
    }
  }
}

impl From<std::io::Error> for ParseError {
  fn from(error: std::io::Error) -> Self {
    Self::Io(error)
  }
}

pub fn parse_line(
  line: &str,
  requested_source: ParseSource,
) -> Result<Option<NormalizedLogRecord>, ParseError> {
  Ok(parse_records(line, requested_source)?.into_iter().next())
}

pub fn parse_records(
  line: &str,
  requested_source: ParseSource,
) -> Result<Vec<NormalizedLogRecord>, ParseError> {
  let line = line.trim_end_matches(['\r', '\n']);

  if line.trim().is_empty() {
    return Ok(Vec::new());
  }

  builtin_registry().parse_line_records(line, &requested_source, 1)
}

pub fn parse_reader(
  reader: impl BufRead,
  writer: &mut impl Write,
  requested_source: ParseSource,
) -> Result<usize, ParseError> {
  let registry = builtin_registry();
  let mut count = 0;

  for (index, line) in reader.lines().enumerate() {
    let line = line?;
    let line_number = index + 1;

    if line.trim().is_empty() {
      continue;
    }

    for record in registry.parse_line_records(&line, &requested_source, line_number)? {
      writer.write_all(record.to_ndjson_line().as_bytes())?;
      writer.write_all(b"\n")?;
      count += 1;
    }
  }

  Ok(count)
}

fn builtin_registry() -> SourceRegistry<'static> {
  SourceRegistry::new(BUILTIN_PROVIDERS)
}
