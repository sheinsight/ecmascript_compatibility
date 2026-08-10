use crate::{syntax_feature::SyntaxFeatureId, target::Runtime};

use super::SupportRule;

/// ECMAScript 语法特性的兼容性规则表。
///
/// 数据来自 MDN Browser Compat Data 的 JavaScript 条目，但这里的查询入口只接受
/// `SyntaxFeatureId`。运行时 API 条目即使存在于生成表中，也不会从这个领域模型暴露。
#[derive(Debug, Default, Clone, Copy)]
pub struct SyntaxCompatDatabase;

impl SyntaxCompatDatabase {
  pub const fn new() -> Self {
    Self
  }

  pub fn support_rule(
    &self,
    feature: SyntaxFeatureId,
    runtime: Runtime,
  ) -> SupportRule {
    super::mdn_generated::support_rule(feature.mdn_key(), runtime)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn returns_explicit_optional_chaining_support_boundaries() {
    let database = SyntaxCompatDatabase::new();

    let cases = [
      (Runtime::Chrome, SupportRule::Since("80".parse().unwrap())),
      (Runtime::Edge, SupportRule::Since("80".parse().unwrap())),
      (Runtime::Firefox, SupportRule::Since("74".parse().unwrap())),
      (Runtime::Safari, SupportRule::Since("13.1".parse().unwrap())),
      (Runtime::Ios, SupportRule::Since("13.4".parse().unwrap())),
      (
        Runtime::ChromeAndroid,
        SupportRule::Since("80".parse().unwrap()),
      ),
      (
        Runtime::FirefoxAndroid,
        SupportRule::Since("79".parse().unwrap()),
      ),
      (
        Runtime::SamsungInternet,
        SupportRule::Since("13.0".parse().unwrap()),
      ),
      (Runtime::Node, SupportRule::Since("14.0.0".parse().unwrap())),
      (Runtime::InternetExplorer, SupportRule::Never),
    ];

    for (runtime, expected) in cases {
      assert_eq!(
        database.support_rule(SyntaxFeatureId::OptionalChaining, runtime),
        expected,
      );
    }
  }

  #[test]
  fn returns_unknown_for_unmapped_runtimes() {
    let database = SyntaxCompatDatabase::new();

    assert_eq!(
      database
        .support_rule(SyntaxFeatureId::OptionalChaining, Runtime::UcAndroid),
      SupportRule::Unknown,
    );
  }

  #[test]
  fn maps_detected_syntax_features_to_mdn_support_rules() {
    let database = SyntaxCompatDatabase::new();

    for feature in [
      SyntaxFeatureId::OptionalChaining,
      SyntaxFeatureId::NullishCoalescing,
      SyntaxFeatureId::LogicalAndAssignment,
      SyntaxFeatureId::LogicalOrAssignment,
      SyntaxFeatureId::NullishCoalescingAssignment,
      SyntaxFeatureId::DynamicImport,
      SyntaxFeatureId::ImportMeta,
      SyntaxFeatureId::BigIntLiteral,
      SyntaxFeatureId::ArrowFunction,
      SyntaxFeatureId::AsyncFunction,
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
      SyntaxFeatureId::ImportStatement,
      SyntaxFeatureId::ExportStatement,
      SyntaxFeatureId::ExportDefaultStatement,
      SyntaxFeatureId::ExportNamespaceStatement,
      SyntaxFeatureId::ImportAttribute,
    ] {
      assert_ne!(
        database.support_rule(feature, Runtime::Chrome),
        SupportRule::Unknown,
        "{feature:?} is not mapped to an MDN BCD rule",
      );
    }
  }
}
