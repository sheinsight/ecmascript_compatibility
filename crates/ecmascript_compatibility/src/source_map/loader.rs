use std::path::PathBuf;

use super::source_map_reference::SourceMapReference;

/// Source Map 文档加载边界。
///
/// resolver 负责决定“加载哪个引用”，loader 只负责根据引用取回字节内容。
/// 这样后续可以分别接入文件、data URI 或 HTTP loader，而不污染解析编排逻辑。
pub trait SourceMapLoader {
  fn load(
    &self,
    reference: &SourceMapReference,
  ) -> Result<Vec<u8>, SourceMapLoadError>;
}

/// Source Map 引用加载失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceMapLoadError {
  /// 本地 Source Map 文件不存在。
  NotFound(PathBuf),
  /// 默认策略禁止远程 Source Map 加载。
  UnsupportedRemoteUrl(String),
  /// `data:` 引用格式不合法或无法解码。
  InvalidDataUri(String),
  /// 底层 I/O 失败。
  Io { path: PathBuf, message: String },
}

#[cfg(test)]
mod tests {
  use super::*;

  struct FailingLoader;

  impl SourceMapLoader for FailingLoader {
    fn load(
      &self,
      reference: &SourceMapReference,
    ) -> Result<Vec<u8>, SourceMapLoadError> {
      match reference {
        SourceMapReference::RemoteUrl(url) => {
          Err(SourceMapLoadError::UnsupportedRemoteUrl(url.clone()))
        }
        SourceMapReference::InlineData(_)
        | SourceMapReference::LocalFile(_) => Ok(Vec::new()),
      }
    }
  }

  #[test]
  fn loader_boundary_can_reject_remote_references() {
    let reference = SourceMapReference::remote_url("https://example.com/a.map")
      .expect("remote url should be accepted");

    assert_eq!(
      FailingLoader.load(&reference),
      Err(SourceMapLoadError::UnsupportedRemoteUrl(
        "https://example.com/a.map".to_string(),
      )),
    );
  }
}
