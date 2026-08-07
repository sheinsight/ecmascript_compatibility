use std::path::Path;

use ecmascript_compatibility::{
  CompatAnalyzer, CompatStatus, Runtime, RuntimeRelease, SourceMapStatus,
  TargetCompatStatus,
  source_map::{SourceLocation, SourceMapping},
};

const STATICS_DIR: &str = "/Users/10015448/Git/modb-front/dist/statics";
const SAMPLE_FILE: &str = "vendors-node_modules_pnpm_mermaid_11_14_0_node_modules_mermaid_dist_chunks_mermaid_core_quadr-f24755.03e53447901e.chunk.js";
const TARGET_QUERIES: &[&str] = &["chrome 60"];

fn main() {
  let path = Path::new(STATICS_DIR).join(SAMPLE_FILE);
  let report = CompatAnalyzer::new()
    .analyze_path(&path, TARGET_QUERIES.iter().copied())
    .expect("sample compatibility analysis should succeed");

  print_report_summary(&report);

  if report.diagnostics().is_empty() {
    println!();
    println!("No compatibility diagnostics.");
    return;
  }

  println!();
  println!("Diagnostics");
  println!("===========");

  for (index, diagnostic) in report.diagnostics().iter().enumerate() {
    println!("#{:03} {:?}", index + 1, diagnostic.feature(),);
    println!(
      "  generated : {}",
      generated_location_label(
        diagnostic.path(),
        report.path(),
        diagnostic.generated_position().line() + 1,
        diagnostic.generated_position().col() + 1,
      )
    );
    print_original_location(diagnostic.source_mapping());
    print_target_statuses(diagnostic.target_statuses());
    println!();
  }
}

fn print_report_summary(report: &ecmascript_compatibility::CompatReport) {
  let status_counts = StatusCounts::from_diagnostics(report.diagnostics());

  println!("ECMAScript compatibility sample");
  println!("===============================");
  println!("sample file      : {}", report.path().display());
  println!("target queries   : {}", TARGET_QUERIES.join(", "));
  println!(
    "resolved targets : {}",
    report
      .targets()
      .iter()
      .map(|target| format!(
        "{} {}",
        runtime_label(target.runtime()),
        release_label(target.release())
      ))
      .collect::<Vec<_>>()
      .join(", ")
  );
  println!(
    "source map       : {}",
    source_map_status_label(report.source_map_status())
  );
  println!("detected usages  : {}", report.detected_usage_count());
  println!("diagnostics      : {}", report.diagnostics().len());
  println!(
    "target statuses  : unsupported={}, mixed={}, unknown={}",
    status_counts.unsupported, status_counts.mixed, status_counts.unknown,
  );
}

fn print_original_location(source_mapping: &SourceMapping) {
  match source_mapping {
    SourceMapping::Mapped(location) => {
      println!(
        "  original  : {}:{}:{}",
        source_label(location),
        location.start().line() + 1,
        location.start().col() + 1,
      );
    }
    SourceMapping::Unavailable(reason) => {
      println!("  original  : unavailable ({reason:?})");
    }
    SourceMapping::NotResolved => println!("  original  : not resolved"),
  }
}

fn print_target_statuses(target_statuses: &[TargetCompatStatus]) {
  println!("  targets   :");
  for target_status in target_statuses {
    let target = target_status.target();
    println!(
      "    - {:<18} [{:<11}] {}",
      format!(
        "{} {}",
        runtime_label(target.runtime()),
        release_label(target.release())
      ),
      status_label(target_status.status()),
      status_hint(target_status.status()),
    );
  }
}

fn source_label(location: &SourceLocation) -> String {
  if let Some(path) = location.source().as_file() {
    path.display().to_string()
  } else if let Some(source) = location.source().as_str() {
    source.to_string()
  } else {
    "<unknown>".to_string()
  }
}

fn generated_location_label(
  diagnostic_path: &Path,
  report_path: &Path,
  line: u32,
  col: u32,
) -> String {
  if diagnostic_path == report_path {
    format!("line {line}, column {col} (bundle)")
  } else {
    format!("{}:{line}:{col}", diagnostic_path.display())
  }
}

fn source_map_status_label(status: &SourceMapStatus) -> String {
  match status {
    SourceMapStatus::Resolved {
      discovery_kind,
      reference,
      source_count,
    } => format!(
      "resolved via {discovery_kind:?}, {source_count} sources ({reference:?})"
    ),
    SourceMapStatus::Unavailable(reason) => format!("unavailable ({reason:?})"),
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

fn status_hint(status: CompatStatus) -> &'static str {
  match status {
    CompatStatus::Supported => "",
    CompatStatus::Unsupported => "feature is not supported by this target",
    CompatStatus::Mixed => "target range crosses the support boundary",
    CompatStatus::Unknown => "support data is missing or not comparable",
  }
}

#[derive(Default)]
struct StatusCounts {
  unsupported: usize,
  mixed: usize,
  unknown: usize,
}

impl StatusCounts {
  fn from_diagnostics(
    diagnostics: &[ecmascript_compatibility::CompatDiagnostic],
  ) -> Self {
    let mut counts = Self::default();

    for diagnostic in diagnostics {
      for target_status in diagnostic.target_statuses() {
        match target_status.status() {
          CompatStatus::Unsupported => counts.unsupported += 1,
          CompatStatus::Mixed => counts.mixed += 1,
          CompatStatus::Unknown => counts.unknown += 1,
          CompatStatus::Supported => {}
        }
      }
    }

    counts
  }
}
