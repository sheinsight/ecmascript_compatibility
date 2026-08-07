mod diagnostic;
mod error;
mod generated_source_index;
mod report;

use std::{fs, path::Path};

pub use diagnostic::{CompatDiagnostic, TargetCompatStatus};
pub use error::CompatAnalysisError;
pub use report::{CompatReport, SourceMapStatus};

use crate::{
  CompatDatabase, CompatStatus, FeatureDetector, SourceFile, TargetQuery,
  TargetResolver, evaluate,
  source_map::{
    DefaultSourceMapLoader, SourceMapDiscoveryError, SourceMapResolveError,
    SourceMapResolver, SourceMapUnavailable,
  },
};

use generated_source_index::GeneratedSourceIndex;

/// 兼容性分析的对外入口。
///
/// 调用方只需要提供源文件和目标运行时查询；内部会完成特性检测、Source Map 回源
/// 和兼容性规则评估。底层模块仍然保留，但常规使用不需要手动拼接这些步骤。
#[derive(Debug, Default, Clone)]
pub struct CompatAnalyzer {
  detector: FeatureDetector,
  database: CompatDatabase,
  target_resolver: TargetResolver,
  source_map_resolver: SourceMapResolver,
  source_map_loader: DefaultSourceMapLoader,
}

impl CompatAnalyzer {
  pub fn new() -> Self {
    Self::default()
  }

  /// 从文件系统读取并分析一个源文件或构建产物文件。
  ///
  /// Source Map 会按 `sourceMappingURL` 或同名 `.map` 文件自动发现；找不到
  /// Source Map 不会中断分析，报告会保留 generated 位置并标记映射不可用。
  pub fn analyze_path<I, S>(
    &self,
    path: impl AsRef<Path>,
    target_queries: I,
  ) -> Result<CompatReport, CompatAnalysisError>
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let path = path.as_ref();
    let source_text = fs::read_to_string(path).map_err(|source| {
      CompatAnalysisError::ReadSource {
        path: path.to_path_buf(),
        source,
      }
    })?;
    let source = SourceFile::from_path(path.to_path_buf(), source_text)?;

    self.analyze_source(source, target_queries)
  }

  /// 分析调用方已经构造好的 `SourceFile`。
  ///
  /// 这个入口适合虚拟文件、内存内容或上层已经完成读取的场景。`SourceFile::path`
  /// 仍会用于 Source Map 相对路径解析和最终诊断展示。
  pub fn analyze_source<I, S>(
    &self,
    source: SourceFile,
    target_queries: I,
  ) -> Result<CompatReport, CompatAnalysisError>
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let target_query = TargetQuery::new(target_queries)?;
    let targets = self.target_resolver.resolve(&target_query)?;
    let detection = self.detector.detect(&source)?;
    let source_index = GeneratedSourceIndex::new(source.source_text());

    // Source Map 是报告的增强信息，不是兼容性检测成立的前提。
    // 因此 resolver 的结果先被折叠成文件级 `SourceMapStatus`，后面每条 usage
    // 再根据这个状态决定是映射到 original source，还是保留 generated 位置并说明降级原因。
    let source_map_result = self
      .source_map_resolver
      .resolve_source_file(&source, &self.source_map_loader);
    let source_map_status =
      source_map_status(source.path(), &source_map_result);
    let resolved_source_map =
      source_map_result.as_ref().ok().and_then(Option::as_ref);

    let mut diagnostics = Vec::new();

    for usage in detection.usages() {
      let generated_position =
        source_index.position_for_offset(usage.span().start());

      // detector 给出的是 byte span；Source Map 查询需要零基 UTF-16 行列。
      // 这里用 span 起点作为 diagnostic 的主要定位点，span 本身仍保留在结果里。
      let source_mapping = resolved_source_map
        .as_ref()
        .and_then(|resolved| resolved.document().lookup(generated_position))
        .map_or_else(
          || {
            source_map_status.unavailable_reason().cloned().map_or_else(
              || {
                SourceMapUnavailableOrLocation::Unavailable(
                  SourceMapUnavailable::UnmappedPosition,
                )
              },
              SourceMapUnavailableOrLocation::Unavailable,
            )
          },
          SourceMapUnavailableOrLocation::Location,
        );

      let mut target_statuses = Vec::new();

      for target in &targets {
        let rule = self
          .database
          .support_rule(usage.feature(), target.runtime());
        let status = evaluate(rule, target.release());

        // Report 默认只记录需要调用方关注的状态：
        // Unsupported、Mixed 和 Unknown。明确 Supported 的 target 仍保留在
        // report.targets() 中，但不进入单条 diagnostic，避免诊断结果被正常状态淹没。
        if status == CompatStatus::Supported {
          continue;
        }

        target_statuses.push(TargetCompatStatus::new(*target, status));
      }

      if !target_statuses.is_empty() {
        // 一条 diagnostic 对应一个 feature usage。多个 target 的结果聚合在
        // `target_statuses` 中，避免把 usage × target 展开成重复诊断。
        diagnostics.push(CompatDiagnostic::new(
          usage.feature(),
          detection.path().to_path_buf(),
          usage.span(),
          generated_position,
          source_mapping.into_source_mapping(),
          target_statuses,
        ));
      }
    }

    Ok(CompatReport::new(
      detection.path().to_path_buf(),
      targets,
      detection.usages().len(),
      source_map_status,
      diagnostics,
    ))
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceMapUnavailableOrLocation {
  Unavailable(SourceMapUnavailable),
  Location(crate::source_map::SourceLocation),
}

