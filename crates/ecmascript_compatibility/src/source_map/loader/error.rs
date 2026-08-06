use std::path::PathBuf;

/// Source Map 引用加载失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMapLoadError {
  /// 本地 Source Map 文件不存在。
  NotFound(PathBuf),
  /// 当前 loader 不支持传入的引用类型。
  UnsupportedReferenceKind {
    expected: &'static str,
    actual: &'static str,
  },
  /// 默认策略禁止远程 Source Map 加载。
  UnsupportedRemoteUrl(String),
  /// `data:` 引用格式不合法或无法解码。
  InvalidDataUri(String),
  /// 底层 I/O 失败。
  Io { path: PathBuf, message: String },
}
