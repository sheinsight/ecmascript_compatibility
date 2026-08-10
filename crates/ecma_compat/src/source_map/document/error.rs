/// Source Map 文档解析失败原因。
///
/// loader 层只说明“字节从哪里来、是否取到了”；document 层负责说明这些字节
/// 能否成为一个可查询的 Source Map v3 文档。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceMapDocumentParseError {
  /// 文档缺少 `version` 字段，无法确认 Source Map 规范版本。
  #[error("source map document is missing `version`")]
  MissingVersion,
  /// 当前只支持 Source Map v3。
  #[error("unsupported source map version {0}; only version 3 is supported")]
  UnsupportedVersion(u64),
  /// 第三方 sourcemap parser 拒绝了输入。
  #[error("invalid source map document: {0}")]
  InvalidDocument(String),
}

impl From<sourcemap::Error> for SourceMapDocumentParseError {
  fn from(error: sourcemap::Error) -> Self {
    Self::InvalidDocument(error.to_string())
  }
}
