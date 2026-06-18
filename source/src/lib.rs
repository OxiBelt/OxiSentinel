pub mod config;
pub mod diagnostics;
pub mod runtime;

pub use config::AnalyzerConfig;
pub use diagnostics::{HealthReport, HealthStatus, health_report};
pub use runtime::{RuntimeRole, describe_runtime};

#[cfg(test)]
mod tests {
  use super::{AnalyzerConfig, HealthStatus, health_report};

  #[test]
  fn default_analyzer_config_names_oxisentinel() {
    let config = AnalyzerConfig::default();

    assert_eq!(config.program_name(), "oxisentinel");
    assert_eq!(config.bind_addr(), "127.0.0.1:8080");
  }

  #[test]
  fn health_report_starts_ready() {
    let report = health_report();

    assert!(matches!(report.status(), HealthStatus::Ready));
    assert!(report.is_ready());
  }
}
