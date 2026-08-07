use crate::source_map::SourcePosition;

/// 把 detector 的 UTF-8 byte offset 转为 Source Map 使用的零基 UTF-16 行列。
///
/// OXC span 使用 byte offset；Source Map lookup 使用行号和 UTF-16 code unit 列号。
/// 这个索引把转换集中起来，避免调用方或分析流程里散落重复的偏移计算。
#[derive(Debug, Clone)]
pub(super) struct GeneratedSourceIndex<'source> {
  line_starts: Vec<usize>,
  line_is_ascii: Vec<bool>,
  source_text: &'source str,
}

impl<'source> GeneratedSourceIndex<'source> {
  pub(super) fn new(source_text: &'source str) -> Self {
    let mut line_starts = vec![0];
    let mut line_is_ascii = Vec::new();
    let mut current_line_is_ascii = true;

    for (index, byte) in source_text.bytes().enumerate() {
      current_line_is_ascii &= byte.is_ascii();

      if byte == b'\n' {
        line_starts.push(index + 1);
        line_is_ascii.push(current_line_is_ascii);
        current_line_is_ascii = true;
      }
    }

    line_is_ascii.push(current_line_is_ascii);

    Self {
      line_starts,
      line_is_ascii,
      source_text,
    }
  }

  #[cfg(test)]
  fn position_for_offset(&self, offset: u32) -> SourcePosition {
    let offset = offset as usize;
    let line = match self.line_starts.binary_search(&offset) {
      Ok(line) => line,
      Err(next_line) => next_line.saturating_sub(1),
    };
    let offset = offset.min(self.source_text.len());

    self.position_for_offset_in_line(offset, line)
  }

  pub(super) fn positions_for_offsets(
    &self,
    offsets: &[u32],
  ) -> Vec<SourcePosition> {
    let mut indexed_offsets = offsets
      .iter()
      .copied()
      .enumerate()
      .collect::<Vec<(usize, u32)>>();
    indexed_offsets.sort_unstable_by_key(|(_, offset)| *offset);

    let mut positions = vec![SourcePosition::new(0, 0); offsets.len()];
    let mut line = 0;

    for (index, offset) in indexed_offsets {
      let offset = (offset as usize).min(self.source_text.len());

      while line + 1 < self.line_starts.len()
        && self.line_starts[line + 1] <= offset
      {
        line += 1;
      }

      positions[index] = self.position_for_offset_in_line(offset, line);
    }

    positions
  }

  fn position_for_offset_in_line(
    &self,
    offset: usize,
    line: usize,
  ) -> SourcePosition {
    let line_start = self.line_starts[line];
    let col = if self.line_is_ascii[line] {
      (offset - line_start) as u32
    } else {
      self.source_text[line_start..offset].encode_utf16().count() as u32
    };

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

  #[test]
  fn uses_fast_byte_columns_for_ascii_lines_after_non_ascii_lines() {
    let index = GeneratedSourceIndex::new("😀\nabc?.d");

    assert_eq!(index.position_for_offset(6), SourcePosition::new(1, 1));
  }

  #[test]
  fn converts_offsets_in_batch_while_preserving_input_order() {
    let index = GeneratedSourceIndex::new("a\nbc?.d\n😀?.name");

    assert_eq!(
      index.positions_for_offsets(&[8, 3, 0, "a\nbc?.d\n😀".len() as u32]),
      vec![
        SourcePosition::new(2, 0),
        SourcePosition::new(1, 1),
        SourcePosition::new(0, 0),
        SourcePosition::new(2, 2),
      ],
    );
  }
}
