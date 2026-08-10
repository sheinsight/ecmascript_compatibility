# Source Map Crate Research - Documentation Index

**Research Focus**: Evaluating Rust source map crates for ecma_compat Phase 1-3 implementation

**Research Date**: 2026-08-06

**Status**: Complete with recommendation

---

## Documents in This Research

### 1. Executive Summary (Quick Reference)
**File**: [`sourcemap_crate_evaluation_summary.md`](./sourcemap_crate_evaluation_summary.md)

**For**: Project managers, architects, decision makers

**Contains**:
- ✅ Recommendation: oxc_sourcemap v8.1.2
- ✅ Feature matrix for Phase 1-5
- ✅ Risk assessment
- ✅ Implementation roadmap (4-5 weeks for Phase 1-3)
- ✅ Approval checkpoints
- **Read time**: 10 minutes

### 2. Detailed Technical Comparison
**File**: [`sourcemap_crate_comparison.md`](./sourcemap_crate_comparison.md)

**For**: Developers, architects, evaluation committee

**Contains**:
- ✅ 24 detailed comparison sections
- ✅ All design doc (source-map-implementation-plan.md) requirements mapped
- ✅ Code examples and API walkthroughs
- ✅ Performance characteristics
- ✅ Test coverage analysis
- ✅ Dependency analysis
- ✅ Key file references and import paths
- **Read time**: 45-60 minutes

### 3. Design Requirements Document (Referenced)
**File**: [`source-map-implementation-plan.md`](./source-map-implementation-plan.md)

**Status**: Original design document (pre-existing)

**Sections**:
- §1-3: Goals, test materials, scope
- §4: Architecture principles
- §5-7: Domain models and strategies
- §8-11: Implementation details
- §12-19: Module planning, phases, fixtures, performance
- **Referenced in**: Both new documents

---

## Key Findings Summary

### Recommendation

**Use oxc_sourcemap v8.1.2** as primary implementation for Phase 1-3

**Rationale**:
1. Performance: O(log n) lookup vs O(n) in sourcemap
2. Memory efficiency: Zero-copy Cow<'a, str> during parsing
3. Modern Rust: Edition 2024, MSRV 1.95.0
4. Design alignment: Explicit indexing matches §15 requirements
5. Lightweight: 7 dependencies vs 9 in sourcemap

### Critical Implementation Gaps

Both crates need wrapping; neither is standalone:

| Component | Phase | Effort | Solution |
|---|---|---|---|
| sourceMappingURL detection | 1 | 2-3 days | Port sourcemap's detector pattern |
| Data URI loading | 1 | 1-2 days | Simple base64 decoder wrapper |
| File reference resolution | 1 | 2-3 days | Path normalization + security checks |
| UTF-8 → UTF-16 conversion | 3 | 3-5 days | Build GeneratedPositionIndex wrapper |
| Line indexing | 3 | 1-2 days | Reuse oxc_sourcemap lookup table pattern |

### What Each Crate Does Well

**sourcemap v9.3.2 (Sentry)**:
- ✅ Complete ecosystem support (Index Maps, Hermes, RAM bundles)
- ✅ Reference detection for `sourceMappingURL` built-in
- ✅ Data URI handling (`decode_data_url`)
- ✅ Very stable API (9.x series stable for 1+ year)
- ✅ Extensive test fixtures and documentation
- ❌ O(n) linear lookup performance
- ❌ Owned Arc<str> for all strings

**oxc_sourcemap v8.1.2 (OXC Project)**:
- ✅ Binary search O(log n) lookup (with pre-built index)
- ✅ Zero-copy Cow<'a, str> parsing
- ✅ Modern Rust tooling (2024 edition)
- ✅ Lighter dependencies (7 vs 9)
- ✅ ConcatSourceMapBuilder for composition
- ✅ TC39 spec compliance tests
- ❌ No reference detection
- ❌ No Index Map support (Phase 5 OK)
- ❌ Rapidly evolving (breaking changes v8.0, v8.1)

---

## Design Document Alignment

### Phase 1: Reference Detection & Loading
| Requirement | Status | Notes |
|---|---|---|
| `//# sourceMappingURL=` | sourcemap ✅ | Must wrap oxc_sourcemap |
| Relative path resolution | sourcemap ✅ | File URL handling needed |
| Data URI inline maps | sourcemap ✅ | Decoding needed |
| Fallback to .js.map | Neither | Must implement |

**Recommendation for Phase 1**: Use oxc_sourcemap core + custom reference detector layer

