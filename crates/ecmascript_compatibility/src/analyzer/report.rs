use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::source_map::{
  SourceMapDiscoveryKind, SourceMapReference, SourceMapUnavailable,
};

use super::CompatDiagnostic;

/// 单文件分析各阶段耗时。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompatAnalysisTiming {
  read: Duration,
  parse_detect: Duration,
  generated_position: Duration,
  source_map: Duration,
  target_evaluate: Duration,
}

impl CompatAnalysisTiming {
  pub(crate) const fn new(
    read: Duration,
    parse_detect: Duration,
    generated_position: Duration,
    source_map: Duration,
    target_evaluate: Duration,
  ) -> Self {
    Self {
      read,
      parse_detect,
      generated_position,
      source_map,
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
}

impl SourceMapStatus {
  pub const fn unavailable_reason(&self) -> Option<&SourceMapUnavailable> {
    match self {
      Self::Resolved { .. } => None,
      Self::Unavailable(reason) => Some(reason),
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
