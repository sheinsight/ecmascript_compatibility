use std::{
  fs,
  path::{Path, PathBuf},
  time::{Duration, Instant},
};

use ecmascript_compatibility::{
  CompatAnalyzer, CompatDiagnostic, CompatReport, CompatStatus, Runtime,
  RuntimeRelease, RuntimeTarget, SourceMapStatus, SourceSpan,
  TargetCompatStatus,
  source_map::{
    SourceIdentity, SourceLocation, SourceMapLoader, SourceMapReference,
    SourceMapping, SourcePosition,
  },
};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use rayon::prelude::*;

const DEFAULT_EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "jsx"];

#[napi(object)]
pub struct AnalyzeCwdOptions {
  pub include_supported_targets: Option<bool>,
  pub extensions: Option<Vec<String>>,
  pub parallelism: Option<u32>,
  pub exclude_empty_reports: Option<bool>,
}

#[napi(object)]
pub struct AnalyzePathOptions {
  pub include_supported_targets: Option<bool>,
}

#[napi(object)]
pub struct JsCompatDirectoryReport {
  pub cwd: String,
  pub targets: Vec<JsRuntimeTarget>,
  pub file_count: u32,
  pub diagnostic_count: u32,
  pub reports: Vec<JsCompatFileReport>,
  pub errors: Vec<JsCompatFileError>,
  pub timing: JsDirectoryTiming,
}

#[napi(object)]
pub struct JsCompatFileError {
  pub path: String,
  pub message: String,
}

#[napi(object)]
pub struct JsCompatFileReport {
  pub path: String,
  pub source_map_status: JsSourceMapStatus,
  pub diagnostics: Vec<JsCompatDiagnostic>,
}

#[napi(object)]
pub struct JsDirectoryTiming {
  pub elapsed_ms: f64,
  pub read_ms: f64,
  pub parse_detect_ms: f64,
  pub generated_position_ms: f64,
  pub source_map_ms: f64,
  pub target_evaluate_ms: f64,
  pub dto_conversion_ms: f64,
}

#[napi(object)]
pub struct JsRuntimeTarget {
  pub runtime: String,
  pub release: String,
}

#[napi(object)]
pub struct JsSourceMapStatus {
  pub kind: String,
  pub discovery_kind: Option<String>,
  pub reference: Option<String>,
  pub reason: Option<String>,
}

#[napi(object)]
pub struct JsCompatDiagnostic {
  pub feature: String,
  pub span: JsSourceSpan,
  pub position: JsSourcePosition,
  pub source_mapping: JsSourceMapping,
  pub target_statuses: Vec<JsTargetCompatStatus>,
}

#[napi(object)]
pub struct JsSourceSpan {
  pub start: u32,
  pub end: u32,
}

#[napi(object)]
pub struct JsSourcePosition {
  pub line: u32,
  pub column: u32,
}

#[napi(object)]
pub struct JsSourceMapping {
  pub kind: String,
  pub source: Option<String>,
  pub start: Option<JsSourcePosition>,
  pub end: Option<JsSourcePosition>,
  pub reason: Option<String>,
}

#[napi(object)]
pub struct JsTargetCompatStatus {
  pub target_index: u32,
  pub status: String,
}

#[napi(js_name = "analyzeCwd")]
pub fn analyze_cwd(
  cwd: String,
  targets: Vec<String>,
  options: Option<AnalyzeCwdOptions>,
) -> Result<JsCompatDirectoryReport> {
  let elapsed_started_at = Instant::now();
  let cwd = normalize_cwd(cwd)?;
  let extensions = options
    .as_ref()
    .and_then(|options| options.extensions.as_ref())
    .map_or_else(default_extensions, |extensions| {
      normalize_extensions(extensions)
    });
  let include_supported_targets = options
    .as_ref()
    .and_then(|options| options.include_supported_targets);
  let parallelism = options.as_ref().and_then(|options| options.parallelism);
  let exclude_empty_reports = options
    .as_ref()
    .and_then(|options| options.exclude_empty_reports)
    .unwrap_or(true);

  analyze_cwd_with_analyzer(
    &cwd,
    &extensions,
    &targets,
    parallelism,
    exclude_empty_reports,
    elapsed_started_at,
    &analyzer_from_options(include_supported_targets),
  )
}

