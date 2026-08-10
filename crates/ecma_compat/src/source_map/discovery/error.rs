/// Source Map 引用发现失败原因。
///
/// 这类错误发生在读取 `.map` 文档之前，通常说明 generated 文件本身给出的
/// `sourceMappingURL` 信息不够明确。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceMapDiscoveryError {
  /// 文件中存在多个不同的显式 Source Map 引用，discovery 不能安全地替调用方选择。
  #[error("ambiguous explicit source map references: {0:?}")]
  AmbiguousExplicitReferences(Vec<String>),
}
