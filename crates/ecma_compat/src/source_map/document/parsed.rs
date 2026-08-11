use std::path::PathBuf;

use serde_json::Value;
use sourcemap::{SourceMap, Token};

use super::SourceMapDocumentParseError;
use crate::source_map::{
  source_identity::SourceIdentity, source_location::SourceLocation,
  source_position::SourcePosition,
};

/// 已解析、可按 generated 位置查询 original 位置的 Source Map 文档。
///
/// 当前实现用 `sourcemap` crate 解析 JSON 和 VLQ mappings，但不把第三方类型泄漏到
/// resolver/detector 等上层模块。上层只依赖这个领域对象和项目自己的位置模型。
#[derive(Debug, Clone)]
pub struct SourceMapDocument {
  inner: SourceMap,
}

impl SourceMapDocument {
  pub fn parse(bytes: &[u8]) -> Result<Self, SourceMapDocumentParseError> {
    validate_source_map_version(bytes)?;

    let inner = SourceMap::from_slice(bytes)?;

    Ok(Self { inner })
  }

  pub fn lookup(&self, generated: SourcePosition) -> Option<SourceLocation> {
    self
      .lookup_mapped_token(generated)
      .map(|token| SourceLocation::new(token.source, token.original, None))
  }

  pub fn lookup_range(
    &self,
    generated_start: SourcePosition,
    generated_end: SourcePosition,
  ) -> Option<SourceLocation> {
    let start = self.lookup_mapped_token(generated_start)?;
    let end = self.lookup_mapped_token(generated_end);
    let original_end = end
      .filter(|end| start.can_form_range_with(end))
      .map(|end| end.original);

    Some(SourceLocation::new(
      start.source,
      start.original,
      original_end,
    ))
  }

  pub fn source_count(&self) -> u32 {
    self.inner.get_source_count()
  }

  pub fn source_contents(&self, source: &SourceIdentity) -> Option<&str> {
    (0..self.inner.get_source_count()).find_map(|source_id| {
      let candidate = source_identity(self.inner.get_source(source_id));

      if &candidate == source {
        self.inner.get_source_contents(source_id)
      } else {
        None
      }
    })
  }

