use crate::source_map::SourcePosition;

/// 把 detector 的 UTF-8 byte offset 转为 Source Map 使用的零基 UTF-16 行列。
///
/// OXC span 使用 byte offset；Source Map lookup 使用行号和 UTF-16 code unit 列号。
/// 这个索引把转换集中起来，避免调用方或分析流程里散落重复的偏移计算。
#[derive(Debug, Clone)]
pub(super) struct GeneratedSourceIndex {
  line_starts: Vec<usize>,
  source_text: String,
}

impl GeneratedSourceIndex {
  pub(super) fn new(source_text: &str) -> Self {
    let mut line_starts = vec![0];

    for (index, byte) in source_text.bytes().enumerate() {
      if byte == b'\n' {
        line_starts.push(index + 1);
      }
    }

    Self {
      line_starts,
      source_text: source_text.to_string(),
    }
  }

  pub(super) fn position_for_offset(&self, offset: u32) -> SourcePosition {
    let offset = offset as usize;
    let line = match self.line_starts.binary_search(&offset) {
      Ok(line) => line,
      Err(next_line) => next_line.saturating_sub(1),
    };
    let line_start = self.line_starts[line];
    let line_text =
      &self.source_text[line_start..offset.min(self.source_text.len())];
    let col = line_text.encode_utf16().count() as u32;

    SourcePosition::new(line as u32, col)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn converts_byte_offsets_to_zero_based_positions() {
    let index = GeneratedSourceIndex::new("a\nbc?.d");

    assert_eq!(index.position_for_offset(3), SourcePosition::new(1, 1));
  }

  #[test]
  fn uses_utf16_columns_for_non_ascii_text() {
    let index = GeneratedSourceIndex::new("😀?.name");

    assert_eq!(
      index.position_for_offset("😀".len() as u32),
      SourcePosition::new(0, 2)
    );
  }
}