impl SourceMapUnavailableOrLocation {
  // analyze_source 需要先临时保存“映射成功的位置”或“映射不可用原因”。
  // 最终进入报告时再统一转回领域模型 `SourceMapping`，避免在主流程里重复 match。
  fn into_source_mapping(self) -> crate::source_map::SourceMapping {
    match self {
      Self::Unavailable(reason) => {
        crate::source_map::SourceMapping::Unavailable(reason)
      }
      Self::Location(location) => {
        crate::source_map::SourceMapping::Mapped(location)
      }
    }
  }
}

impl From<SourceMapUnavailable> for SourceMapUnavailableOrLocation {
  fn from(reason: SourceMapUnavailable) -> Self {
    Self::Unavailable(reason)
  }
}

fn source_map_status(
  source_path: &Path,
  result: &Result<
    Option<crate::source_map::ResolvedSourceMap>,
    SourceMapResolveError,
  >,
) -> SourceMapStatus {
  // `SourceMapStatus` 是文件级状态：它描述这次分析有没有成功接上某个 Source Map。
  // 即使文件级状态是 Resolved，单个 generated 位置仍可能查不到 mapping。
  match result {
    Ok(Some(resolved)) => SourceMapStatus::Resolved {
      discovery_kind: resolved.discovery_kind(),
      reference: resolved.reference().clone(),
      source_count: resolved.document().source_count(),
    },
    Ok(None) => SourceMapStatus::Unavailable(SourceMapUnavailable::NotFound {
      fallback_path: adjacent_source_map_path(source_path),
    }),
    Err(error) => SourceMapStatus::Unavailable(source_map_unavailable(error)),
  }
}

fn source_map_unavailable(
  error: &SourceMapResolveError,
) -> SourceMapUnavailable {
  // resolver 的错误按阶段划分；报告层把它们转换成调用方更容易展示的
  // source-map-unavailable 原因。
  match error {
    SourceMapResolveError::Discovery(
      SourceMapDiscoveryError::AmbiguousExplicitReferences(references),
    ) => SourceMapUnavailable::AmbiguousReference {
      references: references.clone(),
    },
    SourceMapResolveError::Load(error) => {
      SourceMapUnavailable::ExplicitReferenceUnavailable {
        reference: "sourceMappingURL".to_string(),
        message: error.to_string(),
      }
    }
    SourceMapResolveError::Parse(error) => {
      SourceMapUnavailable::InvalidDocument {
        location: "source map".to_string(),
        message: error.to_string(),
      }
    }
  }
}

fn adjacent_source_map_path(source_path: &Path) -> std::path::PathBuf {
  let mut path = source_path.as_os_str().to_os_string();
  path.push(".map");

  path.into()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn analyzes_source_and_returns_non_supported_diagnostics() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let report = CompatAnalyzer::new()
      .analyze_source(source, ["chrome 79"])
      .unwrap();

    assert_eq!(report.detected_usage_count(), 1);
    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(report.diagnostics()[0].path(), Path::new("input.js"));
    assert_eq!(
      report.diagnostics()[0].target_statuses()[0].status(),
      CompatStatus::Unsupported,
    );
  }

  #[test]
  fn keeps_supported_targets_out_of_diagnostics() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let report = CompatAnalyzer::new()
      .analyze_source(source, ["chrome 80"])
      .unwrap();

    assert!(report.diagnostics().is_empty());
  }

  #[test]
  fn groups_target_statuses_by_detected_usage() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let report = CompatAnalyzer::new()
      .analyze_source(source, ["chrome 79", "firefox 73"])
      .unwrap();

    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(report.diagnostics()[0].target_statuses().len(), 2);
  }

  #[test]
  fn records_missing_source_map_as_report_status() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let report = CompatAnalyzer::new()
      .analyze_source(source, ["chrome 79"])
      .unwrap();

    assert!(matches!(
      report.source_map_status(),
      SourceMapStatus::Unavailable(SourceMapUnavailable::NotFound { .. }),
    ));
  }
}
