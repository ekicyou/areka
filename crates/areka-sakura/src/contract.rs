//! 契約層（contract）— kanade との授受型・出力契約・cue 写像の宣言。
//!
//! kanade（③）・seriko（⑤）・emo（⑥）・ghost-setup が消費するメッセージ／出力／
//! 終端型を宣言し、cue ドメインの正本型（dola）を re-export する。
//!
//! - relevance 分類器 [`cue_target_of`]（cue→演者担当）と配送エンベロープ [`TalkCue`] は
//!   dola（`dola::cue`）が単一権威として所有し、本層は re-export のみ（二重定義しない）。
//!   下流は `areka_sakura::contract::*` のパスで参照する。
//! - [`TalkId`]/[`StartTalk`]/[`TalkDone`]/[`TalkEndReason`]／[`TalkCommand`]/[`ChoiceWaiting`]
//!   は正本 `areka-talk` からの re-export（kanade↔sakura 授受契約の唯一の物理定義は `areka-talk`）。
//! - `SakuraMsg`/`TalkHandle` は sakura 固有（kanade↔sakura 契約の外）ゆえ本層に物理定義を置く。
//! - dola cue 型・parsers 値型は re-export し二重定義しない。

// ── sakura 固有（kanade↔sakura 契約の外・本層に物理定義） ──

/// sakura アクターの inbox メッセージ（areka-actor inbox 規約・投函経路は inbox 一貫）。
#[non_exhaustive]
pub enum SakuraMsg {
    /// talk 起動（spawn_talk が spawn 直後に自己投函する。外部からは送らない）。
    Start(StartTalk),
    /// 時刻前進（注入式）。最初の刻みの時刻が台本の起点になる。
    ///
    /// 本番は areka-ghost の dispatcher（`dispatcher.rs` の `on_tick`）が送る（スコープ外シーム）。
    Tick(TimePoint),
    /// kanade からの中断（単一 Close funnel）。areka-actor 停止規約の Close 相当。
    Close,
    /// 選択解決。選択待ち（barrier）で止まった talk へ、選ばれた選択肢 id を
    /// **talk アクター境界の型付き入力**として投入する唯一の口。`CuePlayer::resolve_choice` を
    /// 外部から直接呼ぶ経路は構造的に存在せず（アクター内に閉じている）、投函はこの層の外の
    /// 領分（ここは口の定義のみ）。
    ResolveChoice { id: String },
}

/// spawn_talk の返り値: 中断/時刻注入の投函端＋join ハンドル。
pub struct TalkHandle {
    /// Tick / Close の投函端（Start は spawn_talk が投函済み）。
    pub inbox: std::sync::mpsc::Sender<SakuraMsg>,
    /// 非 RAII join ハンドル（テストの終了同期・本番は kanade 裁量）。
    pub actor: areka_actor::ActorHandle,
}

// ── kanade との授受（正本=areka-talk・re-export） ──

pub use areka_talk::{ChoiceWaiting, StartTalk, TalkCommand, TalkDone, TalkEndReason, TalkId};

// 出どころの語彙（正本=areka-talk・受け口の areka が直接の依存なしで使う）。
pub use areka_talk::{ScriptOrigin, SecurityLevel, SstpKind};

// ── 再輸出（二重定義しない） ──
//
// システム変数展開の消費側契約型（正本は `crate::sysvar`）を re-export し、下流は安定パス
// `areka_sakura::contract::*`（＝lib.rs で glob 再輸出）から参照できる（値源非所有・凍結像消費）。
pub use crate::sysvar::SystemVarSnapshot;

// relevance 分類器 [`cue_target_of`] は dola（`dola::cue::sink`）が単一権威として所有し、
// 本層は re-export のみ（下流 `areka_sakura::contract::cue_target_of` パスは不変・網羅檻は
// dola 側 `crates/dola/tests/cue/sink_test.rs` の `cue_target_of_classifies_every_variant`）。
pub use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, CueSink, CueTarget, TalkCue,
    cue_target_of,
};

// 合図の欄の型。テストの組み立て（`dola::time::testing`）は本番の道から再公開しない。
pub use dola::time::{TimeDelta, TimePoint, TimeSpan};

// ── `\f` 文字装飾の運搬名 ──

/// `\f` 装飾を運ぶ `CueCommand::Custom` のコマンド名（消費側はこの名前で自己選別する）。
///
/// キーごとに typed な cue を新設せず、`\!` コマンドと同じ 1 本の汎用キャリア
/// （[`CueCommand::command_carrier`]）へ載せる。名前が合わない消費者
/// （ghost・seriko）は既存の `\!` の規約どおり無視する。
pub const FONT_TAG_CARRIER: &str = "\\f";
