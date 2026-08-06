use super::{
  loader::{SourceMapLoadError, SourceMapLoader},
  source_map_discovery_kind::SourceMapDiscoveryKind,
  source_map_reference::SourceMapReference,
};

/// 已完成发现和加载的 Source Map 文档。
///
/// 这个类型仍然只表示“找到了并读到了 map 字节”。解码 v3 文档和查询 mappings
/// 属于后续 decoder 阶段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSourceMap {
  discovery_kind: SourceMapDiscoveryKind,
  reference: SourceMapReference,
  bytes: Vec<u8>,
}

impl ResolvedSourceMap {
  pub fn new(
    discovery_kind: SourceMapDiscoveryKind,
    reference: SourceMapReference,
    bytes: Vec<u8>,
  ) -> Self {
    Self {
      discovery_kind,
      reference,
      bytes,
    }
  }

  pub const fn discovery_kind(&self) -> SourceMapDiscoveryKind {
    self.discovery_kind
  }

  pub const fn reference(&self) -> &SourceMapReference {
    &self.reference
  }

  pub fn bytes(&self) -> &[u8] {
    &self.bytes
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
  ) -> Result<ResolvedSourceMap, SourceMapLoadError>
  where
    L: SourceMapLoader,
  {
    let bytes = loader.load(&reference)?;

    Ok(ResolvedSourceMap::new(discovery_kind, reference, bytes))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct StaticLoader;

  impl SourceMapLoader for StaticLoader {
    fn load(
      &self,
      _reference: &SourceMapReference,
    ) -> Result<Vec<u8>, SourceMapLoadError> {
      Ok(br#"{"version":3}"#.to_vec())
    }
  }

  #[test]
  fn load_reference_preserves_discovery_kind_reference_and_bytes() {
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
    assert_eq!(resolved.bytes(), br#"{"version":3}"#);
  }
}
