//! # areka-kanade — kanade（奏でる）エンジン
//!
//! kanade は areka の**運行表（scheduling state machine）の正本**であり、
//! ghost の boot / steady / close 各フェーズの運行判断を担うエンジンである。
//! アーキテクチャは **純粋状態機械＋アクターシェル＋メッセージ境界差し替え**の
//! 三層構造をとる（純粋な運行判断・副作用実行のアクターシェル・差し替え可能な
//! SHIORI 境界）。
//!
//! ## 本クレートが持つ正本（canonical source）
//!
//! - **運行表の正本**: 運行状態機械（`schedule/`・後続タスクで実装）が ukadoc
//!   Reference 表に基づく遷移判断を一手に担う。mock fixture・状態機械の期待列・
//!   ハーネスの assert はすべてこの正本から導出される。
//! - **talk 契約の正本**: [`talk`] モジュールが talk 起動契約型
//!   （[`TalkId`] / [`StartTalk`] / [`TalkDone`]）を唯一定義する。消費側
//!   （sakura-engine）はこれを再定義してはならない。
//!
//! ## 依存規律
//!
//! [`talk`] モジュールは `std` のみに依存し、host32 型・areka-actor 型に一切
//! 依存しない（DD-1）。将来の契約クレート切り出しは [`talk`] の機械的移動だけで
//! 完結する。

pub mod msg;
// schedule の消費者（actor.rs シェル・後続タスク）が未登場のため、この時点では
// 状態機械 API と後続タスクが埋めるフェーズ分岐スタブが lib ビルドから未使用となる。
// テストビルド（`#[cfg(test)]`）では全アームを網羅する。actor.rs 実装で解消される。
#[allow(dead_code)]
pub(crate) mod schedule;
pub mod talk;

pub use msg::{
    CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriCall, ShioriFailure, ShioriMsg,
    ShioriOutcome,
};
pub use talk::{StartTalk, TalkDone, TalkId};
