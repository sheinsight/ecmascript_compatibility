use super::super::source_map_reference::SourceMapReference;
use super::{
  DataUriSourceMapLoader, FileSourceMapLoader, SourceMapLoadError,
  SourceMapLoader,
};

/// 默认 Source Map loader。
///
/// 第一版默认只允许本地文件和内联 data URI。远程 URL 会被明确拒绝，避免在
/// 兼容性检测流程里发生隐式网络访问。
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultSourceMapLoader {
  file: FileSourceMapLoader,
  data_uri: DataUriSourceMapLoader,
}

impl SourceMapLoader for DefaultSourceMapLoader {
  fn load(
    &self,
    reference: &SourceMapReference,
  ) -> Result<Vec<u8>, SourceMapLoadError> {
    match reference {
      SourceMapReference::LocalFile(_) => self.file.load(reference),
      SourceMapReference::InlineData(_) => self.data_uri.load(reference),
      SourceMapReference::RemoteUrl(url) => {
        Err(SourceMapLoadError::UnsupportedRemoteUrl(url.clone()))
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
  };

  #[test]
  fn default_loader_dispatches_local_files_and_data_uris() {
    let path = temp_source_map_path();
    fs::write(&path, br#"{"version":3}"#).unwrap();

    let local_result = DefaultSourceMapLoader::default()
      .load(&SourceMapReference::local_file(path.clone()));
    fs::remove_file(&path).unwrap();

    let inline =
      SourceMapReference::inline_data("data:application/json,%7B%7D")
        .expect("data URI should be accepted");
    let inline_result = DefaultSourceMapLoader::default().load(&inline);

    assert_eq!(local_result.unwrap(), br#"{"version":3}"#);
    assert_eq!(inline_result.unwrap(), br#"{}"#);
  }

  fn temp_source_map_path() -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();

    env::temp_dir().join(format!("ecma_compat_source_map_{nanos}.js.map",))
  }
}
