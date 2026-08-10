use std::path::{Path, PathBuf};

use crate::error::SourceKindError;

use super::kind::SourceKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
  path: PathBuf,
  kind: SourceKind,
  source_text: String,
}

impl SourceFile {
  pub fn new(
    path: impl Into<PathBuf>,
    kind: SourceKind,
    source_text: impl Into<String>,
  ) -> Self {
    Self {
      path: path.into(),
      kind,
      source_text: source_text.into(),
    }
  }

  pub fn from_path(
    path: impl Into<PathBuf>,
    source_text: impl Into<String>,
  ) -> Result<Self, SourceKindError> {
    let path = path.into();
    let kind = SourceKind::from_path(&path)?;

    Ok(Self::new(path, kind, source_text))
  }

  pub fn javascript(
    path: impl Into<PathBuf>,
    source_text: impl Into<String>,
  ) -> Self {
    Self::new(path, SourceKind::JavaScript, source_text)
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  pub const fn kind(&self) -> SourceKind {
    self.kind
  }

  pub fn source_text(&self) -> &str {
    &self.source_text
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn creates_javascript_source() {
    let source = SourceFile::javascript("test.js", "user?.name");

    assert_eq!(source.path(), Path::new("test.js"));
    assert_eq!(source.kind(), SourceKind::JavaScript);
    assert_eq!(source.source_text(), "user?.name");
  }

  #[test]
  fn accepts_owned_path_and_source_text() {
    let source = SourceFile::javascript(
      PathBuf::from("src/index.js"),
      String::from("console.log('hello');"),
    );

    assert_eq!(source.path(), Path::new("src/index.js"));
    assert_eq!(source.source_text(), "console.log('hello');");
  }

  #[test]
  fn infers_source_file_kind_from_its_path() {
    let source =
      SourceFile::from_path("src/index.tsx", "const view = <App />;").unwrap();

    assert_eq!(source.path(), Path::new("src/index.tsx"));
    assert_eq!(source.kind(), SourceKind::Tsx);
    assert_eq!(source.source_text(), "const view = <App />;");
  }

  #[test]
  fn preserves_an_explicit_source_kind() {
    let source = SourceFile::new(
      "virtual-file.js",
      SourceKind::TypeScript,
      "const value: number = 1;",
    );

    assert_eq!(source.path(), Path::new("virtual-file.js"));
    assert_eq!(source.kind(), SourceKind::TypeScript);
    assert_eq!(source.source_text(), "const value: number = 1;");
  }

  #[test]
  fn source_file_from_path_propagates_source_kind_errors() {
    assert_eq!(
      SourceFile::from_path("index.d.ts", "declare const value: number;"),
      Err(SourceKindError::DeclarationFile {
        path: PathBuf::from("index.d.ts"),
      }),
    );
    assert_eq!(
      SourceFile::from_path("README.md", "# readme"),
      Err(SourceKindError::UnsupportedExtension {
        path: PathBuf::from("README.md"),
      }),
    );
  }
}
