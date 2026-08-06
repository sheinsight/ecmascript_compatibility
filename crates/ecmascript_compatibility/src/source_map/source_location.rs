use super::{source_identity::SourceIdentity, source_position::SourcePosition};

/// Source Map 返回的 original source 位置。
///
/// Source Map 原生按点位映射，不保证能还原完整源码范围。因此 `end` 是可选的：
/// 只有 span 起点和终点都能可靠映射到同一 source 时才应填写。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
  source: SourceIdentity,
  start: SourcePosition,
  end: Option<SourcePosition>,
}

impl SourceLocation {
  pub const fn new(
    source: SourceIdentity,
    start: SourcePosition,
    end: Option<SourcePosition>,
  ) -> Self {
    Self { source, start, end }
  }

  pub const fn source(&self) -> &SourceIdentity {
    &self.source
  }

  pub const fn start(&self) -> SourcePosition {
    self.start
  }

  pub const fn end(&self) -> Option<SourcePosition> {
    self.end
  }
}
