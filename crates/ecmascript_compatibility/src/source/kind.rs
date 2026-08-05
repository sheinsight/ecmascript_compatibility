use std::path::Path;

use oxc::span::SourceType;

use crate::error::SourceKindError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
  JavaScript,
  JavaScriptModule,
  CommonJs,
  Jsx,
  TypeScript,
  TypeScriptModule,
  CommonTypeScript,
  Tsx,
}

impl SourceKind {
  pub fn from_path(path: &Path) -> Result<Self, SourceKindError> {
    if is_declaration_file(path) {
      return Err(SourceKindError::DeclarationFile {
        path: path.to_path_buf(),
      });
    }

    match path.extension().and_then(|extension| extension.to_str()) {
      Some("js") => Ok(Self::JavaScript),
      Some("mjs") => Ok(Self::JavaScriptModule),
      Some("cjs") => Ok(Self::CommonJs),
      Some("jsx") => Ok(Self::Jsx),
      Some("ts") => Ok(Self::TypeScript),
      Some("mts") => Ok(Self::TypeScriptModule),
      Some("cts") => Ok(Self::CommonTypeScript),
      Some("tsx") => Ok(Self::Tsx),
      _ => Err(SourceKindError::UnsupportedExtension {
        path: path.to_path_buf(),
      }),
    }
  }

  pub(crate) const fn source_type(self) -> SourceType {
    match self {
      Self::JavaScript => SourceType::unambiguous(),
      Self::JavaScriptModule => SourceType::mjs(),
      Self::CommonJs => SourceType::cjs(),
      Self::Jsx => SourceType::jsx(),
      Self::TypeScript => SourceType::ts(),
      Self::TypeScriptModule => SourceType::ts().with_module(true),
      Self::CommonTypeScript => SourceType::ts().with_commonjs(true),
      Self::Tsx => SourceType::tsx(),
    }
  }
}

fn is_declaration_file(path: &Path) -> bool {
  let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
    return false;
  };

  file_name.ends_with(".d.ts")
    || file_name.ends_with(".d.mts")
    || file_name.ends_with(".d.cts")
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use super::*;

  #[test]
  fn infers_all_supported_source_kinds() {
    let cases = [
      ("index.js", SourceKind::JavaScript),
      ("index.mjs", SourceKind::JavaScriptModule),
      ("index.cjs", SourceKind::CommonJs),
      ("index.jsx", SourceKind::Jsx),
      ("index.ts", SourceKind::TypeScript),
      ("index.mts", SourceKind::TypeScriptModule),
      ("index.cts", SourceKind::CommonTypeScript),
      ("index.tsx", SourceKind::Tsx),
    ];

    for (path, expected) in cases {
      assert_eq!(SourceKind::from_path(Path::new(path)).unwrap(), expected,);
    }
  }

  #[test]
  fn maps_every_source_kind_to_the_expected_oxc_source_type() {
    let cases = [
      (SourceKind::JavaScript, SourceType::unambiguous()),
      (SourceKind::JavaScriptModule, SourceType::mjs()),
      (SourceKind::CommonJs, SourceType::cjs()),
      (SourceKind::Jsx, SourceType::jsx()),
      (SourceKind::TypeScript, SourceType::ts()),
      (
        SourceKind::TypeScriptModule,
        SourceType::ts().with_module(true),
      ),
      (
        SourceKind::CommonTypeScript,
        SourceType::ts().with_commonjs(true),
      ),
      (SourceKind::Tsx, SourceType::tsx()),
    ];

    for (kind, expected) in cases {
      assert_eq!(kind.source_type(), expected);
    }
  }

  #[test]
  fn accepts_multiple_dots_and_ignores_extensions_in_directory_names() {
    let cases = [
      ("src.ts/index.test.js", SourceKind::JavaScript),
      ("types.d.ts/index.mts", SourceKind::TypeScriptModule),
      ("src/component.test.tsx", SourceKind::Tsx),
    ];

    for (path, expected) in cases {
      assert_eq!(SourceKind::from_path(Path::new(path)).unwrap(), expected);
    }
  }

  #[test]
  fn rejects_typescript_declaration_files_and_preserves_the_path() {
    for path in [
      "index.d.ts",
      "index.d.mts",
      "index.d.cts",
      "src/models/user.test.d.ts",
    ] {
      assert_eq!(
        SourceKind::from_path(Path::new(path)),
        Err(SourceKindError::DeclarationFile {
          path: PathBuf::from(path),
        }),
      );
    }
  }

  #[test]
  fn does_not_treat_declaration_like_names_as_declaration_files() {
    let cases = [
      ("index.d.tsx", SourceKind::Tsx),
      ("index.d.js", SourceKind::JavaScript),
      ("index.d.ts.js", SourceKind::JavaScript),
    ];

    for (path, expected) in cases {
      assert_eq!(SourceKind::from_path(Path::new(path)).unwrap(), expected);
    }
  }

  #[test]
  fn rejects_unsupported_or_missing_extensions_and_preserves_the_path() {
    for path in ["styles.css", "index", ".gitignore", "index.", ""] {
      assert_eq!(
        SourceKind::from_path(Path::new(path)),
        Err(SourceKindError::UnsupportedExtension {
          path: PathBuf::from(path),
        }),
      );
    }
  }

  #[test]
  fn treats_source_extensions_as_case_sensitive() {
    for path in ["index.JS", "index.TS", "index.TSX", "index.D.TS"] {
      assert_eq!(
        SourceKind::from_path(Path::new(path)),
        Err(SourceKindError::UnsupportedExtension {
          path: PathBuf::from(path),
        }),
      );
    }
  }
}
