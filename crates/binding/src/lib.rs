use std::{
  env,
  path::{Path, PathBuf},
  time::{Duration, Instant},
};

use path_slash::PathExt;

use ecma_compat::{
  CompatAnalysisTiming, CompatAnalyzer,
  CompatDiagnostic as RustCompatDiagnostic, CompatReport, CompatStatus,
  Runtime, RuntimeRelease, RuntimeTarget as RustRuntimeTarget,
  SourceMapStatus as RustSourceMapStatus, SourceSpan as RustSourceSpan,
  TargetCompatStatus as RustTargetCompatStatus,
  source_map::{
    SourceIdentity, SourceLocation, SourceMapLoader, SourceMapReference,
    SourceMapping as RustSourceMapping, SourcePosition as RustSourcePosition,
  },
};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use rayon::prelude::*;

mod file_discovery;

use file_discovery::{FileDiscoveryOptions, discover_files, normalize_cwd};

#[napi(object)]
pub struct CheckFilesOptions {
  pub include_supported_targets: Option<bool>,
  pub cwd: Option<String>,
  pub extensions: Option<Vec<String>>,
  pub respect_gitignore: Option<bool>,
  pub ignore_hidden: Option<bool>,
  pub parallelism: Option<u32>,
  pub exclude_empty_reports: Option<bool>,
}

#[napi(object)]
pub struct CheckFileOptions {
  pub include_supported_targets: Option<bool>,
}

#[napi(object)]
pub struct CompatFilesReport {
  pub cwd: String,
  pub targets: Vec<RuntimeTarget>,
  pub counts: CompatFilesCounts,
  pub reports: Vec<CompatFileReport>,
  pub errors: Vec<CompatFileError>,
  pub timing: FilesTiming,
}

#[napi(object)]
pub struct CompatFilesCounts {
  pub matched_files: u32,
  pub analyzed_files: u32,
  pub reported_files: u32,
  pub diagnostics: u32,
  pub errors: u32,
}

#[napi(object)]
pub struct CompatFileError {
  pub path: String,
  pub message: String,
}

#[napi(object)]
pub struct CompatFileReport {
  pub path: String,
  pub source_map_status: SourceMapStatus,
  pub diagnostics: Vec<CompatDiagnostic>,
}

#[napi(object)]
pub struct FilesTiming {
  pub elapsed_ms: f64,
  pub read_ms: f64,
  pub parse_detect_ms: f64,
  pub generated_position_ms: f64,
  pub source_map_ms: f64,
  pub original_range_recovery_ms: f64,
  pub target_evaluate_ms: f64,
  pub dto_conversion_ms: f64,
}

#[napi(object)]
pub struct RuntimeTarget {
  pub runtime: String,
  pub release: String,
}

#[napi(object)]
pub struct SourceMapStatus {
  pub kind: String,
  pub discovery_kind: Option<String>,
  pub reference: Option<String>,
  pub reason: Option<String>,
}

#[napi(object)]
pub struct CompatDiagnostic {
  pub feature: String,
  pub span: SourceSpan,
  pub position: SourcePosition,
  pub source_mapping: SourceMapping,
  pub target_statuses: Vec<TargetCompatStatus>,
}

#[napi(object)]
pub struct SourceSpan {
  pub start: u32,
  pub end: u32,
}

#[napi(object)]
pub struct SourcePosition {
  pub line: u32,
  pub column: u32,
}

#[napi(object)]
pub struct SourceMapping {
  pub kind: String,
  pub source: Option<String>,
  pub start: Option<SourcePosition>,
  pub end: Option<SourcePosition>,
  pub reason: Option<String>,
}

#[napi(object)]
pub struct TargetCompatStatus {
  pub target_index: u32,
  pub status: String,
}

