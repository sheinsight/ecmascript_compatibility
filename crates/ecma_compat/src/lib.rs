pub mod analyzer;
pub mod checker;
pub mod database;
mod detector;
mod error;
mod source;
pub mod source_map;
mod syntax_feature;
pub mod target;

pub use analyzer::{
  CompatAnalysisError, CompatAnalysisTiming, CompatAnalyzer,
  CompatAnalyzerBuilder, CompatDiagnostic, CompatReport, SourceMapPolicy,
  SourceMapSkipReason, SourceMapStatus, TargetCompatStatus,
};
pub use checker::{CompatStatus, evaluate};
pub use database::{SupportRule, SyntaxCompatDatabase};
pub use detector::{SyntaxDetectionResult, SyntaxFeatureDetector};
pub use error::{
  SourceKindError, SyntaxFeatureDetectionError, TargetQueryError,
  TargetResolveError,
};
pub use source::{SourceFile, SourceKind};
pub use syntax_feature::{SourceSpan, SyntaxFeatureId, SyntaxFeatureUsage};
pub use target::{
  Runtime, RuntimeRelease, RuntimeTarget, TargetQuery, TargetResolver,
};
