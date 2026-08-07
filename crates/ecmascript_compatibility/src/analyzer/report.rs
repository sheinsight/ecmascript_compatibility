use std::path::{Path, PathBuf};

use crate::{
  source_map::{
    SourceMapDiscoveryKind, SourceMapReference, SourceMapUnavailable,
  },
  target::RuntimeTarget,
};

use super::CompatDiagnostic;

/// Source Map 在一次分析中的整体状态。
///
/// diagnostic 自身会保留每个 usage 的映射结果；这里记录的是文件级别的发现和解析
/// 状态，方便调用方展示“是否成功接上 Source Map”。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMapStatus {
  Resolved {
    discovery_kind: SourceMapDiscoveryKind,
    reference: SourceMapReference,
    source_count: u32,
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
  /// target query 解析后的完整目标集合。
  targets: Vec<RuntimeTarget>,
  /// syntax detector 发现的 usage 总数，包含最终没有形成 diagnostic 的 supported usage。
  detected_usage_count: usize,
  /// 文件级 Source Map 状态。
  source_map_status: SourceMapStatus,
  /// 需要调用方关注的兼容性诊断。
  diagnostics: Vec<CompatDiagnostic>,
}

impl CompatReport {
  pub(crate) fn new(
    path: PathBuf,
    targets: Vec<RuntimeTarget>,
    detected_usage_count: usize,
    source_map_status: SourceMapStatus,
    diagnostics: Vec<CompatDiagnostic>,
  ) -> Self {
    Self {
      path,
      targets,
      detected_usage_count,
      source_map_status,
      diagnostics,
    }
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub fn targets(&self) -> &[RuntimeTarget] {
    &self.targets
  }

  pub const fn detected_usage_count(&self) -> usize {
    self.detected_usage_count
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
