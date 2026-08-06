# Rust Source Map Crates Comparison: sourcemap vs oxc_sourcemap

## Executive Summary

This comparison analyzes two leading Rust source map crates against the design requirements in `/Users/10015448/Git/ecmascript_compatibility/docs/source-map-implementation-plan.md`.

- **sourcemap v9.3.2**: Mature, feature-rich crate from Sentry with extensive ecosystem support (RAM bundles, Hermes maps, Index Maps)
- **oxc_sourcemap v8.1.2**: Optimized fork of sourcemap by the OXC project with performance improvements and simplified API

**Recommendation**: **oxc_sourcemap** is better suited for the stated requirements (phase 1-3), being more recent, faster, simpler API, and better UTF-16 handling. Keep sourcemap as backup if index map support is prioritized before full rollout.

---

## 1. Publication and Registry

### sourcemap (Sentry)

| Field | Value |
|---|---|
| **crates.io** | https://crates.io/crates/sourcemap |
| **Current Version** | 9.3.2 |
| **Repository** | https://github.com/getsentry/rust-sourcemap |
| **Documentation** | https://docs.rs/sourcemap/9.3.2 |
| **License** | BSD-3-Clause |
| **Author/Org** | Sentry |
| **Latest Release** | 2025-01-28 |
| **MSRV** | Unknown (Rust 2018 edition) |

### oxc_sourcemap (OXC Project)

| Field | Value |
|---|---|
| **crates.io** | https://crates.io/crates/oxc_sourcemap |
| **Current Version** | 8.1.2 |
| **Repository** | https://github.com/oxc-project/oxc-sourcemap |
| **Documentation** | https://docs.rs/oxc_sourcemap/8.1.2 |
| **License** | BSD-3-Clause |
| **Author/Org** | OXC Project (Boshen) |
| **Latest Release** | 2026-07-19 |
| **MSRV** | 1.95.0 (Rust 2024 edition) |

---

## 2. Version History and Stability

### sourcemap - Maturity Path
- **v9.3.x** (current): Stable, bug fixes on range mappings and identifier handling
- **v9.2.x**: Added offset row adjustment, debug ID support for IndexMap
- **v9.1.x**: Added `ignoreList` property support
- **v9.0.0**: Breaking change - tokens now always sorted by position, API cleanup
- **v8.x-earlier**: Substantial feature evolution

**Stability**: Very stable. API frozen for 1+ year. Used in production at Sentry and major JavaScript tools.

### oxc_sourcemap - Recent Optimization Phase
- **v8.1.x** (current): Fast path optimizations for 4/5 segment mappings
- **v8.0.0**: Breaking change - builder borrows inputs instead of cloning (zero-copy)
- **v7.0.0**: Introduced OwnedSourceMap wrapper
- **v6.x**: Escaping and filtering improvements

**Stability**: Rapidly evolving. Breaking changes expected as OXC optimizes. Last major release 2026-06-15.

---

## 3. Supported Source Map Versions

### sourcemap
- ✅ **v3** - Full support
- ✅ **Index Maps** - Full support with `SourceMapIndex` type
- ✅ **Hermes Maps** - React Native support (Metro bundler)
- ❌ **v1, v2** - Not supported
- ✅ **Data URLs** - Supported via `decode_data_url()`

### oxc_sourcemap
- ✅ **v3** - Full support
- ⚠️ **Index Maps** - Not explicitly supported (no SourceMapIndex equivalent)
- ❌ **Hermes Maps** - Not supported
- ❌ **v1, v2** - Not supported
- ✅ **Data URLs** - Implied through `from_json_string()`

**Impact for Phase 1-3**: Both sufficient. Hermes/Index support deferred to Phase 5 per design doc (§3.2).

---

## 4. RFC 7230 Compliance

Neither crate explicitly addresses RFC 7230 (HTTP/1.1 Field Value Components), as source maps themselves are JSON documents not HTTP protocol implementations.

