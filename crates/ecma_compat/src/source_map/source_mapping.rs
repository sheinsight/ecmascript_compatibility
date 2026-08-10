use std::path::PathBuf;

use super::source_location::SourceLocation;

/// 单条特性使用的 Source Map 映射状态。
///
/// 这个状态区分“尚未尝试映射”和“已尝试但不可用”，避免最终诊断把未处理
/// 的 usage 静默当成正常的 generated-only 结果。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceMapping {
  /// detector 刚产生 usage，尚未进入 Source Map 解析或映射阶段。
  NotResolved,
  /// generated 位置已经成功映射到 original source。
  Mapped(SourceLocation),
  /// 已尝试映射，但无法得到可靠的 original location。
  Unavailable(SourceMapUnavailable),
}

/// Source Map 不可用或无法完成映射的结构化原因。
///
/// 这些状态都不应让兼容性 diagnostic 消失；调用方应保留产物位置，并按需要展示
/// 对应的降级原因。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceMapUnavailable {
  /// 没有显式引用，且同名 `.map` 回退文件不存在。
  NotFound { fallback_path: PathBuf },
  /// 存在显式 `sourceMappingURL`，但该引用无法加载。
  ExplicitReferenceUnavailable { reference: String, message: String },
  /// 文件中存在多个互相冲突的显式 Source Map 引用。
  AmbiguousReference { references: Vec<String> },
  /// Source Map 文档无法解析或缺少必要结构。
  InvalidDocument { location: String, message: String },
  /// Source Map 版本不在当前支持范围内。
  UnsupportedVersion { version: String },
  /// Source Map 存在，但没有覆盖当前 generated 位置。
  UnmappedPosition,
  /// 已得到 original source 身份或位置，但无法获得 original source 文本。
  OriginalSourceUnavailable { source: String },
}
