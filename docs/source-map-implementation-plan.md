# Source Map 定位能力实施计划

## 1. 文档目标

本文档用于指导 `ecmascript_compatibility` 的 Source Map 定位能力建设。

目标不是让 Source Map 参与语法兼容性判断，而是在产物文件中检测到不兼容语法后，尽可能把问题定位回原始源码；无法映射时，必须保留产物位置并给出明确的降级原因。

完整链路如下：

```text
产物文件
  ↓
FeatureDetector
  ↓
产物中的 FeatureUsage
  ↓
SourceMapResolver
  ↓
SourceMapMapper
  ↓
产物位置 + 可选的源码位置
  ↓
兼容性检查与诊断报告
```

## 2. 当前测试素材

当前可用于手工验收的真实构建产物：

```text
/Users/10015448/Git/modb-front/dist/statics/main.d5b4492ea606.js
/Users/10015448/Git/modb-front/dist/statics/main.d5b4492ea606.js.map
```

已确认的素材特征：

- 产物文件约 295 KB；
- Source Map 文件约 350 KB；
- 产物文件内没有 `sourceMappingURL`；
- 同目录存在追加 `.map` 后缀的 Source Map；
- Source Map 的 `version` 为 `3`；
- Source Map 包含 76 个 `sources`；
- 76 个 source 都包含对应的 `sourcesContent`；
- source 身份主要使用 `webpack://modb-front/...` 虚拟 URL；
- 产物中没有 `?.`，但 `sourcesContent` 中包含可选链语法，说明构建工具已经转译该语法。

自动化测试不能依赖上述绝对路径。实现阶段需要在仓库内创建最小化 fixtures；真实文件仅用于最终人工验收，或者经过裁剪后作为专用集成测试素材提交。

## 3. 范围与非目标

### 3.1 第一阶段目标

- 支持 JavaScript 产物；
- 支持 Source Map v3；
- 支持普通 Source Map 文档；
- 支持文件内显式 `sourceMappingURL`；
- 兼容 `//#` 与历史 `//@` 注释；
- 支持相对本地路径、绝对本地路径、`file://` 和内联 `data:`；
- 未声明 `sourceMappingURL` 时，尝试 `产物文件名 + .map`；
- 支持 `sourceRoot`、`sources`、`sourcesContent`、`names` 和 `mappings`；
- 将 OXC UTF-8 字节偏移正确转换为 Source Map 所需的 UTF-16 行列；
- 映射失败时保留产物位置和检测结果；
- 保留 `webpack://` 等虚拟 source 身份；
- 为所有降级情况提供结构化原因。

### 3.2 后续能力

- Index Source Map 的 `sections`；
- `ignoreList` 与兼容字段 `x_google_ignoreList`；
- HTTP `SourceMap` 响应头；
- 可选的 HTTP(S) Source Map 加载器；
- 多级 Source Map 链式映射；
- 远程 original source 获取；
- 映射结果缓存与跨文件共享。

### 3.3 暂不支持

- Source Map v1、v2；
- CSS Source Map；
- WebAssembly Source Map；
- 默认联网下载 Source Map 或源码；
- 把 Source Map 缺失当作兼容性检测失败；
- 通过 Source Map 推断产物中已经不存在的语法。

## 4. 必须保持的架构原则

### 4.1 Detector 只负责语法检测

`FeatureDetector` 只负责：

- 根据 `SourceKind` 解析源文本；
- 遍历 AST；
- 识别 `FeatureId`；
- 记录产物中的 OXC byte span。

它不负责：

- 寻找 `.map` 文件；
- 解析 URL；
- 读取网络资源；
- 解码 VLQ；
- 映射源码位置；
- 决定 Source Map 失败是否需要警告。

### 4.2 产物位置永远是事实

产物中的语法由 detector 直接观察得到，因此 `generated_span` 永远保留。

Source Map 提供的是附加解释，可能缺失、损坏、过期或没有覆盖当前位置。成功映射后也不能覆盖或删除产物位置。

### 4.3 一个产物对应多个源码

`DetectionResult` 保存一个产物路径，但不同 `FeatureUsage` 可以映射到不同 original source。

因此不能设计成：

```rust
pub struct DetectionResult {
  generated_path: PathBuf,
  original_path: PathBuf,
  usages: Vec<FeatureUsage>,
}
```

源码身份必须位于每条 usage 的映射结果中。

