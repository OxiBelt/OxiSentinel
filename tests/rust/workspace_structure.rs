use oxisentinel::{RuntimeRole, ServiceConfig, describe_runtime};

#[test]
fn service_runtime_description_uses_package_defaults() {
  let config = ServiceConfig::default();

  assert_eq!(
    describe_runtime(RuntimeRole::Service, &config),
    "oxisentinel service listening on 127.0.0.1:8080"
  );
}
