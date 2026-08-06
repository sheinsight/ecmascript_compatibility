/// Source Map 引用的发现方式。
///
/// 显式引用具有更高优先级；只有完全没有显式引用时，resolver 才应该尝试同名
/// `.map` 回退。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceMapDiscoveryKind {
  /// 来自文件内 `sourceMappingURL` 注释。
  Explicit,
  /// 来自 `产物文件名 + .map` 的同目录回退。
  AdjacentFallback,
}
