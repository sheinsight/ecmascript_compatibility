use crate::source::SourceFile;

use super::{
  discovery::{SourceMapDiscovery, SourceMapDiscoveryError},
  document::{SourceMapDocument, SourceMapDocumentParseError},
  loader::{SourceMapLoadError, SourceMapLoader},
  source_map_discovery_kind::SourceMapDiscoveryKind,
  source_map_reference::SourceMapReference,
};

/// 已完成发现、加载和解析的 Source Map 文档。
///
/// resolver 的产物不是裸字节，而是后续可以直接执行 generated -> original
/// 查询的文档对象。这样 detector/mapper 不需要知道 Source Map 从哪里加载而来。
#[derive(Debug, Clone)]
pub struct ResolvedSourceMap {
  discovery_kind: SourceMapDiscoveryKind,
  reference: SourceMapReference,
  document: SourceMapDocument,
}

impl ResolvedSourceMap {
  pub fn new(
    discovery_kind: SourceMapDiscoveryKind,
    reference: SourceMapReference,
    document: SourceMapDocument,
  ) -> Self {
    Self {
      discovery_kind,
      reference,
      document,
    }
  }

  pub const fn discovery_kind(&self) -> SourceMapDiscoveryKind {
    self.discovery_kind
  }

  pub const fn reference(&self) -> &SourceMapReference {
    &self.reference
  }

  pub const fn document(&self) -> &SourceMapDocument {
    &self.document
  }
}

/// Source Map 解析编排失败原因。
///
/// 加载失败和文档解析失败属于不同阶段：前者说明引用不可读取，后者说明读取到的
/// 字节不是当前支持的 Source Map 文档。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceMapResolveError {
  #[error(transparent)]
  Discovery(SourceMapDiscoveryError),
  #[error(transparent)]
  Load(SourceMapLoadError),
  #[error(transparent)]
  Parse(SourceMapDocumentParseError),
}

impl From<SourceMapDiscoveryError> for SourceMapResolveError {
  fn from(error: SourceMapDiscoveryError) -> Self {
    Self::Discovery(error)
  }
}

impl From<SourceMapLoadError> for SourceMapResolveError {
  fn from(error: SourceMapLoadError) -> Self {
    Self::Load(error)
  }
}

impl From<SourceMapDocumentParseError> for SourceMapResolveError {
  fn from(error: SourceMapDocumentParseError) -> Self {
    Self::Parse(error)
  }
}

/// Source Map 解析编排入口。
///
/// resolver 不直接解析 `sourceMappingURL`。它先让 discovery 产出候选引用，再负责
/// 调 loader 取字节、调 document 解析，并保留 fallback NotFound 的降级语义。
#[derive(Debug, Default, Clone, Copy)]
pub struct SourceMapResolver {
  discovery: SourceMapDiscovery,
}

impl SourceMapResolver {
  pub const fn new() -> Self {
    Self {
      discovery: SourceMapDiscovery::new(),
    }
  }

  pub fn resolve_source_file<L>(
    &self,
    source: &SourceFile,
    loader: &L,
  ) -> Result<Option<ResolvedSourceMap>, SourceMapResolveError>
  where
    L: SourceMapLoader,
  {
    let discovered = self.discovery.discover(source)?;
    let (discovery_kind, reference) = discovered.into_parts();

    match self.load_reference(discovery_kind, reference, loader) {
      Ok(resolved) => Ok(Some(resolved)),
      Err(SourceMapResolveError::Load(SourceMapLoadError::NotFound(_)))
        if discovery_kind == SourceMapDiscoveryKind::AdjacentFallback =>
      {
        Ok(None)
      }
      Err(error) => Err(error),
    }
  }

