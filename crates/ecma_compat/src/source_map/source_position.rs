/// Source Map 使用的零基行列位置。
///
/// `col` 按 Source Map 规范表示 UTF-16 code unit 列号，不是 UTF-8 byte
/// offset。detector 的 `SourceSpan` 需要经过位置索引转换后才能用于 lookup。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourcePosition {
  line: u32,
  col: u32,
}

impl SourcePosition {
  pub const fn new(line: u32, col: u32) -> Self {
    Self { line, col }
  }

  pub const fn line(self) -> u32 {
    self.line
  }

  pub const fn col(self) -> u32 {
    self.col
  }
}
