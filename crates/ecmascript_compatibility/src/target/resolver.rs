use std::cmp::Ordering;

use browserslist::{Opts, resolve};

use crate::error::TargetResolveError;

use super::{Runtime, RuntimeRelease, RuntimeTarget, query::TargetQuery};

pub struct TargetResolver;

impl TargetResolver {
  pub fn resolve(
    &self,
    query: &TargetQuery,
  ) -> Result<Vec<RuntimeTarget>, TargetResolveError> {
    let distributions =
      resolve(query.queries(), &Opts::default()).map_err(|error| {
        TargetResolveError::Browserslist {
          queries: query.queries().to_vec(),
          message: error.to_string(),
        }
      })?;

    if distributions.is_empty() {
      return Err(TargetResolveError::Empty);
    }

    let mut targets = Vec::with_capacity(distributions.len());

    for distribution in distributions {
      let runtime = parse_runtime(distribution.name())?;
      let release = distribution.version().parse().map_err(|source| {
        TargetResolveError::InvalidRelease {
          runtime: distribution.name().to_owned(),
          release: distribution.version().to_owned(),
          source,
        }
      })?;

      targets.push(RuntimeTarget::new(runtime, release));
    }

    targets.sort_by(compare_targets);
    targets.dedup();

    Ok(targets)
  }
}

fn parse_runtime(name: &str) -> Result<Runtime, TargetResolveError> {
  match name {
    "ie" => Ok(Runtime::InternetExplorer),
    "edge" => Ok(Runtime::Edge),
    "firefox" => Ok(Runtime::Firefox),
    "chrome" => Ok(Runtime::Chrome),
    "safari" => Ok(Runtime::Safari),
    "opera" => Ok(Runtime::Opera),
    "ios_saf" => Ok(Runtime::Ios),
    "op_mini" => Ok(Runtime::OperaMini),
    "android" => Ok(Runtime::Android),
    "bb" => Ok(Runtime::Blackberry),
    "op_mob" => Ok(Runtime::OperaMobile),
    "and_chr" => Ok(Runtime::ChromeAndroid),
    "and_ff" => Ok(Runtime::FirefoxAndroid),
    "ie_mob" => Ok(Runtime::InternetExplorerMobile),
    "and_uc" => Ok(Runtime::UcAndroid),
    "samsung" => Ok(Runtime::SamsungInternet),
    "and_qq" => Ok(Runtime::QqAndroid),
    "baidu" => Ok(Runtime::Baidu),
    "kaios" => Ok(Runtime::KaiOS),
    "node" => Ok(Runtime::Node),
    _ => Err(TargetResolveError::UnsupportedRuntime {
      name: name.to_owned(),
    }),
  }
}

fn compare_targets(left: &RuntimeTarget, right: &RuntimeTarget) -> Ordering {
  left
    .runtime()
    .browserslist_name()
    .cmp(right.runtime().browserslist_name())
    .then_with(|| compare_releases(left.release(), right.release()))
}

fn compare_releases(left: RuntimeRelease, right: RuntimeRelease) -> Ordering {
  match (left.numeric_bounds(), right.numeric_bounds()) {
    (Some((left_start, left_end)), Some((right_start, right_end))) => {
      left_start.cmp(&right_start).then(left_end.cmp(&right_end))
    }
    (Some(_), None) => Ordering::Less,
    (None, Some(_)) => Ordering::Greater,
    (None, None) => {
      special_release_rank(left).cmp(&special_release_rank(right))
    }
  }
}

