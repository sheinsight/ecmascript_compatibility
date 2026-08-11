use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::source_map::{
  SourceMapDiscoveryKind, SourceMapReference, SourceMapUnavailable,
};

use super::CompatDiagnostic;

/// Source Map 解析策略。
///
/// 这个策略控制 analyzer 什么时候为文件尝试解析 Source Map。兼容性检测本身只依赖
/// generated 代码；Source Map 是诊断定位增强信息，因此批量扫描可以按需跳过。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMapPolicy {
  /// 总是尝试解析 Source Map，保持完整文件级 Source Map 状态。
  Always,
  /// 只有存在最终 diagnostic 时才解析 Source Map。
  DiagnosticsOnly,
  /// 完全跳过 Source Map 解析，只返回 generated 位置。
  Disabled,
}

/// Source Map 在分析中被主动跳过的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMapSkipReason {
  /// 调用方显式关闭 Source Map。
  Disabled,
  /// 当前文件没有需要报告的 diagnostic。
  NoDiagnostics,
}

/// 单文件分析各阶段耗时。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompatAnalysisTiming {
  read: Duration,
  parse_detect: Duration,
  generated_position: Duration,
  source_map: Duration,
  original_range_recovery: Duration,
  target_evaluate: Duration,
}

impl CompatAnalysisTiming {
  pub(crate) const fn new(
    read: Duration,
    parse_detect: Duration,
    generated_position: Duration,
    source_map: Duration,
    original_range_recovery: Duration,
    target_evaluate: Duration,
  ) -> Self {
    Self {
      read,
      parse_detect,
      generated_position,
      source_map,
      original_range_recovery,
      target_evaluate,
    }
  }

  pub const fn read(self) -> Duration {
    self.read
  }

  pub const fn parse_detect(self) -> Duration {
    self.parse_detect
  }

  pub const fn generated_position(self) -> Duration {
    self.generated_position
  }

  pub const fn source_map(self) -> Duration {
    self.source_map
  }

  pub const fn original_range_recovery(self) -> Duration {
    self.original_range_recovery
  }

  pub const fn target_evaluate(self) -> Duration {
    self.target_evaluate
  }
}

/// Source Map 在一次分析中的整体状态。
///
/// diagnostic 自身会保留每个 usage 的映射结果；这里记录的是文件级别的发现和解析
/// 状态，方便调用方展示“是否成功接上 Source Map”。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMapStatus {
  Resolved {
    discovery_kind: SourceMapDiscoveryKind,
    reference: SourceMapReference,
  },
  Unavailable(SourceMapUnavailable),
  Skipped(SourceMapSkipReason),
}

impl SourceMapStatus {
  pub const fn unavailable_reason(&self) -> Option<&SourceMapUnavailable> {
    match self {
      Self::Resolved { .. } | Self::Skipped(_) => None,
      Self::Unavailable(reason) => Some(reason),
    }
  }

  pub const fn skip_reason(&self) -> Option<SourceMapSkipReason> {
    match self {
      Self::Resolved { .. } | Self::Unavailable(_) => None,
      Self::Skipped(reason) => Some(*reason),
    }
  }
}

/// 单个文件的兼容性分析报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatReport {
  /// 本次分析对应的输入文件路径。
  path: PathBuf,
  /// 文件级 Source Map 状态。
  source_map_status: SourceMapStatus,
  /// 需要调用方关注的兼容性诊断。
  diagnostics: Vec<CompatDiagnostic>,
}

impl CompatReport {
  pub(crate) fn new(
    path: PathBuf,
    source_map_status: SourceMapStatus,
    diagnostics: Vec<CompatDiagnostic>,
  ) -> Self {
    Self {
      path,
      source_map_status,
      diagnostics,
    }
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub const fn source_map_status(&self) -> &SourceMapStatus {
    &self.source_map_status
  }

  pub fn diagnostics(&self) -> &[CompatDiagnostic] {
    &self.diagnostics
  }

  pub fn unsupported_diagnostics(
    &self,
  ) -> impl Iterator<Item = &CompatDiagnostic> {
    // diagnostics 本身可能包含 Mixed 或 Unknown；这个派生视图只返回包含
    // Unsupported target 的诊断，适合 CLI 默认高亮或 CI fail 条件。
    self
      .diagnostics
      .iter()
      .filter(|diagnostic| diagnostic.has_unsupported_target())
  }
}
