use std::collections::BTreeMap;

pub(super) type JsonObject = BTreeMap<String, JsonValue>;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum JsonValue {
  Null,
  Bool(bool),
  Number(String),
  String(String),
  Array(Vec<JsonValue>),
  Object(JsonObject),
}

impl JsonValue {
  pub(super) fn to_compact_json(&self) -> String {
    let mut output = String::new();
    self.push_compact_json(&mut output);
    output
  }

  fn push_compact_json(&self, output: &mut String) {
    match self {
      Self::Null => output.push_str("null"),
      Self::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
      Self::Number(value) => output.push_str(value),
      Self::String(value) => push_json_string(output, value),
      Self::Array(values) => {
        output.push('[');
        for (index, value) in values.iter().enumerate() {
          if index > 0 {
            output.push(',');
          }
          value.push_compact_json(output);
        }
        output.push(']');
      }
      Self::Object(values) => {
        output.push('{');
        for (index, (key, value)) in values.iter().enumerate() {
          if index > 0 {
            output.push(',');
          }
          push_json_string(output, key);
          output.push(':');
          value.push_compact_json(output);
        }
        output.push('}');
      }
    }
  }
}

pub(super) fn parse_json_object(input: &str) -> Result<JsonObject, String> {
  let mut parser = JsonParser::new(input);
  let value = parser.parse_value()?;
  parser.skip_whitespace();

  if !parser.is_eof() {
    return Err("trailing characters after JSON value".to_owned());
  }

  match value {
    JsonValue::Object(object) => Ok(object),
    _ => Err("top-level JSON value must be an object".to_owned()),
  }
}

pub(super) fn push_json_string(output: &mut String, value: &str) {
  output.push('"');
  for character in value.chars() {
    match character {
      '"' => output.push_str("\\\""),
      '\\' => output.push_str("\\\\"),
      '\n' => output.push_str("\\n"),
      '\r' => output.push_str("\\r"),
      '\t' => output.push_str("\\t"),
      '\u{08}' => output.push_str("\\b"),
      '\u{0C}' => output.push_str("\\f"),
      character if character < ' ' => {
        output.push_str("\\u");
        output.push_str(&format!("{:04x}", character as u32));
      }
      character => output.push(character),
    }
  }
  output.push('"');
}

pub(super) fn push_json_pair(output: &mut String, key: &str, value: &str, needs_comma: bool) {
  if needs_comma {
    output.push(',');
  }
  push_json_string(output, key);
  output.push(':');
  push_json_string(output, value);
}

struct JsonParser<'a> {
  input: &'a str,
  index: usize,
}

impl<'a> JsonParser<'a> {
  fn new(input: &'a str) -> Self {
    Self { input, index: 0 }
  }

  fn parse_value(&mut self) -> Result<JsonValue, String> {
    self.skip_whitespace();

    match self.peek_byte() {
      Some(b'{') => self.parse_object().map(JsonValue::Object),
      Some(b'[') => self.parse_array().map(JsonValue::Array),
      Some(b'"') => self.parse_string().map(JsonValue::String),
      Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
      Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
      Some(b'n') => self.parse_literal("null", JsonValue::Null),
      Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
      Some(other) => Err(format!("unexpected byte '{}'", other as char)),
      None => Err("unexpected end of JSON input".to_owned()),
    }
  }

  fn parse_object(&mut self) -> Result<JsonObject, String> {
    self.expect_byte(b'{')?;
    self.skip_whitespace();

    let mut object = JsonObject::new();

    if self.consume_byte(b'}') {
      return Ok(object);
    }

    loop {
      self.skip_whitespace();
      let key = self.parse_string()?;
      self.skip_whitespace();
      self.expect_byte(b':')?;
      let value = self.parse_value()?;
      object.insert(key, value);
      self.skip_whitespace();

      if self.consume_byte(b'}') {
        break;
      }

      self.expect_byte(b',')?;
    }

    Ok(object)
  }

