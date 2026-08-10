use super::super::source_map_reference::SourceMapReference;
use super::{SourceMapLoadError, SourceMapLoader};

/// 内联 data URI Source Map loader。
///
/// 支持 `data:...,...` 和 `data:...;base64,...`。它只负责把 data URI 解成
/// Source Map 文档字节，不校验 JSON 或 mappings。
#[derive(Debug, Default, Clone, Copy)]
pub struct DataUriSourceMapLoader;

impl SourceMapLoader for DataUriSourceMapLoader {
  fn load(
    &self,
    reference: &SourceMapReference,
  ) -> Result<Vec<u8>, SourceMapLoadError> {
    let SourceMapReference::InlineData(data_uri) = reference else {
      return Err(SourceMapLoadError::UnsupportedReferenceKind {
        expected: "inline data URI",
        actual: reference.kind(),
      });
    };

    decode_data_uri(data_uri)
  }
}

fn decode_data_uri(data_uri: &str) -> Result<Vec<u8>, SourceMapLoadError> {
  let Some(payload) = data_uri.strip_prefix("data:") else {
    return Err(SourceMapLoadError::InvalidDataUri(
      "missing data URI prefix".to_string(),
    ));
  };
  let Some((metadata, data)) = payload.split_once(',') else {
    return Err(SourceMapLoadError::InvalidDataUri(
      "missing data URI comma separator".to_string(),
    ));
  };

  if metadata
    .split(';')
    .any(|part| part.eq_ignore_ascii_case("base64"))
  {
    decode_base64(data)
  } else {
    decode_percent_encoded(data)
  }
}

fn decode_percent_encoded(value: &str) -> Result<Vec<u8>, SourceMapLoadError> {
  let bytes = value.as_bytes();
  let mut decoded = Vec::with_capacity(bytes.len());
  let mut index = 0;

  while index < bytes.len() {
    if bytes[index] == b'%' {
      if index + 2 >= bytes.len() {
        return Err(SourceMapLoadError::InvalidDataUri(
          "incomplete percent escape".to_string(),
        ));
      }

      let high = hex_value(bytes[index + 1])?;
      let low = hex_value(bytes[index + 2])?;
      decoded.push((high << 4) | low);
      index += 3;
    } else {
      decoded.push(bytes[index]);
      index += 1;
    }
  }

  Ok(decoded)
}

fn hex_value(byte: u8) -> Result<u8, SourceMapLoadError> {
  match byte {
    b'0'..=b'9' => Ok(byte - b'0'),
    b'a'..=b'f' => Ok(byte - b'a' + 10),
    b'A'..=b'F' => Ok(byte - b'A' + 10),
    _ => Err(SourceMapLoadError::InvalidDataUri(
      "invalid percent escape".to_string(),
    )),
  }
}

fn decode_base64(value: &str) -> Result<Vec<u8>, SourceMapLoadError> {
  let mut buffer = 0_u32;
  let mut bits = 0_u8;
  let mut decoded = Vec::with_capacity(value.len() * 3 / 4);
  let mut seen_padding = false;

  for byte in value.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
    if byte == b'=' {
      seen_padding = true;
      continue;
    }

    if seen_padding {
      return Err(SourceMapLoadError::InvalidDataUri(
        "base64 data after padding".to_string(),
      ));
    }

    let value = base64_value(byte)?;
    buffer = (buffer << 6) | u32::from(value);
    bits += 6;

    while bits >= 8 {
      bits -= 8;
      decoded.push((buffer >> bits) as u8);
      buffer &= (1 << bits) - 1;
    }
  }

  Ok(decoded)
}

fn base64_value(byte: u8) -> Result<u8, SourceMapLoadError> {
  match byte {
    b'A'..=b'Z' => Ok(byte - b'A'),
    b'a'..=b'z' => Ok(byte - b'a' + 26),
    b'0'..=b'9' => Ok(byte - b'0' + 52),
    b'+' => Ok(62),
    b'/' => Ok(63),
    _ => Err(SourceMapLoadError::InvalidDataUri(
      "invalid base64 character".to_string(),
    )),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn data_uri_loader_decodes_base64_source_maps() {
    let reference = SourceMapReference::inline_data(
      "data:application/json;base64,eyJ2ZXJzaW9uIjozfQ==",
    )
    .expect("data URI should be accepted");

    assert_eq!(
      DataUriSourceMapLoader.load(&reference).unwrap(),
      br#"{"version":3}"#,
    );
  }

  #[test]
  fn data_uri_loader_decodes_percent_encoded_source_maps() {
    let reference = SourceMapReference::inline_data(
      "data:application/json,%7B%22version%22%3A3%7D",
    )
    .expect("data URI should be accepted");

    assert_eq!(
      DataUriSourceMapLoader.load(&reference).unwrap(),
      br#"{"version":3}"#,
    );
  }
}