#[napi(js_name = "checkFiles")]
pub fn check_files(
  patterns: Vec<String>,
  targets: Vec<String>,
  options: Option<CheckFilesOptions>,
) -> Result<CompatFilesReport> {
  let elapsed_started_at = Instant::now();
  let cwd = options
    .as_ref()
    .and_then(|options| options.cwd.as_ref())
    .map(PathBuf::from)
    .map_or_else(env::current_dir, Ok)
    .map_err(to_napi_error)?;
  let cwd = normalize_cwd(cwd).map_err(to_napi_error)?;
  let discovery_options = FileDiscoveryOptions::new(
    &patterns,
    options
      .as_ref()
      .and_then(|options| options.extensions.as_deref()),
    options
      .as_ref()
      .and_then(|options| options.respect_gitignore)
      .unwrap_or(false),
    options
      .as_ref()
      .and_then(|options| options.ignore_hidden)
      .unwrap_or(false),
  );
  let include_supported_targets = options
    .as_ref()
    .and_then(|options| options.include_supported_targets);
  let parallelism = options.as_ref().and_then(|options| options.parallelism);
  let exclude_empty_reports = options
    .as_ref()
    .and_then(|options| options.exclude_empty_reports)
    .unwrap_or(true);

  check_files_with_analyzer(
    &cwd,
    &discovery_options,
    &targets,
    parallelism,
    exclude_empty_reports,
    elapsed_started_at,
    &analyzer_from_options(include_supported_targets),
  )
}

fn check_files_with_analyzer<L>(
  cwd: &Path,
  discovery_options: &FileDiscoveryOptions,
  targets: &[String],
  parallelism: Option<u32>,
  exclude_empty_reports: bool,
  elapsed_started_at: Instant,
  analyzer: &CompatAnalyzer<L>,
) -> Result<CompatFilesReport>
where
  L: SourceMapLoader + Sync,
{
  let files = discover_files(cwd, discovery_options).map_err(to_napi_error)?;
  let resolved_targets = analyzer
    .resolve_targets(targets.iter().map(String::as_str))
    .map_err(to_napi_error)?;
  let entries = if let Some(parallelism) = parallelism {
    let pool = rayon::ThreadPoolBuilder::new()
      .num_threads(parallelism.max(1) as usize)
      .build()
      .map_err(to_napi_error)?;

    pool.install(|| {
      analyze_files_in_parallel(&files, &resolved_targets, analyzer)
    })
  } else {
    analyze_files_in_parallel(&files, &resolved_targets, analyzer)
  };

  let mut analyzed_reports = Vec::new();
  let mut errors = Vec::new();

  for entry in entries {
    match entry {
      FileAnalysisEntry::Report(analyzed_report) => {
        analyzed_reports.push(analyzed_report);
      }
      FileAnalysisEntry::Error(error) => errors.push(error),
    }
  }

  let matched_files = files.len() as u32;
  let analyzed_files = analyzed_reports.len() as u32;
  let diagnostics = analyzed_reports
    .iter()
    .map(|entry| entry.report.diagnostics.len() as u32)
    .sum();
  let timing = FilesTiming::from_file_timings(
    elapsed_started_at.elapsed(),
    &analyzed_reports,
  );
  let reported_reports = analyzed_reports
    .into_iter()
    .filter(|entry| {
      !exclude_empty_reports || !entry.report.diagnostics.is_empty()
    })
    .map(|entry| entry.report)
    .collect::<Vec<_>>();
  let reported_files = reported_reports.len() as u32;

  Ok(CompatFilesReport {
    cwd: path_label(cwd),
    targets: resolved_targets.iter().copied().map(Into::into).collect(),
    counts: CompatFilesCounts {
      matched_files,
      analyzed_files,
      reported_files,
      diagnostics,
      errors: errors.len() as u32,
    },
    reports: reported_reports,
    errors,
    timing,
  })
}

fn analyze_files_in_parallel<L>(
  files: &[PathBuf],
  targets: &[RustRuntimeTarget],
  analyzer: &CompatAnalyzer<L>,
) -> Vec<FileAnalysisEntry>
where
  L: SourceMapLoader + Sync,
{
  files
    .par_iter()
    .map(
      |path| match analyzer.analyze_path_with_timing(path, targets) {
        Ok((report, timing)) => FileAnalysisEntry::Report(
          JsAnalyzedFileReport::from_report_and_timing(report, timing),
        ),
        Err(error) => FileAnalysisEntry::Error(CompatFileError {
          path: path_label(path),
          message: error.to_string(),
        }),
      },
    )
    .collect()
}

enum FileAnalysisEntry {
  Report(JsAnalyzedFileReport),
  Error(CompatFileError),
}

#[napi(js_name = "checkFile")]
pub fn check_file(
  path: String,
  targets: Vec<String>,
  options: Option<CheckFileOptions>,
) -> Result<CompatFileReport> {
  let include_supported_targets = options
    .as_ref()
    .and_then(|options| options.include_supported_targets);

  let analyzer = analyzer_from_options(include_supported_targets);
  let resolved_targets = analyzer
    .resolve_targets(targets.iter().map(String::as_str))
    .map_err(to_napi_error)?;

  analyzer
    .analyze_path(path, &resolved_targets)
    .map(CompatFileReport::from_report)
    .map_err(to_napi_error)
}

