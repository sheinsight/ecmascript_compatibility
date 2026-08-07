mod builtin;
mod mdn_generated;
mod rule;

pub use builtin::SyntaxCompatDatabase;
pub use mdn_generated::{
  MDN_BCD_JAVASCRIPT_ENTRY_COUNT, MDN_BCD_PACKAGE_VERSION,
};
pub use rule::SupportRule;
