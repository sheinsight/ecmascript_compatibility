# API 使用说明

当前对外推荐入口是 `CompatAnalyzer`。调用方先把目标运行时查询解析为 `RuntimeTarget`，再把 JavaScript 文件和解析后的 targets 交给分析器，最终返回 `CompatReport`。

## 最小用法

```rust
use ecma_compat::CompatAnalyzer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let analyzer = CompatAnalyzer::new();
  let targets = analyzer.resolve_targets(["chrome 60", "safari 13"])?;
  let report = analyzer.analyze_path("dist/app.js", &targets)?;

  for diagnostic in report.diagnostics() {
    println!(
      "{:?} at generated {:?}",
      diagnostic.feature(),
      diagnostic.position()
    );
  }

  Ok(())
}
```

`diagnostics()` 默认只包含需要关注的 target 状态：`Unsupported`、`Mixed` 和 `Unknown`。明确 `Supported` 的 target 默认不会进入诊断，避免报告噪声。

## 目标运行时查询

目标查询字符串通过 `resolve_targets` 解析，`analyze_path` 接受解析后的 targets：

```rust
let analyzer = CompatAnalyzer::new();
let targets = analyzer.resolve_targets(["chrome 60", "firefox 78", "node 14"])?;
let report = analyzer.analyze_path("dist/app.js", &targets)?;
```

解析结果是 `RuntimeTarget` 列表。诊断里的 `target_index` 指向这次传入 `analyze_path` 的 targets 列表。

## Source Map 结果

报告里有两层 Source Map 信息：

- `report.source_map_status()`：文件级 Source Map 发现和解析状态。
- `diagnostic.source_mapping()`：单条语法 usage 的 original source 映射结果。

示例：

```rust
use ecma_compat::{CompatAnalyzer, source_map::SourceMapping};

let analyzer = CompatAnalyzer::new();
let targets = analyzer.resolve_targets(["chrome 60"])?;
let report = analyzer.analyze_path("dist/app.js", &targets)?;

for diagnostic in report.diagnostics() {
  match diagnostic.source_mapping() {
    SourceMapping::Mapped(location) => {
      println!(
        "original source: {:?} {}:{}",
        location.source(),
        location.start().line() + 1,
        location.start().col() + 1
      );
    }
    SourceMapping::Unavailable(reason) => {
      println!("source map unavailable: {reason:?}");
    }
    SourceMapping::NotResolved => {
      println!("source map was not resolved for this usage");
    }
  }
}
```

注意：Source Map 只增强定位，不影响语法兼容性判断。Source Map 缺失时，诊断仍然保留 generated 文件位置。

## Node.js 目录分析

napi binding 提供 `checkDirectory(cwd, targets, options)`，会递归扫描目录下的 JavaScript 文件并返回目录级报告。

```js
const { checkDirectory } = require("@shined/ecmascript-compatibility");

const report = checkDirectory("dist/statics", ["chrome 60"], {
  excludeEmptyReports: false,
});
```

`excludeEmptyReports` 默认为 `true`，只影响 JS 返回值：`reports` 中会过滤掉 `diagnostics.length === 0` 的文件报告，`errors` 仍会保留。需要全量文件报告时传 `excludeEmptyReports: false`。

## 分析内存中的内容

如果调用方已经读取了文件，或输入来自虚拟文件，可以构造 `SourceFile` 后调用 `analyze_source`：

```rust
use ecma_compat::{CompatAnalyzer, SourceFile};

let source = SourceFile::from_path(
  "dist/app.js".into(),
  "const value = object?.field;".to_string(),
)?;

let analyzer = CompatAnalyzer::new();
let targets = analyzer.resolve_targets(["chrome 60"])?;
let report = analyzer.analyze_source(source, &targets)?;
```

`SourceFile` 的 path 仍用于 Source Map 相对路径解析和诊断展示。

## Builder

默认分析器只报告非 Supported 状态。如果调试时需要完整 target 矩阵，可以使用 builder：

```rust
let analyzer = ecma_compat::CompatAnalyzer::builder()
  .include_supported_targets(true)
  .build();
```

后续如需自定义 Source Map loader，也应通过 builder 扩展，而不是让调用方直接组装 detector、resolver 和 database。

## 示例程序

```sh
cargo run -p ecma_compat --example analyze_file -- dist/app.js "chrome 60"
```
