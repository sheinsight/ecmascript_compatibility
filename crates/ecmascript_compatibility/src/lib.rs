pub mod analyzer;
pub mod checker;
pub mod database;
mod detector;
mod error;
mod feature;
mod source;
pub mod source_map;
pub mod target;

pub use analyzer::{
  CompatAnalysisError, CompatAnalyzer, CompatDiagnostic, CompatReport,
  SourceMapStatus, TargetCompatStatus,
};
pub use checker::{CompatStatus, evaluate};
pub use database::{CompatDatabase, SupportRule};
pub use detector::{DetectionResult, FeatureDetector};
pub use error::{
  FeatureDetectionError, SourceKindError, TargetQueryError, TargetResolveError,
};
pub use feature::{FeatureId, FeatureUsage, SourceSpan};
pub use source::{SourceFile, SourceKind};
pub use target::{
  Runtime, RuntimeRelease, RuntimeTarget, TargetQuery, TargetResolver,
};
