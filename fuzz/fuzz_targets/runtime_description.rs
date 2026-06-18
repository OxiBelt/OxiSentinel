#![no_main]

use libfuzzer_sys::fuzz_target;
use oxisentinel::{RuntimeRole, ServiceConfig, describe_runtime};

fuzz_target!(|input: &[u8]| {
  let bind_addr = String::from_utf8_lossy(input);
  let config = ServiceConfig::new("oxisentinel", bind_addr);
  let _ = describe_runtime(RuntimeRole::Service, &config);
});
