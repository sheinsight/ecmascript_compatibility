use std::path::Path;

use oxc::{
  allocator::Allocator, parser::Parser, semantic::SemanticBuilder,
  span::SourceType,
};

mod checker;
mod database;
mod detector;
mod error;
mod feature;
mod source;
mod target;

pub use error::SourceKindError;
pub use source::{SourceFile, SourceKind};

#[derive(Debug, Clone)]
pub struct CompatibilityOptions {
  queries: Vec<String>,
  cwd: String,
}

pub fn compatibility(options: CompatibilityOptions) {
  // This function is a placeholder for compatibility features.
  // Implement compatibility logic here.

  let alloc = Allocator::default();

  let file = Path::new(&options.cwd).join("compatibility_output.txt");

  let source_type = match file.extension().and_then(|ext| ext.to_str()) {
    Some("ts") => SourceType::ts(),
    Some("tsx") => SourceType::tsx(),
    Some("jsx") => SourceType::jsx(),
    Some("cjs") => SourceType::cjs(),
    _ => SourceType::jsx(),
  };

  let source_code = "";

  let parser = Parser::new(&alloc, &source_code, source_type);

  let parse = parser.parse();

  let program = alloc.alloc(&parse.program);

  let semantic_return = SemanticBuilder::new()
    .with_check_syntax_error(false)
    // TODO 很多场景下是不需要开启的，只有 oxlint 下需要开启，这可能对性能会产生一定的影响
    .with_cfg(true)
    .build(program);

  let nodes = semantic_return.semantic.nodes();

  for node in nodes.iter() {}
}
