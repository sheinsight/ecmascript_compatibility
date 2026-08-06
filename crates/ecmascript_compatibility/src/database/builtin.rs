use crate::{
  feature::FeatureId,
  target::{Runtime, Version},
};

use super::SupportRule;

#[derive(Debug, Default, Clone, Copy)]
pub struct CompatibilityDatabase;

impl CompatibilityDatabase {
  pub const fn new() -> Self {
    Self
  }

  pub const fn support_rule(
    &self,
    feature: FeatureId,
    runtime: Runtime,
  ) -> SupportRule {
    match feature {
      FeatureId::OptionalChaining => optional_chaining(runtime),
    }
  }
}

// Source: MDN Browser Compatibility Data, optional chaining operator.
// Only explicit `version_added` entries are encoded here. Entries marked as
// `mirror` remain unknown until runtime-version inheritance is normalized.
// https://github.com/mdn/browser-compat-data/blob/main/javascript/operators/optional_chaining.json
const fn optional_chaining(runtime: Runtime) -> SupportRule {
  match runtime {
    Runtime::Chrome => SupportRule::Since(Version::from_major(80)),
    Runtime::Firefox => SupportRule::Since(Version::from_major(74)),
    Runtime::Safari => SupportRule::Since(Version::new(13, 1, 0, 0)),
    Runtime::Node => SupportRule::Since(Version::new(14, 0, 0, 0)),
    _ => SupportRule::Unknown,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn returns_explicit_optional_chaining_support_boundaries() {
    let database = CompatibilityDatabase::new();

    let cases = [
      (Runtime::Chrome, SupportRule::Since(Version::from_major(80))),
      (
        Runtime::Firefox,
        SupportRule::Since(Version::from_major(74)),
      ),
      (
        Runtime::Safari,
        SupportRule::Since(Version::new(13, 1, 0, 0)),
      ),
      (Runtime::Node, SupportRule::Since(Version::new(14, 0, 0, 0))),
    ];

    for (runtime, expected) in cases {
      assert_eq!(
        database.support_rule(FeatureId::OptionalChaining, runtime),
        expected,
      );
    }
  }

  #[test]
  fn returns_unknown_for_unresolved_mirror_data() {
    let database = CompatibilityDatabase::new();

    assert_eq!(
      database.support_rule(FeatureId::OptionalChaining, Runtime::Edge),
      SupportRule::Unknown,
    );
    assert_eq!(
      database.support_rule(FeatureId::OptionalChaining, Runtime::Ios),
      SupportRule::Unknown,
    );
  }
}
