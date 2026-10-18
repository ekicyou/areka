//! 実機の試験の窓口（spec: areka-P0-real-run-mcp-kit）。
//!
//! 子の areka を立てる・MCP のツールを呼ぶ・閉じる・落ちた回の証拠を残す部品を、
//! ワークスペースで 1 か所にまとめる。呼び手は `[dev-dependencies]` からだけ引く。

mod close;
mod evidence;
mod i686;
mod launch;
mod mcp;
mod output;
mod run;

pub use close::{Ended, EndedBy};
pub use output::ChildExited;
