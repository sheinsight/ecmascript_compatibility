# MDN 数据同步说明

项目的兼容性规则来自 MDN Browser Compat Data，但源码中不会内置完整 JavaScript BCD 表。同步脚本只保留 `SyntaxFeatureId::mdn_key()` 实际引用的条目。

## 为什么只生成语法条目

当前项目定位是 ECMAScript 语法兼容性检测。完整 JavaScript BCD 表包含大量运行时 API，例如 `Promise.any`、`Array.prototype.at`、`Object.hasOwn`。这些 API 不会被当前 detector 识别，放进生成表只会扩大代码体积并模糊项目边界。

因此数据链路是：

```text
SyntaxFeatureId
  -> mdn_key()
  -> scripts/sync_mdn_bcd.js 白名单
  -> database/mdn_generated.rs
  -> SyntaxCompatDatabase::support_rule()
```

## 同步命令

```sh
node scripts/sync_mdn_bcd.js
```

脚本会：

- 从 `@mdn/browser-compat-data` 获取 `data.json` 和 `package.json`。
- 读取 `crates/ecmascript_compatibility/src/syntax_feature.rs` 中的 `javascript.*` key。
- 只生成这些 key 对应的 support rule。
- 如果某个 `SyntaxFeatureId` 引用的 key 在 MDN 数据中不存在，脚本直接失败。

生成结果在：

```text
crates/ecmascript_compatibility/src/database/mdn_generated.rs
```

## 人工校对点

每次同步后至少检查：

- `MDN_BCD_PACKAGE_VERSION` 是否是预期版本。
- `MDN_BCD_SYNTAX_ENTRY_COUNT` 是否和当前语法特性数量匹配。
- 新增或变化的 MDN key 是否仍然表达“语法特性”，而不是运行时 API。
- `BigIntLiteral` 当前映射到 `javascript.builtins.BigInt`，这是因为 MDN BCD 没有更细的 BigInt literal 语法 key；这条规则需要人工关注。
- import attributes、class static block、private field `in` 等较新语法在目标 runtime 上是否符合预期。

## 验证命令

```sh
cargo fmt --all
cargo test -p ecmascript_compatibility
cargo clippy -p ecmascript_compatibility --all-targets -- -D warnings
```

## 新增语法特性的流程

1. 在 `SyntaxFeatureId` 增加枚举值和注释。
2. 在 `SyntaxFeatureId::mdn_key()` 绑定对应 MDN key。
3. 在 `SyntaxFeatureDetector` 中增加 AST 访问逻辑。
4. 增加 detector 单元测试。
5. 运行 `node scripts/sync_mdn_bcd.js`。
6. 运行测试和 clippy。

不要为了新增运行时 API 直接扩展 `SyntaxFeatureId`。运行时 API 如果未来要做，应建立独立 detector 和独立领域模型。