fn analyze_cwd_with_analyzer<L>(
  cwd: &Path,
  extensions: &[String],
  targets: &[String],
  parallelism: Option<u32>,
  exclude_empty_reports: bool,
  elapsed_started_at: Instant,
  analyzer: &CompatAnalyzer<L>,
) -> Result<JsCompatDirectoryReport>
where
  L: SourceMapLoader + Sync,
{
  let files = discover_js_files(cwd, extensions)?;
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

  if exclude_empty_reports {
    analyzed_reports.retain(|entry| !entry.report.diagnostics.is_empty());
  }

  let diagnostic_count = analyzed_reports
    .iter()
    .map(|entry| entry.report.diagnostics.len() as u32)
    .sum();
  let timing = JsDirectoryTiming::from_file_timings(
    elapsed_started_at.elapsed(),
    &analyzed_reports,
  );
  let reports = analyzed_reports
    .into_iter()
    .map(|entry| entry.report)
    .collect::<Vec<_>>();

  Ok(JsCompatDirectoryReport {
    cwd: path_label(cwd),
    targets: resolved_targets.iter().copied().map(Into::into).collect(),
    file_count: reports.len() as u32,
    diagnostic_count,
    reports,
    errors,
    timing,
  })
}

fn analyze_files_in_parallel<L>(
  files: &[PathBuf],
  targets: &[RuntimeTarget],
  analyzer: &CompatAnalyzer<L>,
) -> Vec<FileAnalysisEntry>
where
  L: SourceMapLoader + Sync,
{
  files
    .par_iter()
    .map(|path| match analyzer.analyze_path(path, targets) {
      Ok(report) => {
        FileAnalysisEntry::Report(JsAnalyzedFileReport::from_report(report))
      }
      Err(error) => FileAnalysisEntry::Error(JsCompatFileError {
        path: path_label(path),
        message: error.to_string(),
      }),
    })
    .collect()
}

enum FileAnalysisEntry {
  Report(JsAnalyzedFileReport),
  Error(JsCompatFileError),
}

#[napi(js_name = "analyzePath")]
pub fn analyze_path(
  path: String,
  targets: Vec<String>,
  options: Option<AnalyzePathOptions>,
) -> Result<JsCompatFileReport> {
  let include_supported_targets = options
    .as_ref()
    .and_then(|options| options.include_supported_targets);

  let analyzer = analyzer_from_options(include_supported_targets);
  let resolved_targets = analyzer
    .resolve_targets(targets.iter().map(String::as_str))
    .map_err(to_napi_error)?;

  analyzer
    .analyze_path(path, &resolved_targets)
    .map(JsCompatFileReport::from_report)
    .map_err(to_napi_error)
}

fn analyzer_from_options(
  include_supported_targets: Option<bool>,
) -> CompatAnalyzer {
  CompatAnalyzer::builder()
    .include_supported_targets(include_supported_targets.unwrap_or(false))
    .build()
}

fn normalize_cwd(cwd: String) -> Result<PathBuf> {
  let path = PathBuf::from(cwd);
  let metadata = fs::metadata(&path).map_err(to_napi_error)?;

  if !metadata.is_dir() {
    return Err(Error::new(
      Status::InvalidArg,
      format!("cwd is not a directory: `{}`", path.display()),
    ));
  }

  path.canonicalize().map_err(to_napi_error)
}

fn discover_js_files(
  cwd: &Path,
  extensions: &[String],
) -> Result<Vec<PathBuf>> {
  let mut files = Vec::new();
  collect_js_files(cwd, extensions, &mut files)?;
  files.sort();
  Ok(files)
}

fn collect_js_files(
  dir: &Path,
  extensions: &[String],
  files: &mut Vec<PathBuf>,
) -> Result<()> {
  for entry in fs::read_dir(dir).map_err(to_napi_error)? {
    let entry = entry.map_err(to_napi_error)?;
    let path = entry.path();
    let file_type = entry.file_type().map_err(to_napi_error)?;

    if file_type.is_dir() {
      collect_js_files(&path, extensions, files)?;
    } else if file_type.is_file() && has_extension(&path, extensions) {
      files.push(path);
    }
  }

  Ok(())
}

fn has_extension(path: &Path, extensions: &[String]) -> bool {
  path
    .extension()
    .and_then(|extension| extension.to_str())
    .is_some_and(|extension| {
      extensions
        .iter()
        .any(|expected| extension.eq_ignore_ascii_case(expected))
    })
}

fn default_extensions() -> Vec<String> {
  DEFAULT_EXTENSIONS
    .iter()
    .map(|extension| (*extension).to_string())
    .collect()
}

