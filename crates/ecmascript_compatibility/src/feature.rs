pub enum FeatureId {
  OptionalChaining,
}

pub struct SourceSpan {
  pub start: u32,
  pub end: u32,
}

pub struct FeatureUsage {
  pub feature: FeatureId,
  pub span: SourceSpan,
}
