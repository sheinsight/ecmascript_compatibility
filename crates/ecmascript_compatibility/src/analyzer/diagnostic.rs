use crate::{
  CompatStatus, SourceSpan, SyntaxFeatureId,
  source_map::{SourceMapping, SourcePosition},
};

/// 单个语法特性使用位置上的兼容性诊断。
///
/// 一条 diagnostic 以 syntax detector 发现的 usage 为中心，聚合这个 usage 在多个
/// target 上的非 Supported 状态。这样报告表达的是“这个源码位置有问题”，而不是
/// “这个源码位置和每个 target 的笛卡尔积”。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatDiagnostic {
  /// syntax detector 识别出的 ECMAScript 语法特性。
  feature: SyntaxFeatureId,
  /// usage 在当前分析文件里的 UTF-8 byte span。
  span: SourceSpan,
  /// usage 起点转换后的行列，位置基于当前分析文件。
  position: SourcePosition,
  /// Source Map 对该 usage 的映射结果；缺失时仍保留当前分析文件位置。
  source_mapping: SourceMapping,
  /// 这个 usage 在各目标运行时上的非 Supported 状态。
  target_statuses: Vec<TargetCompatStatus>,
}

/// 单个目标运行时上的兼容性评估结果。
///
/// `CompatReport` 默认只保留需要关注的状态，也就是 Unsupported、Mixed 和 Unknown；
/// 明确 Supported 的 target 不进入 diagnostic。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCompatStatus {
  /// 被评估目标在 `CompatReport::targets()` 中的下标。
  target_index: usize,
  /// 该运行时版本对当前 feature 的支持状态。
  status: CompatStatus,
}

impl CompatDiagnostic {
  pub(crate) fn new(
    feature: SyntaxFeatureId,
    span: SourceSpan,
    position: SourcePosition,
    source_mapping: SourceMapping,
    target_statuses: Vec<TargetCompatStatus>,
  ) -> Self {
    Self {
      feature,
      span,
      position,
      source_mapping,
      target_statuses,
    }
  }

  pub const fn feature(&self) -> SyntaxFeatureId {
    self.feature
  }

  pub const fn span(&self) -> SourceSpan {
    self.span
  }

  pub const fn position(&self) -> SourcePosition {
    self.position
  }

  pub const fn source_mapping(&self) -> &SourceMapping {
    &self.source_mapping
  }

  pub fn target_statuses(&self) -> &[TargetCompatStatus] {
    &self.target_statuses
  }

  pub fn unsupported_targets(
    &self,
  ) -> impl Iterator<Item = &TargetCompatStatus> {
    // `target_statuses` 可能包含 Mixed 或 Unknown；这个视图专门服务只关心
    // 明确不支持目标的调用方。
    self
      .target_statuses
      .iter()
      .filter(|target| target.status() == CompatStatus::Unsupported)
  }

  pub fn has_unsupported_target(&self) -> bool {
    self.unsupported_targets().next().is_some()
  }
}

impl TargetCompatStatus {
  pub(crate) const fn new(target_index: usize, status: CompatStatus) -> Self {
    Self {
      target_index,
      status,
    }
  }

  pub const fn target_index(self) -> usize {
    self.target_index
  }

  pub const fn status(self) -> CompatStatus {
    self.status
  }
}