const fn special_release_rank(release: RuntimeRelease) -> u8 {
  match release {
    RuntimeRelease::Preview => 0,
    RuntimeRelease::All => 1,
    RuntimeRelease::Exact(_) | RuntimeRelease::Range(_) => 0,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    error::TargetResolveError,
    target::{Version, VersionRange},
  };

  fn exact(runtime: Runtime, version: &str) -> RuntimeTarget {
    RuntimeTarget::new(runtime, RuntimeRelease::Exact(version.parse().unwrap()))
  }

  #[test]
  fn resolves_an_exact_browser_version() {
    let query = TargetQuery::new(["chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [exact(Runtime::Chrome, "79")]);
  }

  #[test]
  fn resolves_a_version_with_minor_components() {
    let query = TargetQuery::new(["ios_saf 13.2"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [exact(Runtime::Ios, "13.2")]);
  }

  #[test]
  fn resolves_a_combined_browser_release_range() {
    let query = TargetQuery::new(["ios_saf 15.2-15.3"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [RuntimeTarget::new(
        Runtime::Ios,
        RuntimeRelease::Range(
          VersionRange::new(
            "15.2".parse::<Version>().unwrap(),
            "15.3".parse::<Version>().unwrap(),
          )
          .unwrap(),
        ),
      )],
    );
  }

  #[test]
  fn resolves_preview_and_unversioned_releases() {
    let safari = TargetQuery::new(["safari TP"]).unwrap();
    let opera_mini = TargetQuery::new(["op_mini all"]).unwrap();

    assert_eq!(
      TargetResolver.resolve(&safari).unwrap(),
      [RuntimeTarget::new(Runtime::Safari, RuntimeRelease::Preview,)],
    );
    assert_eq!(
      TargetResolver.resolve(&opera_mini).unwrap(),
      [RuntimeTarget::new(Runtime::OperaMini, RuntimeRelease::All,)],
    );
  }

  #[test]
  fn resolves_a_node_runtime() {
    let query = TargetQuery::new(["node 14.0.0"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [exact(Runtime::Node, "14.0.0")]);
  }

  #[test]
  fn sorts_targets_by_runtime_then_numeric_release() {
    let query =
      TargetQuery::new(["firefox 72", "chrome 80", "chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [
        exact(Runtime::Chrome, "79"),
        exact(Runtime::Chrome, "80"),
        exact(Runtime::Firefox, "72"),
      ],
    );
  }

  #[test]
  fn removes_duplicate_targets() {
    let query = TargetQuery::new(["chrome 79", "chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [exact(Runtime::Chrome, "79")]);
  }

  #[test]
  fn resolves_a_bounded_version_range_query() {
    let query = TargetQuery::new(["chrome 79-81"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [
        exact(Runtime::Chrome, "79"),
        exact(Runtime::Chrome, "80"),
        exact(Runtime::Chrome, "81"),
      ],
    );
  }

  #[test]
  fn resolves_an_intersection_with_comparators() {
    let query = TargetQuery::new(["chrome >= 79 and chrome <= 81"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [
        exact(Runtime::Chrome, "79"),
        exact(Runtime::Chrome, "80"),
        exact(Runtime::Chrome, "81"),
      ],
    );
  }

  #[test]
  fn resolves_an_expression_with_an_exclusion() {
    let query = TargetQuery::new(["chrome 79-81, not chrome 80"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [exact(Runtime::Chrome, "79"), exact(Runtime::Chrome, "81")],
    );
  }

  #[test]
  fn resolves_an_or_expression() {
    let query = TargetQuery::new(["firefox 72 or chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [exact(Runtime::Chrome, "79"), exact(Runtime::Firefox, "72")],
    );
  }

  #[test]
  fn resolves_a_relative_last_versions_expression() {
    let query = TargetQuery::new(["last 2 chrome versions"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets.len(), 2);
    assert!(
      targets
        .iter()
        .all(|target| target.runtime() == Runtime::Chrome),
    );

    let [first, second] = targets.as_slice() else {
      panic!("expected exactly two Chrome targets");
    };

    assert_eq!(
      compare_releases(first.release(), second.release()),
      Ordering::Less,
    );
  }

  #[test]
  fn rejects_a_query_that_resolves_to_no_targets() {
    let query = TargetQuery::new(["chrome 79 and chrome 80"]).unwrap();

    let error = TargetResolver.resolve(&query).unwrap_err();

    assert_eq!(error, TargetResolveError::Empty);
  }

  #[test]
  fn reports_the_query_when_browserslist_rejects_it() {
    let query = TargetQuery::new(["invalid-browser 1"]).unwrap();

    let error = TargetResolver.resolve(&query).unwrap_err();

    assert!(matches!(
      error,
      TargetResolveError::Browserslist { ref queries, .. }
        if queries == &["invalid-browser 1"],
    ));
  }

  #[test]
  fn rejects_an_unknown_runtime_name_at_the_adapter_boundary() {
    assert_eq!(
      parse_runtime("unknown"),
      Err(TargetResolveError::UnsupportedRuntime {
        name: "unknown".to_owned(),
      }),
    );
  }
}
