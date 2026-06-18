#![no_main]

use libfuzzer_sys::fuzz_target;
use oxisentinel::{AnalyzerConfig, RuntimeRole, describe_runtime};

fuzz_target!(|input: &[u8]| {
  let bind_addr = String::from_utf8_lossy(input);
  let config = AnalyzerConfig::new("oxisentinel", bind_addr);
  let _ = describe_runtime(RuntimeRole::Daemon, &config);
});
