use std::net::IpAddr;

use online_dsl_forge::{
  BodyAccess, CapabilityMeta, DynamicRegistry, EvalError, RegexFlavor, RuntimeCallContext,
  RuntimePatternSets, SourceSpan, Value, register_oxirule_pattern_set_methods,
};
use regex::Regex;

pub(crate) fn condition_registry(pattern_sets: RuntimePatternSets) -> DynamicRegistry {
  let mut registry = DynamicRegistry::new();
  register_oxirule_pattern_set_methods(&mut registry, pattern_sets);
  register_string_methods(&mut registry);
  register_collection_methods(&mut registry);
  registry
}

fn register_string_methods(registry: &mut DynamicRegistry) {
  registry.register_method_capability(
    CapabilityMeta::method("contains", 1).with_body_access(BodyAccess::PrefixBytes),
    |receiver, args| match (receiver, &args[0]) {
      (Value::String(receiver), Value::String(needle)) => {
        Ok(Value::Bool(receiver.contains(needle)))
      }
      (Value::Array(values), needle) => Ok(Value::Bool(values.iter().any(|value| value == needle))),
      (other, _) => Err(error(format!(
        "contains requires string or array receiver, got {}",
        other.type_name()
      ))),
    },
  );
  registry.register_method("startsWith", 1, string_predicate("startsWith"));
  registry.register_method("endsWith", 1, string_predicate("endsWith"));
  registry.register_method("lowerAscii", 0, |receiver, _| match receiver {
    Value::String(value) => Ok(Value::String(value.to_ascii_lowercase())),
    other => Err(error(format!(
      "lowerAscii requires string receiver, got {}",
      other.type_name()
    ))),
  });
  registry.register_method("upperAscii", 0, |receiver, _| match receiver {
    Value::String(value) => Ok(Value::String(value.to_ascii_uppercase())),
    other => Err(error(format!(
      "upperAscii requires string receiver, got {}",
      other.type_name()
    ))),
  });
  registry.register_method("size", 0, value_size);
  registry.register_method("inCidr", 1, |receiver, args| match (receiver, &args[0]) {
    (Value::String(ip), Value::String(cidr)) => ip_in_cidr(ip, cidr).map(Value::Bool),
    (Value::String(_), other) => Err(error(format!(
      "inCidr requires string CIDR argument, got {}",
      other.type_name()
    ))),
    (other, _) => Err(error(format!(
      "inCidr requires string IP receiver, got {}",
      other.type_name()
    ))),
  });
  registry.register_method_capability_with_context(
    CapabilityMeta::method("matches", 1)
      .with_body_access(BodyAccess::PrefixBytes)
      .with_regex_arg(0, RegexFlavor::Default),
    regex_string_method("matches"),
  );
}

fn register_collection_methods(registry: &mut DynamicRegistry) {
  registry.register_method("count", 0, value_size);
  registry.register_method("has", 1, |receiver, args| {
    key_or_value_exists(receiver, &args[0])
  });
  registry.register_method("get", 1, |receiver, args| match (receiver, &args[0]) {
    (Value::Object(values), Value::String(key)) => {
      Ok(values.get(key).cloned().unwrap_or(Value::Null))
    }
    (Value::Object(_), other) => Err(error(format!(
      "get requires string key, got {}",
      other.type_name()
    ))),
    (other, _) => Err(error(format!(
      "get requires object receiver, got {}",
      other.type_name()
    ))),
  });
  registry.register_method("getAll", 1, |receiver, args| match (receiver, &args[0]) {
    (Value::Object(values), Value::String(key)) => match values.get(key) {
      Some(Value::Array(values)) => Ok(Value::Array(values.clone())),
      Some(value) => Ok(Value::Array(vec![value.clone()])),
      None => Ok(Value::Array(Vec::new())),
    },
    (Value::Object(_), other) => Err(error(format!(
      "getAll requires string key, got {}",
      other.type_name()
    ))),
    (other, _) => Err(error(format!(
      "getAll requires object receiver, got {}",
      other.type_name()
    ))),
  });
  registry.register_method("anyValueContains", 1, |receiver, args| {
    let needle =
      expect_string(&args[0], "anyValueContains").map_err(|message| error(message.message))?;
    Ok(Value::Bool(
      string_values(receiver).any(|value| value.contains(needle)),
    ))
  });
  registry.register_method_capability_with_context(
    CapabilityMeta::method("anyNameMatches", 1)
      .with_regex_arg(0, RegexFlavor::Default)
      .with_regex_arg(0, RegexFlavor::HeaderName),
    regex_collection_method(CollectionRegexMode::KeysAny),
  );
  registry.register_method_capability_with_context(
    CapabilityMeta::method("anyValueMatches", 1).with_regex_arg(0, RegexFlavor::Default),
    regex_collection_method(CollectionRegexMode::ValuesAny),
  );
  registry.register_method_capability_with_context(
    CapabilityMeta::method("anyKeyMatches", 1).with_regex_arg(0, RegexFlavor::Default),
    regex_collection_method(CollectionRegexMode::KeysAny),
  );
  registry.register_method_capability_with_context(
    CapabilityMeta::method("anyMatches", 1).with_regex_arg(0, RegexFlavor::Default),
    regex_collection_method(CollectionRegexMode::Any),
  );
  registry.register_method_capability_with_context(
    CapabilityMeta::method("anyEntryMatches", 2)
      .with_regex_arg(0, RegexFlavor::Default)
      .with_regex_arg(0, RegexFlavor::HeaderName)
      .with_regex_arg(1, RegexFlavor::Default),
    regex_entry_method(false),
  );
  registry.register_method_capability_with_context(
    CapabilityMeta::method("allEntriesMatch", 2)
      .with_regex_arg(0, RegexFlavor::HeaderName)
      .with_regex_arg(1, RegexFlavor::Default),
    regex_entry_method(true),
  );
}

