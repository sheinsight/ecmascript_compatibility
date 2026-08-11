# ECMAScript Syntax Compatibility Analyzer

这个仓库提供一个 Rust 实现的 ECMAScript 语法兼容性分析器。它读取 JavaScript 文件，识别当前代码里实际出现的语法特性，并根据内置的 MDN Browser Compat Data 规则判断这些语法在目标运行时中是否可用。

当前定位是 **syntax compatibility**，不是完整的 JavaScript 运行时兼容性扫描。因此它会检测 `?.`、`??`、class fields、ESM import/export 等语法事实；不会检测 `Promise.any()`、`Array.prototype.at()`、`Object.hasOwn()` 这类运行时 API。

## 能力边界

- 分析 JavaScript 构建产物或源码文件。
- 自动发现 `sourceMappingURL`，并支持相邻 `.map` 文件回退。
- 支持本地文件 Source Map 和 data URI Source Map。
- 报告保留 generated 文件位置；Source Map 成功时额外提供 original source 位置。
- 兼容性数据来自手工校对后的内置静态表，当前同步来源是 MDN Browser Compat Data。

不覆盖的场景：

- 运行时 API 兼容性。
- 被构建工具转换掉的原始语法。detector 只能看到输入文件中仍然存在的语法。
- TypeScript 类型语法。输入应当是可被 JavaScript parser 解析的代码。
- HTTP Source Map、Source Map response header、index source map、多级 Source Map 链路。

## CLI

```sh
cargo run -p ecmascript_compatibility -- <generated-js-file> <target> [target...]
```

示例：

```sh
cargo run -p ecmascript_compatibility -- dist/app.js "chrome 60" "safari 13"
```

CLI 会输出：

- 输入文件和解析后的 targets。
- Source Map 文件级状态。
- Unsupported、Mixed、Unknown 诊断。
- 每条诊断的 generated 位置和可用的 original source 位置。

## Library API

最常用入口是 `CompatAnalyzer::analyze_path`：

```rust
use ecmascript_compatibility::CompatAnalyzer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let analyzer = CompatAnalyzer::new();
  let targets = analyzer.resolve_targets(["chrome 60", "safari 13"])?;
  let report = analyzer.analyze_path("dist/app.js", &targets)?;

  for diagnostic in report.diagnostics() {
    println!(
      "{:?} at {:?}",
      diagnostic.feature(),
      diagnostic.position()
    );
  }

  Ok(())
}
```

也可以运行仓库内示例：

```sh
cargo run -p ecmascript_compatibility --example analyze_file -- dist/app.js "chrome 60"
```

更完整的 API 说明见 [docs/api-usage.md](docs/api-usage.md)。

## Node.js API

仓库提供 napi-rs binding 包，JS 侧可以直接传入 `cwd`，由 native 层递归扫描
目录下的 `.js`、`.mjs`、`.cjs` 和 `.jsx` 文件并批量分析：

```js
const { checkDirectory } = require("@shined/ecmascript-compatibility");

const report = checkDirectory(process.cwd(), ["chrome 60", "safari 13"]);

console.log(report.fileCount);
console.log(report.diagnosticCount);
```

`checkDirectory` 会并行分析文件。需要限制 worker 数时可以传 `parallelism`。
默认只返回有诊断的文件；需要保留空诊断文件报告时可以传 `excludeEmptyReports: false`。

本地构建 binding：

```sh
pnpm --filter @shined/ecmascript-compatibility build
```

## 数据同步

MDN 数据同步脚本只生成 `SyntaxFeatureId` 实际引用的条目，不把完整 JavaScript BCD 表写进源码：

```sh
node scripts/sync_mdn_bcd.js
```

同步后需要人工检查生成表和语法特性映射是否符合当前项目边界。

详细流程见 [docs/mdn-data-sync.md](docs/mdn-data-sync.md)。

## 验证

```sh
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm --filter @shined/ecmascript-compatibility build
```
