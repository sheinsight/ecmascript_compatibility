use std::{fs, io, path::PathBuf};

use super::super::source_map_reference::SourceMapReference;
use super::{SourceMapLoadError, SourceMapLoader};

/// 本地文件 Source Map loader。
///
/// 只处理 `SourceMapReference::LocalFile`。引用发现、路径解析和显式/回退优先级
/// 属于 resolver，不放在这里。
#[derive(Debug, Default, Clone, Copy)]
pub struct FileSourceMapLoader;

impl SourceMapLoader for FileSourceMapLoader {
  fn load(
    &self,
    reference: &SourceMapReference,
  ) -> Result<Vec<u8>, SourceMapLoadError> {
    let SourceMapReference::LocalFile(path) = reference else {
      return Err(SourceMapLoadError::UnsupportedReferenceKind {
        expected: "local file",
        actual: reference.kind(),
      });
    };

    fs::read(path).map_err(|error| file_load_error(path.clone(), error))
  }
}

fn file_load_error(path: PathBuf, error: io::Error) -> SourceMapLoadError {
  if error.kind() == io::ErrorKind::NotFound {
    SourceMapLoadError::NotFound(path)
  } else {
    SourceMapLoadError::Io {
      path,
      message: error.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
  };

  #[test]
  fn file_loader_reads_local_source_map_bytes() {
    let path = temp_source_map_path();
    fs::write(&path, br#"{"version":3}"#).unwrap();

    let result =
      FileSourceMapLoader.load(&SourceMapReference::local_file(path.clone()));

    fs::remove_file(&path).unwrap();

    assert_eq!(result.unwrap(), br#"{"version":3}"#);
  }

  #[test]
  fn file_loader_reports_missing_local_files() {
    let path = temp_source_map_path();

    assert_eq!(
      FileSourceMapLoader.load(&SourceMapReference::local_file(path.clone())),
      Err(SourceMapLoadError::NotFound(path)),
    );
  }

  fn temp_source_map_path() -> PathBuf {
    let nanos = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();

    env::temp_dir().join(format!("ecma_compat_source_map_{nanos}.js.map",))
  }
}
