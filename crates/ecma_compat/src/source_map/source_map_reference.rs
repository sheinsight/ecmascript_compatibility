use std::path::PathBuf;

use path_slash::PathExt;

/// resolver 归一化后的 Source Map 文档引用。
///
/// 这里描述"Source Map 文件/文档在哪里"，例如 `dist/main.js.map` 或内联
/// `data:` 文档。它不表示构建产物文件，也不表示 Source Map 内部的
/// original source。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceMapReference {
  /// 内联 `data:` Source Map 内容。
  InlineData(String),
  /// 本地 Source Map 文件路径，例如 `dist/main.js.map`。
  LocalFile(PathBuf),
  /// 远程 Source Map 文档 URL，默认策略不加载。
  RemoteUrl(String),
}

impl SourceMapReference {
  pub fn inline_data(data_uri: impl Into<String>) -> Option<Self> {
    non_empty(data_uri.into()).map(Self::InlineData)
  }

  /// 构造本地文件引用，路径统一规范化为 `/` 分隔符。
  pub fn local_file(path: impl Into<PathBuf>) -> Self {
    let path = path.into();
    Self::LocalFile(PathBuf::from(path.to_slash_lossy().as_ref()))
  }

  pub fn remote_url(url: impl Into<String>) -> Option<Self> {
    non_empty(url.into()).map(Self::RemoteUrl)
  }
}

fn non_empty(value: String) -> Option<String> {
  if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_empty_string_references() {
    assert_eq!(SourceMapReference::inline_data(""), None);
    assert_eq!(SourceMapReference::remote_url(""), None);
  }

  #[test]
  fn keeps_local_file_references_as_paths() {
    assert_eq!(
      SourceMapReference::local_file("dist/main.js.map"),
      SourceMapReference::LocalFile(PathBuf::from("dist/main.js.map")),
    );
  }

  #[test]
  fn normalizes_platform_separators_in_local_file_paths() {
    // 在 Windows 上 Path::join 会产出 `\`，to_slash_lossy 将其转为 `/`。
    // 在 Unix 上 Path::join 只产出 `/`，无需转换。
    let joined = std::path::Path::new("dist").join("app.js.map");
    let reference = SourceMapReference::local_file(joined);
    assert_eq!(
      reference,
      SourceMapReference::LocalFile(PathBuf::from("dist/app.js.map")),
    );
  }
}
