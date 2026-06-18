use oxisentinel::{AnalyzerConfig, RuntimeRole, describe_runtime};

#[test]
fn daemon_runtime_description_uses_package_defaults() {
  let config = AnalyzerConfig::default();

  assert_eq!(
    describe_runtime(RuntimeRole::Daemon, &config),
    "oxisentinel daemon listening on 127.0.0.1:8080"
  );
}
