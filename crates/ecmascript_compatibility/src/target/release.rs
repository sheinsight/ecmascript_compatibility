use std::str::FromStr;

use crate::error::RuntimeReleaseParseError;

use super::{Version, VersionRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeRelease {
  Exact(Version),
  Range(VersionRange),
  Preview,
  All,
}

impl RuntimeRelease {
  pub(crate) const fn numeric_bounds(self) -> Option<(Version, Version)> {
    match self {
      Self::Exact(version) => Some((version, version)),
      Self::Range(range) => Some((range.start(), range.end())),
      Self::Preview | Self::All => None,
    }
  }
}

impl FromStr for RuntimeRelease {
  type Err = RuntimeReleaseParseError;

  fn from_str(input: &str) -> Result<Self, Self::Err> {
    if input.eq_ignore_ascii_case("tp") {
      return Ok(Self::Preview);
    }

    if input.eq_ignore_ascii_case("all") {
      return Ok(Self::All);
    }

    if let Some((start, end)) = input.split_once('-') {
      let start = parse_version(input, start)?;
      let end = parse_version(input, end)?;
      let range = VersionRange::new(start, end).map_err(|source| {
        RuntimeReleaseParseError::InvalidRange {
          value: input.to_owned(),
          source,
        }
      })?;

      return Ok(Self::Range(range));
    }

    parse_version(input, input).map(Self::Exact)
  }
}

fn parse_version(
  release: &str,
  version: &str,
) -> Result<Version, RuntimeReleaseParseError> {
  version
    .parse()
    .map_err(|source| RuntimeReleaseParseError::InvalidVersion {
      value: release.to_owned(),
      source,
    })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_exact_range_preview_and_all_releases() {
    assert_eq!(
      "13.1".parse(),
      Ok(RuntimeRelease::Exact(Version::new(13, 1, 0, 0))),
    );
    assert_eq!(
      "15.2-15.3".parse(),
      Ok(
        RuntimeRelease::Range(
          VersionRange::new(
            Version::new(15, 2, 0, 0),
            Version::new(15, 3, 0, 0),
          )
          .unwrap(),
        )
      ),
    );
    assert_eq!("TP".parse(), Ok(RuntimeRelease::Preview));
    assert_eq!("all".parse(), Ok(RuntimeRelease::All));
  }

  #[test]
  fn rejects_invalid_release_values() {
    assert!(matches!(
      "13.x".parse::<RuntimeRelease>(),
      Err(RuntimeReleaseParseError::InvalidVersion { .. }),
    ));
    assert!(matches!(
      "15.3-15.2".parse::<RuntimeRelease>(),
      Err(RuntimeReleaseParseError::InvalidRange { .. }),
    ));
  }
}
