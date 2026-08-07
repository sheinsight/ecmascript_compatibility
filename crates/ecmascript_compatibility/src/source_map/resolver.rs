use std::path::{Path, PathBuf};

use crate::source::SourceFile;

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
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceMapResolveError {
  #[error(transparent)]
  Discovery(SourceMapDiscoveryError),
  #[error(transparent)]
  Load(SourceMapLoadError),
  #[error(transparent)]
  Parse(SourceMapDocumentParseError),
}

/// Source Map 引用发现失败原因。
///
/// 这类错误发生在读取 `.map` 文档之前，通常说明 generated 文件本身给出的
/// `sourceMappingURL` 信息不够明确。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceMapDiscoveryError {
  /// 文件中存在多个不同的显式 Source Map 引用，resolver 不能安全地替调用方选择。
  #[error("ambiguous explicit source map references: {0:?}")]
  AmbiguousExplicitReferences(Vec<String>),
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

  pub fn resolve_source_file<L>(
    &self,
    source: &SourceFile,
    loader: &L,
  ) -> Result<Option<ResolvedSourceMap>, SourceMapResolveError>
  where
    L: SourceMapLoader,
  {
    if let Some(reference) = explicit_source_map_reference(source)? {
      return self
        .load_reference(SourceMapDiscoveryKind::Explicit, reference, loader)
        .map(Some);
    }

    let fallback =
      SourceMapReference::local_file(adjacent_source_map_path(source.path()));

    match self.load_reference(
      SourceMapDiscoveryKind::AdjacentFallback,
      fallback,
      loader,
    ) {
      Ok(resolved) => Ok(Some(resolved)),
      Err(SourceMapResolveError::Load(SourceMapLoadError::NotFound(_))) => {
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

fn explicit_source_map_reference(
  source: &SourceFile,
) -> Result<Option<SourceMapReference>, SourceMapDiscoveryError> {
  let mut references = source
    .source_text()
    .lines()
    .filter_map(source_mapping_url)
    .map(|url| reference_from_url(url, source.path()))
    .collect::<Vec<_>>();

  references.dedup();

  match references.as_slice() {
    [] => Ok(None),
    [reference] => Ok(Some(reference.clone())),
    _ => Err(SourceMapDiscoveryError::AmbiguousExplicitReferences(
      references.iter().map(reference_label).collect(),
    )),
  }
}

fn source_mapping_url(line: &str) -> Option<&str> {
  let line = line.trim();

  let directive = line
    .strip_prefix("//#")
    .or_else(|| line.strip_prefix("//@"))
    .or_else(|| {
      line
        .strip_prefix("/*#")
        .or_else(|| line.strip_prefix("/*@"))
        .and_then(|line| line.strip_suffix("*/"))
    })?
    .trim_start();

  directive
    .strip_prefix("sourceMappingURL=")
    .map(str::trim)
    .filter(|url| !url.is_empty())
}

fn reference_from_url(url: &str, generated_path: &Path) -> SourceMapReference {
  if url.starts_with("data:") {
    SourceMapReference::inline_data(url)
      .expect("sourceMappingURL parser rejects empty values")
  } else if url.starts_with("http://") || url.starts_with("https://") {
    SourceMapReference::remote_url(url)
      .expect("sourceMappingURL parser rejects empty values")
  } else {
    let path = PathBuf::from(url);
    if path.is_absolute() {
      SourceMapReference::local_file(path)
    } else {
      SourceMapReference::local_file(
        generated_path
          .parent()
          .unwrap_or_else(|| Path::new(""))
          .join(path),
      )
    }
  }
}

fn adjacent_source_map_path(generated_path: &Path) -> PathBuf {
  let mut path = generated_path.as_os_str().to_os_string();
  path.push(".map");

  PathBuf::from(path)
}

fn reference_label(reference: &SourceMapReference) -> String {
  match reference {
    SourceMapReference::InlineData(data_uri) => data_uri.clone(),
    SourceMapReference::LocalFile(path) => path.display().to_string(),
    SourceMapReference::RemoteUrl(url) => url.clone(),
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

    assert_eq!(resolved.is_none(), true);
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
      SourceMapResolveError::Load(SourceMapLoadError::NotFound(PathBuf::from(
        "dist/missing.js.map"
      ),)),
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
