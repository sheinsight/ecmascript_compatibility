use crate::{
  database::SupportRule,
  target::{RuntimeRelease, Version},
};

use super::CompatStatus;

#[must_use]
pub fn evaluate(rule: SupportRule, release: RuntimeRelease) -> CompatStatus {
  match rule {
    SupportRule::Always => CompatStatus::Supported,
    SupportRule::Never => CompatStatus::Unsupported,
    SupportRule::Unknown => CompatStatus::Unknown,
    SupportRule::Since(required) => evaluate_since(required, release),
    SupportRule::AtOrBefore(known_supported) => {
      evaluate_at_or_before(known_supported, release)
    }
  }
}

fn evaluate_since(required: Version, release: RuntimeRelease) -> CompatStatus {
  match release {
    RuntimeRelease::Exact(actual) => {
      if actual >= required {
        CompatStatus::Supported
      } else {
        CompatStatus::Unsupported
      }
    }
    RuntimeRelease::Range(range) => {
      if range.start() >= required {
        CompatStatus::Supported
      } else if range.end() < required {
        CompatStatus::Unsupported
      } else {
        CompatStatus::Mixed
      }
    }
    RuntimeRelease::Preview | RuntimeRelease::All => CompatStatus::Unknown,
  }
}

fn evaluate_at_or_before(
  known_supported: Version,
  release: RuntimeRelease,
) -> CompatStatus {
  match release {
    RuntimeRelease::Exact(actual) => {
      if actual >= known_supported {
        CompatStatus::Supported
      } else {
        CompatStatus::Unknown
      }
    }
    RuntimeRelease::Range(range) => {
      if range.start() >= known_supported {
        CompatStatus::Supported
      } else if range.end() < known_supported {
        CompatStatus::Unknown
      } else {
        CompatStatus::Mixed
      }
    }
    RuntimeRelease::Preview | RuntimeRelease::All => CompatStatus::Unknown,
  }
}

#[cfg(test)]
mod tests {
  use crate::target::VersionRange;

  use super::*;

  fn version(major: u32) -> Version {
    Version::from_major(major)
  }

  fn range(start: u32, end: u32) -> RuntimeRelease {
    RuntimeRelease::Range(
      VersionRange::new(version(start), version(end)).unwrap(),
    )
  }

  #[test]
  fn always_is_supported_for_every_release_shape() {
    let releases = [
      RuntimeRelease::Exact(version(80)),
      range(79, 81),
      RuntimeRelease::Preview,
      RuntimeRelease::All,
    ];

    for release in releases {
      assert_eq!(
        evaluate(SupportRule::Always, release),
        CompatStatus::Supported,
      );
    }
  }

  #[test]
  fn never_is_unsupported_for_every_release_shape() {
    let releases = [
      RuntimeRelease::Exact(version(80)),
      range(79, 81),
      RuntimeRelease::Preview,
      RuntimeRelease::All,
    ];

    for release in releases {
      assert_eq!(
        evaluate(SupportRule::Never, release),
        CompatStatus::Unsupported,
      );
    }
  }

  #[test]
  fn unknown_remains_unknown_for_every_release_shape() {
    let releases = [
      RuntimeRelease::Exact(version(80)),
      range(79, 81),
      RuntimeRelease::Preview,
      RuntimeRelease::All,
    ];

    for release in releases {
      assert_eq!(
        evaluate(SupportRule::Unknown, release),
        CompatStatus::Unknown,
      );
    }
  }

  #[test]
  fn evaluates_exact_releases_against_the_required_version() {
    let rule = SupportRule::Since(version(80));

    assert_eq!(
      evaluate(rule, RuntimeRelease::Exact(version(79))),
      CompatStatus::Unsupported,
    );
    assert_eq!(
      evaluate(rule, RuntimeRelease::Exact(version(80))),
      CompatStatus::Supported,
    );
    assert_eq!(
      evaluate(rule, RuntimeRelease::Exact(version(81))),
      CompatStatus::Supported,
    );
  }

  #[test]
  fn evaluates_ranges_against_the_required_version() {
    let rule = SupportRule::Since(version(80));

    assert_eq!(evaluate(rule, range(78, 79)), CompatStatus::Unsupported,);
    assert_eq!(evaluate(rule, range(80, 81)), CompatStatus::Supported,);
    assert_eq!(evaluate(rule, range(79, 80)), CompatStatus::Mixed,);
    assert_eq!(evaluate(rule, range(79, 81)), CompatStatus::Mixed,);
  }

  #[test]
  fn cannot_infer_a_numeric_boundary_for_special_releases() {
    let rule = SupportRule::Since(version(80));

    assert_eq!(
      evaluate(rule, RuntimeRelease::Preview),
      CompatStatus::Unknown,
    );
    assert_eq!(evaluate(rule, RuntimeRelease::All), CompatStatus::Unknown,);
  }

  #[test]
  fn evaluates_at_or_before_without_inventing_an_older_boundary() {
    let rule = SupportRule::AtOrBefore(version(37));

    assert_eq!(
      evaluate(rule, RuntimeRelease::Exact(version(36))),
      CompatStatus::Unknown,
    );
    assert_eq!(
      evaluate(rule, RuntimeRelease::Exact(version(37))),
      CompatStatus::Supported,
    );
    assert_eq!(
      evaluate(rule, RuntimeRelease::Exact(version(38))),
      CompatStatus::Supported,
    );
    assert_eq!(evaluate(rule, range(36, 37)), CompatStatus::Mixed);
  }
}
