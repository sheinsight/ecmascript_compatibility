#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatStatus {
  Supported,
  Unsupported,
  Mixed,
  Unknown,
}
