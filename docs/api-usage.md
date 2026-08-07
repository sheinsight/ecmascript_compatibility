# API 使用说明

当前对外推荐入口是 `CompatAnalyzer`。调用方提供 JavaScript 文件和目标运行时查询，分析器返回 `CompatReport`。

## 最小用法

```rust
use ecmascript_compatibility::CompatAnalyzer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let report = CompatAnalyzer::new()
    .analyze_path("dist/app.js", ["chrome 60", "safari 13"])?;

  for diagnostic in report.diagnostics() {
    println!(
      "{:?} at generated {:?}",
      diagnostic.feature(),
      diagnostic.generated_position()
    );
  }

  Ok(())
}
```

`diagnostics()` 默认只包含需要关注的 target 状态：`Unsupported`、`Mixed` 和 `Unknown`。明确 `Supported` 的 target 默认不会进入诊断，避免报告噪声。

## 目标运行时查询

`analyze_path` 接受字符串列表：

```rust
let report = CompatAnalyzer::new()
  .analyze_path("dist/app.js", ["chrome 60", "firefox 78", "node 14"])?;
```

查询字符串会被解析为 `RuntimeTarget`。解析结果可以通过 `report.targets()` 获取。

## Source Map 结果

报告里有两层 Source Map 信息：

- `report.source_map_status()`：文件级 Source Map 发现和解析状态。
- `diagnostic.source_mapping()`：单条语法 usage 的 original source 映射结果。

示例：

```rust
use ecmascript_compatibility::{CompatAnalyzer, source_map::SourceMapping};

let report = CompatAnalyzer::new()
  .analyze_path("dist/app.js", ["chrome 60"])?;

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

## 分析内存中的内容

如果调用方已经读取了文件，或输入来自虚拟文件，可以构造 `SourceFile` 后调用 `analyze_source`：

```rust
use ecmascript_compatibility::{CompatAnalyzer, SourceFile};

let source = SourceFile::from_path(
  "dist/app.js".into(),
  "const value = object?.field;".to_string(),
)?;

let report = CompatAnalyzer::new()
  .analyze_source(source, ["chrome 60"])?;
```

`SourceFile` 的 path 仍用于 Source Map 相对路径解析和诊断展示。

## Builder

默认分析器只报告非 Supported 状态。如果调试时需要完整 target 矩阵，可以使用 builder：

```rust
let analyzer = ecmascript_compatibility::CompatAnalyzer::builder()
  .include_supported_targets(true)
  .build();
```

后续如需自定义 Source Map loader，也应通过 builder 扩展，而不是让调用方直接组装 detector、resolver 和 database。

## 示例程序

```sh
cargo run -p ecmascript_compatibility --example analyze_file -- dist/app.js "chrome 60"
```
