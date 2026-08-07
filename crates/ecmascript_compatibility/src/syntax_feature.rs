use crate::source_map::SourceMapping;

/// 兼容性检查能够识别的 ECMAScript 语法特性。
///
/// 这里的 feature 只描述解析后 AST 中能直接确认的语法事实，例如 `?.`、
/// class fields 或 ESM import/export。它不覆盖运行时 API 使用，例如
/// `Promise.any()`、`Array.prototype.at()` 或 `Object.hasOwn()`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxFeatureId {
  /// 可选链表达式，例如 `object?.property`。
  OptionalChaining,
  /// 空值合并表达式，例如 `value ?? fallback`。
  NullishCoalescing,
  /// 逻辑与赋值表达式，例如 `value &&= next`。
  LogicalAndAssignment,
  /// 逻辑或赋值表达式，例如 `value ||= next`。
  LogicalOrAssignment,
  /// 空值合并赋值表达式，例如 `value ??= fallback`。
  NullishCoalescingAssignment,
  /// 动态 import 表达式，例如 `import("./module.js")`。
  DynamicImport,
  /// `import.meta` 元属性。
  ImportMeta,
  /// BigInt 字面量，例如 `1n`。
  BigIntLiteral,
  /// 箭头函数表达式，例如 `value => value + 1`。
  ArrowFunction,
  /// async 函数或 async 箭头函数。
  AsyncFunction,
  /// generator 函数。
  GeneratorFunction,
  /// async generator 函数。
  AsyncGeneratorFunction,
  /// class 声明或 class 表达式。
  Class,
  /// public class field，例如 `class C { value = 1 }`。
  PublicClassField,
  /// private class field，例如 `class C { #value = 1 }`。
  PrivateClassField,
  /// class static initialization block，例如 `class C { static {} }`。
  ClassStaticInitializationBlock,
  /// `for...of` 语句。
  ForOf,
  /// `for await...of` 语句。
  ForAwaitOf,
  /// spread 语法，例如 `fn(...args)` 或 `[...items]`。
  Spread,
  /// object literal spread property，例如 `{ ...source }`。
  ObjectSpreadProperty,
  /// destructuring 绑定或赋值，例如 `const { value } = source`。
  Destructuring,
  /// array destructuring rest，例如 `const [head, ...tail] = values`。
  ArrayRestDestructuring,
  /// object destructuring rest，例如 `const { value, ...rest } = source`。
  ObjectRestDestructuring,
  /// 函数默认参数，例如 `function run(value = 1) {}`。
  DefaultParameter,
  /// 函数 rest 参数，例如 `function run(...values) {}`。
  RestParameter,
  /// 模板字面量，例如 `` `hello ${name}` ``。
  TemplateLiteral,
  /// 数字分隔符，例如 `1_000`。
  NumericSeparator,
  /// 可选 catch binding，例如 `try {} catch {}`。
  OptionalCatchBinding,
  /// await 表达式，例如 `await task`。
  Await,
  /// private field `in` 检查，例如 `#value in object`。
  PrivateClassFieldIn,
  /// private class method，例如 `class C { #method() {} }`。
  PrivateClassMethod,
  /// 方法定义语法，例如 `{ run() {} }` 或 `class C { run() {} }`。
  MethodDefinition,
  /// async 方法定义语法，例如 `{ async run() {} }`。
  AsyncMethod,
  /// async generator 方法定义语法，例如 `{ async *run() {} }`。
  AsyncGeneratorMethod,
  /// computed object property name，例如 `{ [key]: value }`。
  ComputedObjectPropertyName,
  /// object shorthand property，例如 `{ value }`。
  ShorthandObjectProperty,
  /// object shorthand method，例如 `{ run() {} }`。
  ShorthandObjectMethod,
  /// ESM import 声明，例如 `import value from "pkg"`。
  ImportStatement,
  /// ESM export 声明，例如 `export { value }`。
  ExportStatement,
  /// ESM default export 声明，例如 `export default value`。
  ExportDefaultStatement,
  /// ESM namespace export 声明，例如 `export * as ns from "pkg"`。
  ExportNamespaceStatement,
  /// import attributes/assertions，例如 `import data from "./x.json" with { type: "json" }`。
  ImportAttribute,
}