### 4.4 Original source 不等于本地文件路径

Source Map 中的 source 可能是：

```text
src/index.ts
file:///project/src/index.ts
https://example.com/src/index.ts
webpack://project/src/index.ts
vite://project/src/index.ts
null
```

因此 original source 不能直接建模为 `PathBuf`。必须使用能表达本地文件、URL、虚拟 source 和未知 source 的领域类型。

### 4.5 Source Map 是可失败的增强层

以下情况都不能删除兼容性发现：

- 未声明 Source Map；
- 同名 `.map` 不存在；
- 显式 Source Map 无法加载；
- Source Map JSON 损坏；
- Source Map 版本不支持；
- 当前 generated position 没有 mapping；
- original source 没有 `sourcesContent`；
- original source 是无法加载的虚拟 URL。

这些情况必须降级为“只报告产物位置”。

### 4.6 显式引用优先且具有权威性

存在显式 `sourceMappingURL` 时，只使用显式引用。

如果显式引用失效，默认不继续尝试同名 `.map`。否则可能错误关联旧产物或其他 bundle 的 map。

只有完全没有显式引用时，才尝试：

```text
main.js → main.js.map
```

未来可以提供非默认配置 `fallback_after_explicit_failure`，但默认值必须是 `false`。

## 5. 目标领域模型

### 5.1 检测结果

`DetectionResult.path` 建议重命名为 `generated_path`，避免加入 original source 后出现语义歧义。

```rust
pub struct DetectionResult {
  generated_path: PathBuf,
  usages: Vec<FeatureUsage>,
}
```

### 5.2 特性使用

每条 usage 永远保存产物 byte span，并保存结构化的源码映射状态。

```rust
pub struct FeatureUsage {
  feature: FeatureId,
  generated_span: SourceSpan,
  source_mapping: SourceMapping,
}
```

### 5.3 源码映射状态

不能只使用 `Option<OriginalLocation>`，否则无法区分“尚未执行映射”和“已经执行但不可用”。

```rust
pub enum SourceMapping {
  NotResolved,
  Mapped(OriginalLocation),
  Unavailable(SourceMapUnavailable),
}
```

状态含义：

- `NotResolved`：detector 刚刚产生 usage，尚未进入 Source Map 阶段；
- `Mapped`：已获得 original source 和源码位置；
- `Unavailable`：已尝试解析，但无法获得 original location。

最终公共检查流程不应把 `NotResolved` 当作最终报告结果。编排层必须把它转换为 `Mapped` 或 `Unavailable`。

### 5.4 Original source 身份

```rust
pub enum SourceIdentity {
  File(PathBuf),
  Url(String),
  Virtual(String),
  Unknown,
}
```

第一版可以内部保存原始字符串，并提供分类后的只读访问器；不要为了方便而把 `webpack://` 强制转换为本地路径。

### 5.5 Original position

当前 `SourceSpan` 表示 UTF-8 byte range，不适合直接表示 Source Map 行列。

```rust
pub struct SourcePosition {
  line: u32,
  column: u32,
}

pub struct OriginalLocation {
  source: SourceIdentity,
  start: SourcePosition,
  end: Option<SourcePosition>,
}
```

`end` 是可选的，因为 Source Map 原生映射的是位置点，不保证能恢复准确的源码范围。

如果 generated span 的起点和终点能够映射到同一个 original source，并且位置有序，可以形成 original range；否则只保留起点。

### 5.6 Source Map 来源

```rust
pub enum SourceMapOrigin {
  Explicit,
  AdjacentFallback,
}

pub enum SourceMapReference {
  InlineData(String),
  LocalFile(PathBuf),
  RemoteUrl(String),
}
```

生成文件、Source Map 和 original source 是三个不同身份：

```text
generated: dist/main.js
source map: dist/main.js.map
original: webpack://project/src/index.ts
```

## 6. Span 与位置偏差

这是实现中最容易产生静默错误的部分，必须单独测试。

### 6.1 当前 span 的语义

OXC 的 `Span`：

- `start` 和 `end` 是 UTF-8 byte offset；
- `end` 是 exclusive；
- offset 基于完整产物文本。

例如：

```rust
SourceSpan { start: 120, end: 135 }
```

表示产物字节区间 `120..135`。

### 6.2 Source Map 的语义

JavaScript Source Map 使用：

- 零基行号；
- 零基列号；
- 列号按 UTF-16 code unit 计算。

