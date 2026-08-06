use std::path::{Path, PathBuf};

use crate::feature::FeatureUsage;

/// 单个源文件的特性检测结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionResult {
  path: PathBuf,
  usages: Vec<FeatureUsage>,
}

impl DetectionResult {
  pub(crate) fn new(path: PathBuf, usages: Vec<FeatureUsage>) -> Self {
    Self { path, usages }
  }

  /// 返回本次检测对应的源文件路径。
  pub fn path(&self) -> &Path {
    &self.path
  }

  /// 返回源文件中检测到的全部特性使用位置。
  pub fn usages(&self) -> &[FeatureUsage] {
    &self.usages
  }
}