fn analyzer_from_options(
  include_supported_targets: Option<bool>,
) -> CompatAnalyzer {
  CompatAnalyzer::builder()
    .include_supported_targets(include_supported_targets.unwrap_or(false))
    .build()
}

impl CompatFileReport {
  fn from_report(report: CompatReport) -> Self {
    CompatFileReport {
      path: path_label(report.path()),
      source_map_status: report.source_map_status().into(),
      diagnostics: report
        .diagnostics()
        .iter()
        .map(CompatDiagnostic::from)
        .collect(),
    }
  }
}

struct JsAnalyzedFileReport {
  report: CompatFileReport,
  timing: FileTiming,
}

struct FileTiming {
  read_ms: f64,
  parse_detect_ms: f64,
  generated_position_ms: f64,
  source_map_ms: f64,
  original_range_recovery_ms: f64,
  target_evaluate_ms: f64,
  dto_conversion_ms: f64,
}

impl JsAnalyzedFileReport {
  fn from_report_and_timing(
    report: CompatReport,
    timing: CompatAnalysisTiming,
  ) -> Self {
    let dto_conversion_started_at = Instant::now();

    let report = CompatFileReport {
      path: path_label(report.path()),
      source_map_status: report.source_map_status().into(),
      diagnostics: report
        .diagnostics()
        .iter()
        .map(CompatDiagnostic::from)
        .collect(),
    };
    let mut timing = FileTiming {
      read_ms: duration_ms(timing.read()),
      parse_detect_ms: duration_ms(timing.parse_detect()),
      generated_position_ms: duration_ms(timing.generated_position()),
      source_map_ms: duration_ms(timing.source_map()),
      original_range_recovery_ms: duration_ms(timing.original_range_recovery()),
      target_evaluate_ms: duration_ms(timing.target_evaluate()),
      dto_conversion_ms: 0.0,
    };

    timing.dto_conversion_ms = duration_ms(dto_conversion_started_at.elapsed());

    Self { report, timing }
  }
}

impl FilesTiming {
  fn from_file_timings(
    elapsed: Duration,
    reports: &[JsAnalyzedFileReport],
  ) -> Self {
    Self {
      elapsed_ms: duration_ms(elapsed),
      read_ms: reports.iter().map(|report| report.timing.read_ms).sum(),
      parse_detect_ms: reports
        .iter()
        .map(|report| report.timing.parse_detect_ms)
        .sum(),
      generated_position_ms: reports
        .iter()
        .map(|report| report.timing.generated_position_ms)
        .sum(),
      source_map_ms: reports
        .iter()
        .map(|report| report.timing.source_map_ms)
        .sum(),
      original_range_recovery_ms: reports
        .iter()
        .map(|report| report.timing.original_range_recovery_ms)
        .sum(),
      target_evaluate_ms: reports
        .iter()
        .map(|report| report.timing.target_evaluate_ms)
        .sum(),
      dto_conversion_ms: reports
        .iter()
        .map(|report| report.timing.dto_conversion_ms)
        .sum(),
    }
  }
}

impl From<RustRuntimeTarget> for RuntimeTarget {
  fn from(target: RustRuntimeTarget) -> Self {
    Self {
      runtime: runtime_label(target.runtime()).to_string(),
      release: release_label(target.release()),
    }
  }
}

impl From<&RustSourceMapStatus> for SourceMapStatus {
  fn from(status: &RustSourceMapStatus) -> Self {
    match status {
      RustSourceMapStatus::Resolved {
        discovery_kind,
        reference,
      } => Self {
        kind: "resolved".to_string(),
        discovery_kind: Some(format!("{discovery_kind:?}")),
        reference: Some(source_map_reference_label(reference)),
        reason: None,
      },
      RustSourceMapStatus::Unavailable(reason) => Self {
        kind: "unavailable".to_string(),
        discovery_kind: None,
        reference: None,
        reason: Some(format!("{reason:?}")),
      },
    }
  }
}

