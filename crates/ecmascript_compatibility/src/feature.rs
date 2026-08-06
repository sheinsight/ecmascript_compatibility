#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureId {
  OptionalChaining,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
  start: u32,
  end: u32,
}

impl SourceSpan {
  pub(crate) const fn new(start: u32, end: u32) -> Self {
    Self { start, end }
  }

  pub const fn start(self) -> u32 {
    self.start
  }

  pub const fn end(self) -> u32 {
    self.end
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureUsage {
  feature: FeatureId,
  span: SourceSpan,
}

impl FeatureUsage {
  pub(crate) const fn new(feature: FeatureId, span: SourceSpan) -> Self {
    Self { feature, span }
  }

  pub const fn feature(self) -> FeatureId {
    self.feature
  }

  pub const fn span(self) -> SourceSpan {
    self.span
  }
}
