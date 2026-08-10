use std::{io, path::PathBuf};

use crate::{
  SourceKindError, SyntaxFeatureDetectionError,
  error::{TargetQueryError, TargetResolveError},
  source_map::SourceMapResolveError,
};

/// 对外分析入口的失败原因。
///
/// 这里把“读取输入、识别文件类型、解析 target、检测语法、解析 Source Map”放在同一个
/// 边界上，是为了让调用方只处理一次分析失败；每个变体仍保留原始错误信息。
#[derive(Debug, thiserror::Error)]
pub enum CompatAnalysisError {
  #[error("failed to read source file `{path}`: {source}")]
  ReadSource {
    path: PathBuf,
    #[source]
    source: io::Error,
  },

  #[error(transparent)]
  SourceKind(#[from] SourceKindError),

  #[error(transparent)]
  TargetQuery(#[from] TargetQueryError),

  #[error(transparent)]
  TargetResolve(#[from] TargetResolveError),

  #[error(transparent)]
  Detection(#[from] SyntaxFeatureDetectionError),

  #[error(transparent)]
  SourceMap(#[from] SourceMapResolveError),
}