  pub fn load_reference<L>(
    &self,
    discovery_kind: SourceMapDiscoveryKind,
    reference: SourceMapReference,
    loader: &L,
  ) -> Result<ResolvedSourceMap, SourceMapResolveError>
  where
    L: SourceMapLoader,
  {
    let bytes = loader.load(&reference)?;
    let document = SourceMapDocument::parse(&bytes)?;

    Ok(ResolvedSourceMap::new(discovery_kind, reference, document))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::source_map::{
    source_identity::SourceIdentity, source_position::SourcePosition,
  };
  use std::path::PathBuf;

  struct StaticLoader;

  impl SourceMapLoader for StaticLoader {
    fn load(
      &self,
      _reference: &SourceMapReference,
    ) -> Result<Vec<u8>, SourceMapLoadError> {
      Ok(valid_source_map().to_vec())
    }
  }

  struct FailingLoader;

  impl SourceMapLoader for FailingLoader {
    fn load(
      &self,
      _reference: &SourceMapReference,
    ) -> Result<Vec<u8>, SourceMapLoadError> {
      Err(SourceMapLoadError::UnsupportedRemoteUrl(
        "https://example.com/app.js.map".to_string(),
      ))
    }
  }

  struct InvalidDocumentLoader;

  impl SourceMapLoader for InvalidDocumentLoader {
    fn load(
      &self,
      _reference: &SourceMapReference,
    ) -> Result<Vec<u8>, SourceMapLoadError> {
      Ok(br#"{"version":2,"sources":[],"names":[],"mappings":""}"#.to_vec())
    }
  }

  struct MissingLoader;

  impl SourceMapLoader for MissingLoader {
    fn load(
      &self,
      reference: &SourceMapReference,
    ) -> Result<Vec<u8>, SourceMapLoadError> {
      match reference {
        SourceMapReference::LocalFile(path) => {
          Err(SourceMapLoadError::NotFound(path.clone()))
        }
        SourceMapReference::InlineData(_)
        | SourceMapReference::RemoteUrl(_) => Ok(valid_source_map().to_vec()),
      }
    }
  }

  struct ExpectingLoader {
    expected: SourceMapReference,
  }

  impl SourceMapLoader for ExpectingLoader {
    fn load(
      &self,
      reference: &SourceMapReference,
    ) -> Result<Vec<u8>, SourceMapLoadError> {
      assert_eq!(reference, &self.expected);

      Ok(valid_source_map().to_vec())
    }
  }

  #[test]
  fn resolve_source_file_loads_an_explicit_relative_reference() {
    let source = SourceFile::javascript(
      "dist/app.js",
      "console.log(1);\n//# sourceMappingURL=app.js.map",
    );

    let resolved = SourceMapResolver::new()
      .resolve_source_file(
        &source,
        &ExpectingLoader {
          expected: SourceMapReference::local_file("dist/app.js.map"),
        },
      )
      .unwrap()
      .expect("explicit source map should be resolved");

    assert_eq!(resolved.discovery_kind(), SourceMapDiscoveryKind::Explicit);
    assert_eq!(
      resolved.reference(),
      &SourceMapReference::local_file("dist/app.js.map"),
    );
  }

  #[test]
  fn resolve_source_file_loads_an_inline_data_uri_reference() {
    let source = SourceFile::javascript(
      "dist/app.js",
      "console.log(1);\n//# sourceMappingURL=data:application/json,%7B%7D",
    );

    let resolved = SourceMapResolver::new()
      .resolve_source_file(
        &source,
        &ExpectingLoader {
          expected: SourceMapReference::inline_data(
            "data:application/json,%7B%7D",
          )
          .unwrap(),
        },
      )
      .unwrap()
      .expect("inline source map should be resolved");

    assert_eq!(resolved.discovery_kind(), SourceMapDiscoveryKind::Explicit);
  }

  #[test]
  fn resolve_source_file_uses_adjacent_fallback_without_explicit_reference() {
    let source = SourceFile::javascript("dist/app.js", "console.log(1);");

    let resolved = SourceMapResolver::new()
      .resolve_source_file(
        &source,
        &ExpectingLoader {
          expected: SourceMapReference::local_file("dist/app.js.map"),
        },
      )
      .unwrap()
      .expect("fallback source map should be resolved");

    assert_eq!(
      resolved.discovery_kind(),
      SourceMapDiscoveryKind::AdjacentFallback,
    );
    assert_eq!(
      resolved.reference(),
      &SourceMapReference::local_file("dist/app.js.map"),
    );
  }

  #[test]
  fn resolve_source_file_treats_missing_fallback_as_absent_source_map() {
    let source = SourceFile::javascript("dist/app.js", "console.log(1);");

    let resolved = SourceMapResolver::new()
      .resolve_source_file(&source, &MissingLoader)
      .unwrap();

    assert!(resolved.is_none());
  }

  #[test]
  fn resolve_source_file_reports_missing_explicit_reference_as_load_error() {
    let source = SourceFile::javascript(
      "dist/app.js",
      "console.log(1);\n//# sourceMappingURL=missing.js.map",
    );

    let result =
      SourceMapResolver::new().resolve_source_file(&source, &MissingLoader);

    assert_eq!(
      result.unwrap_err(),
      SourceMapResolveError::Load(SourceMapLoadError::NotFound(
        PathBuf::from("dist/missing.js.map"),
      )),
    );
  }

  #[test]
  fn resolve_source_file_rejects_conflicting_explicit_references() {
    let source = SourceFile::javascript(
      "dist/app.js",
      "//# sourceMappingURL=one.js.map\n//# sourceMappingURL=two.js.map",
    );

    let result =
      SourceMapResolver::new().resolve_source_file(&source, &StaticLoader);

    assert_eq!(
      result.unwrap_err(),
      SourceMapResolveError::Discovery(
        SourceMapDiscoveryError::AmbiguousExplicitReferences(vec![
          "dist/one.js.map".to_string(),
          "dist/two.js.map".to_string(),
        ]),
      ),
    );
  }

  #[test]
  fn load_reference_preserves_discovery_kind_reference_and_document() {
    let reference = SourceMapReference::local_file("dist/main.js.map");

    let resolved = SourceMapResolver::new()
      .load_reference(
        SourceMapDiscoveryKind::AdjacentFallback,
        reference.clone(),
        &StaticLoader,
      )
      .unwrap();

    assert_eq!(
      resolved.discovery_kind(),
      SourceMapDiscoveryKind::AdjacentFallback,
    );
    assert_eq!(resolved.reference(), &reference);

    let location = resolved
      .document()
      .lookup(SourcePosition::new(0, 0))
      .expect("generated location should be mapped");

    assert_eq!(location.source(), &SourceIdentity::file("src/index.ts"));
    assert_eq!(location.start(), SourcePosition::new(0, 0));
  }

  #[test]
  fn load_reference_reports_loader_failures() {
    let result = SourceMapResolver::new().load_reference(
      SourceMapDiscoveryKind::Explicit,
      SourceMapReference::remote_url("https://example.com/app.js.map")
        .expect("remote url should be accepted"),
      &FailingLoader,
    );

    assert_eq!(
      result.unwrap_err(),
      SourceMapResolveError::Load(SourceMapLoadError::UnsupportedRemoteUrl(
        "https://example.com/app.js.map".to_string(),
      )),
    );
  }

  #[test]
  fn load_reference_reports_parse_failures() {
    let result = SourceMapResolver::new().load_reference(
      SourceMapDiscoveryKind::Explicit,
      SourceMapReference::local_file("dist/main.js.map"),
      &InvalidDocumentLoader,
    );

    assert!(matches!(
      result,
      Err(SourceMapResolveError::Parse(
        SourceMapDocumentParseError::UnsupportedVersion(2),
      )),
    ));
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
}
