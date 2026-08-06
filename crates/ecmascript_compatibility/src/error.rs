use std::path::PathBuf;

use crate::target::InvalidVersionRange;

#[derive(Debug, thiserror::Error)]
pub enum TargetQueryError {
  #[error("at least one non-empty Browserslist query is required")]
  Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceKindError {
  #[error("TypeScript declaration files are not runtime source: `{path}`")]
  DeclarationFile { path: PathBuf },

  #[error("unsupported source file extension: `{path}`")]
  UnsupportedExtension { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeatureDetectionError {
  #[error("failed to parse source file `{path}`: {diagnostics:?}")]
  Parse {
    path: PathBuf,
    diagnostics: Vec<String>,
  },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionParseError {
  #[error("version cannot be empty")]
  Empty,

  #[error("version `{value}` has more than {maximum} components")]
  TooManyComponents { value: String, maximum: usize },

  #[error("version `{value}` contains invalid component `{component}`")]
  InvalidComponent { value: String, component: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeReleaseParseError {
  #[error("invalid runtime release `{value}`: {source}")]
  InvalidVersion {
    value: String,
    #[source]
    source: VersionParseError,
  },

  #[error("invalid runtime release `{value}`: {source}")]
  InvalidRange {
    value: String,
    #[source]
    source: InvalidVersionRange,
  },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetResolveError {
  #[error("failed to resolve Browserslist query {queries:?}: {message}")]
  Browserslist {
    queries: Vec<String>,
    message: String,
  },

  #[error("Browserslist query resolved to no targets")]
  Empty,

  #[error("unsupported runtime `{name}` returned by Browserslist")]
  UnsupportedRuntime { name: String },

  #[error(
    "invalid release `{release}` returned for runtime `{runtime}`: {source}"
  )]
  InvalidRelease {
    runtime: String,
    release: String,
    #[source]
    source: RuntimeReleaseParseError,
  },
}