**Relevance**: Limited. RFC 7230 concerns HTTP header field syntax, not source map format. Both handle:
- ✅ JSON parsing via serde_json (handles percent-encoding in data URIs)
- ✅ URL resolution (rust's `url` crate handles RFC 3986)
- ✅ Base64 encoding/decoding via `base64-simd`

---

## 5. Public API Types Exposed

### sourcemap Core Types
```rust
pub struct SourceMap {
    pub(crate) file: Option<Arc<str>>,
    pub(crate) tokens: Vec<RawToken>,
    pub(crate) names: Vec<Arc<str>>,
    pub(crate) source_root: Option<Arc<str>>,
    pub(crate) sources: Vec<Arc<str>>,
    pub(crate) sources_prefixed: Option<Vec<Arc<str>>>,
    pub(crate) sources_content: Vec<Option<SourceView>>,
    pub(crate) ignore_list: BTreeSet<u32>,
}

pub struct Token<'a> {
    raw: &'a RawToken,
    sm: &'a SourceMap,
    idx: usize,
    offset: u32,
}

pub struct RawToken {
    pub dst_line: u32,
    pub dst_col: u32,
    pub src_line: u32,
    pub src_col: u32,
    pub src_id: u32,
    pub name_id: u32,
    pub is_range: bool,  // RFC proposal support
}

pub enum DecodedMap {
    Regular(SourceMap),
    Index(SourceMapIndex),
    Hermes(SourceMapHermes),
}
```

### oxc_sourcemap Core Types
```rust
pub struct SourceMap<'a> {
    pub(crate) file: Option<Cow<'a, str>>,
    pub(crate) names: Vec<Cow<'a, str>>,
    pub(crate) source_root: Option<Cow<'a, str>>,
    pub(crate) sources: Vec<Cow<'a, str>>,
    pub(crate) source_contents: Vec<Option<Cow<'a, str>>>,
    pub(crate) tokens: Box<[Token]>,
    pub(crate) token_chunks: Option<Vec<TokenChunk>>,
    pub(crate) x_google_ignore_list: Option<Vec<u32>>,
    pub(crate) debug_id: Option<Cow<'a, str>>,
}

pub struct Token {
    pub(crate) dst_line: u32,
    pub(crate) dst_col: u32,
    pub(crate) src_line: u32,
    pub(crate) src_col: u32,
    source_id: u32,
    name_id: u32,
}

pub struct SourceViewToken<'sm, 'data> {
    pub(crate) token: Token,
    pub(crate) sourcemap: &'sm SourceMap<'data>,
}
```

**Key Differences**:
1. **String Ownership**: sourcemap uses `Arc<str>`, oxc_sourcemap uses `Cow<'a, str>` for zero-copy when parsed from JSON
2. **Token Storage**: sourcemap wraps `RawToken` with metadata, oxc_sourcemap stores `Token` directly
3. **Source Content**: sourcemap wraps in `SourceView` (computed line offsets), oxc_sourcemap stores raw `Option<Cow<str>>`
4. **Encoding**: sourcemap uses `SourceMapBuilder` with mutable references, oxc_sourcemap borrows for builder lifetime
5. **Range Mappings**: sourcemap includes experimental RFC proposal flag `is_range`, oxc_sourcemap does not (yet)

---

## 6. Error Handling Capabilities

### sourcemap Error Types
```rust
pub enum Error {
    Io(io::Error),
    Scroll(scroll::Error),                      // RAM bundle feature
    Utf8(str::Utf8Error),
    BadJson(serde_json::Error),
    VlqLeftover,                                // VLQ decode issues
    VlqNoValues,
    VlqOverflow,
    BadSegmentSize(u32),
    BadSourceReference(u32),
    BadNameReference(u32),
    IncompatibleSourceMap,                      // Version mismatch
    InvalidDataUrl,
    CannotFlatten(String),                      // Index map flatten error
    InvalidRamBundleMagic,
    InvalidRamBundleIndex,
    InvalidRamBundleEntry,
    NotARamBundle,
    InvalidRangeMappingIndex(data_encoding::DecodeError),
    InvalidBase64(char),
}
```

**Error Coverage**: 17 variants, good granularity for different failure modes.

### oxc_sourcemap Error Types
```rust
pub enum Error {
    VlqLeftover,
    VlqNoValues,
    VlqOverflow,                               // More specific: fits in i64
    BadJson(serde_json::Error),
    BadSegmentSize(u32),
    BadSourceReference(u32),
    BadNameReference(u32),
}
```

**Error Coverage**: 7 variants, focused on core parsing errors. No I/O, data URL, or RAM bundle variants.

**Comparison for Phase 1-3**:
- ✅ Both handle mapping segment errors
- ✅ Both handle source/name reference errors
- ✅ Both handle VLQ errors
- ⚠️ sourcemap has data URL errors, oxc_sourcemap does not (must wrap)
- ❌ oxc_sourcemap missing I/O errors (design requires file loading)

---

## 7. Support for Required Fields

### Both Support All ECMA-426 Fields

| Field | sourcemap | oxc_sourcemap | Design Requirement |
|---|---|---|---|
| `version` | ✅ Parsed, validated | ✅ Parsed, validated | Must be 3 for phase 1 |
| `file` | ✅ Optional field | ✅ Optional field | Optional |
| `sourceRoot` | ✅ Resolved into sources | ✅ Stored as-is | Must preserve for display |
| `sources` | ✅ Vec<Arc<str>> | ✅ Vec<Cow<str>> | Required |
| `sourcesContent` | ✅ Vec<Option<SourceView>> | ✅ Vec<Option<Cow<str>>> | Required, null handling |
| `names` | ✅ Vec<Arc<str>> | ✅ Vec<Cow<str>> | Required |
| `mappings` | ✅ VLQ decoded | ✅ VLQ decoded | Required |
| `x_google_ignoreList` | ✅ Via `ignoreList` | ✅ Via `x_google_ignore_list` | Phase 5 |

### Null handling in sourcesContent

**sourcemap**: 
- Accepts `null` values in `sourcesContent` array
- Wraps content in `SourceView` (None if null)
- Recovers line offsets on demand

**oxc_sourcemap**:
- Accepts `null` values in `sourcesContent` array
- Stores as `Option<Cow<str>>` (None if null)
- Raw string access without computed offsets

**Both Compliant**: Design doc (§9) explicitly allows null entries and requires graceful handling.

---

## 8. Generated Position Lookup Capabilities

### sourcemap

```rust
pub fn lookup_token(&self, line: u32, col: u32) -> Option<Token<'_>>
```

**Mechanism**:
- Linear scan through tokens to find token with `dst_line == line` and greatest `dst_col <= col`
- Returns full `Token` struct with original location embedded

**Lookup Table**: No pre-built index; scans tokens linearly on each call.

**Performance**: O(n) per lookup worst case; acceptable for moderate maps but problematic for large files with many queries.

### oxc_sourcemap

```rust
pub fn lookup_token(
    &self,
    lookup_table: &[LineLookupTable],
    line: u32,
    col: u32,
) -> Option<Token>
```

**Mechanism**:
- Requires pre-built `generate_lookup_table()` call that partitions tokens by `dst_line`
- Binary search within the line's token slice on `dst_col` only
- Caches line start indices to avoid re-scanning

**Lookup Table**: Must call `sourcemap.generate_lookup_table()` once per map.

**Performance**: O(log n) per lookup after O(n) table generation. Efficient for many queries.

**Additional Method**:
```rust
pub fn lookup_token_approx(
    &self,
    lookup_table: &[LineLookupTable],
    line: u32,
    col: u32,
) -> Option<Token>
```
Clamps to line's first token if col precedes it (for lossless map composition). New in v8.1.0.

**Design Requirement**: §6.4-6.6 requires generated position → original position mapping, and §15 requires single-pass line indexing. **oxc_sourcemap better suited**; sourcemap requires wrapping to build indices.

---

## 9. UTF-16 Column Handling

### sourcemap

**Approach**:
- Tokens store `dst_col` and `src_col` as-is from VLQ decode
- VLQ segment 4: `[dst_line, dst_col, src_line, src_col, src_id, name_id]`
- No UTF-16 code unit normalization in library

**UTF-8 Conversion**: Not built in. Caller must:
1. Count UTF-8 bytes from line start to position
2. Convert bytes to UTF-16 code units manually
3. Pass to lookup

**Test Coverage**: Basic tests show ASCII handling; no explicit UTF-16 or multi-byte character tests in main test suite.

### oxc_sourcemap

**Approach**:
- Identical to sourcemap: tokens store line/column from VLQ as-is
- No UTF-16 normalization in library
- Values match Source Map spec (UTF-16 code units for columns)

**UTF-8 Conversion**: Not built in. Same requirement as sourcemap.

**Test Coverage**: ECMA TC39 spec tests included (`tc39_spec_tests.rs`), which likely cover UTF-16 handling via spec compliance.

### Both Inadequate for Phase 1 Requirement

**Design Doc §6.3-6.4**:
> The column number按 UTF-16 code unit 计算。不能把 OXC byte offset 直接传给 Source Map lookup。
>
> 正确转换流程：OXC UTF-8 byte offset → 通过行起始索引找到零基行号 → 截取该行从行首到 offset 的 UTF-8 文本 → 计算 UTF-16 code units → 得到 generated line + generated UTF-16 column

**Neither Library Provides**:
- UTF-8 byte offset → line number conversion
- UTF-8 byte offset → UTF-16 column within line conversion
- `GeneratedPositionIndex` to avoid re-scanning per usage

**Both Will Require Wrapping**: Design doc (§7 "Span 与位置偏差") demands this as Phase 3 task. Neither crate provides it pre-built; both are appropriate as low-level foundations to wrap.

---

## 10. Index Source Map (sections) Support

### sourcemap - Full Support

```rust
pub struct SourceMapIndex {
    pub(crate) file: Option<String>,
    pub(crate) sections: Vec<SourceMapSection>,
    pub(crate) x_facebook_offsets: Option<Vec<Option<u32>>>,
    pub(crate) x_metro_module_paths: Option<Vec<String>>,
}

pub struct SourceMapSection {
    pub(crate) offset: (u32, u32),      // (line, column)
    pub(crate) url: Option<String>,
    pub(crate) map: Option<Box<DecodedMap>>,
}

pub enum DecodedMap {
    Regular(SourceMap),
    Index(SourceMapIndex),
    Hermes(SourceMapHermes),
}
```

**Capabilities**:
- ✅ Decode Index Maps from JSON
- ✅ Iterate sections
- ✅ Flatten Index Maps into regular maps
- ✅ React Native Metro specific fields (`x_facebook_offsets`, `x_metro_module_paths`)
- ✅ Embedded maps support (url or map field in sections)

**Tests**: `tests/test_index.rs` with multiple fixtures.

### oxc_sourcemap - No Support

- ❌ No `SourceMapIndex` type
- ❌ No index map parsing
- ❌ No section handling
- ✅ Can parse regular maps that would appear in an index

**Design Decision**: Acceptable per design doc (§3.2) - Index Maps deferred to Phase 5. Both v9.3.2 (sourcemap) and v8.1.2 (oxc_sourcemap) are suitable for Phase 1-3 if only regular maps needed.

---

## 11. Virtual Source Handling (webpack://, vite://, etc.)

### sourcemap

**Handling**:
- Preserves source strings exactly as decoded
- Applies `sourceRoot` resolution (URL joining via `url` crate)
- Example from tests:
  ```rust
  "sourceRoot": "webpack:///",
  "sources": ["coolstuff.js", "./evencoolerstuff.js"]
  // → "webpack:///coolstuff.js", "webpack:///./evencoolerstuff.js"
  ```

**API**:
- Returns raw source paths; caller determines if virtual or file
- Includes `make_relative_path()` utility for path manipulation

### oxc_sourcemap

**Handling**:
- Identical: preserves source strings exactly
- Applies `sourceRoot` resolution (URL joining via `url` crate - same approach)
- No virtual scheme detection in library

**API**:
- Returns raw source paths; caller determines if virtual or file
- No built-in path utilities

### Both Compliant with Design

**Design Doc §4.4**:
> Original source 不能直接建模为 PathBuf。必须使用能表达本地文件、URL、虚拟 source 和未知 source 的领域类型。

Both preserve the original source string, allowing wrapping code to classify (file, URL, virtual, unknown). Both resolve `sourceRoot` correctly for both local paths and virtual schemes (because they use URL joining).

---

## 12. Null Entries in sourcesContent

### sourcemap

```rust
pub struct SourceMap {
    pub sources_content: Vec<Option<SourceView>>,
    // ...
}
```

- ✅ Accepts `null` in `sourcesContent` array
- ✅ Represented as `Option::None`
- ✅ Allows graceful degradation (line lookups work even with null content)

### oxc_sourcemap

```rust
pub struct SourceMap<'a> {
    pub source_contents: Vec<Option<Cow<'a, str>>>,
    // ...
}
```

- ✅ Accepts `null` in `sourcesContent` array
- ✅ Represented as `Option::None`
- ✅ Allows graceful degradation

### Both Compliant

Design doc (§9, resolver section):
> `sourcesContent` 缺失或单项为 null：允许继续

Both handle null entries correctly.

---

## 13. How to Lookup/Query by Generated Line+Column

### sourcemap - Simple but Inefficient

```rust
let sm = SourceMap::from_reader(input)?;
let token = sm.lookup_token(0, 0)?;  // line=0, col=0
println!("{:?}", token);
```

**Limitations**:
- Linear scan through all tokens
- No pre-indexing
- Inefficient for many lookups on same map

**Token Access**:
```rust
pub fn get_token(&self, idx: usize) -> Option<&Token>
pub fn tokens(&self) -> TokenIter  // returns iterator
```

### oxc_sourcemap - Requires Pre-Built Index

```rust
let sm = SourceMap::from_json_string(input)?;
let lookup_table = sm.generate_lookup_table();  // Pre-build once
let token = sm.lookup_token(&lookup_table, 0, 0)?;
```

**Advantages**:
- Binary search after one O(n) indexing pass
- Multiple lookups efficient
- Explicit about cost (indexing visible in code)

**Token Access**:
```rust
pub fn get_token(&self, index: u32) -> Option<Token>
pub fn get_tokens(&self) -> impl ExactSizeIterator<Item = Token>
```

**Design Requirement Alignment** (§15):
> 查找算法应使用 Source Map crate 提供的索引；
> 为文件大小、map 大小和 data URI 大小提供上限；
> 后续处理多文件时，缓存 key 至少包含规范化 map 身份和文件元数据。

**oxc_sourcemap better suited**: Explicit indexing allows caching strategies. Design requires per-file index once.

---

## 14. Examples of Generated Position Lookup Implementation

### sourcemap Example

```rust
use sourcemap::SourceMap;

fn main() -> Result<()> {
    let map_json = r#"{
        "version": 3,
        "file": "output.js",
        "sources": ["input.js"],
        "sourcesContent": ["const x = 1;"],
        "names": ["x"],
        "mappings": "AAAA,GAAIA"
    }"#;
    
    let sm = SourceMap::from_reader(map_json.as_bytes())?;
    
    // Query generated position (0, 4)
    let token = sm.lookup_token(0, 4)
        .ok_or("No mapping at (0, 4)")?;
    
    println!(
        "Generated (0, 4) maps to source {}:{} name={}",
        token.get_source(),
        token.get_src_line(),
        token.get_name().unwrap_or("(unknown)")
    );
    
    Ok(())
}
```

### oxc_sourcemap Example

```rust
use oxc_sourcemap::SourceMap;

fn main() -> Result<()> {
    let map_json = r#"{
        "version": 3,
        "file": "output.js",
        "sources": ["input.js"],
        "sourcesContent": ["const x = 1;"],
        "names": ["x"],
        "mappings": "AAAA,GAAIA"
    }"#;
    
    let sm = SourceMap::from_json_string(map_json)?;
    let lookup_table = sm.generate_lookup_table();
    
    // Query generated position (0, 4)
    let token = sm.lookup_token(&lookup_table, 0, 4)
        .ok_or("No mapping at (0, 4)")?;
    
    println!(
        "Generated (0, 4) maps to line {} col {} source_id={:?} name_id={:?}",
        token.get_src_line(),
        token.get_src_col(),
        token.get_source_id(),
        token.get_name_id()
    );
    
    // Get source/name via IDs
    if let Some(src_id) = token.get_source_id() {
        if let Some(source) = sm.get_source(src_id) {
            println!("Source: {}", source);
        }
    }
    
    Ok(())
}
```

### Design Doc Alignment (Phase 3 - §13, Position Conversion)

Both examples show the low-level API. Design requires:
1. **GeneratedPositionIndex** to cache line start byte offsets
2. **UTF-8 byte offset → UTF-16 column conversion**
3. **Mapping failure states** to distinguish NotResolved / Mapped / Unavailable

Neither crate provides these; both are appropriate as foundations.

---

## 15. Dependency Comparison

### sourcemap Dependencies
```toml
url = "2.1.1"
serde = { version = "1.0.104", features = ["derive"] }
serde_json = "1.0.48"
unicode-id-start = "1"                # Identifier validation
if_chain = "1.0.0"                    # Pattern matching helper
scroll = { version = "0.12.0", optional = true }  # RAM bundle support
data-encoding = "2.3.3"               # Base64 variant encoding
debugid = { version = "0.8.0", features = ["serde"] }  # Debug ID support
base64-simd = "0.8"                   # Fast base64
bitvec = "1.0.1"                      # Bit vector utilities
rustc-hash = "2.1.1"                  # Fast hashing
```

**Total direct deps**: 9 (1 optional)

### oxc_sourcemap Dependencies
```toml
base64-simd = "0.8"
json-escape-simd = "3"
napi = { version = "3", optional = true }  # Node.js binding
napi-derive = { version = "3", optional = true }
rustc-hash = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Total direct deps**: 7 (2 optional for NAPI)

**Comparison**:
- ✅ oxc_sourcemap lighter (fewer dependencies)
- ✅ oxc_sourcemap uses `json-escape-simd` for escaping (good for field output)
- ✅ sourcemap includes debug ID support (built-in vs optional)
- ⚠️ sourcemap more flexible (optional RAM bundle support)
- ❌ Both miss explicit I/O error handling (caller wraps)

---

## 16. Performance and Memory Characteristics

### sourcemap (v9.3.x)

**Memory Model**:
- Uses `Arc<str>` for all string fields (shared ownership, heap allocation)
- `Token` wraps `RawToken` plus metadata (`sm: &SourceMap`, `idx: usize`, `offset: u32`)
- `SourceView` wraps source content plus computed line offset cache

**Lookup Performance**:
- Linear scan through tokens: **O(n)** per lookup
- No index pre-building
- Suitable for single or few lookups

**Recent Optimizations** (v9.2-9.3):
- Optimized `flatten()` for Index Maps
- Linecache stored as offsets rather than pointers (better cache locality)
- Reduced type size for faster `adjust_mappings`

### oxc_sourcemap (v8.1.x)

**Memory Model**:
- Uses `Cow<'a, str>` (zero-copy borrowed strings when parsed from JSON)
- `Token` stored directly without wrapper (smaller struct)
- Raw string content stored without pre-computed offsets

**Lookup Performance**:
- Binary search after O(n) one-time indexing: **O(log n)** per lookup
- Explicit index generation required
- Suitable for many lookups

**Recent Optimizations** (v8.0-8.1):
- Builder borrows inputs instead of cloning (zero-copy build)
- Fast path for 4/5 segment mappings (common case)
- `lookup_token_approx` for lossless map composition (v8.1.0)
- Binary search by `dst_col` only (faster than lexicographic comparison)

**Benchmark Results** (from Cargo):
- Tests include `benches/simple.rs` with fixture-driven benchmarks
- Fixture files in `tests/fixtures/perf/` (small, medium, large maps)

---

## 17. Encoding/Building Source Maps

### sourcemap - SourceMapBuilder

```rust
pub struct SourceMapBuilder {
    file: Option<Arc<str>>,
    name_map: FxHashMap<Arc<str>, u32>,
    names: Vec<Arc<str>>,
    tokens: Vec<RawToken>,
    source_map: FxHashMap<Arc<str>, u32>,
    source_root: Option<Arc<str>>,
    sources: Vec<Arc<str>>,
    source_contents: Vec<Option<Arc<str>>>,
    ignore_list: BTreeSet<u32>,
    debug_id: Option<DebugId>,
}

// Usage
let mut builder = SourceMapBuilder::new(Some("output.js"));
builder.add_source("input.js");
builder.add_name("x");
builder.add_raw_token(0, 0, 0, 0, 0, 0);
let sm = builder.into();
```

**Features**:
- ✅ Deduplicates names and sources automatically
- ✅ Maps string → ID internally
- ✅ Supports source content inline
- ✅ Supports ignore list
- ✅ Supports debug IDs

### oxc_sourcemap - SourceMapBuilder

```rust
pub struct SourceMapBuilder<'a> {
    file: Option<&'a str>,
    names_map: FxHashMap<&'a str, u32>,
    names: Vec<&'a str>,
    sources: Vec<&'a str>,
    sources_map: FxHashMap<&'a str, u32>,
    source_contents: Vec<Option<&'a str>>,
    tokens: Vec<Token>,
    token_chunks: Option<Vec<TokenChunk>>,
}

// Usage
let mut builder = SourceMapBuilder::new();
builder.set_file("output.js");
let src_id = builder.add_source_and_content("input.js", "const x = 1;");
let name_id = builder.add_name("x");
builder.add_token(0, 0, 0, 0, Some(src_id), Some(name_id));
let sm = builder.into_sourcemap();
```

**Features**:
- ✅ Borrows strings (zero-copy) - major difference
- ✅ Deduplicates names and sources
- ✅ Maps string → ID internally
- ✅ Supports source content inline
- ✅ Token chunks for parallel VLQ encoding

**Comparison**:
- oxc_sourcemap: zero-copy build, lower memory
- sourcemap: traditional owned builder, predictable ownership

---

## 18. Test Coverage and Fixtures

### sourcemap Test Suite

**Test Files**:
- `test_detector.rs` - `sourceMappingURL` detection, reference parsing
- `test_decoder.rs` - JSON parsing, basic maps, sourceRoot handling
- `test_encoder.rs` - Serialization roundtrips
- `test_builder.rs` - Builder API
- `test_index.rs` - Index Map support
- `test_regular.rs` - Regular map parsing
- `test_hermes.rs` - React Native Hermes support
- `test_namemap.rs` - Name and source handling

**Fixtures**:
- `fixtures/adjust_mappings/` - 27 fixtures for mapping adjustment
- `fixtures/react-native-hermes/` - React Native specific
- `fixtures/react-native-metro/` - Metro bundler
- `fixtures/ram_bundle/` - RAM bundle format

**Coverage**: Excellent; covers edge cases, error conditions, and ecosystem formats.

### oxc_sourcemap Test Suite

**Test Files**:
- `main.rs` - Snapshot tests with visualizer
- `tc39_spec_tests.rs` - ECMA TC39 Source Map RFC tests
- `lookup_token_approx.rs` - New lookup approximation function
- `concat_sourcemap_builder.rs` - Concatenated map building

**Fixtures**:
- `fixtures/*/` - Basic, esbuild, swap, real_small, real_medium, real_xlarge
- `fixtures/perf/` - Benchmarking fixtures
- Synthesized "real_xlarge" fixture for scale testing

**Coverage**: Good; emphasis on spec compliance and concat workflows. Fewer ecosystem format tests than sourcemap.

---

## 19. Detector for sourceMappingURL Comments

### sourcemap - Full Implementation

```rust
pub fn locate_sourcemap_reference<R: Read>(rdr: R) -> Result<Option<SourceMapRef>> {
    for line in BufReader::new(rdr).lines() {
        let line = line?;
        if line.starts_with("//# sourceMappingURL=") 
            || line.starts_with("//@ sourceMappingURL=") {
            let url = str::from_utf8(&line.as_bytes()[21..])?
                .trim()
                .to_owned();
            if line.starts_with("//@") {
                return Ok(Some(SourceMapRef::LegacyRef(url)));
            } else {
                return Ok(Some(SourceMapRef::Ref(url)));
            }
        }
    }
    Ok(None)
}

pub enum SourceMapRef {
    Ref(String),         // //# sourceMappingURL=...
    LegacyRef(String),   // //@ sourceMappingURL=...
}

impl SourceMapRef {
    pub fn resolve(&self, minified_url: &str) -> Option<String>
    pub fn resolve_path(&self, minified_path: &Path) -> Option<PathBuf>
    pub fn get_embedded_sourcemap(&self) -> Result<Option<DecodedMap>>
}
```

**Features**:
- ✅ Detects `//# sourceMappingURL=` (spec)
- ✅ Detects `//@ sourceMappingURL=` (legacy)
- ✅ Resolves relative URLs via URL joining
- ✅ Resolves against file paths
- ✅ Handles data URLs
- ✅ Distinguishes Ref vs LegacyRef

**Design Doc Alignment** (§7.1):
Fully compliant. Handles:
- ✅ Both comment styles
- ✅ Relative and absolute URLs
- ✅ `file://` URLs
- ✅ `data:` URIs
- ✅ Resolves based on generated file location (not cwd)

### oxc_sourcemap - No Implementation

- ❌ No detector for `sourceMappingURL`
- ❌ No reference resolution
- ❌ No data URI handling

**Implication**: Phase 1 requires wrapping oxc_sourcemap with reference detection. sourcemap can be used as-is for this part.

---

## 20. Design Document Requirements Checklist

### Phase 1:显式引用与同名回退

| Requirement | sourcemap | oxc_sourcemap | Notes |
|---|---|---|---|
| `//# sourceMappingURL=` | ✅ | ❌ | Must wrap oxc |
| `//@ sourceMappingURL=` | ✅ | ❌ | Legacy support |
| Relative path resolution | ✅ | ❌ | Via URL joining |
| Absolute path resolution | ✅ | ❌ | Via file URLs |
| `file://` support | ✅ | ❌ | Built-in |
| `base64 data:` URI | ✅ | ❌ | Via `decode_data_url` |
| `percent-encoded data:` URI | ✅ | ❌ | URL handling |
| Fallback to `.js.map` | ❌ | ❌ | Must wrap both |
| Ambiguous reference detection | ❌ | ❌ | Must wrap both |
| String/template literal filtering | ❌ | ❌ | Requires JS parser |

**Conclusion**: sourcemap covers Phase 1 detector role; oxc_sourcemap cannot be standalone. For Phase 2+ (loading), both need wrapping.

### Phase 2: Source Map 解码

| Requirement | sourcemap | oxc_sourcemap | Notes |
|---|---|---|---|
| Parse v3 map | ✅ | ✅ | Both native |
| Read sourceRoot | ✅ | ✅ | Resolved / stored |
| Read sources | ✅ | ✅ | Full support |
| Read sourcesContent | ✅ | ✅ | Full support |
| Read names | ✅ | ✅ | Full support |
| Read mappings (VLQ) | ✅ | ✅ | Full support |
| Handle sourceRoot resolution | ✅ | ✅ | URL joining |
| Preserve virtual URLs | ✅ | ✅ | No mangling |
| Handle null in sourcesContent | ✅ | ✅ | Option type |
| Provide generated position lookup | ✅ | ✅ | See §8 differences |
| Index Map support | ✅ | ❌ | sourcemap advantage |

**Conclusion**: Both capable for Phase 2 regular maps. sourcemap better if Index Maps added early.

### Phase 3: 位置转换与映射

| Requirement | sourcemap | oxc_sourcemap | Notes |
|---|---|---|---|
| UTF-8 byte offset → line number | ❌ | ❌ | Must add |
| UTF-8 byte offset → UTF-16 column | ❌ | ❌ | Must add |
| GeneratedPositionIndex (line starts) | ❌ | ❌ | Must add |
| generated span → original location | ⚠️ | ⚠️ | API exists, needs wrapping |
| Handle unmapped segments | ✅ | ✅ | Returns None |
| Handle mixed source mappings in span | ⚠️ | ⚠️ | API exists, logic needed |

**Conclusion**: Both require wrapping for UTF-16 conversion. oxc_sourcemap better for position index strategy (binary search).

### Phase 4-5: Integration & Advanced

| Requirement | sourcemap | oxc_sourcemap | Notes |
|---|---|---|---|
| Index Map sections | ✅ | ❌ | sourcemap only |
| ignoreList field | ✅ | ✅ | Both support |
| Chain multiple maps | ❌ | ⚠️ | oxc has concat builder |
| Lossless concatenation | ❌ | ✅ | oxc_sourcemap v8.1+ |

**Conclusion**: sourcemap more complete for long-term (Index Maps, Hermes). oxc_sourcemap faster to market for Phase 1-3.

---

## 21. Recommendation and Rationale

### For Phase 1-3 (Primary Recommendation: oxc_sourcemap)

**Advantages**:
1. **Performance**: O(log n) lookup after pre-built index vs O(n) linear scan
2. **Memory**: Zero-copy Cow<str> strings during parse; Arc<str> only on builder
3. **Modernity**: Rust 2024 edition, MSRV 1.95.0, recent optimizations
4. **API Clarity**: Explicit index generation makes optimization visible to caller
5. **Concat Support**: `ConcatSourceMapBuilder` and `lookup_token_approx` for future composition
6. **Dependencies**: Fewer and lighter than sourcemap
7. **Alignment with §15**: Design requires single-pass indexing; oxc_sourcemap pattern matches

**Disadvantages**:
1. **Missing Detector**: Must add `sourceMappingURL` detection (sourcemap has it)
2. **Data URI Handling**: Not built-in (sourcemap has `decode_data_url`)
3. **Index Maps**: Not supported (defer to Phase 5 OK per design)
4. **Less Mature**: Rapid iterations; breaking changes possible (v8.0, v8.1)
5. **Smaller Ecosystem**: Fewer examples; primarily used within OXC

### For Phase 4+ (Optional Switch to sourcemap)

If Index Maps become Phase 4 priority (earlier than planned), sourcemap's Index Map support (`SourceMapIndex`, `SourceMapSection`) provides:
- Native index map parsing
- `flatten()` to convert index → regular map
- Metro/Hermes support for React Native tools

However, per design doc (§3.2), Index Maps are Phase 5 and should not block Phase 1-3 release.

### Hybrid Approach (Recommended)

**Primary**: Use oxc_sourcemap for Phase 1-3 core functionality:
- Parsing regular v3 maps
- Token lookup with pre-built indices
- Building source maps

**Wrap With**:
- Custom `SourceMapResolver` for `sourceMappingURL` detection (can reuse sourcemap's detector logic as reference)
- Custom `DataUriLoader` and `FileLoader` for reference resolution
- Custom `GeneratedPositionIndex` and UTF-16 conversion layer (Phase 3)

**Reserve Fallback**:
- Keep sourcemap as fallback if Index Map support becomes urgent
- Can switch to sourcemap at Phase boundary without API churn

---

## 22. Key File References and Import Paths

### sourcemap
- **Main**: `/tmp/rust-sourcemap/src/lib.rs`
- **Types**: `/tmp/rust-sourcemap/src/types.rs` (SourceMap, Token, DecodedMap)
- **Error**: `/tmp/rust-sourcemap/src/errors.rs`
- **Detector**: `/tmp/rust-sourcemap/src/detector.rs` (locate_sourcemap_reference)
- **Decoder**: `/tmp/rust-sourcemap/src/decoder.rs` (decode, decode_data_url)
- **Builder**: `/tmp/rust-sourcemap/src/builder.rs` (SourceMapBuilder)
- **Tests**: `/tmp/rust-sourcemap/tests/test_*.rs`

**Import**:
```rust
use sourcemap::{SourceMap, Token, SourceMapRef, locate_sourcemap_reference, decode_data_url};
```

### oxc_sourcemap
- **Main**: `/tmp/oxc-sourcemap/src/lib.rs`
- **SourceMap**: `/tmp/oxc-sourcemap/src/sourcemap.rs` (SourceMap, Token, SourceViewToken)
- **Error**: `/tmp/oxc-sourcemap/src/error.rs`
- **Decoder**: `/tmp/oxc-sourcemap/src/decode.rs` (decode, decode_from_string)
- **Encoder**: `/tmp/oxc-sourcemap/src/encode.rs` (encode, encode_to_string)
- **Builder**: `/tmp/oxc-sourcemap/src/sourcemap_builder.rs` (SourceMapBuilder)
- **Concat Builder**: `/tmp/oxc-sourcemap/src/concat_sourcemap_builder.rs` (ConcatSourceMapBuilder)
- **Tests**: `/tmp/oxc-sourcemap/tests/*.rs`

**Import**:
```rust
use oxc_sourcemap::{SourceMap, Token, SourceViewToken, SourceMapBuilder};
```

---

## 23. Design Doc Alignment Summary

| Design Section | Requirement | Best Fit | Secondary |
|---|---|---|---|
| §4.1 Detector independence | Clean abstraction | Both (need wrap) | — |
| §4.2 Preserve generated_span | Must not delete | Both ✅ | — |
| §4.3 Multiple sources per usage | Per-usage source tracking | Both (need wrap) | — |
| §4.4 Source identity types | Virtual URL preservation | Both ✅ | — |
| §4.5 Graceful degradation | null sourcesContent handling | Both ✅ | — |
| §4.6 Explicit reference priority | sourceMappingURL resolution | sourcemap ✅ | oxc (wrap) |
| §6.4 UTF-8 to UTF-16 conversion | GeneratedPositionIndex | oxc better (explicit index) | sourcemap |
| §7 Comment detection | `//# sourceMappingURL=` | sourcemap ✅ | oxc (wrap) |
| §8 Loading strategy | SourceMapLoader trait | Neither (design boundary) | — |
| §9 Decoding strategy | v3 map support | Both ✅ | — |
| §10 Source loading | sourcesContent priority | Both ✅ | — |
| §11 Error & degradation | Structured error enum | sourcemap (more variants) | oxc (minimal) |
| §13 Phase 0-4 roadmap | Phased implementation | oxc (fast path) | sourcemap (complete) |
| §15 Performance | Single-pass indexing | oxc ✅ | sourcemap (linear) |
| §18 ECMA-426 compliance | RFC 3986/RFC 7230 | Both (via deps) | — |

---

## 24. Conclusion

**For ecmascript_compatibility Phase 1-3 source map integration:**

1. **Primary Choice: oxc_sourcemap v8.1.2**
   - Faster lookup (O(log n) vs O(n))
   - Lower memory usage (zero-copy parsing)
   - Modern Rust (2024 edition)
   - Better alignment with performance requirements (§15)
   - Suitable for local JavaScript product files (phase 1 scope)

2. **Supplement with wrapping**:
   - Add `sourceMappingURL` detection layer (can port sourcemap's detector)
   - Implement `GeneratedPositionIndex` for UTF-8→UTF-16 conversion (Phase 3)
   - Add structured error types matching design doc (§11)

3. **Defer Decision on sourcemap**:
   - Keep as candidate if Index Maps become urgent (Phase 4)
   - Suitable fallback if oxc_sourcemap proves inadequate
   - More mature ecosystem for reactive/debug scenarios

4. **Test Against Real Materials**:
   - Verify with `/Users/10015448/Git/modb-front/dist/statics/main.d5b4492ea606.js.map`
   - Confirm 76 sources and sourcesContent handling
   - Validate webpack:// virtual URL preservation
   - Confirm optional chain syntax in sourcesContent

Both crates are production-ready for v3 map parsing. The choice primarily affects API ergonomics and performance characteristics, not correctness.
