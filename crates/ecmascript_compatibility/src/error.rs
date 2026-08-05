use std::path::PathBuf;

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
