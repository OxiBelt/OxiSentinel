#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyzerConfig {
  program_name: String,
  bind_addr: String,
}

impl AnalyzerConfig {
  pub fn new(program_name: impl Into<String>, bind_addr: impl Into<String>) -> Self {
    Self {
      program_name: program_name.into(),
      bind_addr: bind_addr.into(),
    }
  }

  pub fn program_name(&self) -> &str {
    &self.program_name
  }

  pub fn bind_addr(&self) -> &str {
    &self.bind_addr
  }
}

impl Default for AnalyzerConfig {
  fn default() -> Self {
    Self::new("oxisentinel", "127.0.0.1:8080")
  }
}