  fn parse_array(&mut self) -> Result<Vec<JsonValue>, String> {
    self.expect_byte(b'[')?;
    self.skip_whitespace();

    let mut values = Vec::new();

    if self.consume_byte(b']') {
      return Ok(values);
    }

    loop {
      values.push(self.parse_value()?);
      self.skip_whitespace();

      if self.consume_byte(b']') {
        break;
      }

      self.expect_byte(b',')?;
    }

    Ok(values)
  }

  fn parse_string(&mut self) -> Result<String, String> {
    self.expect_byte(b'"')?;
    let mut output = String::new();

    while !self.is_eof() {
      let character = self
        .next_char()
        .ok_or_else(|| "unterminated string".to_owned())?;

      match character {
        '"' => return Ok(output),
        '\\' => output.push(self.parse_escape()?),
        character if character < ' ' => return Err("control character in string".to_owned()),
        character => output.push(character),
      }
    }

    Err("unterminated string".to_owned())
  }

  fn parse_escape(&mut self) -> Result<char, String> {
    match self.next_char() {
      Some('"') => Ok('"'),
      Some('\\') => Ok('\\'),
      Some('/') => Ok('/'),
      Some('b') => Ok('\u{08}'),
      Some('f') => Ok('\u{0C}'),
      Some('n') => Ok('\n'),
      Some('r') => Ok('\r'),
      Some('t') => Ok('\t'),
      Some('u') => self.parse_unicode_escape(),
      Some(character) => Err(format!("unsupported escape sequence \\{character}")),
      None => Err("unterminated escape sequence".to_owned()),
    }
  }

  fn parse_unicode_escape(&mut self) -> Result<char, String> {
    let mut value = 0_u32;

    for _ in 0..4 {
      let Some(character) = self.next_char() else {
        return Err("unterminated unicode escape".to_owned());
      };
      let Some(digit) = character.to_digit(16) else {
        return Err("invalid unicode escape".to_owned());
      };
      value = (value << 4) | digit;
    }

    char::from_u32(value).ok_or_else(|| "invalid unicode scalar".to_owned())
  }

  fn parse_number(&mut self) -> Result<String, String> {
    let start = self.index;

    if self.consume_byte(b'-') && self.is_eof() {
      return Err("invalid number".to_owned());
    }

    let digit_start = self.index;
    self.consume_digits();

    if digit_start == self.index {
      return Err("invalid number".to_owned());
    }

    if self.consume_byte(b'.') {
      if !self.peek_byte().is_some_and(|byte| byte.is_ascii_digit()) {
        return Err("invalid number fraction".to_owned());
      }
      self.consume_digits();
    }

    if self
      .peek_byte()
      .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
      self.index += 1;
      if self
        .peek_byte()
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
      {
        self.index += 1;
      }
      if !self.peek_byte().is_some_and(|byte| byte.is_ascii_digit()) {
        return Err("invalid number exponent".to_owned());
      }
      self.consume_digits();
    }

    if start == self.index {
      return Err("invalid number".to_owned());
    }

    Ok(self.input[start..self.index].to_owned())
  }

  fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, String> {
    if self.input[self.index..].starts_with(literal) {
      self.index += literal.len();
      Ok(value)
    } else {
      Err(format!("expected literal {literal}"))
    }
  }

  fn consume_digits(&mut self) {
    while self.peek_byte().is_some_and(|byte| byte.is_ascii_digit()) {
      self.index += 1;
    }
  }

  fn skip_whitespace(&mut self) {
    while self
      .peek_byte()
      .is_some_and(|byte| byte.is_ascii_whitespace())
    {
      self.index += 1;
    }
  }

  fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
    if self.consume_byte(expected) {
      Ok(())
    } else {
      Err(format!("expected '{}'", expected as char))
    }
  }

  fn consume_byte(&mut self, expected: u8) -> bool {
    if self.peek_byte() == Some(expected) {
      self.index += 1;
      true
    } else {
      false
    }
  }

  fn next_char(&mut self) -> Option<char> {
    let character = self.input[self.index..].chars().next()?;
    self.index += character.len_utf8();
    Some(character)
  }

  fn peek_byte(&self) -> Option<u8> {
    self.input.as_bytes().get(self.index).copied()
  }

  fn is_eof(&self) -> bool {
    self.index >= self.input.len()
  }
}
