//! 実機の試験の窓口（spec: areka-P0-real-run-mcp-kit）。
//!
//! 子の areka を立てる・MCP のツールを呼ぶ・閉じる・落ちた回の証拠を残す部品を、
//! ワークスペースで 1 か所にまとめる。呼び手は `[dev-dependencies]` からだけ引く。
//!
//! 立てる → 呼ぶ → 閉じるの例（建てるだけで走らせない）。試験では実行ファイルの場所を
//! `env!("CARGO_BIN_EXE_areka")` で渡す。使い方の手引きは `doc/testing.md` の「実機の試験」の節。
//!
//! ```no_run
//! use real_run_kit::{EndedBy, Launch, RealRun};
//!
//! // 立てる: 検体 emo2 の写しを取り、空いた番号で MCP を待ち受ける子を起こす。
//! let mut run = RealRun::new(r"C:\path\to\target\debug\areka.exe")
//!     .launch(Launch::Sample {
//!         name: "emo2".into(),
//!         balloon: Some("emo2-kakukaku".into()),
//!     })
//!     .spawn();
//!
//! // 呼ぶ: 答えの文に目当ての文字列が現れるまで呼び直す（失敗は Err・締切は panic）。
//! let reply = run
//!     .call_until("get_log", r#"{"log_type":"status"}"#, "本物のゴースト窓を開きました")
//!     .unwrap_or_else(|e| panic!("get_log を呼べませんでした: {e}\n{}", run.output()));
//! assert!(!reply.is_error, "{}", reply.text);
//!
//! // 閉じる: 利用者と同じ閉じ方で閉じ、その閉じ方で終わったことを見る。
//! let ended = run.close();
//! assert_eq!(ended.by, EndedBy::UserClose, "{}", run.output());
//! ```

mod close;
mod evidence;
mod i686;
mod launch;
mod mcp;
mod output;
mod run;

pub use close::{Ended, EndedBy};
pub use launch::{Launch, RealRun, Staged};
pub use mcp::{McpError, ToolReply};
pub use output::ChildExited;
pub use run::{Limits, Run};