因此不能把 OXC byte offset 直接传给 Source Map lookup。

### 6.3 UTF-8 与 UTF-16 偏差

不同字符的计数方式不同：

| 字符 | UTF-8 bytes | UTF-16 code units |
| --- | ---: | ---: |
| `A` | 1 | 1 |
| `中` | 3 | 1 |
| `🔥` | 4 | 2 |

例如：

```javascript
const 中文 = object?.value;
```

如果直接把 `object` 的 UTF-8 byte offset 当成 Source Map column，中文之后的列会发生偏移。

### 6.4 正确转换流程

```text
OXC UTF-8 byte offset
  ↓
通过行起始索引找到零基行号
  ↓
截取该行从行首到 offset 的 UTF-8 文本
  ↓
计算 UTF-16 code units
  ↓
得到 generated line + generated UTF-16 column
  ↓
执行 Source Map lookup
```

需要新增可复用的 `GeneratedPositionIndex`，一次扫描产物文本建立行起始 byte offsets，避免每条 usage 从文件开头重复扫描。

### 6.5 换行差异

必须覆盖：

- `\n`；
- `\r\n`；
- 文件末尾无换行；
- 空行；
- 超长压缩行。

行号和列号必须基于实际输入文本，不允许先做换行标准化，否则会让 Source Map 与产物错位。

### 6.6 generated span 的终点映射

Source Map 通常提供离散 mapping 点，而不是完整范围。

第一版规则：

1. generated span 起点必须尝试映射；
2. 起点无 mapping，则整个 usage 降级为 `UnmappedPosition`；
3. 起点有 mapping 后，再尝试映射终点附近的位置；
4. 起点和终点映射到同一个 original source 且位置有序时，填写 `end`；
5. 否则只保留 original `start`；
6. 永远保留完整 `generated_span`。

终点具体使用 exclusive `end` 还是最后一个字符位置 `end - 1`，必须通过所选 Source Map crate 的 lookup 语义和 fixtures 验证后固定，不允许凭经验直接决定。

## 7. Source Map 发现策略

### 7.1 提取显式引用

支持：

```javascript
//# sourceMappingURL=main.js.map
//@ sourceMappingURL=main.js.map
//# sourceMappingURL=../maps/main.js.map
//# sourceMappingURL=file:///project/main.js.map
//# sourceMappingURL=https://cdn.example.com/main.js.map
//# sourceMappingURL=data:application/json;base64,...
```

不能只用简单正则扫描全文，否则字符串或模板字符串中的伪指令可能被误识别。

实现阶段优先使用 JavaScript comment/token 信息提取真实注释。需要收集全部匹配项：

- 没有匹配项：进入同名回退；
- 多次出现且值相同：视为同一个显式引用；
- 多次出现且值不同：返回 `AmbiguousReference`；
- 不允许静默选择任意一个冲突值。

### 7.2 相对引用解析

相对 `sourceMappingURL` 基于产物文件所在目录解析：

```text
dist/js/main.js
//# sourceMappingURL=../maps/main.js.map

→ dist/maps/main.js.map
```

不要基于进程当前工作目录解析。

### 7.3 同名回退

只有不存在显式引用时才尝试：

```text
main.js → main.js.map
bundle.min.js → bundle.min.js.map
```

这是追加 `.map`，不是把 `.js` 替换为 `.map`。

### 7.4 显式引用失败

默认行为：

```text
存在显式引用
  ├── 成功：使用显式 map
  └── 失败：Unavailable(ExplicitReferenceUnavailable)
```

不继续尝试同名回退。

## 8. Source Map 加载策略

定义加载器边界，resolver 不直接绑定文件系统或 HTTP 客户端：

```rust
pub trait SourceMapLoader {
  fn load(
    &self,
    reference: &SourceMapReference,
  ) -> Result<Vec<u8>, SourceMapLoadError>;
}
```

第一阶段实现：

- `FileSourceMapLoader`；
- `DataUriSourceMapLoader`。

后续可选实现：

- `HttpSourceMapLoader`。

安全约束：

- 默认禁止网络访问；
- 设置 Source Map 最大字节数；
- 设置 data URI 最大解码尺寸；
- original source 文件读取需要受允许根目录约束；
- 对 `..`、符号链接和 URL 编码后的路径执行规范化检查；
- 不自动把未知 scheme 当成本地路径；
- 不在错误信息中输出完整内联 Source Map 内容。

