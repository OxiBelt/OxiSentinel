use std::io::Write;
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
fn control_parse_command_reads_stdin_and_emits_ndjson() {
  let mut command = Command::new(env!("CARGO_BIN_EXE_oxisentinelctl"))
    .args(["parse", "--source", "auto", "--input", "-"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("spawn oxisentinelctl");

  command
    .stdin
    .as_mut()
    .expect("stdin available")
    .write_all(
      br#"{"log":"{\"level\":\"INFO\",\"service\":\"oxibelt\",\"msg\":\"allowed\"}\n","stream":"stdout","time":"2026-06-18T10:00:00.000000000Z"}
"#,
    )
    .expect("write stdin");

  let output = command.wait_with_output().expect("read output");

  assert!(
    output.status.success(),
    "stderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");

  assert!(stdout.contains(r#""schema":"oxisentinel.log.v1""#));
  assert!(stdout.contains(r#""source":"docker_logs""#));
  assert!(stdout.contains(r#""level":"info""#));
  assert!(stdout.contains(r#""service":"oxibelt""#));
  assert!(stdout.contains(r#""message":"allowed""#));
  assert!(stdout.contains(r#""stream":"stdout""#));
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
}
