/// 兼容性规则对一个目标发布版本的评估结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatStatus {
  /// 目标发布版本完整满足特性的支持要求。
  Supported,

  /// 目标发布版本明确不满足特性的支持要求。
  Unsupported,

  /// 目标版本区间横跨支持边界，区间内同时存在支持和不支持的版本。
  Mixed,

  /// 缺少支持数据，或者目标发布形式无法与数字版本边界可靠比较。
  Unknown,
}
