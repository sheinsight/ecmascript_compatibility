use super::super::{
  source_map_discovery_kind::SourceMapDiscoveryKind,
  source_map_reference::SourceMapReference,
};

/// discovery 阶段归一化后的 Source Map 候选引用。
///
/// 这里同时携带发现方式和引用本身，resolver 可以据此决定加载失败时是否允许降级。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSourceMap {
  kind: SourceMapDiscoveryKind,
  reference: SourceMapReference,
}

impl DiscoveredSourceMap {
  pub const fn new(
    kind: SourceMapDiscoveryKind,
    reference: SourceMapReference,
  ) -> Self {
    Self { kind, reference }
  }

  pub const fn kind(&self) -> SourceMapDiscoveryKind {
    self.kind
  }

  pub const fn reference(&self) -> &SourceMapReference {
    &self.reference
  }

  pub fn into_parts(self) -> (SourceMapDiscoveryKind, SourceMapReference) {
    (self.kind, self.reference)
  }
}
