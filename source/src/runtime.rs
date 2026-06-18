use crate::AnalyzerConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeRole {
  Daemon,
  Control,
}

pub fn describe_runtime(role: RuntimeRole, config: &AnalyzerConfig) -> String {
  let role_name = match role {
    RuntimeRole::Daemon => "daemon",
    RuntimeRole::Control => "control",
  };

  format!(
    "{} {} listening on {}",
    config.program_name(),
    role_name,
    config.bind_addr()
  )
}
