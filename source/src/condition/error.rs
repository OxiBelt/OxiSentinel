#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConditionError {
  message: String,
}

impl ConditionError {
  pub(crate) fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }

  pub fn message(&self) -> &str {
    &self.message
  }
}

impl std::fmt::Display for ConditionError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(&self.message)
  }
}

impl std::error::Error for ConditionError {}
