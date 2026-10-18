//! 運行表の SSTP の入口（外からの台本・外から頼まれたイベント）。
//!
//! 今は記録の固定のラベルの定数だけを置く（受け付けの判定と 2 つの腕は後の仕事で足す）。

/// SSTP SEND のトークの `ActiveTalk::origin`（記録用）。
#[expect(dead_code, reason = "SSTP SEND の腕（運行表）を足す仕事で使い始める")]
pub(crate) const SEND_ORIGIN: &str = "SstpSend";
/// SSTP NOTIFY の応答の `origin`（殻が `EventId::Sstp` の往復に載せる固定のラベル）。
pub(crate) const NOTIFY_ORIGIN: &str = "SstpNotify";