impl SyntaxFeatureId {
  pub(crate) const fn mdn_key(self) -> &'static str {
    match self {
      Self::OptionalChaining => "javascript.operators.optional_chaining",
      Self::NullishCoalescing => "javascript.operators.nullish_coalescing",
      Self::LogicalAndAssignment => {
        "javascript.operators.logical_and_assignment"
      }
      Self::LogicalOrAssignment => "javascript.operators.logical_or_assignment",
      Self::NullishCoalescingAssignment => {
        "javascript.operators.nullish_coalescing_assignment"
      }
      Self::DynamicImport => "javascript.operators.import",
      Self::ImportMeta => "javascript.operators.import_meta",
      Self::BigIntLiteral => "javascript.builtins.BigInt",
      Self::ArrowFunction => "javascript.functions.arrow_functions",
      Self::AsyncFunction => "javascript.operators.async_function",
      Self::GeneratorFunction => "javascript.operators.generator_function",
      Self::AsyncGeneratorFunction => {
        "javascript.operators.async_generator_function"
      }
      Self::Class => "javascript.classes",
      Self::PublicClassField => "javascript.classes.public_class_fields",
      Self::PrivateClassField => "javascript.classes.private_class_fields",
      Self::ClassStaticInitializationBlock => {
        "javascript.classes.static.initialization_blocks"
      }
      Self::ForOf => "javascript.statements.for_of",
      Self::ForAwaitOf => "javascript.statements.for_await_of",
      Self::Spread => "javascript.operators.spread",
      Self::ObjectSpreadProperty => {
        "javascript.operators.spread.spread_in_object_literals"
      }
      Self::Destructuring => "javascript.operators.destructuring",
      Self::ArrayRestDestructuring => {
        "javascript.operators.destructuring.rest_in_arrays"
      }
      Self::ObjectRestDestructuring => {
        "javascript.operators.destructuring.rest_in_objects"
      }
      Self::DefaultParameter => "javascript.functions.default_parameters",
      Self::RestParameter => "javascript.functions.rest_parameters",
      Self::TemplateLiteral => "javascript.grammar.template_literals",
      Self::NumericSeparator => "javascript.grammar.numeric_separators",
      Self::OptionalCatchBinding => {
        "javascript.statements.try_catch.optional_catch_binding"
      }
      Self::Await => "javascript.operators.await",
      Self::PrivateClassFieldIn => "javascript.classes.private_class_fields_in",
      Self::PrivateClassMethod => "javascript.classes.private_class_methods",
      Self::MethodDefinition => "javascript.functions.method_definitions",
      Self::AsyncMethod => {
        "javascript.functions.method_definitions.async_methods"
      }
      Self::AsyncGeneratorMethod => {
        "javascript.functions.method_definitions.async_generator_methods"
      }
      Self::ComputedObjectPropertyName => {
        "javascript.operators.object_initializer.computed_property_names"
      }
      Self::ShorthandObjectProperty => {
        "javascript.operators.object_initializer.shorthand_property_names"
      }
      Self::ShorthandObjectMethod => {
        "javascript.operators.object_initializer.shorthand_method_names"
      }
      Self::ImportStatement => "javascript.statements.import",
      Self::ExportStatement => "javascript.statements.export",
      Self::ExportDefaultStatement => "javascript.statements.export.default",
      Self::ExportNamespaceStatement => {
        "javascript.statements.export.namespace"
      }
      Self::ImportAttribute => "javascript.statements.import.import_attributes",
    }
  }
}

/// syntax detector 从输入文本中直接观察到的 UTF-8 byte span。
///
/// `start` 和 `end` 都基于完整输入文本的字节偏移，且 `end` 是 exclusive。
/// 这个 span 表示产物中的事实位置，不会被 Source Map 映射结果覆盖。
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

/// 单次语法特性使用记录。
///
/// `span` 始终表示 syntax detector 在当前输入文件中看到的位置。Source Map 只提供
/// 附加的源码定位状态，因此单独保存在 `source_mapping` 中。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyntaxFeatureUsage {
  feature: SyntaxFeatureId,
  span: SourceSpan,
  source_mapping: SourceMapping,
}

impl SyntaxFeatureUsage {
  pub(crate) const fn new(feature: SyntaxFeatureId, span: SourceSpan) -> Self {
    Self {
      feature,
      span,
      source_mapping: SourceMapping::NotResolved,
    }
  }

  pub const fn feature(&self) -> SyntaxFeatureId {
    self.feature
  }

  pub const fn span(&self) -> SourceSpan {
    self.span
  }

  pub const fn source_mapping(&self) -> &SourceMapping {
    &self.source_mapping
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_feature_usage_starts_with_unresolved_source_mapping() {
    let usage = SyntaxFeatureUsage::new(
      SyntaxFeatureId::OptionalChaining,
      SourceSpan::new(10, 20),
    );

    assert_eq!(usage.feature(), SyntaxFeatureId::OptionalChaining);
    assert_eq!(usage.span(), SourceSpan::new(10, 20));
    assert_eq!(usage.source_mapping(), &SourceMapping::NotResolved);
  }
}
