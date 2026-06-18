use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageBuildTarget {
  platform: &'static str,
  rust_target: &'static str,
  target_cpu: Option<&'static str>,
  artifact_suffix: &'static str,
}

impl ImageBuildTarget {
  pub const fn platform(&self) -> &'static str {
    self.platform
  }

  pub const fn rust_target(&self) -> &'static str {
    self.rust_target
  }

  pub const fn target_cpu(&self) -> Option<&'static str> {
    self.target_cpu
  }

  pub fn artifact_name(&self, binary: &str) -> String {
    format!("{binary}-{}", self.artifact_suffix)
  }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetError {
  platform: String,
  target_cpu: String,
}

impl TargetError {
  fn new(platform: impl Into<String>, target_cpu: impl Into<String>) -> Self {
    Self {
      platform: platform.into(),
      target_cpu: target_cpu.into(),
    }
  }
}

impl fmt::Display for TargetError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      formatter,
      "unsupported image target platform={} target_cpu={}",
      self.platform, self.target_cpu
    )
  }
}

impl std::error::Error for TargetError {}

pub fn validate_image_target(
  platform: &str,
  target_cpu: Option<&str>,
) -> Result<ImageBuildTarget, TargetError> {
  let normalized_cpu = target_cpu
    .filter(|value| !value.is_empty())
    .unwrap_or("generic");

  match (platform, normalized_cpu) {
    ("linux/amd64", "x86-64-v2") => Ok(amd64("x86-64-v2", "linux-amd64-x86-64-v2")),
    ("linux/amd64", "x86-64-v3") => Ok(amd64("x86-64-v3", "linux-amd64-x86-64-v3")),
    ("linux/amd64", "x86-64-v4") => Ok(amd64("x86-64-v4", "linux-amd64-x86-64-v4")),
    ("linux/arm64", "generic" | "arm64") => Ok(ImageBuildTarget {
      platform: "linux/arm64",
      rust_target: "aarch64-unknown-linux-musl",
      target_cpu: None,
      artifact_suffix: "linux-arm64-generic",
    }),
    ("linux/riscv64", "generic" | "riscv64gc") => Ok(ImageBuildTarget {
      platform: "linux/riscv64",
      rust_target: "riscv64gc-unknown-linux-musl",
      target_cpu: None,
      artifact_suffix: "linux-riscv64-riscv64gc",
    }),
    _ => Err(TargetError::new(platform, normalized_cpu)),
  }
}

fn amd64(target_cpu: &'static str, artifact_suffix: &'static str) -> ImageBuildTarget {
  ImageBuildTarget {
    platform: "linux/amd64",
    rust_target: "x86_64-unknown-linux-musl",
    target_cpu: Some(target_cpu),
    artifact_suffix,
  }
}

#[cfg(test)]
mod tests {
  use super::validate_image_target;

  #[test]
  fn validates_supported_x86_64_cpu_levels() {
    for target_cpu in ["x86-64-v2", "x86-64-v3", "x86-64-v4"] {
      let target = validate_image_target("linux/amd64", Some(target_cpu)).expect("target valid");

      assert_eq!(target.platform(), "linux/amd64");
      assert_eq!(target.rust_target(), "x86_64-unknown-linux-musl");
      assert_eq!(target.target_cpu(), Some(target_cpu));
      assert_eq!(
        target.artifact_name("oxisentinel"),
        format!("oxisentinel-linux-amd64-{target_cpu}")
      );
    }
  }

  #[test]
  fn validates_arm64_and_riscv64_targets() {
    let arm64 = validate_image_target("linux/arm64", Some("generic")).expect("arm64 valid");
    let riscv64 = validate_image_target("linux/riscv64", None).expect("riscv64 valid");

    assert_eq!(arm64.rust_target(), "aarch64-unknown-linux-musl");
    assert_eq!(arm64.target_cpu(), None);
    assert_eq!(
      arm64.artifact_name("oxisentinelctl"),
      "oxisentinelctl-linux-arm64-generic"
    );
    assert_eq!(riscv64.rust_target(), "riscv64gc-unknown-linux-musl");
    assert_eq!(
      riscv64.artifact_name("oxisentinel"),
      "oxisentinel-linux-riscv64-riscv64gc"
    );
  }

  #[test]
  fn rejects_unsupported_cpu_for_platform() {
    let error =
      validate_image_target("linux/arm64", Some("x86-64-v3")).expect_err("target invalid");

    assert_eq!(
      error.to_string(),
      "unsupported image target platform=linux/arm64 target_cpu=x86-64-v3"
    );
  }
}
