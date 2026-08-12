mod builder;
mod diagnostic;
mod error;
mod generated_source_index;
mod original_range_recovery;
mod report;

use std::{
  fs,
  path::Path,
  time::{Duration, Instant},
};

pub use builder::CompatAnalyzerBuilder;
pub use diagnostic::{CompatDiagnostic, TargetCompatStatus};
pub use error::CompatAnalysisError;
pub use report::{
  CompatAnalysisTiming, CompatReport, SourceMapPolicy, SourceMapSkipReason,
  SourceMapStatus,
};

use crate::{
  CompatStatus, RuntimeTarget, SourceFile, SourceSpan, SyntaxCompatDatabase,
  SyntaxFeatureDetector, SyntaxFeatureId, TargetQuery, TargetResolver,
  evaluate,
  source_map::{
    DefaultSourceMapLoader, SourceMapDiscoveryError, SourceMapLoader,
    SourceMapResolveError, SourceMapResolver, SourceMapUnavailable,
  },
};

use generated_source_index::GeneratedSourceIndex;
use original_range_recovery::OriginalRangeRecoverer;

/// 兼容性分析的对外入口。
///
/// 调用方先把目标运行时查询解析为 `RuntimeTarget`，再把源文件和 targets
/// 交给 analyzer；内部会完成语法特性检测、Source Map 回源和兼容性规则评估。
/// 这个 analyzer 的定位是 ECMAScript syntax compat，不检测运行时 API 调用。
#[derive(Debug, Clone)]
pub struct CompatAnalyzer<L = DefaultSourceMapLoader> {
  detector: SyntaxFeatureDetector,
  database: SyntaxCompatDatabase,
  target_resolver: TargetResolver,
  source_map_resolver: SourceMapResolver,
  source_map_loader: L,
  source_map_policy: SourceMapPolicy,
  include_supported_targets: bool,
}

impl CompatAnalyzer<DefaultSourceMapLoader> {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn builder() -> CompatAnalyzerBuilder {
    CompatAnalyzerBuilder::default()
  }
}

impl Default for CompatAnalyzer<DefaultSourceMapLoader> {
  fn default() -> Self {
    Self::builder().build()
  }
}

impl<L> CompatAnalyzer<L> {
  pub(crate) fn from_parts(
    source_map_loader: L,
    source_map_policy: SourceMapPolicy,
    include_supported_targets: bool,
  ) -> Self {
    Self {
      detector: SyntaxFeatureDetector::new(),
      database: SyntaxCompatDatabase::new(),
      target_resolver: TargetResolver,
      source_map_resolver: SourceMapResolver::new(),
      source_map_loader,
      source_map_policy,
      include_supported_targets,
    }
  }
}

