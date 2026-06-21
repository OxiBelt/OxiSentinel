use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::condition::ConditionConfig;
use crate::judgment::JudgmentConfig;

#[derive(Clone, Debug, PartialEq)]
pub struct AnalyzerConfig {
  program_name: String,
  bind_addr: String,
  base_dir: PathBuf,
  condition: ConditionConfig,
  judgment: JudgmentConfig,
}

impl AnalyzerConfig {
  pub fn new(program_name: impl Into<String>, bind_addr: impl Into<String>) -> Self {
    Self {
      program_name: program_name.into(),
      bind_addr: bind_addr.into(),
      base_dir: PathBuf::from("."),
      condition: ConditionConfig::default(),
      judgment: JudgmentConfig::default(),
    }
  }

  pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path).map_err(|error| ConfigError {
      message: format!("failed to read config {}: {error}", path.display()),
    })?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    Self::from_toml_str(&raw, base_dir)
  }

  pub fn from_toml_str(raw: &str, base_dir: impl AsRef<Path>) -> Result<Self, ConfigError> {
    let file: ConfigFile = toml::from_str(raw).map_err(|error| ConfigError {
      message: format!("failed to parse analyzer config: {error}"),
    })?;
    let analyzer = file.analyzer.unwrap_or_default();

    Ok(Self {
      program_name: analyzer.name.unwrap_or_else(|| "oxisentinel".to_owned()),
      bind_addr: analyzer
        .bind_addr
        .unwrap_or_else(|| "127.0.0.1:8080".to_owned()),
      base_dir: base_dir.as_ref().to_path_buf(),
      condition: file.condition.unwrap_or_default(),
      judgment: file.judgment.unwrap_or_default(),
    })
  }

  pub fn program_name(&self) -> &str {
    &self.program_name
  }

  pub fn bind_addr(&self) -> &str {
    &self.bind_addr
  }

  pub fn base_dir(&self) -> &Path {
    &self.base_dir
  }

  pub fn condition(&self) -> &ConditionConfig {
    &self.condition
  }

  pub fn judgment(&self) -> &JudgmentConfig {
    &self.judgment
  }
}

impl Default for AnalyzerConfig {
  fn default() -> Self {
    Self::new("oxisentinel", "127.0.0.1:8080")
  }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConfigError {
  message: String,
}

impl std::fmt::Display for ConfigError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
  #[serde(default)]
  analyzer: Option<AnalyzerSection>,
  #[serde(default)]
  condition: Option<ConditionConfig>,
  #[serde(default)]
  judgment: Option<JudgmentConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnalyzerSection {
  #[serde(default)]
  name: Option<String>,
  #[serde(default)]
  bind_addr: Option<String>,
}
