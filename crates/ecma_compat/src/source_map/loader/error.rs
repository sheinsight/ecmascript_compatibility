use std::path::PathBuf;

/// Source Map 引用加载失败原因。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceMapLoadError {
  /// 本地 Source Map 文件不存在。
  #[error("local source map file not found: `{0}`")]
  NotFound(PathBuf),
  /// 当前 loader 不支持传入的引用类型。
  #[error(
    "unsupported source map reference kind: expected {expected}, got {actual}"
  )]
  UnsupportedReferenceKind {
    expected: &'static str,
    actual: &'static str,
  },
  /// 默认策略禁止远程 Source Map 加载。
  #[error(
    "remote source map URL is not supported by the default loader: `{0}`"
  )]
  UnsupportedRemoteUrl(String),
  /// `data:` 引用格式不合法或无法解码。
  #[error("invalid source map data URI: {0}")]
  InvalidDataUri(String),
  /// 底层 I/O 失败。
  #[error("failed to read source map file `{path}`: {message}")]
  Io { path: PathBuf, message: String },
}