impl<L> CompatAnalyzer<L>
where
  L: SourceMapLoader,
{
  /// 从文件系统读取并分析一个源文件或构建产物文件。
  ///
  /// Source Map 会按 `sourceMappingURL` 或同名 `.map` 文件自动发现；找不到
  /// Source Map 不会中断分析，报告会保留 generated 位置并标记映射不可用。
  pub fn analyze_path(
    &self,
    path: impl AsRef<Path>,
    targets: &[RuntimeTarget],
  ) -> Result<CompatReport, CompatAnalysisError> {
    self
      .analyze_path_with_timing(path, targets)
      .map(|(report, _timing)| report)
  }

  /// 从文件系统读取并分析一个源文件，同时返回单文件分析阶段耗时。
  ///
  /// `CompatReport` 只表达兼容性结果；timing 是性能观测元数据，目录级 binding
  /// 可以用它汇总整体耗时，但它不属于单文件 report 模型。
  pub fn analyze_path_with_timing(
    &self,
    path: impl AsRef<Path>,
    targets: &[RuntimeTarget],
  ) -> Result<(CompatReport, CompatAnalysisTiming), CompatAnalysisError> {
    let path = path.as_ref();
    let read_started_at = Instant::now();
    let source_text = fs::read_to_string(path).map_err(|source| {
      CompatAnalysisError::ReadSource {
        path: path.to_path_buf(),
        source,
      }
    })?;
    let source = SourceFile::from_path(path.to_path_buf(), source_text)?;
    let read = read_started_at.elapsed();

    self.analyze_source_with_read_timing(source, targets, read)
  }

  /// 解析 target 查询，供需要批量分析多个文件的调用方复用结果。
  ///
  /// 目录级或批量调用方应先调用这个方法，再把结果传给 `analyze_path` 或
  /// `analyze_source`，避免对同一组 Browserslist 查询重复解析。
  pub fn resolve_targets<I, S>(
    &self,
    target_queries: I,
  ) -> Result<Vec<RuntimeTarget>, CompatAnalysisError>
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let target_query = TargetQuery::new(target_queries)?;
    self
      .target_resolver
      .resolve(&target_query)
      .map_err(CompatAnalysisError::from)
  }

  /// 分析调用方已经构造好的 `SourceFile`。
  ///
  /// 这个入口适合虚拟文件、内存内容或上层已经完成读取的场景。`SourceFile::path`
  /// 仍会用于 Source Map 相对路径解析和最终诊断展示。
  pub fn analyze_source(
    &self,
    source: SourceFile,
    targets: &[RuntimeTarget],
  ) -> Result<CompatReport, CompatAnalysisError> {
    self
      .analyze_source_with_read_timing(source, targets, Duration::ZERO)
      .map(|(report, _timing)| report)
  }

  fn analyze_source_with_read_timing(
    &self,
    source: SourceFile,
    targets: &[RuntimeTarget],
    read: Duration,
  ) -> Result<(CompatReport, CompatAnalysisTiming), CompatAnalysisError> {
    let parse_detect_started_at = Instant::now();
    let detection = self.detector.detect(&source)?;
    let parse_detect = parse_detect_started_at.elapsed();

    let mut diagnostics = Vec::new();
    let mut target_evaluate = Duration::ZERO;
    let mut pending_diagnostics = Vec::new();

    for usage in detection.usages() {
      let target_evaluate_started_at = Instant::now();
      let mut target_statuses = Vec::new();

      for (target_index, target) in targets.iter().enumerate() {
        let rule = self
          .database
          .support_rule(usage.feature(), target.runtime());
        let status = evaluate(rule, target.release());

        // Report 默认只记录需要调用方关注的状态：
        // Unsupported、Mixed 和 Unknown。`include_supported_targets` 打开后，
        // diagnostic 会保留完整 target 矩阵，适合调试和报表场景。
        if status == CompatStatus::Supported && !self.include_supported_targets
        {
          continue;
        }

        target_statuses.push(TargetCompatStatus::new(target_index, status));
      }
      target_evaluate += target_evaluate_started_at.elapsed();

      if !target_statuses.is_empty() {
        pending_diagnostics.push(PendingDiagnostic::new(
          usage.feature(),
          usage.span(),
          target_statuses,
        ));
      }
    }

    // Source Map 是报告的增强信息，不是兼容性检测成立的前提。先生成最终需要报告的
    // diagnostics，再根据策略决定是否解析 Source Map，避免批量扫描为空报告做无效工作。
    let source_map_started_at = Instant::now();
    let source_map_skip_reason =
      self.source_map_skip_reason(pending_diagnostics.is_empty());
    let source_map_result = source_map_skip_reason.is_none().then(|| {
      self
        .source_map_resolver
        .resolve_source_file(&source, &self.source_map_loader)
    });
    let source_map_status = source_map_result.as_ref().map_or_else(
      || {
        SourceMapStatus::Skipped(
          source_map_skip_reason.expect("source map was skipped"),
        )
      },
      |result| source_map_status(source.path(), result),
    );
    let resolved_source_map = source_map_result
      .as_ref()
      .and_then(|result| result.as_ref().ok())
      .and_then(Option::as_ref);
    let mut source_map = source_map_started_at.elapsed();

    if pending_diagnostics.is_empty() {
      let timing = CompatAnalysisTiming::new(
        read,
        parse_detect,
        Duration::ZERO,
        source_map,
        Duration::ZERO,
        target_evaluate,
      );

      return Ok((
        CompatReport::new(
          detection.path().to_path_buf(),
          source_map_status,
          diagnostics,
        ),
        timing,
      ));
    }

    let generated_position_started_at = Instant::now();
    let source_index = GeneratedSourceIndex::new(source.source_text());
    let generated_offsets = pending_diagnostics
      .iter()
      .flat_map(|diagnostic| [diagnostic.span.start(), diagnostic.span.end()])
      .collect::<Vec<_>>();
    let generated_positions =
      source_index.positions_for_offsets(&generated_offsets);
    let generated_position = generated_position_started_at.elapsed();
    let mut original_range_recovery = Duration::ZERO;
    let mut original_range_recoverer = resolved_source_map.map(|resolved| {
      OriginalRangeRecoverer::new(&self.detector, resolved.document())
    });

    let generated_ranges =
      generated_positions.chunks_exact(2).map(|positions| {
        GeneratedSourceRange {
          start: positions[0],
          end: positions[1],
        }
      });

    for (pending, generated_range) in
      pending_diagnostics.into_iter().zip(generated_ranges)
    {
      // syntax detector 给出的是 byte span；Source Map 查询需要零基 UTF-16 行列。
      // diagnostic 的 generated position 仍以 usage 起点作为主要定位点；source map
      // 则按范围查询，并由 document 层决定 original end 是否可靠。
      let source_map_lookup_started_at = Instant::now();
      let source_mapping = resolved_source_map
        .as_ref()
        .and_then(|resolved| {
          resolved
            .document()
            .lookup_range(generated_range.start, generated_range.end)
        })
        .map_or_else(
          || {
            source_map_status.unavailable_reason().cloned().map_or_else(
              || {
                if source_map_status.skip_reason().is_some() {
                  SourceMapUnavailableOrLocation::NotResolved
                } else {
                  SourceMapUnavailableOrLocation::Unavailable(
                    SourceMapUnavailable::UnmappedPosition,
                  )
                }
              },
              SourceMapUnavailableOrLocation::Unavailable,
            )
          },
          SourceMapUnavailableOrLocation::Location,
        );
      source_map += source_map_lookup_started_at.elapsed();
      let source_mapping = match source_mapping {
        SourceMapUnavailableOrLocation::Location(location) => {
          let recovered_end =
            original_range_recoverer.as_mut().and_then(|recoverer| {
              let original_range_recovery_started_at = Instant::now();
              let recovered_end =
                recoverer.recover_end(pending.feature, &location);
              original_range_recovery +=
                original_range_recovery_started_at.elapsed();
              recovered_end
            });

          SourceMapUnavailableOrLocation::Location(recovered_end.map_or(
            location.clone(),
            |end| {
              crate::source_map::SourceLocation::new(
                location.source().clone(),
                location.start(),
                Some(end),
              )
            },
          ))
        }
        SourceMapUnavailableOrLocation::Unavailable(reason) => {
          SourceMapUnavailableOrLocation::Unavailable(reason)
        }
        SourceMapUnavailableOrLocation::NotResolved => {
          SourceMapUnavailableOrLocation::NotResolved
        }
      };

      // 一条 diagnostic 对应一个 syntax feature usage。多个 target 的结果聚合在
      // `target_statuses` 中，避免把 usage × target 展开成重复诊断。
      diagnostics.push(CompatDiagnostic::new(
        pending.feature,
        pending.span,
        generated_range.start,
        source_mapping.into_source_mapping(),
        pending.target_statuses,
      ));
    }

    let timing = CompatAnalysisTiming::new(
      read,
      parse_detect,
      generated_position,
      source_map,
      original_range_recovery,
      target_evaluate,
    );

    Ok((
      CompatReport::new(
        detection.path().to_path_buf(),
        source_map_status,
        diagnostics,
      ),
      timing,
    ))
  }

  const fn source_map_skip_reason(
    &self,
    pending_diagnostics_is_empty: bool,
  ) -> Option<SourceMapSkipReason> {
    match self.source_map_policy {
      SourceMapPolicy::Always => None,
      SourceMapPolicy::DiagnosticsOnly if pending_diagnostics_is_empty => {
        Some(SourceMapSkipReason::NoDiagnostics)
      }
      SourceMapPolicy::DiagnosticsOnly => None,
      SourceMapPolicy::Disabled => Some(SourceMapSkipReason::Disabled),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeneratedSourceRange {
  start: crate::source_map::SourcePosition,
  end: crate::source_map::SourcePosition,
}

#[derive(Debug)]
struct PendingDiagnostic {
  feature: SyntaxFeatureId,
  span: SourceSpan,
  target_statuses: Vec<TargetCompatStatus>,
}

impl PendingDiagnostic {
  fn new(
    feature: SyntaxFeatureId,
    span: SourceSpan,
    target_statuses: Vec<TargetCompatStatus>,
  ) -> Self {
    Self {
      feature,
      span,
      target_statuses,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceMapUnavailableOrLocation {
  NotResolved,
  Unavailable(SourceMapUnavailable),
  Location(crate::source_map::SourceLocation),
}

impl SourceMapUnavailableOrLocation {
  // analyze_source 需要先临时保存“映射成功的位置”或“映射不可用原因”。
  // 最终进入报告时再统一转回领域模型 `SourceMapping`，避免在主流程里重复 match。
  fn into_source_mapping(self) -> crate::source_map::SourceMapping {
    match self {
      Self::NotResolved => crate::source_map::SourceMapping::NotResolved,
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
  use crate::source_map::{SourceMapLoadError, SourceMapReference};
  use sourcemap::SourceMapBuilder;

  #[test]
  fn analyzes_source_and_returns_non_supported_diagnostics() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let analyzer = CompatAnalyzer::new();
    let targets = analyzer.resolve_targets(["chrome 79"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(
      report.diagnostics()[0].target_statuses()[0].target_index(),
      0
    );
    assert_eq!(
      report.diagnostics()[0].target_statuses()[0].status(),
      CompatStatus::Unsupported,
    );
  }

  #[test]
  fn keeps_supported_targets_out_of_diagnostics() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let analyzer = CompatAnalyzer::new();
    let targets = analyzer.resolve_targets(["chrome 80"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    assert!(report.diagnostics().is_empty());
  }

  #[test]
  fn diagnostics_only_policy_skips_source_map_without_diagnostics() {
    struct PanicSourceMapLoader;

    impl SourceMapLoader for PanicSourceMapLoader {
      fn load(
        &self,
        _reference: &SourceMapReference,
      ) -> Result<Vec<u8>, SourceMapLoadError> {
        panic!("source map loader should not be called")
      }
    }

    let source = SourceFile::javascript(
      "dist/input.js",
      "const name = user?.name;\n//# sourceMappingURL=input.js.map",
    );

    let analyzer = CompatAnalyzer::builder()
      .source_map_loader(PanicSourceMapLoader)
      .source_map_policy(SourceMapPolicy::DiagnosticsOnly)
      .build();
    let targets = analyzer.resolve_targets(["chrome 80"]).unwrap();
    let (report, timing) = analyzer
      .analyze_source_with_read_timing(source, &targets, Duration::ZERO)
      .unwrap();

    assert!(report.diagnostics().is_empty());
    assert_eq!(
      report.source_map_status(),
      &SourceMapStatus::Skipped(SourceMapSkipReason::NoDiagnostics),
    );
    assert_eq!(timing.generated_position(), Duration::ZERO);
  }

  #[test]
  fn disabled_source_map_policy_keeps_generated_diagnostics() {
    struct PanicSourceMapLoader;

    impl SourceMapLoader for PanicSourceMapLoader {
      fn load(
        &self,
        _reference: &SourceMapReference,
      ) -> Result<Vec<u8>, SourceMapLoadError> {
        panic!("source map loader should not be called")
      }
    }

    let source = SourceFile::javascript(
      "dist/input.js",
      "const name = user?.name;\n//# sourceMappingURL=input.js.map",
    );

    let analyzer = CompatAnalyzer::builder()
      .source_map_loader(PanicSourceMapLoader)
      .source_map_policy(SourceMapPolicy::Disabled)
      .build();
    let targets = analyzer.resolve_targets(["chrome 79"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    assert_eq!(
      report.source_map_status(),
      &SourceMapStatus::Skipped(SourceMapSkipReason::Disabled),
    );
    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(
      report.diagnostics()[0].source_mapping(),
      &crate::source_map::SourceMapping::NotResolved,
    );
  }

  #[test]
  fn can_include_supported_targets_in_diagnostics() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let analyzer = CompatAnalyzer::builder()
      .include_supported_targets(true)
      .build();
    let targets = analyzer.resolve_targets(["chrome 80"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(
      report.diagnostics()[0].target_statuses()[0].target_index(),
      0
    );
    assert_eq!(
      report.diagnostics()[0].target_statuses()[0].status(),
      CompatStatus::Supported,
    );
  }

  #[test]
  fn can_use_a_custom_source_map_loader() {
    struct StaticSourceMapLoader;

    impl SourceMapLoader for StaticSourceMapLoader {
      fn load(
        &self,
        _reference: &SourceMapReference,
      ) -> Result<Vec<u8>, SourceMapLoadError> {
        Ok(
          br#"{
            "version":3,
            "sources":["src/input.ts"],
            "names":[],
            "mappings":"AAAA"
          }"#
            .to_vec(),
        )
      }
    }

    let source = SourceFile::javascript("dist/input.js", "user?.name;");

    let analyzer = CompatAnalyzer::builder()
      .source_map_loader(StaticSourceMapLoader)
      .build();
    let targets = analyzer.resolve_targets(["chrome 79"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    assert!(matches!(
      report.source_map_status(),
      SourceMapStatus::Resolved { .. },
    ));
  }

  #[test]
  fn maps_diagnostic_original_end_when_source_map_has_range_tokens() {
    struct RangeSourceMapLoader;

    impl SourceMapLoader for RangeSourceMapLoader {
      fn load(
        &self,
        _reference: &SourceMapReference,
      ) -> Result<Vec<u8>, SourceMapLoadError> {
        Ok(range_source_map())
      }
    }

    let source = SourceFile::javascript("dist/input.js", "user?.name;");

    let analyzer = CompatAnalyzer::builder()
      .source_map_loader(RangeSourceMapLoader)
      .build();
    let targets = analyzer.resolve_targets(["chrome 79"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    let crate::source_map::SourceMapping::Mapped(location) =
      report.diagnostics()[0].source_mapping()
    else {
      panic!("expected mapped source location");
    };

    assert_eq!(
      location.start(),
      crate::source_map::SourcePosition::new(5, 7)
    );
    assert_eq!(
      location.end(),
      Some(crate::source_map::SourcePosition::new(5, 17)),
    );
  }

  #[test]
  fn recovers_diagnostic_original_end_from_sources_content() {
    struct SourcesContentSourceMapLoader;

    impl SourceMapLoader for SourcesContentSourceMapLoader {
      fn load(
        &self,
        _reference: &SourceMapReference,
      ) -> Result<Vec<u8>, SourceMapLoadError> {
        Ok(
          br#"{
            "version":3,
            "sources":["src/input.js"],
            "sourcesContent":["user?.name;"],
            "names":[],
            "mappings":"AAAA"
          }"#
            .to_vec(),
        )
      }
    }

    let source = SourceFile::javascript("dist/input.js", "user?.name;");

    let analyzer = CompatAnalyzer::builder()
      .source_map_loader(SourcesContentSourceMapLoader)
      .build();
    let targets = analyzer.resolve_targets(["chrome 79"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    let crate::source_map::SourceMapping::Mapped(location) =
      report.diagnostics()[0].source_mapping()
    else {
      panic!("expected mapped source location");
    };

    assert_eq!(
      location.start(),
      crate::source_map::SourcePosition::new(0, 0)
    );
    assert_eq!(
      location.end(),
      Some(crate::source_map::SourcePosition::new(0, 10)),
    );
  }

  #[test]
  fn groups_target_statuses_by_detected_usage() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let analyzer = CompatAnalyzer::new();
    let targets = analyzer
      .resolve_targets(["chrome 79", "firefox 73"])
      .unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    assert_eq!(report.diagnostics().len(), 1);
    assert_eq!(report.diagnostics()[0].target_statuses().len(), 2);
    assert_eq!(
      report.diagnostics()[0].target_statuses()[0].target_index(),
      0
    );
    assert_eq!(
      report.diagnostics()[0].target_statuses()[1].target_index(),
      1
    );
  }

  #[test]
  fn records_missing_source_map_as_report_status() {
    let source = SourceFile::javascript("input.js", "const name = user?.name;");

    let analyzer = CompatAnalyzer::new();
    let targets = analyzer.resolve_targets(["chrome 79"]).unwrap();
    let report = analyzer.analyze_source(source, &targets).unwrap();

    assert!(matches!(
      report.source_map_status(),
      SourceMapStatus::Unavailable(SourceMapUnavailable::NotFound { .. }),
    ));
  }

  fn range_source_map() -> Vec<u8> {
    let mut builder = SourceMapBuilder::new(None);
    builder.add(0, 0, 5, 7, Some("src/input.ts"), None, true);

    let mut output = Vec::new();
    builder.into_sourcemap().to_writer(&mut output).unwrap();
    output
  }
}
