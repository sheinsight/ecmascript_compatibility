use std::path::{Path, PathBuf};

/// Source Map 中 original source 的身份。
///
/// 这表示 Source Map 映射出来的“原始源码是谁”，例如
/// `webpack://app/src/App.tsx` 或 `src/App.tsx`。它不是构建产物文件，也不是
/// `.map` 文档本身。
///
/// original source 不一定是本地文件，也可能是 `webpack://`、`vite://`、HTTP URL
/// 或缺失值。因此这里不能直接用 `PathBuf` 表示所有来源。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceIdentity {
  /// 可解析为本地文件系统路径的 source。
  File(PathBuf),
  /// 标准 URL source，例如 `file://` 或 `https://`。
  Url(String),
  /// bundler 虚拟 source，例如 `webpack://project/src/index.ts`。
  Virtual(String),
  /// Source Map 没有提供可识别的 source 身份。
  Unknown,
}

impl SourceIdentity {
  pub fn file(path: impl Into<PathBuf>) -> Self {
    Self::File(path.into())
  }

  pub fn url(url: impl Into<String>) -> Option<Self> {
    non_empty(url.into()).map(Self::Url)
  }

  pub fn virtual_source(source: impl Into<String>) -> Option<Self> {
    non_empty(source.into()).map(Self::Virtual)
  }

  pub fn as_file(&self) -> Option<&Path> {
    match self {
      Self::File(path) => Some(path),
      Self::Url(_) | Self::Virtual(_) | Self::Unknown => None,
    }
  }

  pub fn as_str(&self) -> Option<&str> {
    match self {
      Self::Url(url) | Self::Virtual(url) => Some(url),
      Self::File(_) | Self::Unknown => None,
    }
  }
}

fn non_empty(value: String) -> Option<String> {
  if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn source_identity_constructors_reject_empty_strings() {
    assert_eq!(SourceIdentity::url(""), None);
    assert_eq!(SourceIdentity::virtual_source(""), None);
  }

  #[test]
  fn source_identity_does_not_force_virtual_sources_into_paths() {
    let source = SourceIdentity::virtual_source("webpack://app/src/main.ts")
      .expect("virtual source should be accepted");

    assert_eq!(source.as_file(), None);
    assert_eq!(source.as_str(), Some("webpack://app/src/main.ts"));
  }
}