  fn lookup_mapped_token(
    &self,
    generated: SourcePosition,
  ) -> Option<MappedToken> {
    let token = self.inner.lookup_token(generated.line(), generated.col())?;

    if !token.has_source() {
      return None;
    }

    Some(MappedToken::from_token(&token))
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MappedToken {
  raw_token: sourcemap::RawToken,
  source: SourceIdentity,
  original: SourcePosition,
}

impl MappedToken {
  fn from_token(token: &Token<'_>) -> Self {
    Self {
      raw_token: token.get_raw_token(),
      source: source_identity(token.get_source()),
      original: SourcePosition::new(token.get_src_line(), token.get_src_col()),
    }
  }

  fn can_form_range_with(&self, end: &Self) -> bool {
    self.raw_token == end.raw_token
      && self.raw_token.is_range
      && self.source == end.source
      && self.original <= end.original
  }
}

fn validate_source_map_version(
  bytes: &[u8],
) -> Result<(), SourceMapDocumentParseError> {
  let json = serde_json::from_slice::<Value>(bytes).map_err(|error| {
    SourceMapDocumentParseError::InvalidDocument(error.to_string())
  })?;

  match json.get("version").and_then(Value::as_u64) {
    Some(3) => Ok(()),
    Some(version) => {
      Err(SourceMapDocumentParseError::UnsupportedVersion(version))
    }
    None => Err(SourceMapDocumentParseError::MissingVersion),
  }
}

fn source_identity(source: Option<&str>) -> SourceIdentity {
  let Some(source) = source.filter(|source| !source.is_empty()) else {
    return SourceIdentity::Unknown;
  };

  if is_standard_url(source) {
    SourceIdentity::url(source).unwrap_or(SourceIdentity::Unknown)
  } else if source.contains("://") {
    SourceIdentity::virtual_source(source).unwrap_or(SourceIdentity::Unknown)
  } else {
    SourceIdentity::file(PathBuf::from(source))
  }
}

fn is_standard_url(source: &str) -> bool {
  source.starts_with("file://")
    || source.starts_with("http://")
    || source.starts_with("https://")
}

#[cfg(test)]
mod tests {
  use super::*;
  use sourcemap::SourceMapBuilder;

  #[test]
  fn parses_a_valid_source_map_document() {
    let document = SourceMapDocument::parse(valid_source_map()).unwrap();

    assert_eq!(document.source_count(), 1);
  }

  #[test]
  fn rejects_invalid_json_documents() {
    assert!(matches!(
      SourceMapDocument::parse(b"{"),
      Err(SourceMapDocumentParseError::InvalidDocument(_)),
    ));
  }

  #[test]
  fn rejects_unsupported_source_map_versions() {
    assert!(matches!(
      SourceMapDocument::parse(
        br#"{"version":2,"sources":["src/index.ts"],"names":[],"mappings":""}"#,
      ),
      Err(SourceMapDocumentParseError::UnsupportedVersion(2)),
    ));
  }

  #[test]
  fn looks_up_generated_positions_as_source_locations() {
    let document = SourceMapDocument::parse(valid_source_map()).unwrap();

    let location = document.lookup(SourcePosition::new(0, 0)).unwrap();

    assert_eq!(location.source(), &SourceIdentity::file("src/index.ts"),);
    assert_eq!(location.start(), SourcePosition::new(0, 0));
    assert_eq!(location.end(), None);
  }

  #[test]
  fn keeps_regular_source_map_ranges_open_ended() {
    let document = SourceMapDocument::parse(valid_source_map()).unwrap();

    let location = document
      .lookup_range(SourcePosition::new(0, 0), SourcePosition::new(0, 5))
      .unwrap();

    assert_eq!(location.source(), &SourceIdentity::file("src/index.ts"),);
    assert_eq!(location.start(), SourcePosition::new(0, 0));
    assert_eq!(location.end(), None);
  }

  #[test]
  fn maps_original_end_for_range_tokens() {
    let document = SourceMapDocument::parse(&range_source_map()).unwrap();

    let location = document
      .lookup_range(SourcePosition::new(0, 0), SourcePosition::new(0, 5))
      .unwrap();

    assert_eq!(location.source(), &SourceIdentity::file("src/index.ts"),);
    assert_eq!(location.start(), SourcePosition::new(10, 3));
    assert_eq!(location.end(), Some(SourcePosition::new(10, 8)));
  }

  #[test]
  fn returns_none_when_the_document_has_no_matching_mapping() {
    let document = SourceMapDocument::parse(
      br#"{"version":3,"sources":["src/index.ts"],"names":[],"mappings":""}"#,
    )
    .unwrap();

    assert_eq!(document.lookup(SourcePosition::new(0, 0)), None);
  }

  #[test]
  fn keeps_bundler_sources_as_virtual_identities() {
    let document = SourceMapDocument::parse(
      br#"{
        "version":3,
        "sources":["webpack://app/src/index.ts"],
        "names":[],
        "mappings":"AAAA"
      }"#,
    )
    .unwrap();

    let location = document.lookup(SourcePosition::new(0, 0)).unwrap();

    assert_eq!(
      location.source(),
      &SourceIdentity::virtual_source("webpack://app/src/index.ts").unwrap(),
    );
  }

  fn valid_source_map() -> &'static [u8] {
    br#"{
      "version":3,
      "sources":["src/index.ts"],
      "sourcesContent":["const value = 1;"],
      "names":[],
      "mappings":"AAAA"
    }"#
  }

  fn range_source_map() -> Vec<u8> {
    let mut builder = SourceMapBuilder::new(None);
    builder.add(0, 0, 10, 3, Some("src/index.ts"), None, true);

    let mut output = Vec::new();
    builder.into_sourcemap().to_writer(&mut output).unwrap();
    output
  }
}
