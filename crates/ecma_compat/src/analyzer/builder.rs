use crate::source_map::DefaultSourceMapLoader;

use super::{CompatAnalyzer, SourceMapPolicy};

/// `CompatAnalyzer` 的构造器。
///
/// builder 只暴露真正影响分析行为的扩展点。默认配置面向 CLI/CI：使用默认
/// Source Map loader，并且只在 diagnostic 中保留需要关注的 target 状态。
#[derive(Debug, Clone)]
pub struct CompatAnalyzerBuilder<L = DefaultSourceMapLoader> {
  source_map_loader: L,
  source_map_policy: SourceMapPolicy,
  include_supported_targets: bool,
}

impl Default for CompatAnalyzerBuilder<DefaultSourceMapLoader> {
  fn default() -> Self {
    Self {
      source_map_loader: DefaultSourceMapLoader::default(),
      source_map_policy: SourceMapPolicy::Always,
      include_supported_targets: false,
    }
  }
}

impl<L> CompatAnalyzerBuilder<L> {
  /// 替换 Source Map loader。
  ///
  /// 这允许调用方接入远程加载、缓存、内存映射或测试专用 loader，而不影响
  /// detector、target resolver 和 report 领域模型。
  pub fn source_map_loader<N>(
    self,
    source_map_loader: N,
  ) -> CompatAnalyzerBuilder<N> {
    CompatAnalyzerBuilder {
      source_map_loader,
      source_map_policy: self.source_map_policy,
      include_supported_targets: self.include_supported_targets,
    }
  }

  /// 控制 Source Map 解析时机。
  ///
  /// 默认值是 `Always`，保持 library/CLI 的完整文件级 Source Map 状态。批量调用方
  /// 可以选择 `DiagnosticsOnly`，避免为最终没有 diagnostic 的文件解析 Source Map。
  pub const fn source_map_policy(
    mut self,
    source_map_policy: SourceMapPolicy,
  ) -> Self {
    self.source_map_policy = source_map_policy;
    self
  }

  /// 控制 diagnostic 是否包含明确 Supported 的 target 状态。
  ///
  /// 默认值是 `false`，报告只保留 Unsupported、Mixed 和 Unknown。设为 `true`
  /// 时，每条 usage diagnostic 会包含完整 target 评估矩阵。
  pub const fn include_supported_targets(
    mut self,
    include_supported_targets: bool,
  ) -> Self {
    self.include_supported_targets = include_supported_targets;
    self
  }

  pub fn build(self) -> CompatAnalyzer<L> {
    CompatAnalyzer::from_parts(
      self.source_map_loader,
      self.source_map_policy,
      self.include_supported_targets,
    )
  }
}
