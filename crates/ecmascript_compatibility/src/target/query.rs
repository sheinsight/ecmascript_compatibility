use crate::error::TargetQueryError;

pub struct TargetQuery {
  queries: Vec<String>,
}

impl TargetQuery {
  pub fn new<I, S>(queries: I) -> Result<Self, TargetQueryError>
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let queries = queries
      .into_iter()
      .map(Into::into)
      .filter(|query| !query.trim().is_empty())
      .collect::<Vec<_>>();

    if queries.is_empty() {
      return Err(TargetQueryError::Empty);
    }

    Ok(Self { queries })
  }

  pub fn queries(&self) -> &[String] {
    &self.queries
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn creates_query_from_non_empty_strs() {
    let query = TargetQuery::new(["chrome 79", "firefox 72"]).unwrap();

    assert_eq!(query.queries(), ["chrome 79", "firefox 72"]);
  }

  #[test]
  fn accepts_owned_strings() {
    let query = TargetQuery::new(vec![String::from("chrome 79")]).unwrap();

    assert_eq!(query.queries(), ["chrome 79"]);
  }

  #[test]
  fn filters_empty_and_whitespace_only_queries() {
    let query =
      TargetQuery::new(["", "  \t\n", " chrome 79 ", "firefox 72"]).unwrap();

    assert_eq!(query.queries(), [" chrome 79 ", "firefox 72"]);
  }

  #[test]
  fn rejects_an_empty_input() {
    let result = TargetQuery::new(std::iter::empty::<&str>());

    assert!(matches!(result, Err(TargetQueryError::Empty)));
  }

  #[test]
  fn rejects_input_containing_only_empty_queries() {
    let result = TargetQuery::new(["", " ", "\t", "\n"]);

    assert!(matches!(result, Err(TargetQueryError::Empty)));
  }
}
