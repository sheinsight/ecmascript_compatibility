use crate::source_map::SourceMapping;

/// 兼容性检查能够识别的 ECMAScript 特性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureId {
  /// 可选链表达式，例如 `object?.property`。
  OptionalChaining,
}

/// detector 从输入文本中直接观察到的 UTF-8 byte span。
///
/// `start` 和 `end` 都基于完整输入文本的字节偏移，且 `end` 是 exclusive。
/// 这个 span 表示产物中的事实位置，不会被 Source Map 映射结果覆盖。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
  start: u32,
  end: u32,
}

impl SourceSpan {
  pub(crate) const fn new(start: u32, end: u32) -> Self {
    Self { start, end }
  }

  pub const fn start(self) -> u32 {
    self.start
  }

  pub const fn end(self) -> u32 {
    self.end
  }
}

/// 单次特性使用记录。
///
/// `span` 始终表示 detector 在当前输入文件中看到的位置。Source Map 只提供
/// 附加的源码定位状态，因此单独保存在 `source_mapping` 中。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FeatureUsage {
  feature: FeatureId,
  span: SourceSpan,
  source_mapping: SourceMapping,
}

impl FeatureUsage {
  pub(crate) const fn new(feature: FeatureId, span: SourceSpan) -> Self {
    Self {
      feature,
      span,
      source_mapping: SourceMapping::NotResolved,
    }
  }

  pub const fn feature(&self) -> FeatureId {
    self.feature
  }

  pub const fn span(&self) -> SourceSpan {
    self.span
  }

  pub const fn source_mapping(&self) -> &SourceMapping {
    &self.source_mapping
  }

  pub(crate) fn with_source_mapping(
    mut self,
    source_mapping: SourceMapping,
  ) -> Self {
    self.source_mapping = source_mapping;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_feature_usage_starts_with_unresolved_source_mapping() {
    let usage =
      FeatureUsage::new(FeatureId::OptionalChaining, SourceSpan::new(10, 20));

    assert_eq!(usage.feature(), FeatureId::OptionalChaining);
    assert_eq!(usage.span(), SourceSpan::new(10, 20));
    assert_eq!(usage.source_mapping(), &SourceMapping::NotResolved);
  }
}
