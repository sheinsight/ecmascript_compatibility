use std::path::{Path, PathBuf};

use crate::{
  CompatStatus, SourceSpan, SyntaxFeatureId,
  source_map::{SourceMapping, SourcePosition},
  target::RuntimeTarget,
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
  /// syntax detector 实际分析的文件路径，通常是构建产物文件。
  path: PathBuf,
  /// usage 在 `path` 对应文本里的 UTF-8 byte span。
  span: SourceSpan,
  /// usage 起点转换后的 generated 行列，便于 Source Map 查询和用户展示。
  generated_position: SourcePosition,
  /// Source Map 对该 usage 的映射结果；缺失时仍保留 generated 位置。
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
  /// 被评估的运行时及版本。
  target: RuntimeTarget,
  /// 该运行时版本对当前 feature 的支持状态。
  status: CompatStatus,
}

impl CompatDiagnostic {
  pub(crate) fn new(
    feature: SyntaxFeatureId,
    path: PathBuf,
    span: SourceSpan,
    generated_position: SourcePosition,
    source_mapping: SourceMapping,
    target_statuses: Vec<TargetCompatStatus>,
  ) -> Self {
    Self {
      feature,
      path,
      span,
      generated_position,
      source_mapping,
      target_statuses,
    }
  }

  pub const fn feature(&self) -> SyntaxFeatureId {
    self.feature
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub const fn span(&self) -> SourceSpan {
    self.span
  }

  pub const fn generated_position(&self) -> SourcePosition {
    self.generated_position
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
  pub(crate) const fn new(target: RuntimeTarget, status: CompatStatus) -> Self {
    Self { target, status }
  }

  pub const fn target(self) -> RuntimeTarget {
    self.target
  }

  pub const fn status(self) -> CompatStatus {
    self.status
  }
}