### Phase 2: Source Map Decoding
| Requirement | Status | Notes |
|---|---|---|
| Parse v3 maps | Both ✅ | Native support |
| sourceRoot resolution | Both ✅ | Automatic |
| sourcesContent (including null) | Both ✅ | Option<T> handling |
| Virtual URLs (webpack://) | Both ✅ | Preserved exactly |
| Generated position lookup | Both ✅ | Different APIs |

**Recommendation for Phase 2**: Either crate works; oxc_sourcemap preferred for performance

### Phase 3: Position Mapping
| Requirement | Status | Notes |
|---|---|---|
| UTF-8 byte offset → line | Neither ❌ | Must implement |
| UTF-8 byte offset → UTF-16 column | Neither ❌ | Must implement |
| GeneratedPositionIndex | Neither ❌ | Can build on oxc pattern |

**Recommendation for Phase 3**: Build wrapper using oxc_sourcemap's lookup table approach

### Phase 4-5: Advanced Features
| Requirement | Status | Notes |
|---|---|---|
| Index Maps | sourcemap ✅ | Defer to Phase 5 |
| ignoreList field | Both ✅ | Not Phase 1-3 priority |
| Chain composition | oxc ✅ | ConcatSourceMapBuilder |
| Hermes/React Native | sourcemap ✅ | Not Phase 1-3 scope |

**Recommendation for Phase 4+**: Evaluate switching to sourcemap if Index Maps become urgent

---

## File Locations and References

### Repository Clones Used for Research
```
/tmp/rust-sourcemap/          # github.com/getsentry/rust-sourcemap
/tmp/oxc-sourcemap/           # github.com/oxc-project/oxc-sourcemap
```

### Key Source Files Analyzed

**sourcemap**:
- `/src/lib.rs` - Main API surface
- `/src/types.rs` - SourceMap, Token, RawToken structures
- `/src/decoder.rs` - JSON parsing and VLQ decoding
- `/src/detector.rs` - `sourceMappingURL` detection
- `/src/builder.rs` - SourceMapBuilder implementation
- `/src/errors.rs` - Error type definitions

**oxc_sourcemap**:
- `/src/lib.rs` - Main API surface
- `/src/sourcemap.rs` - SourceMap, Token structures
- `/src/decode.rs` - JSON parsing and VLQ decoding
- `/src/encode.rs` - Serialization
- `/src/sourcemap_builder.rs` - Zero-copy builder
- `/src/concat_sourcemap_builder.rs` - Composition support
- `/src/token.rs` - Token structure and SourceViewToken

### crates.io Links
- sourcemap: https://crates.io/crates/sourcemap
- oxc_sourcemap: https://crates.io/crates/oxc_sourcemap

### Documentation
- sourcemap docs: https://docs.rs/sourcemap/9.3.2
- oxc_sourcemap docs: https://docs.rs/oxc_sourcemap/8.1.2

---

## How to Use These Documents

### For Implementation Planning

1. **Read**: sourcemap_crate_evaluation_summary.md (10 min)
   - Get recommendation and rationale
   - Understand critical gaps

2. **Review**: Phase roadmap section
   - Understand 4-week implementation timeline
   - Identify approval checkpoints

3. **Deep Dive**: sourcemap_crate_comparison.md as needed
   - Specific comparison section for decisions
   - Code examples for prototyping

### For Technical Decision-Making

1. **Section Reference**:
   - Looking for error handling? → §6
   - Need UTF-16 details? → §9
   - Want API comparison? → §5
   - Need performance specs? → §16
   - Looking for builder model? → §17

2. **Design Alignment**:
   - Find specific requirement → §20 (Design Document Requirements Checklist)
   - Maps directly to design doc sections (§4-15)

### For Risk Assessment

1. **Read**: "Risk Assessment" in summary document
2. **Cross-reference**: §22 (Key File References) in detailed doc
3. **Verify**: Against real test materials (modb-front example in design doc)

---

## Next Steps

### Before Phase 1 Kickoff

- [ ] **Verify**: Parse real source map from modb-front with oxc_sourcemap
- [ ] **Benchmark**: Memory usage on large (350 KB) sourcesContent
- [ ] **Profile**: Lookup performance with 100k+ queries on same map
- [ ] **Test**: Zero-copy guarantees with Cow<'a, str> builder

### Phase 1 Tasks

- [ ] Implement SourceMapReference detection
- [ ] Create FileSourceMapLoader
- [ ] Create DataUriSourceMapLoader  
- [ ] Add structured error types matching §11

### Phase 2 Tasks

- [ ] Integrate oxc_sourcemap for v3 map parsing
- [ ] Verify sourceRoot resolution (webkit://, file://, relative paths)
- [ ] Test sourcesContent null handling

### Phase 3 Tasks

- [ ] Build GeneratedPositionIndex
- [ ] Implement UTF-8 byte offset → UTF-16 column conversion
- [ ] Test with multi-byte characters (Chinese, emoji)
- [ ] Verify against design doc fixtures (§14)

---

## Questions & Answers

### Q: Why oxc_sourcemap over sourcemap?

**A**: Performance and design alignment. Design doc (§15) requires single-pass line indexing and per-file caching. oxc_sourcemap's explicit `generate_lookup_table()` pattern enables exactly this strategy. Performance: O(log n) vs O(n) lookup. Memory: zero-copy on parse.

### Q: What if we need Index Maps in Phase 2?

**A**: Plan to switch to sourcemap. Keep this branch ready: 1-2 weeks added to Phase 2. Current design doc defers to Phase 5.

### Q: How much wrapping is required?

**A**: ~200-300 lines of wrapper code for Phase 1-3:
- SourceMapReference detector (~80 lines)
- Loaders (~50 lines each)  
- GeneratedPositionIndex (~100 lines)
- Error wrapping (~50 lines)

### Q: Can we use both crates?

**A**: Not recommended. Adds complexity. Use oxc_sourcemap, reference sourcemap's patterns where needed.

### Q: What about Unicode handling?

**A**: Both crates handle it correctly; UTF-16 column semantics are built-in per ECMA-426. Phase 3 wrapper adds UTF-8 byte offset conversion (required by design).

---

## Approval Sign-Off

**Recommendation**: Use oxc_sourcemap v8.1.2 as primary implementation

**Prepared by**: Research Agent

**Date**: 2026-08-06

**Status**: Ready for architecture review

**Next Review**: Before Phase 1 kickoff (verification with real materials)

---

## References

- [Design Document](./source-map-implementation-plan.md) - Full project requirements
- [Detailed Comparison](./sourcemap_crate_comparison.md) - 24-section technical analysis
- [Summary Document](./sourcemap_crate_evaluation_summary.md) - Executive overview
- ECMA-426: https://tc39.es/ecma426/
- RFC 3986 (URI Resolution): https://tools.ietf.org/html/rfc3986
- UTF-16 Code Units: ECMA-262 §6.1.4

---

**Document Version**: 1.0  
**Last Updated**: 2026-08-06  
**Status**: Complete and Ready for Review
