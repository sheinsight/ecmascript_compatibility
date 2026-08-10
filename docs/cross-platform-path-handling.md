# 跨平台路径处理方案

## 问题背景

在 Rust 中，`std::path::PathBuf` 和 `Path` 会使用**宿主平台的路径分隔符**：

- Linux/macOS: `/`
- Windows: `\`

当你用 `Path::join()` 或 `parent()` 等方法操作路径时，结果会包含平台原生分隔符。这在处理 Web 生态相关的路径（如 Source Map、URL、import specifier）时会导致跨平台不一致。

```rust
use std::path::PathBuf;

let base = PathBuf::from("dist/app.js");
let resolved = base.parent().unwrap().join("app.js.map");

// macOS/Linux: "dist/app.js.map" ✓
// Windows:    "dist\\app.js.map" ✗ — 与 Web 规范不一致
```

---

## 方案：入口处规范化

**核心思路**：在路径进入领域模型的边界点统一规范化为 `/`，而非让每个消费者自行处理。

### 示例：Source Map 引用

```rust
pub fn local_file(path: impl Into<PathBuf>) -> Self {
    Self::LocalFile(normalize_path(path.into()))
}

fn normalize_path(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.to_string_lossy().replace('\\', "/"))
    } else {
        path
    }
}
```

优势：
- 所有下游代码看到的路径格式统一
- 错误信息在所有平台上一致
- 测试可以直接用字面量 `"dist/app.js.map"` 断言，无需平台适配

---

## 常见踩坑场景

### 1. `Path::join()` 产出平台分隔符

```rust
// ❌ 测试在 Windows 上会失败
assert_eq!(
    resolve("dist/app.js", "app.js.map"),
    "dist/app.js.map"  // Windows 实际产出 "dist\\app.js.map"
);

// ✅ 在构造时规范化
fn resolve(base: &str, relative: &str) -> String {
    let path = Path::new(base).parent().unwrap().join(relative);
    normalize(path)
}
```

### 2. `PathBuf::from()` 不会转换分隔符

```rust
// 两边都从字面量构造 — 不经过 join，任何平台都一致
PathBuf::from("dist/app.js.map") == PathBuf::from("dist/app.js.map") // ✓ 始终成立

// 但一旦经过 join，Windows 上就变了
PathBuf::from("dist").join("app.js.map")
// macOS: "dist/app.js.map"
// Windows: "dist\\app.js.map"
```

### 3. `display().to_string()` 输出平台原生分隔符

```rust
let path = PathBuf::from("dist").join("app.js.map");

// macOS: "dist/app.js.map"
// Windows: "dist\\app.js.map"
println!("{}", path.display());
```

如果这个字符串会出现在错误信息、日志、或序列化输出中，必须规范化。

---

## 何时需要规范化

| 场景 | 是否需要 |
|------|----------|
| 纯本地文件 I/O（`fs::read`, `fs::write`） | ❌ 系统 API 接受两种分隔符 |
| 错误信息中展示路径 | ✅ 用户看到的内容应跨平台一致 |
| 与 Web 规范交互（Source Map, URL, import path） | ✅ 规范要求 `/` |
| 序列化到 JSON/配置文件 | ✅ 消费方通常期望 `/` |
| 路径相等性比较 | ✅ 否则同一路径在不同平台上不相等 |
| 临时中间变量，不对外暴露 | ❌ 无需额外开销 |

---

## 最佳实践总结

1. **在领域模型入口处做一次规范化**，而非在每个使用点做转换
2. **用 `cfg!(windows)` 条件编译**，非 Windows 平台零开销
3. **测试直接用 `/` 字面量**，不要用 `PathBuf::from("a").join("b")` 适配平台
4. **区分"文件系统路径"和"逻辑路径"**：前者交给 OS 处理，后者统一为 `/`
5. **`to_string_lossy()` 足够安全**：Source Map 路径只含 ASCII，不会丢失信息

---

## 社区参考

主流 JS 工具链中的做法：

- **swc** / **oxc**：内部路径表示统一用 `/`，仅在与文件系统交互时使用原生路径
- **webpack** / **rollup**：输出的 Source Map `sources` 字段始终用 `/`
- **Parcel**：有专门的 `normalizeSeparators` 工具函数
- **TypeScript 编译器**：内部用 `normalizeSlashes()` 将 `\` 转为 `/`

这不是个别项目的偏好，而是 Web 生态的事实标准 — Source Map v3 规范中 `sources` 字段就是 URL 格式。
