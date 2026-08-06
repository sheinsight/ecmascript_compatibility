mod data_uri;
mod default;
mod error;
mod file;

use super::source_map_reference::SourceMapReference;

pub use data_uri::DataUriSourceMapLoader;
pub use default::DefaultSourceMapLoader;
pub use error::SourceMapLoadError;
pub use file::FileSourceMapLoader;

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

impl SourceMapReference {
  /// 返回引用类型的人类可读名称，用于构造 loader 边界上的错误信息。
  pub(crate) fn kind(&self) -> &'static str {
    match self {
      Self::InlineData(_) => "inline data URI",
      Self::LocalFile(_) => "local file",
      Self::RemoteUrl(_) => "remote URL",
    }
  }
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

  #[test]
  fn specific_loaders_reject_unsupported_reference_kinds() {
    let data_uri =
      SourceMapReference::inline_data("data:application/json,%7B%7D")
        .expect("data URI should be accepted");
    let local_file = SourceMapReference::local_file("dist/main.js.map");

    assert_eq!(
      FileSourceMapLoader.load(&data_uri),
      Err(SourceMapLoadError::UnsupportedReferenceKind {
        expected: "local file",
        actual: "inline data URI",
      }),
    );
    assert_eq!(
      DataUriSourceMapLoader.load(&local_file),
      Err(SourceMapLoadError::UnsupportedReferenceKind {
        expected: "inline data URI",
        actual: "local file",
      }),
    );
  }
}