fn string_predicate(
  method: &'static str,
) -> impl Fn(&Value, &[Value]) -> Result<Value, EvalError> + Send + Sync + 'static {
  move |receiver, args| match (receiver, &args[0]) {
    (Value::String(receiver), Value::String(argument)) => {
      let matched = match method {
        "startsWith" => receiver.starts_with(argument),
        "endsWith" => receiver.ends_with(argument),
        _ => false,
      };
      Ok(Value::Bool(matched))
    }
    (Value::String(_), other) => Err(error(format!(
      "{method} requires string argument, got {}",
      other.type_name()
    ))),
    (other, _) => Err(error(format!(
      "{method} requires string receiver, got {}",
      other.type_name()
    ))),
  }
}

fn regex_string_method(
  method: &'static str,
) -> impl for<'a> Fn(RuntimeCallContext<'a>, &Value, &[Value]) -> Result<Value, EvalError>
+ Send
+ Sync
+ 'static {
  move |context, receiver, args| match (receiver, &args[0]) {
    (Value::String(receiver), Value::String(pattern)) => {
      regex_is_match(context.span(), pattern, receiver).map(Value::Bool)
    }
    (Value::String(_), other) => Err(EvalError::new(
      format!(
        "{method} requires string pattern, got {}",
        other.type_name()
      ),
      context.span(),
    )),
    (other, _) => Err(EvalError::new(
      format!(
        "{method} requires string receiver, got {}",
        other.type_name()
      ),
      context.span(),
    )),
  }
}

#[derive(Clone, Copy)]
enum CollectionRegexMode {
  KeysAny,
  ValuesAny,
  Any,
}

fn regex_collection_method(
  mode: CollectionRegexMode,
) -> impl for<'a> Fn(RuntimeCallContext<'a>, &Value, &[Value]) -> Result<Value, EvalError>
+ Send
+ Sync
+ 'static {
  move |context, receiver, args| {
    let pattern = expect_string(&args[0], "regex collection method")
      .map_err(|error| EvalError::new(error.message, context.span()))?;
    let regex = Regex::new(pattern).map_err(|error| {
      EvalError::new(
        format!("invalid regex pattern {pattern}: {error}"),
        context.span(),
      )
    })?;
    let matched = match (mode, receiver) {
      (CollectionRegexMode::KeysAny, Value::Object(values)) => {
        values.keys().any(|value| regex.is_match(value))
      }
      (CollectionRegexMode::ValuesAny, value) => {
        string_values(value).any(|value| regex.is_match(value))
      }
      (CollectionRegexMode::Any, Value::Object(values)) => values.iter().any(|(key, value)| {
        regex.is_match(key) || value.as_string().is_some_and(|value| regex.is_match(value))
      }),
      (CollectionRegexMode::Any, value) => string_values(value).any(|value| regex.is_match(value)),
      (_, other) => {
        return Err(EvalError::new(
          format!(
            "regex collection method requires object, got {}",
            other.type_name()
          ),
          context.span(),
        ));
      }
    };
    Ok(Value::Bool(matched))
  }
}

