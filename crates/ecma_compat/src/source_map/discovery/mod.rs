mod error;
mod result;

use std::path::{Path, PathBuf};

use path_slash::PathExt;

pub use error::SourceMapDiscoveryError;
pub use result::DiscoveredSourceMap;

use crate::source::SourceFile;

use super::{
  source_map_discovery_kind::SourceMapDiscoveryKind,
  source_map_reference::SourceMapReference,
};

/// Source Map 引用发现器。
///
/// discovery 只回答“应该尝试加载哪个 Source Map 引用”。它不读取文件、不解析
/// Source Map 文档，也不决定加载失败是否可以降级；这些属于 resolver/loader 边界。
#[derive(Debug, Default, Clone, Copy)]
pub struct SourceMapDiscovery;

impl SourceMapDiscovery {
  pub const fn new() -> Self {
    Self
  }

  pub fn discover(
    &self,
    source: &SourceFile,
  ) -> Result<DiscoveredSourceMap, SourceMapDiscoveryError> {
    if let Some(reference) = explicit_source_map_reference(source)? {
      return Ok(DiscoveredSourceMap::new(
        SourceMapDiscoveryKind::Explicit,
        reference,
      ));
    }

    Ok(DiscoveredSourceMap::new(
      SourceMapDiscoveryKind::AdjacentFallback,
      SourceMapReference::local_file(adjacent_source_map_path(source.path())),
    ))
  }
}

fn explicit_source_map_reference(
  source: &SourceFile,
) -> Result<Option<SourceMapReference>, SourceMapDiscoveryError> {
  let mut references = source
    .source_text()
    .lines()
    .filter_map(source_mapping_url)
    .map(|url| reference_from_url(url, source.path()))
    .collect::<Vec<_>>();

  references.dedup();

  match references.as_slice() {
    [] => Ok(None),
    [reference] => Ok(Some(reference.clone())),
    _ => Err(SourceMapDiscoveryError::AmbiguousExplicitReferences(
      references.iter().map(reference_label).collect(),
    )),
  }
}

fn source_mapping_url(line: &str) -> Option<&str> {
  let line = line.trim();

  let directive = line
    .strip_prefix("//#")
    .or_else(|| line.strip_prefix("//@"))
    .or_else(|| {
      line
        .strip_prefix("/*#")
        .or_else(|| line.strip_prefix("/*@"))
        .and_then(|line| line.strip_suffix("*/"))
    })?
    .trim_start();

  directive
    .strip_prefix("sourceMappingURL=")
    .map(str::trim)
    .filter(|url| !url.is_empty())
}

fn reference_from_url(url: &str, generated_path: &Path) -> SourceMapReference {
  if url.starts_with("data:") {
    SourceMapReference::inline_data(url)
      .expect("sourceMappingURL parser rejects empty values")
  } else if url.starts_with("http://") || url.starts_with("https://") {
    SourceMapReference::remote_url(url)
      .expect("sourceMappingURL parser rejects empty values")
  } else {
    let path = PathBuf::from(url);
    if path.is_absolute() {
      SourceMapReference::local_file(path)
    } else {
      SourceMapReference::local_file(
        generated_path
          .parent()
          .unwrap_or_else(|| Path::new(""))
          .join(path),
      )
    }
  }
}

fn adjacent_source_map_path(generated_path: &Path) -> PathBuf {
  let mut path = generated_path.as_os_str().to_os_string();
  path.push(".map");

  PathBuf::from(path)
}

fn reference_label(reference: &SourceMapReference) -> String {
  match reference {
    SourceMapReference::InlineData(data_uri) => data_uri.clone(),
    SourceMapReference::LocalFile(path) => {
      path.to_slash_lossy().into_owned()
    }
    SourceMapReference::RemoteUrl(url) => url.clone(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn discovers_an_explicit_relative_reference() {
    let source = SourceFile::javascript(
      "dist/app.js",
      "console.log(1);\n//# sourceMappingURL=app.js.map",
    );

    let discovered = SourceMapDiscovery::new().discover(&source).unwrap();

    assert_eq!(discovered.kind(), SourceMapDiscoveryKind::Explicit);
    assert_eq!(
      discovered.reference(),
      &SourceMapReference::local_file("dist/app.js.map"),
    );
  }

  #[test]
  fn discovers_an_inline_data_uri_reference() {
    let source = SourceFile::javascript(
      "dist/app.js",
      "console.log(1);\n//# sourceMappingURL=data:application/json,%7B%7D",
    );

    let discovered = SourceMapDiscovery::new().discover(&source).unwrap();

    assert_eq!(discovered.kind(), SourceMapDiscoveryKind::Explicit);
    assert_eq!(
      discovered.reference(),
      &SourceMapReference::inline_data("data:application/json,%7B%7D").unwrap(),
    );
  }

  #[test]
  fn discovers_an_absolute_local_reference() {
    let source = SourceFile::javascript(
      "/dist/app.js",
      "console.log(1);\n//# sourceMappingURL=/tmp/app.js.map",
    );

    let discovered = SourceMapDiscovery::new().discover(&source).unwrap();

    assert_eq!(
      discovered.reference(),
      &SourceMapReference::local_file("/tmp/app.js.map"),
    );
  }

  #[test]
  fn discovers_adjacent_fallback_without_explicit_reference() {
    let source = SourceFile::javascript("dist/app.js", "console.log(1);");

    let discovered = SourceMapDiscovery::new().discover(&source).unwrap();

    assert_eq!(discovered.kind(), SourceMapDiscoveryKind::AdjacentFallback);
    assert_eq!(
      discovered.reference(),
      &SourceMapReference::local_file("dist/app.js.map"),
    );
  }

  #[test]
  fn rejects_conflicting_explicit_references() {
    let source = SourceFile::javascript(
      "dist/app.js",
      "//# sourceMappingURL=one.js.map\n//# sourceMappingURL=two.js.map",
    );

    let result = SourceMapDiscovery::new().discover(&source);

    assert_eq!(
      result.unwrap_err(),
      SourceMapDiscoveryError::AmbiguousExplicitReferences(vec![
        "dist/one.js.map".to_string(),
        "dist/two.js.map".to_string(),
      ]),
    );
  }
}
