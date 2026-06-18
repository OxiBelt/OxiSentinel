use crate::ServiceConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeRole {
  Service,
  Control,
}

pub fn describe_runtime(role: RuntimeRole, config: &ServiceConfig) -> String {
  let role_name = match role {
    RuntimeRole::Service => "service",
    RuntimeRole::Control => "control",
  };

  format!(
    "{} {} listening on {}",
    config.service_name(),
    role_name,
    config.bind_addr()
  )
}
