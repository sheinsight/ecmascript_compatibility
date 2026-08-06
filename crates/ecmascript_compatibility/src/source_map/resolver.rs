use super::{
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMapResolveError {
  Load(SourceMapLoadError),
  Parse(SourceMapDocumentParseError),
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

/// Source Map 发现编排入口。
///
/// 当前阶段先固定 resolver 与 loader 的协作边界；真实的 `sourceMappingURL`
/// 提取、同名回退和冲突处理会在下一阶段补充到这里。
#[derive(Debug, Default, Clone, Copy)]
pub struct SourceMapResolver;

impl SourceMapResolver {
  pub const fn new() -> Self {
    Self
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
