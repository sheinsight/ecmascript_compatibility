use oxc::{
  allocator::Allocator,
  ast::ast::{
    ArrayPattern, ArrowFunctionExpression, AssignmentExpression, BigIntLiteral,
    CatchClause, ChainExpression, Class, ExportAllDeclaration,
    ExportDeclaration, ExportDefaultDeclaration, ExportFromDeclaration,
    ExportNamedDeclaration, Expression, ForOfStatement, FormalParameter,
    FormalParameterRest, Function, ImportDeclaration, ImportExpression,
    ImportMeta, LogicalExpression, MethodDefinition, NumericLiteral,
    ObjectPattern, ObjectProperty, ObjectPropertyKind, PrivateInExpression,
    PropertyDefinition, PropertyKey, SpreadElement, StaticBlock,
    TemplateLiteral,
  },
  ast_visit::{
    Visit,
    walk::{
      walk_array_pattern, walk_arrow_function_expression,
      walk_assignment_expression, walk_big_int_literal, walk_catch_clause,
      walk_chain_expression, walk_class, walk_export_all_declaration,
      walk_export_declaration, walk_export_default_declaration,
      walk_export_from_declaration, walk_export_named_declaration,
      walk_for_of_statement, walk_formal_parameter, walk_formal_parameter_rest,
      walk_function, walk_import_declaration, walk_import_expression,
      walk_import_meta, walk_logical_expression, walk_method_definition,
      walk_numeric_literal, walk_object_pattern, walk_object_property,
      walk_object_property_kind, walk_private_in_expression,
      walk_property_definition, walk_spread_element, walk_static_block,
      walk_template_literal,
    },
  },
  parser::Parser,
  semantic::ScopeFlags,
  span::Span,
  syntax::operator::{AssignmentOperator, LogicalOperator},
};

use crate::{
  error::SyntaxFeatureDetectionError,
  source::SourceFile,
  syntax_feature::{SourceSpan, SyntaxFeatureId, SyntaxFeatureUsage},
};

use super::SyntaxDetectionResult;

#[derive(Debug, Default, Clone, Copy)]
pub struct SyntaxFeatureDetector;

impl SyntaxFeatureDetector {
  pub const fn new() -> Self {
    Self
  }

  pub fn detect(
    &self,
    source: &SourceFile,
  ) -> Result<SyntaxDetectionResult, SyntaxFeatureDetectionError> {
    let allocator = Allocator::default();
    let parsed = Parser::new(
      &allocator,
      source.source_text(),
      source.kind().source_type(),
    )
    .parse();

    if parsed.diagnostics.has_errors() {
      return Err(SyntaxFeatureDetectionError::Parse {
        path: source.path().to_path_buf(),
        diagnostics: parsed
          .diagnostics
          .errors()
          .map(|diagnostic| diagnostic.to_string())
          .collect(),
      });
    }

    let mut visitor = SyntaxFeatureVisitor::default();
    visitor.visit_program(&parsed.program);

    Ok(SyntaxDetectionResult::new(
      source.path().to_path_buf(),
      visitor.usages,
    ))
  }
}

#[derive(Debug, Default)]
struct SyntaxFeatureVisitor {
  usages: Vec<SyntaxFeatureUsage>,
}

impl SyntaxFeatureVisitor {
  fn push_usage(&mut self, feature: SyntaxFeatureId, span: Span) {
    self.usages.push(SyntaxFeatureUsage::new(
      feature,
      SourceSpan::new(span.start, span.end),
    ));
  }

  fn push_method_features(
    &mut self,
    span: Span,
    key: &PropertyKey<'_>,
    r#async: bool,
    generator: bool,
  ) {
    self.push_usage(SyntaxFeatureId::MethodDefinition, span);
    if matches!(key, PropertyKey::PrivateIdentifier(_)) {
      self.push_usage(SyntaxFeatureId::PrivateClassMethod, span);
    }
    match (r#async, generator) {
      (true, true) => {
        self.push_usage(SyntaxFeatureId::AsyncGeneratorMethod, span)
      }
      (true, false) => self.push_usage(SyntaxFeatureId::AsyncMethod, span),
      (false, _) => {}
    }
  }
}

