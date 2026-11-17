//! 報告の本文の組み立ての入口。
//!
//! 集計は `tally`、束は `bundle`、ドメイン別の本文は `domain`、全体の本文は `summary`。

pub mod bundle;
pub mod domain;
pub mod summary;
pub mod tally;