## 9. Source Map 解码策略

优先评估成熟 Rust crate，不自行实现 Base64 VLQ 和 mapping 查找。

依赖选择必须验证：

- 普通 v3 map；
- Index Source Map；
- generated position lookup；
- `sourceRoot`；
- `sourcesContent`；
- 自定义 scheme；
- 空 mapping 与 unmapped segment；
- UTF-16 column 约定；
- 错误类型是否可包装为本 crate 的领域错误；
- 是否允许对外隐藏第三方 crate 类型。

第三方 Source Map 类型不能直接泄漏到公共 API。需要使用本 crate 的 `DecodedSourceMap` 或私有 adapter 包装。

解析规则：

- `version != 3`：`UnsupportedVersion`；
- JSON 非法：`InvalidDocument`；
- 必填字段类型错误：`InvalidDocument`；
- `sourcesContent` 缺失或单项为 `null`：允许继续；
- mapping 不覆盖某个 generated position：`UnmappedPosition`；
- 虚拟 source 无法转成本地文件：仍可返回 source 身份和 original position。

## 10. Original source 获取策略

优先级：

```text
1. 使用 sourcesContent
2. sourcesContent 对应项缺失或为 null
3. 根据 sourceRoot + sources 解析 source 身份
4. 通过受控 OriginalSourceLoader 尝试读取
5. 仍不可用时保留 source 身份和行列，不提供源码片段
```

`webpack://`、`vite://` 等虚拟 URL：

- 有 `sourcesContent`：直接使用内嵌源码；
- 无 `sourcesContent`：默认不尝试当作文件系统路径；
- 后续可以通过 bundler-specific adapter 处理。

## 11. 错误与降级模型

Source Map 不可用不是 detector 的致命错误，需要使用领域状态表示：

```rust
pub enum SourceMapUnavailable {
  NotFound {
    fallback_path: PathBuf,
  },
  ExplicitReferenceUnavailable {
    reference: String,
    message: String,
  },
  AmbiguousReference {
    references: Vec<String>,
  },
  InvalidDocument {
    location: String,
    message: String,
  },
  UnsupportedVersion {
    version: String,
  },
  UnmappedPosition,
  OriginalSourceUnavailable {
    source: String,
  },
}
```

严重程度建议：

| 情况 | 行为 | 建议严重程度 |
| --- | --- | --- |
| 没有显式 map 且同名 map 不存在 | 使用产物位置 | 信息或静默 |
| 显式 map 无法读取 | 使用产物位置 | 警告 |
| map JSON 损坏 | 使用产物位置 | 警告 |
| map 版本不支持 | 使用产物位置 | 警告 |
| generated position 无 mapping | 使用产物位置 | 信息 |
| original source 文本缺失 | 保留 original URL 和行列 | 信息 |

真正应该终止当前文件检测的错误仍然是：

- 产物文件无法读取；
- 产物编码无法处理；
- OXC 无法解析产物且当前策略不允许恢复。

## 12. 模块规划

建议目录：

```text
src/
├── detector/
│   ├── mod.rs
│   ├── javascript.rs
│   └── result.rs
├── source_map/
│   ├── mod.rs
│   ├── reference.rs
│   ├── resolver.rs
│   ├── loader.rs
│   ├── decoder.rs
│   ├── position.rs
│   ├── mapper.rs
│   ├── source.rs
│   └── error.rs
├── feature.rs
└── lib.rs
```

职责：

| 文件 | 职责 |
| --- | --- |
| `reference.rs` | `sourceMappingURL` 和引用类型建模 |
| `resolver.rs` | 显式引用、同名回退和优先级编排 |
| `loader.rs` | 文件、data URI、未来 HTTP 加载边界 |
| `decoder.rs` | 第三方 Source Map crate adapter |
| `position.rs` | UTF-8 byte offset 与 UTF-16 行列转换 |
| `mapper.rs` | 将 `FeatureUsage` 映射为 original location |
| `source.rs` | `SourceIdentity`、`OriginalLocation` 与 source content |
| `error.rs` | Source Map 加载、解析、映射错误 |

## 13. 分阶段实施步骤

### 阶段 0：固定领域模型

修改：

