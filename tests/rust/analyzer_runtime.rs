use std::process::{Command, Stdio};

use oxisentinel::{AnalyzerConfig, RuntimeRole, describe_runtime};

#[test]
fn daemon_runtime_description_uses_package_defaults() {
  let config = AnalyzerConfig::default();

  assert_eq!(
    describe_runtime(RuntimeRole::Daemon, &config),
    "oxisentinel daemon listening on 127.0.0.1:8080"
  );
}

#[test]
fn control_health_command_uses_package_defaults() {
  let output = Command::new(env!("CARGO_BIN_EXE_oxisentinelctl"))
    .arg("health")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("run oxisentinelctl health");

  assert!(
    output.status.success(),
    "stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
  assert!(stdout.contains("oxisentinel control listening on 127.0.0.1:8080"));
  assert!(stdout.contains("health: workspace scaffold is ready"));
}

#[test]
fn control_parse_command_is_not_exposed() {
  let output = Command::new(env!("CARGO_BIN_EXE_oxisentinelctl"))
    .arg("parse")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("run oxisentinelctl parse");

  assert!(!output.status.success(), "parse command should fail");

  let stderr = String::from_utf8(output.stderr).expect("stderr is utf-8");
  assert!(stderr.contains("unknown command: parse"));
  assert!(stderr.contains("oxisentinelctl health"));
}

#[test]
fn control_judgment_check_validates_config() {
  let path = std::env::temp_dir().join(format!(
    "oxisentinel-judgment-check-{}.toml",
    std::process::id()
  ));
  std::fs::write(
    &path,
    r#"
[condition]
enabled = true

[[condition.rules]]
name = "denied"
when = "Log.Message.contains('denied')"

[judgment]
enabled = true

[[judgment.handlers]]
name = "record-denied"
condition = "denied"

[[judgment.handlers.actions]]
type = "emit_decision"
severity = "warning"
"#,
  )
  .expect("write temp config");

  let output = Command::new(env!("CARGO_BIN_EXE_oxisentinelctl"))
    .args(["judgment", "check", "--config"])
    .arg(&path)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .output()
    .expect("run oxisentinelctl judgment check");
  let _ = std::fs::remove_file(&path);

  assert!(
    output.status.success(),
    "stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
  assert!(stdout.contains(r#""enabled":true"#));
  assert!(stdout.contains(r#""name":"denied""#));
}

#[test]
fn dockerfile_installs_runtime_and_control_binaries() {
  let dockerfile = include_str!("../../source/ops/Dockerfile");

  assert!(
    dockerfile.contains("/usr/local/bin/oxisentinel"),
    "runtime binary must be installed in the image"
  );
  assert!(
    dockerfile.contains("/usr/local/bin/oxisentinelctl"),
    "control utility must be installed in the image"
  );
  assert!(
    dockerfile.contains(r#"ENTRYPOINT ["/usr/local/bin/oxisentinel"]"#),
    "container entrypoint must stay on the analyzer runtime"
  );
  assert!(
    dockerfile.contains(r#"ENV PATH="/usr/local/bin:/usr/bin:/bin""#),
    "control utility must be reachable by name through docker exec"
  );
  assert!(
    dockerfile.contains("FROM build AS parser-tests"),
    "internal parser tests should have a Docker build target"
  );
}
