use crate::source_map::SourcePosition;

/// 把 detector 的 UTF-8 byte offset 转为 Source Map 使用的零基 UTF-16 行列。
///
/// OXC span 使用 byte offset；Source Map lookup 使用行号和 UTF-16 code unit 列号。
/// 这个索引把转换集中起来，避免调用方或分析流程里散落重复的偏移计算。
#[derive(Debug, Clone)]
pub(super) struct GeneratedSourceIndex<'source> {
  lines: Vec<LineColumnIndex>,
  source_text: &'source str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LineColumnIndex {
  start: usize,
  corrections: Vec<ColumnCorrection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColumnCorrection {
  byte_offset: usize,
  utf16_adjustment: i32,
}

impl<'source> GeneratedSourceIndex<'source> {
  pub(super) fn new(source_text: &'source str) -> Self {
    let mut lines = vec![LineColumnIndex::new(0)];
    let mut utf16_adjustment = 0;

    for (index, character) in source_text.char_indices() {
      if character == '\n' {
        lines.push(LineColumnIndex::new(index + character.len_utf8()));
        utf16_adjustment = 0;
        continue;
      }

      let adjustment_delta =
        character.len_utf16() as i32 - character.len_utf8() as i32;

      if adjustment_delta != 0 {
        utf16_adjustment += adjustment_delta;
        lines
          .last_mut()
          .expect("there is always at least one line")
          .corrections
          .push(ColumnCorrection {
            byte_offset: index + character.len_utf8(),
            utf16_adjustment,
          });
      }
    }

    Self { lines, source_text }
  }

  #[cfg(test)]
  fn position_for_offset(&self, offset: u32) -> SourcePosition {
    let offset = offset as usize;
    let line = match self.lines.binary_search_by_key(&offset, |line| line.start)
    {
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

      while line + 1 < self.lines.len() && self.lines[line + 1].start <= offset
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
    let line_index = &self.lines[line];
    let byte_column = offset - line_index.start;
    let utf16_adjustment = line_index.utf16_adjustment_for_offset(offset);
    let col = (byte_column as i32 + utf16_adjustment) as u32;

    SourcePosition::new(line as u32, col)
  }
}

impl LineColumnIndex {
  fn new(start: usize) -> Self {
    Self {
      start,
      corrections: Vec::new(),
    }
  }

  fn utf16_adjustment_for_offset(&self, offset: usize) -> i32 {
    let correction_index = match self
      .corrections
      .binary_search_by_key(&offset, |correction| correction.byte_offset)
    {
      Ok(index) => Some(index),
      Err(0) => None,
      Err(next_index) => Some(next_index - 1),
    };

    correction_index
      .map(|index| self.corrections[index].utf16_adjustment)
      .unwrap_or(0)
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
  fn uses_sparse_utf16_corrections_for_non_ascii_lines() {
    let index = GeneratedSourceIndex::new("abc中def😀ghi");

    assert_eq!(index.position_for_offset(3), SourcePosition::new(0, 3));
    assert_eq!(
      index.position_for_offset("abc中".len() as u32),
      SourcePosition::new(0, 4),
    );
    assert_eq!(
      index.position_for_offset("abc中def😀".len() as u32),
      SourcePosition::new(0, 9),
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
