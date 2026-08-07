use std::path::Path;

use ecmascript_compatibility::{
  CompatAnalyzer,
  source_map::{SourceLocation, SourceMapping},
};

const STATICS_DIR: &str = "/Users/10015448/Git/modb-front/dist/statics";
const SAMPLE_FILE: &str = "vendors-node_modules_pnpm_mermaid_11_14_0_node_modules_mermaid_dist_chunks_mermaid_core_quadr-f24755.03e53447901e.chunk.js";
const TARGET_QUERIES: &[&str] = &["chrome 79"];

fn main() {
  let path = Path::new(STATICS_DIR).join(SAMPLE_FILE);
  let report = CompatAnalyzer::new()
    .analyze_path(&path, TARGET_QUERIES.iter().copied())
    .expect("sample compatibility analysis should succeed");

  // println!("Source Map demo directory: {STATICS_DIR}");
  // println!("sample file: {}", report.path().display());
  // println!("source map status: {:?}", report.source_map_status());
  // println!("detected feature usages: {}", report.detected_usage_count());
  // println!("target queries: {}", TARGET_QUERIES.join(", "));
  // println!("resolved targets: {}", report.targets().len());
  // println!("diagnostics: {}", report.diagnostics().len());

  for diagnostic in report.diagnostics() {
    println!(
      "- {:?} at {}:{}:{}",
      diagnostic.feature(),
      diagnostic.path().display(),
      diagnostic.generated_position().line() + 1,
      diagnostic.generated_position().col() + 1,
    );

    match diagnostic.source_mapping() {
      SourceMapping::Mapped(location) => print_original_location(location),
      SourceMapping::Unavailable(reason) => {
        println!("  original: <unavailable: {reason:?}>");
      }
      SourceMapping::NotResolved => println!("  original: <not resolved>"),
    }

    println!("  target status:");
    for target_status in diagnostic.target_statuses() {
      println!(
        "  - {:?} {:?}: {:?}",
        target_status.target().runtime(),
        target_status.target().release(),
        target_status.status(),
      );
    }
  }
}

fn print_original_location(location: &SourceLocation) {
  println!(
    r#"
original: 
{}:{}:{}
"#,
    source_label(location),
    location.start().line() + 1,
    location.start().col() + 1,
  );
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
