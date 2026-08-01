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
//! - **talk 契約**: talk 起動契約型（[`TalkId`] / [`StartTalk`] / [`TalkDone`] /
//!   [`TalkEndReason`]）の物理正本は `areka-talk` クレートである。[`talk`] モジュールは
//!   `areka_kanade::talk::*` という既存の参照パスを保つための再エクスポートに徹し、
//!   二重定義は行わない。消費側（sakura-engine）も同じく `areka-talk` を直接参照する。
//!
//! ## 依存規律
//!
//! talk 契約の物理正本（`areka-talk`）は `std` のみに依存し、host32 型・areka-actor 型に
//! 一切依存しない（DD-1）。

pub mod actor;
pub mod msg;
// schedule の消費者はランタイム層の actor.rs シェル（[`crate::actor::spawn_kanade`]）。
// schedule 内には後続タスクが埋めるフェーズ分岐スタブが残り lib ビルドから未使用となるため、
// クレート全体は `#[allow(dead_code)]` を付さず schedule 側の該当箇所に限局する（下記）。
#[allow(dead_code)]
pub(crate) mod schedule;
pub mod shiori;
pub mod status;
pub mod talk;

pub use actor::spawn_kanade;
pub use msg::{
    ChoiceInput, CloseReason, EventId, KanadeConfig, KanadeMsg, MonotonicMs, MouseButton,
    MouseEventKind, MouseInput, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome,
};
pub use shiori::{ShioriBackend, ShioriConnection, spawn_shiori_actor};
pub use status::{ExecutionSnapshot, ExecutionState, ExecutionStatus};
pub use talk::{ChoiceWaiting, StartTalk, TalkCommand, TalkDone, TalkEndReason, TalkId};

/// ukadoc Reference 表の実装正本（純粋関数群）を露出する公開ファサード（DD-9 例外）。
///
/// `schedule/` は `pub(crate)` に閉じるが、`tests/` 配下の統合テストクレートは
/// `pub(crate)` を参照できない。Req 7.1（fixture・検証・実装が単一の正本を共有）を
/// 成立させる唯一の経路として、Reference 表構成関数のみを本ファサード経由で
/// クレート公開面に露出する（`areka_kanade::events::on_boot(&config)` 等で到達可能）。
/// 運行状態機械の内部（Phase／State／Action／Input／step 本体・遷移ロジック）は
/// `pub(crate)` のまま非公開に保つ。
pub mod events {
    pub use crate::schedule::events::{
        ALLOWED_EVENT_IDS, baseware_version, is_allowed_choice_event, is_allowed_event_id, on_boot,
        on_choice_named, on_choice_select, on_choice_select_ex, on_choice_timeout, on_close,
        on_close_notify, on_first_boot, on_initialize, on_mouse_double_click, on_mouse_move,
        on_second_change,
    };
}

/// SHIORI Resource 照会増分の公開ファサード（DD-9 例外・[`events`] と同じ露出規律）。
///
/// boot 系列の username prefetch（`schedule/boot.rs`・OnInitialize 後・OnFirstBoot 前）で使う
/// リソース照会構築関数・結果語彙・注入シームを、クレート公開面へ露出する。ghost（タスク 8.2）が
/// [`spawn_kanade`] へ [`resources::ResourceSink`] を注入し、`tests/` がハーネスの期待値導出
/// （`expected_call(resources::resource_username(..))`）に用いる唯一の経路（Req 7.1・DD-9）。
/// 運行状態機械の内部（Phase／State／Action／step 本体）は `pub(crate)` のまま非公開に保つ。
pub mod resources {
    pub use crate::schedule::resources::{
        ALLOWED_RESOURCE_IDS, ResourceOutcome, ResourceSink, is_allowed_resource_id,
        resource_username,
    };
}
