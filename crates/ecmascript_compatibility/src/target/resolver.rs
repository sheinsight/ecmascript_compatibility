use browserslist::{Opts, resolve};

use crate::target::query::TargetQuery;

pub struct TargetResolver;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeTarget {
  pub browser: String,
  pub version: u32,
}

impl TargetResolver {
  pub fn resolve(
    &self,
    query: &TargetQuery,
  ) -> Result<Vec<RuntimeTarget>, String> {
    // 1. 让 browserslist-rs 解析表达式
    let distributions =
      resolve(query.queries(), &Opts::default()).map_err(|error| {
        format!(
          "failed to resolve Browserslist query {:?}: {error}",
          query.queries(),
        )
      })?;

    // 2. 合法查询也可能最终没有选择任何目标
    if distributions.is_empty() {
      return Err("Browserslist query resolved to no targets".to_owned());
    }

    // 3. 把第三方 Distrib 转换成自己的 RuntimeTarget
    let mut targets = Vec::with_capacity(distributions.len());

    for distribution in distributions {
      let browser = distribution.name().to_owned();
      let raw_version = distribution.version();

      // 最小版本目前只处理 Chrome 79 这种整数版本
      let version = raw_version.parse::<u32>().map_err(|_| {
        format!(
          "unsupported version `{raw_version}` \
             returned for browser `{browser}`"
        )
      })?;

      targets.push(RuntimeTarget { browser, version });
    }

    // 4. 保证结果顺序稳定
    targets.sort_by(|left, right| {
      left
        .browser
        .cmp(&right.browser)
        .then(left.version.cmp(&right.version))
    });

    // 5. 去掉重复目标
    targets.dedup();

    Ok(targets)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn target(browser: &str, version: u32) -> RuntimeTarget {
    RuntimeTarget {
      browser: browser.to_owned(),
      version,
    }
  }

  #[test]
  fn resolves_an_exact_browser_version() {
    let query = TargetQuery::new(["chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [target("chrome", 79)]);
  }

  #[test]
  fn sorts_targets_by_browser_then_version() {
    let query =
      TargetQuery::new(["firefox 72", "chrome 80", "chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [
        target("chrome", 79),
        target("chrome", 80),
        target("firefox", 72),
      ],
    );
  }

  #[test]
  fn removes_duplicate_targets() {
    let query = TargetQuery::new(["chrome 79", "chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [target("chrome", 79)]);
  }

  #[test]
  fn resolves_a_bounded_version_range() {
    let query = TargetQuery::new(["chrome 79-81"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(
      targets,
      [
        target("chrome", 79),
        target("chrome", 80),
        target("chrome", 81),
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
        target("chrome", 79),
        target("chrome", 80),
        target("chrome", 81),
      ],
    );
  }

  #[test]
  fn resolves_an_expression_with_an_exclusion() {
    let query = TargetQuery::new(["chrome 79-81, not chrome 80"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [target("chrome", 79), target("chrome", 81)],);
  }

  #[test]
  fn resolves_an_or_expression() {
    let query = TargetQuery::new(["firefox 72 or chrome 79"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets, [target("chrome", 79), target("firefox", 72)],);
  }

  #[test]
  fn resolves_a_relative_last_versions_expression() {
    let query = TargetQuery::new(["last 2 chrome versions"]).unwrap();

    let targets = TargetResolver.resolve(&query).unwrap();

    assert_eq!(targets.len(), 2);
    assert!(targets.iter().all(|target| target.browser == "chrome"));
    assert!(targets[0].version < targets[1].version);
  }

  #[test]
  fn rejects_a_query_that_resolves_to_no_targets() {
    let query = TargetQuery::new(["chrome 79 and chrome 80"]).unwrap();

    let error = TargetResolver.resolve(&query).unwrap_err();

    assert_eq!(error, "Browserslist query resolved to no targets");
  }

  #[test]
  fn reports_the_query_when_browserslist_rejects_it() {
    let query = TargetQuery::new(["invalid-browser 1"]).unwrap();

    let error = TargetResolver.resolve(&query).unwrap_err();

    assert!(error.starts_with(
      "failed to resolve Browserslist query [\"invalid-browser 1\"]:"
    ));
  }

  #[test]
  fn rejects_non_integer_versions() {
    let query = TargetQuery::new(["ios_saf 13.2"]).unwrap();

    let error = TargetResolver.resolve(&query).unwrap_err();

    assert_eq!(
      error,
      "unsupported version `13.2` returned for browser `ios_saf`"
    );
  }
}
