#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceConfig {
  service_name: String,
  bind_addr: String,
}

impl ServiceConfig {
  pub fn new(service_name: impl Into<String>, bind_addr: impl Into<String>) -> Self {
    Self {
      service_name: service_name.into(),
      bind_addr: bind_addr.into(),
    }
  }

  pub fn service_name(&self) -> &str {
    &self.service_name
  }

  pub fn bind_addr(&self) -> &str {
    &self.bind_addr
  }
}

impl Default for ServiceConfig {
  fn default() -> Self {
    Self::new("oxisentinel", "127.0.0.1:8080")
  }
}
