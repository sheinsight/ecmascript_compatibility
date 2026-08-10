use std::{fmt, str::FromStr};

use crate::error::VersionParseError;

const MAX_COMPONENTS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
  components: [u32; MAX_COMPONENTS],
}

impl Version {
  pub const fn new(major: u32, minor: u32, patch: u32, revision: u32) -> Self {
    Self {
      components: [major, minor, patch, revision],
    }
  }

  pub const fn from_major(major: u32) -> Self {
    Self::new(major, 0, 0, 0)
  }

  pub const fn major(self) -> u32 {
    self.components[0]
  }

  pub const fn minor(self) -> u32 {
    self.components[1]
  }

  pub const fn patch(self) -> u32 {
    self.components[2]
  }

  pub const fn revision(self) -> u32 {
    self.components[3]
  }
}

impl FromStr for Version {
  type Err = VersionParseError;

  fn from_str(input: &str) -> Result<Self, Self::Err> {
    if input.is_empty() {
      return Err(VersionParseError::Empty);
    }

    let raw_components = input.split('.').collect::<Vec<_>>();

    if raw_components.len() > MAX_COMPONENTS {
      return Err(VersionParseError::TooManyComponents {
        value: input.to_owned(),
        maximum: MAX_COMPONENTS,
      });
    }

    let mut components = [0; MAX_COMPONENTS];

    for (index, component) in raw_components.into_iter().enumerate() {
      components[index] = component.parse::<u32>().map_err(|_| {
        VersionParseError::InvalidComponent {
          value: input.to_owned(),
          component: component.to_owned(),
        }
      })?;
    }

    Ok(Self { components })
  }
}

impl fmt::Display for Version {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let last_component = self
      .components
      .iter()
      .rposition(|component| *component != 0)
      .unwrap_or(0);

    for (index, component) in
      self.components[..=last_component].iter().enumerate()
    {
      if index != 0 {
        formatter.write_str(".")?;
      }

      write!(formatter, "{component}")?;
    }

    Ok(())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VersionRange {
  start: Version,
  end: Version,
}

impl VersionRange {
  pub fn new(
    start: Version,
    end: Version,
  ) -> Result<Self, InvalidVersionRange> {
    if start > end {
      return Err(InvalidVersionRange { start, end });
    }

    Ok(Self { start, end })
  }

  pub const fn start(self) -> Version {
    self.start
  }

  pub const fn end(self) -> Version {
    self.end
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("version range start `{start}` is greater than end `{end}`")]
pub struct InvalidVersionRange {
  pub start: Version,
  pub end: Version,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_one_to_four_numeric_components() {
    assert_eq!("79".parse(), Ok(Version::new(79, 0, 0, 0)));
    assert_eq!("13.1".parse(), Ok(Version::new(13, 1, 0, 0)));
    assert_eq!("15.2.1".parse(), Ok(Version::new(15, 2, 1, 0)));
    assert_eq!("131.0.6778.1".parse(), Ok(Version::new(131, 0, 6778, 1)),);
  }

  #[test]
  fn exposes_normalized_version_components() {
    let version = Version::new(131, 0, 6778, 1);

    assert_eq!(Version::from_major(79), Version::new(79, 0, 0, 0));
    assert_eq!(version.major(), 131);
    assert_eq!(version.minor(), 0);
    assert_eq!(version.patch(), 6778);
    assert_eq!(version.revision(), 1);
  }

  #[test]
  fn normalizes_version_comparison() {
    let short = "13.1".parse::<Version>().unwrap();
    let complete = "13.1.0.0".parse::<Version>().unwrap();

    assert_eq!(short, complete);
    assert!("13.2".parse::<Version>().unwrap() > short);
  }

  #[test]
  fn formats_a_normalized_version() {
    assert_eq!(Version::new(79, 0, 0, 0).to_string(), "79");
    assert_eq!(Version::new(13, 1, 0, 0).to_string(), "13.1");
    assert_eq!(Version::new(131, 0, 6778, 1).to_string(), "131.0.6778.1");
  }

  #[test]
  fn rejects_invalid_versions() {
    assert_eq!("".parse::<Version>(), Err(VersionParseError::Empty));
    assert!(matches!(
      "1.2.3.4.5".parse::<Version>(),
      Err(VersionParseError::TooManyComponents { .. }),
    ));
    assert!(matches!(
      "13.x".parse::<Version>(),
      Err(VersionParseError::InvalidComponent { .. }),
    ));
    assert!(matches!(
      "13..1".parse::<Version>(),
      Err(VersionParseError::InvalidComponent { .. }),
    ));
  }

  #[test]
  fn creates_an_inclusive_version_range() {
    let start = "15.2".parse().unwrap();
    let end = "15.3".parse().unwrap();
    let range = VersionRange::new(start, end).unwrap();

    assert_eq!(range.start(), start);
    assert_eq!(range.end(), end);
  }

  #[test]
  fn rejects_a_reversed_version_range() {
    let start = "15.3".parse().unwrap();
    let end = "15.2".parse().unwrap();

    assert_eq!(
      VersionRange::new(start, end),
      Err(InvalidVersionRange { start, end }),
    );
  }
}