- `DetectionResult.path` → `generated_path`；
- `FeatureUsage.span` → `generated_span`；
- 新增 `SourceMapping`；
- 新增 `SourceIdentity`；
- 新增 `SourcePosition`；
- 新增 `OriginalLocation`；
- 新增 `SourceMapUnavailable`。

测试：

- detector 新产生的 usage 状态为 `NotResolved`；
- generated span 与现有检测结果完全一致；
- 不允许使用空字符串伪造 original source；
- 映射成功和失败状态可明确区分。

完成标准：

- 不接入真实 Source Map，也能通过全部现有测试；
- 新模型不丢失产物路径或 generated span。

### 阶段 1：显式引用与同名回退

新增：

- `SourceMapReference`；
- `SourceMapOrigin`；
- `SourceMapResolver`；
- `FileSourceMapLoader`；
- `DataUriSourceMapLoader`。

测试：

- `//# sourceMappingURL=`；
- `//@ sourceMappingURL=`；
- 相对路径；
- 绝对路径；
- `file://`；
- base64 data URI；
- percent-encoded data URI；
- 没有显式引用时找到 `.js.map`；
- 两种方式都没有时返回 `NotFound`；
- 显式引用失败时默认不尝试同名回退；
- 多个相同引用被接受；
- 多个冲突引用返回 `AmbiguousReference`；
- 字符串和模板字符串中的伪指令不被识别。

完成标准：

- 能正确定位当前真实素材中的同名 `.map`；
- resolver 不依赖进程当前工作目录；
- resolver 不直接包含 Source Map 解码逻辑。

### 阶段 2：Source Map 解码

任务：

- 评估并选择 Rust Source Map crate；
- 使用 adapter 隔离第三方类型；
- 解码普通 v3 map；
- 读取 `sourceRoot`、`sources`、`sourcesContent` 和 mappings；
- 保留虚拟 URL；
- 提供 generated position lookup。

测试：

- 合法最小 map；
- 非法 JSON；
- 非对象 JSON；
- `version != 3`；
- 缺失 `sourcesContent`；
- `sourcesContent` 中存在 `null`；
- `webpack://` source；
- unmapped segment；
- 空 mappings。

完成标准：

- 能解码当前真实 `.map`；
- 能确认 76 个 sources 和 sourcesContent；
- 第三方 crate 类型不出现在 crate 公共 API。

### 阶段 3：位置转换与映射

新增：

- `GeneratedPositionIndex`；
- byte offset → zero-based line；
- byte offset → UTF-16 column；
- `SourceMapMapper`；
- `OriginalLocation` 生成逻辑。

测试矩阵：

| 场景 | 必须验证 |
| --- | --- |
| ASCII | byte column 与 UTF-16 column 相同 |
| 中文 | 3-byte UTF-8 对应 1 UTF-16 unit |
| emoji | 4-byte UTF-8 对应 2 UTF-16 units |
| 混合文本 | 多种字符之前和之后的位置正确 |
| `\n` | 行号和列号正确 |
| `\r\n` | 不标准化文本且位置正确 |
| 超长单行 | 性能与列号正确 |
| span 起终点同 source | 形成 original range |
| span 起终点不同 source | 只保留可靠起点 |
| 起点 unmapped | 降级为 generated-only |

完成标准：

- 中英文和 emoji fixture 映射位置准确；
- 每个文件只构建一次行起始索引；
- 每条 usage 不从文件开头重复扫描；
- 映射失败不影响兼容性 finding。

### 阶段 4：接入检查主流程

编排顺序：

```text
读取产物 SourceFile
  ↓
FeatureDetector::detect
  ↓
SourceMapResolver::resolve
  ↓
SourceMapMapper::map_detection
  ↓
TargetResolver
  ↓
CompatDatabase + checker
  ↓
最终诊断
```

要求：

- 一个产物只解析一次 Source Map；
- 多条 usage 共享解码结果和位置索引；
- Source Map 不存在时不反复探测同一路径；
- 最终报告同时展示 generated location 和可用的 original location；
- Source Map 错误不会吞掉 compatibility finding。

### 阶段 5：Index Map 与生态兼容

任务：

- 支持 `sections`；
- 支持 section offset；
- 支持 `ignoreList`；
- 兼容 `x_google_ignoreList`；
- 评估链式 Source Map；
- 评估远程 loader 和 HTTP `SourceMap` header。

这部分不得阻塞第一版本地 JavaScript 产物定位能力发布。

## 14. Fixtures 与测试文件规划

建议创建：