impl<'a> Visit<'a> for SyntaxFeatureVisitor {
  fn visit_chain_expression(&mut self, expression: &ChainExpression<'a>) {
    self.push_usage(SyntaxFeatureId::OptionalChaining, expression.span);

    walk_chain_expression(self, expression);
  }

  fn visit_logical_expression(&mut self, expression: &LogicalExpression<'a>) {
    if matches!(expression.operator, LogicalOperator::Coalesce) {
      self.push_usage(SyntaxFeatureId::NullishCoalescing, expression.span);
    }

    walk_logical_expression(self, expression);
  }

  fn visit_assignment_expression(
    &mut self,
    expression: &AssignmentExpression<'a>,
  ) {
    let feature = match expression.operator {
      AssignmentOperator::LogicalAnd => {
        Some(SyntaxFeatureId::LogicalAndAssignment)
      }
      AssignmentOperator::LogicalOr => {
        Some(SyntaxFeatureId::LogicalOrAssignment)
      }
      AssignmentOperator::LogicalNullish => {
        Some(SyntaxFeatureId::NullishCoalescingAssignment)
      }
      _ => None,
    };

    if let Some(feature) = feature {
      self.push_usage(feature, expression.span);
    }

    walk_assignment_expression(self, expression);
  }

  fn visit_import_expression(&mut self, expression: &ImportExpression<'a>) {
    self.push_usage(SyntaxFeatureId::DynamicImport, expression.span);

    walk_import_expression(self, expression);
  }

  fn visit_import_meta(&mut self, import_meta: &ImportMeta) {
    self.push_usage(SyntaxFeatureId::ImportMeta, import_meta.span);

    walk_import_meta(self, import_meta);
  }

  fn visit_big_int_literal(&mut self, literal: &BigIntLiteral<'a>) {
    self.push_usage(SyntaxFeatureId::BigIntLiteral, literal.span);

    walk_big_int_literal(self, literal);
  }

  fn visit_numeric_literal(&mut self, literal: &NumericLiteral<'a>) {
    if literal
      .raw
      .as_ref()
      .is_some_and(|raw| raw.as_str().contains('_'))
    {
      self.push_usage(SyntaxFeatureId::NumericSeparator, literal.span);
    }

    walk_numeric_literal(self, literal);
  }

  fn visit_template_literal(&mut self, literal: &TemplateLiteral<'a>) {
    self.push_usage(SyntaxFeatureId::TemplateLiteral, literal.span);

    walk_template_literal(self, literal);
  }

  fn visit_spread_element(&mut self, spread: &SpreadElement<'a>) {
    self.push_usage(SyntaxFeatureId::Spread, spread.span);

    walk_spread_element(self, spread);
  }

  fn visit_arrow_function_expression(
    &mut self,
    expression: &ArrowFunctionExpression<'a>,
  ) {
    self.push_usage(SyntaxFeatureId::ArrowFunction, expression.span);
    if expression.r#async {
      self.push_usage(SyntaxFeatureId::AsyncFunction, expression.span);
    }