fn regex_entry_method(
  require_all: bool,
) -> impl for<'a> Fn(RuntimeCallContext<'a>, &Value, &[Value]) -> Result<Value, EvalError>
+ Send
+ Sync
+ 'static {
  move |context, receiver, args| {
    let key_pattern = expect_string(&args[0], "entry key pattern")
      .map_err(|error| EvalError::new(error.message, context.span()))?;
    let value_pattern = expect_string(&args[1], "entry value pattern")
      .map_err(|error| EvalError::new(error.message, context.span()))?;
    let key_regex = Regex::new(key_pattern).map_err(|error| {
      EvalError::new(
        format!("invalid entry key regex pattern {key_pattern}: {error}"),
        context.span(),
      )
    })?;
    let value_regex = Regex::new(value_pattern).map_err(|error| {
      EvalError::new(
        format!("invalid entry value regex pattern {value_pattern}: {error}"),
        context.span(),
      )
    })?;
    let Value::Object(values) = receiver else {
      return Err(EvalError::new(
        format!(
          "entry regex methods require object receiver, got {}",
          receiver.type_name()
        ),
        context.span(),
      ));
    };
    let predicate = |(key, value): (&String, &Value)| {
      key_regex.is_match(key)
        && value
          .as_string()
          .is_some_and(|value| value_regex.is_match(value))
    };
    let matched = if require_all {
      values.iter().all(predicate)
    } else {
      values.iter().any(predicate)
    };
    Ok(Value::Bool(matched))
  }
}

fn value_size(receiver: &Value, _args: &[Value]) -> Result<Value, EvalError> {
  let size = match receiver {
    Value::String(value) => value.len(),
    Value::Array(value) => value.len(),
    Value::Object(value) => value.len(),
    other => {
      return Err(error(format!(
        "size requires string, array, or object receiver, got {}",
        other.type_name()
      )));
    }
  };
  i64::try_from(size)
    .map(Value::Int)
    .map_err(|_| error("size does not fit in i64"))
}

fn key_or_value_exists(receiver: &Value, needle: &Value) -> Result<Value, EvalError> {
  match (receiver, needle) {
    (Value::Object(values), Value::String(key)) => Ok(Value::Bool(values.contains_key(key))),
    (Value::Array(values), needle) => Ok(Value::Bool(values.iter().any(|value| value == needle))),
    (Value::String(value), Value::String(needle)) => Ok(Value::Bool(value.contains(needle))),
    (Value::Object(_), other) => Err(error(format!(
      "has requires string key, got {}",
      other.type_name()
    ))),
    (other, _) => Err(error(format!(
      "has requires object, array, or string receiver, got {}",
      other.type_name()
    ))),
  }
}

fn string_values(value: &Value) -> Box<dyn Iterator<Item = &str> + '_> {
  match value {
    Value::String(value) => Box::new(std::iter::once(value.as_str())),
    Value::Array(values) => Box::new(values.iter().filter_map(ValueString::as_string)),
    Value::Object(values) => Box::new(values.values().filter_map(ValueString::as_string)),
    _ => Box::new(std::iter::empty()),
  }
}

trait ValueString {
  fn as_string(&self) -> Option<&str>;
}

impl ValueString for Value {
  fn as_string(&self) -> Option<&str> {
    match self {
      Value::String(value) => Some(value),
      _ => None,
    }
  }
}

struct LocalEvalMessage {
  message: String,
}

fn expect_string<'a>(value: &'a Value, operation: &str) -> Result<&'a str, LocalEvalMessage> {
  match value {
    Value::String(value) => Ok(value),
    other => Err(LocalEvalMessage {
      message: format!(
        "{operation} requires string argument, got {}",
        other.type_name()
      ),
    }),
  }
}

fn regex_is_match(span: SourceSpan, pattern: &str, value: &str) -> Result<bool, EvalError> {
  Regex::new(pattern)
    .map_err(|error| EvalError::new(format!("invalid regex pattern {pattern}: {error}"), span))
    .map(|regex| regex.is_match(value))
}

fn ip_in_cidr(ip: &str, cidr: &str) -> Result<bool, EvalError> {
  let ip = ip
    .parse::<IpAddr>()
    .map_err(|parse_error| error(format!("invalid IP address {ip}: {parse_error}")))?;
  let (network, prefix) = cidr
    .split_once('/')
    .ok_or_else(|| error(format!("invalid CIDR {cidr}: missing prefix")))?;
  let network = network
    .parse::<IpAddr>()
    .map_err(|parse_error| error(format!("invalid CIDR network {network}: {parse_error}")))?;
  let prefix = prefix
    .parse::<u32>()
    .map_err(|parse_error| error(format!("invalid CIDR prefix {prefix}: {parse_error}")))?;
  match (ip, network) {
    (IpAddr::V4(ip), IpAddr::V4(network)) if prefix <= 32 => {
      let mask = if prefix == 0 {
        0
      } else {
        u32::MAX << (32 - prefix)
      };
      Ok((u32::from(ip) & mask) == (u32::from(network) & mask))
    }
    (IpAddr::V6(ip), IpAddr::V6(network)) if prefix <= 128 => {
      let mask = if prefix == 0 {
        0
      } else {
        u128::MAX << (128 - prefix)
      };
      Ok((u128::from(ip) & mask) == (u128::from(network) & mask))
    }
    (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => {
      Err(error(format!("CIDR prefix {prefix} is out of range")))
    }
    _ => Ok(false),
  }
}

fn error(message: impl Into<String>) -> EvalError {
  EvalError::new(message, SourceSpan::default())
}
