//! 契約層（contract）— kanade との授受型・出力契約・cue 写像の宣言。
//!
//! kanade（③）・seriko（⑤）・emo（⑥）・ghost-setup が消費するメッセージ／出力／
//! 終端型を宣言し、cue ドメインの正本型（dola）を re-export する。
//!
//! - 出力の意味論は cue ドメイン（dola 正本の型）で表現する。relevance 分類器
//!   [`cue_target_of`]（cue→演者担当）は **dola**（`dola::cue::sink`）が単一権威として
//!   所有し、本層は re-export のみを行う（全演者が共有する relevance の唯一の場所・
//!   下流の import パス `areka_sakura::contract::cue_target_of` は不変に保つ）。
//! - 配送エンベロープ [`TalkCue`] の物理定義は **dola**（`dola::cue::TalkCue`）にあり、
//!   本層は re-export のみを行う（cue 再生の"制御"＝配送は dola の責務・二重定義しない）。
//!   下流の import パス（`areka_sakura::contract::TalkCue`）は不変に保つ。
//! - [`TalkId`]/[`StartTalk`]/[`TalkDone`]/[`TalkEndReason`]／[`TalkCommand`]/[`ChoiceWaiting`]
//!   は正本 `areka-talk` からの re-export（DD-1 解消・kanade↔sakura 授受契約の唯一の物理定義は
//!   `areka-talk`）。下流の import パス（`areka_sakura::contract::*`）は不変に保つ。
//! - `SakuraMsg`/`TalkHandle` は sakura 固有（kanade↔sakura 契約の外）ゆえ本層に物理定義
//!   のまま残す。
//! - dola cue 型・parsers 値型は re-export し二重定義しない。

// ── sakura 固有（kanade↔sakura 契約の外・本層に物理定義） ──

/// sakura アクターの inbox メッセージ（areka-actor inbox 規約・投函経路は inbox 一貫）。
#[non_exhaustive]
pub enum SakuraMsg {
    /// talk 起動（spawn_talk が spawn 直後に自己投函する。外部からは送らない）。
    Start(StartTalk),
    /// 時刻前進（注入式）。**talk 起点からの経過秒・0 起点・単調非減少・有限**。
    ///
    /// 本番は外部 ticker（kanade/clock アクター・ghost-setup 結線）が
    /// `dola::runtime::clock::now()` から elapsed を算出して送る（スコープ外シーム）。
    Tick(f64),
    /// kanade からの中断（単一 Close funnel・R7）。areka-actor 停止規約の Close 相当。
    Close,
    /// 選択解決（additive・R2.7）。選択待ち（barrier）で止まった talk へ、選ばれた選択肢 id を
    /// **talk アクター境界の型付き入力**として投入する唯一の口。`CuePlayer::resolve_choice` を
    /// 外部から直接呼ぶ経路は構造的に存在せず（アクター内に閉じている）、投函は W5
    /// （`areka-P0-choice-select-events`）の領分（本 spec は口の定義と檻のみ）。
    ResolveChoice { id: String },
}

/// spawn_talk の返り値: 中断/時刻注入の投函端＋join ハンドル（validation Issue 1 の解決）。
pub struct TalkHandle {
    /// Tick / Close の投函端（Start は spawn_talk が投函済み）。
    pub inbox: std::sync::mpsc::Sender<SakuraMsg>,
    /// 非 RAII join ハンドル（テストの終了同期・本番は kanade 裁量）。
    pub actor: areka_actor::ActorHandle,
}

// ── kanade との授受（正本=areka-talk・re-export） ──

pub use areka_talk::{ChoiceWaiting, StartTalk, TalkCommand, TalkDone, TalkEndReason, TalkId};

// ── 再輸出（二重定義しない） ──
//
// relevance 分類器 [`cue_target_of`] は dola（`dola::cue::sink`）が単一権威として所有し、
// 本層は re-export のみ（下流 `areka_sakura::contract::cue_target_of` パスは不変・網羅檻は
// dola 側 `crates/dola/tests/cue/sink_test.rs` の `cue_target_of_classifies_every_variant`）。
pub use areka_parsers::sakura::{NewLineRatio, SurfaceArg};
// システム変数展開の消費側契約型（R7・正本は `crate::sysvar`）を re-export し、下流は安定パス
// `areka_sakura::contract::*`（＝lib.rs で glob 再輸出）から参照できる（値源非所有・凍結像消費）。
pub use crate::sysvar::{DEFAULT_USERNAME, ResolvedVar, SystemVarSnapshot, resolve_system_var};
pub use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, CueSink, CueTarget, TalkCue,
    cue_target_of,
};
