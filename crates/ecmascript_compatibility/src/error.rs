#[derive(Debug, thiserror::Error)]
pub enum TargetQueryError {
  #[error("at least one non-empty Browserslist query is required")]
  Empty,
}
