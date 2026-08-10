mod builtin;
mod mdn_generated;
mod rule;

pub use builtin::SyntaxCompatDatabase;
pub use mdn_generated::{MDN_BCD_PACKAGE_VERSION, MDN_BCD_SYNTAX_ENTRY_COUNT};
pub use rule::SupportRule;
