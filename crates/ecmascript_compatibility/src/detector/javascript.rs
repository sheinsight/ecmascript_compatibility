use oxc::{
  allocator::Allocator,
  ast::ast::ChainExpression,
  ast_visit::{Visit, walk::walk_chain_expression},
  parser::Parser,
};

use crate::{
  error::FeatureDetectionError,
  feature::{FeatureId, FeatureUsage, SourceSpan},
  source::SourceFile,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct FeatureDetector;

impl FeatureDetector {
  pub const fn new() -> Self {
    Self
  }

  pub fn detect(
    &self,
    source: &SourceFile,
  ) -> Result<Vec<FeatureUsage>, FeatureDetectionError> {
    let allocator = Allocator::default();
    let parsed = Parser::new(
      &allocator,
      source.source_text(),
      source.kind().source_type(),
    )
    .parse();

    if parsed.diagnostics.has_errors() {
      return Err(FeatureDetectionError::Parse {
        path: source.path().to_path_buf(),
        diagnostics: parsed
          .diagnostics
          .errors()
          .map(|diagnostic| diagnostic.to_string())
          .collect(),
      });
    }

    let mut visitor = FeatureVisitor::default();
    visitor.visit_program(&parsed.program);

    Ok(visitor.usages)
  }
}

#[derive(Debug, Default)]
struct FeatureVisitor {
  usages: Vec<FeatureUsage>,
}

impl<'a> Visit<'a> for FeatureVisitor {
  fn visit_chain_expression(&mut self, expression: &ChainExpression<'a>) {
    self.usages.push(FeatureUsage::new(
      FeatureId::OptionalChaining,
      SourceSpan::new(expression.span.start, expression.span.end),
    ));

    walk_chain_expression(self, expression);
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use crate::source::SourceKind;

  use super::*;

  #[test]
  fn detects_each_optional_chaining_form() {
    let source = SourceFile::javascript(
      "input.js",
      "object?.property; object?.[key]; callback?.(); object?.method?.();",
    );

    let usages = FeatureDetector::new().detect(&source).unwrap();

    assert_eq!(usages.len(), 4);
    assert!(
      usages
        .iter()
        .all(|usage| usage.feature() == FeatureId::OptionalChaining),
    );
  }

  #[test]
  fn preserves_the_span_of_the_complete_chain_expression() {
    let source_text = "const name = user?.profile?.name;";
    let source = SourceFile::javascript("input.js", source_text);

    let usages = FeatureDetector::new().detect(&source).unwrap();

    assert_eq!(usages.len(), 1);
    let span = usages[0].span();
    assert_eq!(
      &source_text[span.start() as usize..span.end() as usize],
      "user?.profile?.name",
    );
  }

  #[test]
  fn ignores_equivalent_non_optional_expressions() {
    let source = SourceFile::javascript(
      "input.js",
      "object.property; object[key]; callback(); object.method();",
    );

    let usages = FeatureDetector::new().detect(&source).unwrap();

    assert!(usages.is_empty());
  }

  #[test]
  fn uses_the_source_kind_when_parsing_typescript_and_jsx() {
    let typescript = SourceFile::new(
      "input.ts",
      SourceKind::TypeScript,
      "const value: string | undefined = object?.property;",
    );
    let jsx = SourceFile::new(
      "input.jsx",
      SourceKind::Jsx,
      "const view = <span>{object?.property}</span>;",
    );

    assert_eq!(FeatureDetector::new().detect(&typescript).unwrap().len(), 1,);
    assert_eq!(FeatureDetector::new().detect(&jsx).unwrap().len(), 1);
  }

  #[test]
  fn reports_parser_diagnostics_with_the_source_path() {
    let source = SourceFile::javascript("src/input.js", "const value = ;");

    let error = FeatureDetector::new().detect(&source).unwrap_err();

    match error {
      FeatureDetectionError::Parse { path, diagnostics } => {
        assert_eq!(path, PathBuf::from("src/input.js"));
        assert!(!diagnostics.is_empty());
      }
    }
  }
}
