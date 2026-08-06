use std::path::{Path, PathBuf};

use crate::feature::FeatureUsage;

/// 单个输入文件的特性检测结果。
///
/// 在 Source Map 链路中，这里的 `path` 表示 detector 实际解析的文件，通常是
/// 构建后的产物文件，例如 `dist/main.js`。它不是 Source Map 文档路径，也不是
/// Source Map 映射出来的 original source 路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionResult {
  path: PathBuf,
  usages: Vec<FeatureUsage>,
}

impl DetectionResult {
  pub(crate) fn new(path: PathBuf, usages: Vec<FeatureUsage>) -> Self {
    Self { path, usages }
  }

  /// 返回本次检测对应的输入文件路径。
  ///
  /// 对构建产物做兼容性检测时，这个路径就是 generated file 的路径。
  pub fn path(&self) -> &Path {
    &self.path
  }

  /// 返回源文件中检测到的全部特性使用位置。
  pub fn usages(&self) -> &[FeatureUsage] {
    &self.usages
  }
}