```text
crates/ecmascript_compatibility/tests/fixtures/source_map/
├── adjacent/
│   ├── main.js
│   └── main.js.map
├── explicit/
│   ├── main.js
│   └── maps/main.js.map
├── inline/
│   └── main.js
├── missing/
│   └── main.js
├── invalid/
│   ├── main.js
│   └── main.js.map
├── unicode/
│   ├── main.js
│   └── main.js.map
└── webpack-virtual/
    ├── main.js
    └── main.js.map
```

fixtures 必须满足：

- 文件足够小，方便人工检查 mappings；
- source 文本和期望位置写在测试中；
- 不依赖机器绝对路径；
- 至少包含中文和 emoji；
- 至少包含一个 `webpack://` source；
- 至少包含一个 `sourcesContent: null`；
- 至少包含一个 generated-only segment。

## 15. 性能要求

- 产物文本只读取一次；
- 产物 AST 只解析一次；
- Source Map 只读取和解码一次；
- 行索引只构建一次；
- 同一 Source Map 不为每条 usage 重复克隆；
- `sourcesContent` 不复制到每条 `FeatureUsage`；
- 查找算法应使用 Source Map crate 提供的索引；
- 为文件大小、map 大小和 data URI 大小提供上限；
- 后续处理多文件时，缓存 key 至少包含规范化 map 身份和文件元数据。

## 16. 第一版验收标准

第一版完成时必须满足：

1. 产物内有显式相对 `sourceMappingURL` 时能找到正确 map；
2. 产物内有 inline data URI 时能解码；
3. 没有显式引用时能找到同名 `.js.map`；
4. 显式引用失效时默认不错误回退；
5. 两种查找方式都不可用时仍返回兼容性 finding；
6. finding 保留 generated path 和 generated span；
7. 映射成功时每条 usage 带自己的 original source；
8. 一个产物中的 usages 可以映射到多个 original sources；
9. `webpack://` source 在有 `sourcesContent` 时可展示源码；
10. UTF-8 byte offset 到 UTF-16 column 转换经过中文和 emoji 测试；
11. Source Map 没有可靠终点时不伪造 original range；
12. map JSON 损坏、版本不支持、位置未映射都有不同状态；
13. 全部 Source Map 失败都不会删除 detector finding；
14. 自动化测试不依赖用户机器上的绝对路径；
15. 当前真实素材可以通过同名回退找到并成功解码。

## 17. 已确定的默认决策

| 决策 | 默认值 |
| --- | --- |
| Source Map 版本 | 只接受 v3 |
| 显式引用优先级 | 高于同名回退 |
| 显式引用失效后是否回退 | 否 |
| 无 Source Map 是否中止检测 | 否 |
| 是否保留 generated location | 始终保留 |
| original source 是否固定为 PathBuf | 否 |
| 是否默认联网 | 否 |
| 是否支持 `//@` | 是，作为兼容输入 |
| 是否处理 CSS/Wasm | 第一版不处理 |
| 是否自行实现 VLQ | 否，优先成熟 crate |
| original range 是否强制存在 | 否，终点不可靠时只保留起点 |

## 18. 实施顺序总结

严格按以下顺序推进：

```text
1. 领域模型
2. Source Map 引用发现
3. Source Map 加载
4. Source Map 解码
5. UTF-8 → UTF-16 位置转换
6. generated → original 映射
7. 主流程编排
8. Index Map 和远程能力
```

不要先把所有逻辑塞进 `FeatureDetector::detect`，也不要先修改最终 `compatibility()` 再补领域模型。每个阶段必须先有独立测试，再接入下一层。

## 19. 规范参考

- [ECMA-426 Source Map Format Specification](https://tc39.es/ecma426/)
- [ECMA-426：Source Map Format](https://tc39.es/ecma426/#sec-source-map-format)
- [ECMA-426：Resolving Sources](https://tc39.es/ecma426/#sec-resolving-sources)
- [ECMA-426：Index Source Map](https://tc39.es/ecma426/#sec-index-source-map)
- [ECMA-426：Retrieving Source Maps](https://tc39.es/ecma426/#sec-retrieving-source-maps)
- [ECMA-426：Operations on Source Map Records](https://tc39.es/ecma426/#sec-operations-on-source-map-records)

实现时以 ECMA-426 当前正式语义为基线，生态兼容行为必须单独标记，不能与标准要求混为一谈。