impl From<&RustCompatDiagnostic> for CompatDiagnostic {
  fn from(diagnostic: &RustCompatDiagnostic) -> Self {
    Self {
      feature: format!("{:?}", diagnostic.feature()),
      span: diagnostic.span().into(),
      position: diagnostic.position().into(),
      source_mapping: diagnostic.source_mapping().into(),
      target_statuses: diagnostic
        .target_statuses()
        .iter()
        .copied()
        .map(Into::into)
        .collect(),
    }
  }
}

impl From<RustSourceSpan> for SourceSpan {
  fn from(span: RustSourceSpan) -> Self {
    Self {
      start: span.start(),
      end: span.end(),
    }
  }
}

impl From<RustSourcePosition> for SourcePosition {
  fn from(position: RustSourcePosition) -> Self {
    Self {
      line: position.line(),
      column: position.col(),
    }
  }
}

impl From<&RustSourceMapping> for SourceMapping {
  fn from(mapping: &RustSourceMapping) -> Self {
    match mapping {
      RustSourceMapping::NotResolved => Self {
        kind: "notResolved".to_string(),
        source: None,
        start: None,
        end: None,
        reason: None,
      },
      RustSourceMapping::Mapped(location) => source_location_mapping(location),
      RustSourceMapping::Unavailable(reason) => Self {
        kind: "unavailable".to_string(),
        source: None,
        start: None,
        end: None,
        reason: Some(format!("{reason:?}")),
      },
    }
  }
}

impl From<RustTargetCompatStatus> for TargetCompatStatus {
  fn from(target_status: RustTargetCompatStatus) -> Self {
    Self {
      target_index: target_status.target_index() as u32,
      status: status_label(target_status.status()).to_string(),
    }
  }
}

fn source_location_mapping(location: &SourceLocation) -> SourceMapping {
  SourceMapping {
    kind: "mapped".to_string(),
    source: Some(source_identity_label(location.source())),
    start: Some(location.start().into()),
    end: location.end().map(Into::into),
    reason: None,
  }
}

fn source_identity_label(source: &SourceIdentity) -> String {
  if let Some(path) = source.as_file() {
    path_label(path)
  } else if let Some(source) = source.as_str() {
    source.to_string()
  } else {
    "<unknown>".to_string()
  }
}

fn source_map_reference_label(reference: &SourceMapReference) -> String {
  match reference {
    SourceMapReference::InlineData(data_uri) => data_uri.clone(),
    SourceMapReference::LocalFile(path) => path_label(path),
    SourceMapReference::RemoteUrl(url) => url.clone(),
  }
}

fn runtime_label(runtime: Runtime) -> &'static str {
  match runtime {
    Runtime::InternetExplorer => "ie",
    Runtime::Edge => "edge",
    Runtime::Firefox => "firefox",
    Runtime::Chrome => "chrome",
    Runtime::Safari => "safari",
    Runtime::Opera => "opera",
    Runtime::Ios => "ios",
    Runtime::OperaMini => "opera-mini",
    Runtime::Android => "android",
    Runtime::Blackberry => "blackberry",
    Runtime::OperaMobile => "opera-mobile",
    Runtime::ChromeAndroid => "chrome-android",
    Runtime::FirefoxAndroid => "firefox-android",
    Runtime::InternetExplorerMobile => "ie-mobile",
    Runtime::UcAndroid => "uc-android",
    Runtime::SamsungInternet => "samsung-internet",
    Runtime::QqAndroid => "qq-android",
    Runtime::Baidu => "baidu",
    Runtime::KaiOS => "kaios",
    Runtime::Node => "node",
  }
}

fn release_label(release: RuntimeRelease) -> String {
  match release {
    RuntimeRelease::Exact(version) => version.to_string(),
    RuntimeRelease::Range(range) => {
      format!("{}-{}", range.start(), range.end())
    }
    RuntimeRelease::Preview => "preview".to_string(),
    RuntimeRelease::All => "all".to_string(),
  }
}

fn status_label(status: CompatStatus) -> &'static str {
  match status {
    CompatStatus::Supported => "supported",
    CompatStatus::Unsupported => "unsupported",
    CompatStatus::Mixed => "mixed",
    CompatStatus::Unknown => "unknown",
  }
}

fn path_label(path: &Path) -> String {
  path.to_slash_lossy().into_owned()
}

fn to_napi_error(error: impl std::error::Error) -> Error {
  Error::new(Status::GenericFailure, error.to_string())
}

fn duration_ms(duration: Duration) -> f64 {
  duration.as_secs_f64() * 1000.0
}
