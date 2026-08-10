use std::{env, path::PathBuf, process::ExitCode};

use ecmascript_compatibility::{
  CompatAnalyzer, CompatStatus, RuntimeRelease,
  source_map::{SourceLocation, SourceMapping},
};

fn main() -> ExitCode {
  let Some((path, targets)) = parse_args() else {
    eprintln!(
      "usage: cargo run -p ecmascript_compatibility --example analyze_file -- <js-file> <target> [target...]"
    );
    return ExitCode::from(2);
  };

  let analyzer = CompatAnalyzer::new();
  let resolved_targets =
    match analyzer.resolve_targets(targets.iter().map(String::as_str)) {
      Ok(targets) => targets,
      Err(error) => {
        eprintln!("analysis failed: {error}");
        return ExitCode::FAILURE;
      }
    };

  let report = match analyzer.analyze_path(&path, &resolved_targets) {
    Ok(report) => report,
    Err(error) => {
      eprintln!("analysis failed: {error}");
      return ExitCode::FAILURE;
    }
  };

  println!("file              : {}", report.path().display());
  println!("targets           : {}", targets.join(", "));
  println!("diagnostics       : {}", report.diagnostics().len());

  for diagnostic in report.diagnostics() {
    let position = diagnostic.position();
    println!();
    println!("{:?}", diagnostic.feature());
    println!(
      "  generated       : {}:{}",
      position.line() + 1,
      position.col() + 1
    );
    print_original_location(diagnostic.source_mapping());

    for target_status in diagnostic.target_statuses() {
      let target = resolved_targets[target_status.target_index()];
      println!(
        "  target          : {:?} {} -> {}",
        target.runtime(),
        release_label(target.release()),
        status_label(target_status.status())
      );
    }
  }

  ExitCode::SUCCESS
}

fn parse_args() -> Option<(PathBuf, Vec<String>)> {
  let mut args = env::args().skip(1);
  let path = args.next().map(PathBuf::from)?;
  let targets = args.collect::<Vec<_>>();

  if targets.is_empty() {
    return None;
  }

  Some((path, targets))
}

fn print_original_location(source_mapping: &SourceMapping) {
  match source_mapping {
    SourceMapping::Mapped(location) => {
      println!(
        "  original        : {}:{}:{}",
        source_label(location),
        location.start().line() + 1,
        location.start().col() + 1
      );
    }
    SourceMapping::Unavailable(reason) => {
      println!("  original        : unavailable ({reason:?})");
    }
    SourceMapping::NotResolved => println!("  original        : not resolved"),
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