    walk_arrow_function_expression(self, expression);
  }

  fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
    match (function.r#async, function.generator) {
      (true, true) => {
        self.push_usage(SyntaxFeatureId::AsyncGeneratorFunction, function.span);
      }
      (true, false) => {
        self.push_usage(SyntaxFeatureId::AsyncFunction, function.span)
      }
      (false, true) => {
        self.push_usage(SyntaxFeatureId::GeneratorFunction, function.span);
      }
      (false, false) => {}
    }

    walk_function(self, function, flags);
  }

  fn visit_formal_parameter(&mut self, parameter: &FormalParameter<'a>) {
    if parameter.initializer.is_some() {
      self.push_usage(SyntaxFeatureId::DefaultParameter, parameter.span);
    }

    walk_formal_parameter(self, parameter);
  }

  fn visit_formal_parameter_rest(&mut self, rest: &FormalParameterRest<'a>) {
    self.push_usage(SyntaxFeatureId::RestParameter, rest.span);

    walk_formal_parameter_rest(self, rest);
  }

  fn visit_class(&mut self, class: &Class<'a>) {
    self.push_usage(SyntaxFeatureId::Class, class.span);

    walk_class(self, class);
  }

  fn visit_property_definition(&mut self, property: &PropertyDefinition<'a>) {
    let feature = if matches!(property.key, PropertyKey::PrivateIdentifier(_)) {
      SyntaxFeatureId::PrivateClassField
    } else {
      SyntaxFeatureId::PublicClassField
    };

    self.push_usage(feature, property.span);

    walk_property_definition(self, property);
  }

  fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
    self.push_method_features(
      method.span,
      &method.key,
      method.value.r#async,
      method.value.generator,
    );

    walk_method_definition(self, method);
  }

  fn visit_object_property(&mut self, property: &ObjectProperty<'a>) {
    if property.computed {
      self
        .push_usage(SyntaxFeatureId::ComputedObjectPropertyName, property.span);
    }
    if property.shorthand {
      self.push_usage(SyntaxFeatureId::ShorthandObjectProperty, property.span);
    }
    if property.method {
      let (r#async, generator) = match &property.value {
        Expression::FunctionExpression(function) => {
          (function.r#async, function.generator)
        }
        _ => (false, false),
      };

      self.push_method_features(
        property.span,
        &property.key,
        r#async,
        generator,
      );
      self.push_usage(SyntaxFeatureId::ShorthandObjectMethod, property.span);
    }

    walk_object_property(self, property);
  }

  fn visit_object_property_kind(&mut self, property: &ObjectPropertyKind<'a>) {
    if let ObjectPropertyKind::SpreadProperty(spread) = property {
      self.push_usage(SyntaxFeatureId::ObjectSpreadProperty, spread.span);
    }

    walk_object_property_kind(self, property);
  }

  fn visit_static_block(&mut self, static_block: &StaticBlock<'a>) {
    self.push_usage(
      SyntaxFeatureId::ClassStaticInitializationBlock,
      static_block.span,
    );

    walk_static_block(self, static_block);
  }

  fn visit_for_of_statement(&mut self, statement: &ForOfStatement<'a>) {
    let feature = if statement.r#await {
      SyntaxFeatureId::ForAwaitOf
    } else {
      SyntaxFeatureId::ForOf
    };

    self.push_usage(feature, statement.span);

    walk_for_of_statement(self, statement);
  }

  fn visit_object_pattern(&mut self, pattern: &ObjectPattern<'a>) {
    self.push_usage(SyntaxFeatureId::Destructuring, pattern.span);
    if pattern.rest.is_some() {
      self.push_usage(SyntaxFeatureId::ObjectRestDestructuring, pattern.span);
    }

    walk_object_pattern(self, pattern);
  }

  fn visit_array_pattern(&mut self, pattern: &ArrayPattern<'a>) {
    self.push_usage(SyntaxFeatureId::Destructuring, pattern.span);
    if pattern.rest.is_some() {
      self.push_usage(SyntaxFeatureId::ArrayRestDestructuring, pattern.span);
    }

    walk_array_pattern(self, pattern);
  }

  fn visit_catch_clause(&mut self, clause: &CatchClause<'a>) {
    if clause.param.is_none() {
      self.push_usage(SyntaxFeatureId::OptionalCatchBinding, clause.span);
    }

    walk_catch_clause(self, clause);
  }

  fn visit_await_expression(
    &mut self,
    expression: &oxc::ast::ast::AwaitExpression<'a>,
  ) {
    self.push_usage(SyntaxFeatureId::Await, expression.span);

    oxc::ast_visit::walk::walk_await_expression(self, expression);
  }

  fn visit_private_in_expression(
    &mut self,
    expression: &PrivateInExpression<'a>,
  ) {
    self.push_usage(SyntaxFeatureId::PrivateClassFieldIn, expression.span);

    walk_private_in_expression(self, expression);
  }

  fn visit_import_declaration(&mut self, declaration: &ImportDeclaration<'a>) {
    self.push_usage(SyntaxFeatureId::ImportStatement, declaration.span);
    if declaration.with_clause.is_some() {
      self.push_usage(SyntaxFeatureId::ImportAttribute, declaration.span);
    }

    walk_import_declaration(self, declaration);
  }

  fn visit_export_declaration(&mut self, declaration: &ExportDeclaration<'a>) {
    self.push_usage(SyntaxFeatureId::ExportStatement, declaration.span);

    walk_export_declaration(self, declaration);
  }

  fn visit_export_named_declaration(
    &mut self,
    declaration: &ExportNamedDeclaration<'a>,
  ) {
    self.push_usage(SyntaxFeatureId::ExportStatement, declaration.span);

    walk_export_named_declaration(self, declaration);
  }

  fn visit_export_from_declaration(
    &mut self,
    declaration: &ExportFromDeclaration<'a>,
  ) {
    self.push_usage(SyntaxFeatureId::ExportStatement, declaration.span);
    if declaration.with_clause.is_some() {
      self.push_usage(SyntaxFeatureId::ImportAttribute, declaration.span);
    }

    walk_export_from_declaration(self, declaration);
  }

  fn visit_export_default_declaration(
    &mut self,
    declaration: &ExportDefaultDeclaration<'a>,
  ) {
    self.push_usage(SyntaxFeatureId::ExportStatement, declaration.span);
    self.push_usage(SyntaxFeatureId::ExportDefaultStatement, declaration.span);

    walk_export_default_declaration(self, declaration);
  }

  fn visit_export_all_declaration(
    &mut self,
    declaration: &ExportAllDeclaration<'a>,
  ) {
    self.push_usage(SyntaxFeatureId::ExportStatement, declaration.span);
    if declaration.exported.is_some() {
      self.push_usage(
        SyntaxFeatureId::ExportNamespaceStatement,
        declaration.span,
      );
    }
    if declaration.with_clause.is_some() {
      self.push_usage(SyntaxFeatureId::ImportAttribute, declaration.span);
    }

    walk_export_all_declaration(self, declaration);
  }
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use crate::{source::SourceKind, source_map::SourceMapping};

  use super::*;

  #[test]
  fn detects_each_optional_chaining_form() {
    let source = SourceFile::javascript(
      "input.js",
      "object?.property; object?.[key]; callback?.(); object?.method?.();",
    );

    let result = SyntaxFeatureDetector::new().detect(&source).unwrap();

    assert_eq!(result.path(), source.path());
    assert_eq!(result.usages().len(), 4);
    assert!(
      result
        .usages()
        .iter()
        .all(|usage| usage.feature() == SyntaxFeatureId::OptionalChaining),
    );
  }

  #[test]
  fn preserves_the_span_of_the_complete_chain_expression() {
    let source_text = "const name = user?.profile?.name;";
    let source = SourceFile::javascript("input.js", source_text);

    let result = SyntaxFeatureDetector::new().detect(&source).unwrap();

    assert_eq!(result.usages().len(), 1);
    let span = result.usages()[0].span();
    assert_eq!(
      &source_text[span.start() as usize..span.end() as usize],
      "user?.profile?.name",
    );
  }

  #[test]
  fn marks_detected_usages_as_not_resolved_before_source_map_mapping() {
    let source =
      SourceFile::javascript("input.js", "const value = user?.name;");

    let result = SyntaxFeatureDetector::new().detect(&source).unwrap();

    assert_eq!(result.usages().len(), 1);
    assert_eq!(
      result.usages()[0].source_mapping(),
      &SourceMapping::NotResolved,
    );
  }

  #[test]
  fn ignores_equivalent_non_optional_expressions() {
    let source = SourceFile::javascript(
      "input.js",
      "object.property; object[key]; callback(); object.method();",
    );

    let result = SyntaxFeatureDetector::new().detect(&source).unwrap();

    assert!(result.usages().is_empty());
  }

  #[test]
  fn detects_common_syntax_level_features() {
    let source = SourceFile::javascript(
      "input.js",
      r#"
        const value = object?.property ?? 1n;
        const other = { ...source, [key]: value, value, run() {}, async load() {}, async *stream() {} };
        const [head, ...tail] = values;
        const { name, ...rest } = user;
        state &&= value;
        state ||= fallback;
        state ??= fallback;
        const message = `hello ${name}`;
        const count = 1_000;
        const loader = async () => import("./module.js");
        console.log(import.meta.url);
        async function waitFor(task) { await task; }
        function defaults(value = 1, ...items) {}
        async function load() {}
        function* ids() {}
        async function* stream() {}
        class Example {
          field = 1;
          #secret = 2;
          #run() {}
          has(value) { return #secret in value; }
          static {}
        }
        for (const item of items) {}
        async function iterate(stream) {
          for await (const item of stream) {}
        }
        try {} catch {}
      "#,
    );

    let result = SyntaxFeatureDetector::new().detect(&source).unwrap();
    let features = result
      .usages()
      .iter()
      .map(SyntaxFeatureUsage::feature)
      .collect::<std::collections::HashSet<_>>();

    for feature in [
      SyntaxFeatureId::OptionalChaining,
      SyntaxFeatureId::NullishCoalescing,
      SyntaxFeatureId::BigIntLiteral,
      SyntaxFeatureId::LogicalAndAssignment,
      SyntaxFeatureId::LogicalOrAssignment,
      SyntaxFeatureId::NullishCoalescingAssignment,
      SyntaxFeatureId::ArrowFunction,
      SyntaxFeatureId::AsyncFunction,
      SyntaxFeatureId::DynamicImport,
      SyntaxFeatureId::ImportMeta,
      SyntaxFeatureId::GeneratorFunction,
      SyntaxFeatureId::AsyncGeneratorFunction,
      SyntaxFeatureId::Class,
      SyntaxFeatureId::PublicClassField,
      SyntaxFeatureId::PrivateClassField,
      SyntaxFeatureId::ClassStaticInitializationBlock,
      SyntaxFeatureId::ForOf,
      SyntaxFeatureId::ForAwaitOf,
      SyntaxFeatureId::Spread,
      SyntaxFeatureId::ObjectSpreadProperty,
      SyntaxFeatureId::Destructuring,
      SyntaxFeatureId::ArrayRestDestructuring,
      SyntaxFeatureId::ObjectRestDestructuring,
      SyntaxFeatureId::DefaultParameter,
      SyntaxFeatureId::RestParameter,
      SyntaxFeatureId::TemplateLiteral,
      SyntaxFeatureId::NumericSeparator,
      SyntaxFeatureId::OptionalCatchBinding,
      SyntaxFeatureId::Await,
      SyntaxFeatureId::PrivateClassFieldIn,
      SyntaxFeatureId::PrivateClassMethod,
      SyntaxFeatureId::MethodDefinition,
      SyntaxFeatureId::AsyncMethod,
      SyntaxFeatureId::AsyncGeneratorMethod,
      SyntaxFeatureId::ComputedObjectPropertyName,
      SyntaxFeatureId::ShorthandObjectProperty,
      SyntaxFeatureId::ShorthandObjectMethod,
    ] {
      assert!(features.contains(&feature), "missing {feature:?}");
    }
  }

  #[test]
  fn detects_module_syntax_features() {
    let source = SourceFile::javascript(
      "input.mjs",
      r#"
        import value from "./value.json" with { type: "json" };
        export const named = value;
        export default named;
        export * as tools from "./tools.js";
      "#,
    );

    let result = SyntaxFeatureDetector::new().detect(&source).unwrap();
    let features = result
      .usages()
      .iter()
      .map(SyntaxFeatureUsage::feature)
      .collect::<std::collections::HashSet<_>>();

    for feature in [
      SyntaxFeatureId::ImportStatement,
      SyntaxFeatureId::ImportAttribute,
      SyntaxFeatureId::ExportStatement,
      SyntaxFeatureId::ExportDefaultStatement,
      SyntaxFeatureId::ExportNamespaceStatement,
    ] {
      assert!(features.contains(&feature), "missing {feature:?}");
    }
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

    assert_eq!(
      SyntaxFeatureDetector::new()
        .detect(&typescript)
        .unwrap()
        .usages()
        .len(),
      1,
    );
    assert_eq!(
      SyntaxFeatureDetector::new()
        .detect(&jsx)
        .unwrap()
        .usages()
        .len(),
      1,
    );
  }

  #[test]
  fn reports_parser_diagnostics_with_the_source_path() {
    let source = SourceFile::javascript("src/input.js", "const value = ;");

    let error = SyntaxFeatureDetector::new().detect(&source).unwrap_err();

    match error {
      SyntaxFeatureDetectionError::Parse { path, diagnostics } => {
        assert_eq!(path, PathBuf::from("src/input.js"));
        assert!(!diagnostics.is_empty());
      }
    }
  }
}
