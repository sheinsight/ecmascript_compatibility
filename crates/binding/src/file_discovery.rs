use std::{
  fs,
  path::{Path, PathBuf},
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use path_slash::PathExt;
use thiserror::Error;

const DEFAULT_EXTENSIONS: &[&str] = &["js", "mjs", "cjs", "jsx"];

#[derive(Debug, Clone)]
pub struct FileDiscoveryOptions {
  pub patterns: Vec<String>,
  pub extensions: Vec<String>,
  pub respect_gitignore: bool,
  pub ignore_hidden: bool,
}

impl FileDiscoveryOptions {
  pub fn new(
    patterns: &[String],
    extensions: Option<&[String]>,
    respect_gitignore: bool,
    ignore_hidden: bool,
  ) -> Self {
    Self {
      patterns: normalize_globs(Some(patterns)),
      extensions: normalize_extensions(extensions),
      respect_gitignore,
      ignore_hidden,
    }
  }
}

#[derive(Debug, Error)]
pub enum FileDiscoveryError {
  #[error("cwd is not a directory: `{0}`")]
  CwdNotDirectory(String),

  #[error("at least one file pattern is required")]
  EmptyPatterns,

  #[error("path is outside cwd: `{path}`")]
  PathOutsideCwd { path: String },

  #[error(transparent)]
  Io(#[from] std::io::Error),

  #[error(transparent)]
  Glob(#[from] globset::Error),

  #[error(transparent)]
  Ignore(#[from] ignore::Error),
}

pub fn normalize_cwd(cwd: PathBuf) -> Result<PathBuf, FileDiscoveryError> {
  let path = cwd;
  let metadata = fs::metadata(&path)?;

  if !metadata.is_dir() {
    return Err(FileDiscoveryError::CwdNotDirectory(
      path.to_slash_lossy().into_owned(),
    ));
  }

  Ok(path.canonicalize()?)
}

pub fn discover_files(
  cwd: &Path,
  options: &FileDiscoveryOptions,
) -> Result<Vec<PathBuf>, FileDiscoveryError> {
  if options.patterns.is_empty() {
    return Err(FileDiscoveryError::EmptyPatterns);
  }

  let include_set = build_glob_set(&options.patterns)?;
  let mut builder = WalkBuilder::new(cwd);

  builder
    .hidden(options.ignore_hidden)
    .ignore(options.respect_gitignore)
    .git_ignore(options.respect_gitignore)
    .git_global(options.respect_gitignore)
    .git_exclude(options.respect_gitignore)
    .parents(options.respect_gitignore);

  let mut files = Vec::new();

  for entry in builder.build() {
    let entry = entry?;
    let path = entry.path();

    if !entry
      .file_type()
      .is_some_and(|file_type| file_type.is_file())
    {
      continue;
    }

    if !has_extension(path, &options.extensions) {
      continue;
    }

    if !matches_include(cwd, path, &include_set)? {
      continue;
    }

    files.push(path.to_path_buf());
  }

  files.sort();
  Ok(files)
}

fn build_glob_set(globs: &[String]) -> Result<GlobSet, FileDiscoveryError> {
  let mut builder = GlobSetBuilder::new();

  for glob in globs {
    builder.add(Glob::new(glob)?);
  }

  Ok(builder.build()?)
}

fn matches_include(
  cwd: &Path,
  path: &Path,
  include_set: &GlobSet,
) -> Result<bool, FileDiscoveryError> {
  let relative =
    path
      .strip_prefix(cwd)
      .map_err(|_| FileDiscoveryError::PathOutsideCwd {
        path: path.to_slash_lossy().into_owned(),
      })?;

  Ok(include_set.is_match(relative))
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

fn normalize_extensions(extensions: Option<&[String]>) -> Vec<String> {
  let Some(extensions) = extensions else {
    return default_extensions();
  };

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

fn default_extensions() -> Vec<String> {
  DEFAULT_EXTENSIONS
    .iter()
    .map(|extension| (*extension).to_string())
    .collect()
}

fn normalize_globs(globs: Option<&[String]>) -> Vec<String> {
  globs
    .into_iter()
    .flatten()
    .filter_map(|glob| {
      let glob = glob.trim();

      if glob.is_empty() {
        None
      } else {
        Some(glob.to_string())
      }
    })
    .collect()
}
