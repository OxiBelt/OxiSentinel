#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HealthStatus {
  Ready,
  Degraded { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthReport {
  status: HealthStatus,
  detail: String,
}

impl HealthReport {
  pub fn new(status: HealthStatus, detail: impl Into<String>) -> Self {
    Self {
      status,
      detail: detail.into(),
    }
  }

  pub fn status(&self) -> &HealthStatus {
    &self.status
  }

  pub fn detail(&self) -> &str {
    &self.detail
  }

  pub fn is_ready(&self) -> bool {
    matches!(self.status, HealthStatus::Ready)
  }
}

pub fn health_report() -> HealthReport {
  HealthReport::new(HealthStatus::Ready, "workspace scaffold is ready")
}