fn normalize_extensions(extensions: &[String]) -> Vec<String> {
  let normalized = extensions
    .iter()
    .filter_map(|extension| {
      let extension = extension.trim().trim_start_matches('.');

      if extension.is_empty() {
        None
      } else {
        Some(extension.to_ascii_lowercase())
      }
    })
    .collect::<Vec<_>>();

  if normalized.is_empty() {
    default_extensions()
  } else {
    normalized
  }
}

impl JsCompatFileReport {
  fn from_report(report: CompatReport) -> Self {
    JsAnalyzedFileReport::from_report(report).report
  }
}

struct JsAnalyzedFileReport {
  report: JsCompatFileReport,
  timing: FileTiming,
}

struct FileTiming {
  read_ms: f64,
  parse_detect_ms: f64,
  generated_position_ms: f64,
  source_map_ms: f64,
  target_evaluate_ms: f64,
  dto_conversion_ms: f64,
}

impl JsAnalyzedFileReport {
  fn from_report(report: CompatReport) -> Self {
    let dto_conversion_started_at = Instant::now();
    let timing = report.timing();

    let report = JsCompatFileReport {
      path: path_label(report.path()),
      source_map_status: report.source_map_status().into(),
      diagnostics: report
        .diagnostics()
        .iter()
        .map(JsCompatDiagnostic::from)
        .collect(),
    };
    let mut timing = FileTiming {
      read_ms: duration_ms(timing.read()),
      parse_detect_ms: duration_ms(timing.parse_detect()),
      generated_position_ms: duration_ms(timing.generated_position()),
      source_map_ms: duration_ms(timing.source_map()),
      target_evaluate_ms: duration_ms(timing.target_evaluate()),
      dto_conversion_ms: 0.0,
    };

    timing.dto_conversion_ms = duration_ms(dto_conversion_started_at.elapsed());

    Self { report, timing }
  }
}

impl JsDirectoryTiming {
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

impl From<RuntimeTarget> for JsRuntimeTarget {
  fn from(target: RuntimeTarget) -> Self {
    Self {
      runtime: runtime_label(target.runtime()).to_string(),
      release: release_label(target.release()),
    }
  }
}

impl From<&SourceMapStatus> for JsSourceMapStatus {
  fn from(status: &SourceMapStatus) -> Self {
    match status {
      SourceMapStatus::Resolved {
        discovery_kind,
        reference,
      } => Self {
        kind: "resolved".to_string(),
        discovery_kind: Some(format!("{discovery_kind:?}")),
        reference: Some(source_map_reference_label(reference)),
        reason: None,
      },
      SourceMapStatus::Unavailable(reason) => Self {
        kind: "unavailable".to_string(),
        discovery_kind: None,
        reference: None,
        reason: Some(format!("{reason:?}")),
      },
    }
  }
}

impl From<&CompatDiagnostic> for JsCompatDiagnostic {
  fn from(diagnostic: &CompatDiagnostic) -> Self {
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

impl From<SourceSpan> for JsSourceSpan {
  fn from(span: SourceSpan) -> Self {
    Self {
      start: span.start(),
      end: span.end(),
    }
  }
}

impl From<SourcePosition> for JsSourcePosition {
  fn from(position: SourcePosition) -> Self {
    Self {
      line: position.line(),
      column: position.col(),
    }
  }
}

impl From<&SourceMapping> for JsSourceMapping {
  fn from(mapping: &SourceMapping) -> Self {
    match mapping {
      SourceMapping::NotResolved => Self {
        kind: "notResolved".to_string(),
        source: None,
        start: None,
        end: None,
        reason: None,
      },
      SourceMapping::Mapped(location) => source_location_mapping(location),
      SourceMapping::Unavailable(reason) => Self {
        kind: "unavailable".to_string(),
        source: None,
        start: None,
        end: None,
        reason: Some(format!("{reason:?}")),
      },
    }
  }
}

impl From<TargetCompatStatus> for JsTargetCompatStatus {
  fn from(target_status: TargetCompatStatus) -> Self {
    Self {
      target_index: target_status.target_index() as u32,
      status: status_label(target_status.status()).to_string(),
    }
  }
}

fn source_location_mapping(location: &SourceLocation) -> JsSourceMapping {
  JsSourceMapping {
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
  path.display().to_string()
}

fn to_napi_error(error: impl std::error::Error) -> Error {
  Error::new(Status::GenericFailure, error.to_string())
}

fn duration_ms(duration: Duration) -> f64 {
  duration.as_secs_f64() * 1000.0
}
