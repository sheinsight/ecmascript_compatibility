use std::{collections::HashMap, path::PathBuf};

use crate::{
  SourceFile, SourceKind, SyntaxFeatureDetector, SyntaxFeatureId,
  source_map::{
    SourceIdentity, SourceLocation, SourceMapDocument, SourcePosition,
  },
};

use super::GeneratedSourceIndex;

/// 从 original source 文本中恢复 diagnostic 的完整源码范围。
///
/// 普通 Source Map 只能可靠提供点位。这个 recoverer 把“范围恢复”单独建模为
/// source-map 之后的增强阶段：按 original source 去重解析，再用 mapped start
/// 和 feature 回到 original AST span。
pub(super) struct OriginalRangeRecoverer<'a> {
  detector: &'a SyntaxFeatureDetector,
  document: &'a SourceMapDocument,
  cache: HashMap<SourceIdentity, Option<OriginalSourceRangeIndex>>,
}

impl<'a> OriginalRangeRecoverer<'a> {
  pub(super) fn new(
    detector: &'a SyntaxFeatureDetector,
    document: &'a SourceMapDocument,
  ) -> Self {
    Self {
      detector,
      document,
      cache: HashMap::new(),
    }
  }

  pub(super) fn recover_end(
    &mut self,
    feature: SyntaxFeatureId,
    location: &SourceLocation,
  ) -> Option<SourcePosition> {
    if location.end().is_some() {
      return location.end();
    }

    let source = location.source().clone();
    let index = self.source_index(&source)?;

    index.find_end(feature, location.start())
  }

  fn source_index(
    &mut self,
    source: &SourceIdentity,
  ) -> Option<&OriginalSourceRangeIndex> {
    if !self.cache.contains_key(source) {
      let index = self.build_source_index(source);
      self.cache.insert(source.clone(), index);
    }

    self.cache.get(source).and_then(Option::as_ref)
  }

  fn build_source_index(
    &self,
    source: &SourceIdentity,
  ) -> Option<OriginalSourceRangeIndex> {
    let source_text = self.document.source_contents(source)?;
    let source_file = SourceFile::new(
      source_path(source)?,
      source_kind(source)?,
      source_text.to_string(),
    );
    let detection = self.detector.detect(&source_file).ok()?;
    let source_index = GeneratedSourceIndex::new(source_text);
    let offsets = detection
      .usages()
      .iter()
      .flat_map(|usage| [usage.span().start(), usage.span().end()])
      .collect::<Vec<_>>();
    let positions = source_index.positions_for_offsets(&offsets);
    let usages = detection
      .usages()
      .iter()
      .zip(positions.chunks_exact(2))
      .map(|(usage, positions)| OriginalSourceUsage {
        feature: usage.feature(),
        start: positions[0],
        end: positions[1],
      })
      .collect();

    Some(OriginalSourceRangeIndex { usages })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalSourceRangeIndex {
  usages: Vec<OriginalSourceUsage>,
}

impl OriginalSourceRangeIndex {
  fn find_end(
    &self,
    feature: SyntaxFeatureId,
    mapped_start: SourcePosition,
  ) -> Option<SourcePosition> {
    self
      .usages
      .iter()
      .find(|usage| usage.feature == feature && usage.start == mapped_start)
      .or_else(|| {
        self.usages.iter().find(|usage| {
          usage.feature == feature && usage.contains(mapped_start)
        })
      })
      .map(|usage| usage.end)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OriginalSourceUsage {
  feature: SyntaxFeatureId,
  start: SourcePosition,
  end: SourcePosition,
}

impl OriginalSourceUsage {
  fn contains(self, position: SourcePosition) -> bool {
    self.start <= position && position <= self.end
  }
}

fn source_path(source: &SourceIdentity) -> Option<PathBuf> {
  match source {
    SourceIdentity::File(path) => Some(path.clone()),
    SourceIdentity::Url(url) | SourceIdentity::Virtual(url) => {
      Some(PathBuf::from(url))
    }
    SourceIdentity::Unknown => None,
  }
}

fn source_kind(source: &SourceIdentity) -> Option<SourceKind> {
  SourceKind::from_path(&source_path(source)?).ok()
}
