use crate::target::Version;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportRule {
  Always,
  Since(Version),
  AtOrBefore(Version),
  Never,
  Unknown,
}
