use oxisentinel::{RuntimeRole, ServiceConfig, describe_runtime};

fn main() {
  let config = ServiceConfig::default();
  println!("{}", describe_runtime(RuntimeRole::Service, &config));
}
