//! 契約層（contract）— kanade との授受型・出力契約・cue 写像の宣言。
//!
//! kanade（③）・seriko（⑤）・emo（⑥）・ghost-setup が消費するメッセージ／出力／
//! 終端型と、cue ドメインへの**写像**（[`TalkCue`]・[`cue_target_of`]）を所有する。
//!
//! - 出力の意味論は cue ドメイン（dola 正本の型）で表現し、本層は写像を所有する。
//! - [`TalkId`]/[`StartTalk`]/[`TalkDone`]/[`TalkEndReason`] は正本 `areka-talk` からの
//!   re-export（DD-1 解消・kanade↔sakura 授受契約の唯一の物理定義は `areka-talk`）。
//!   下流の import パス（`areka_sakura::contract::*`）は不変に保つ。
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
}

/// spawn_talk の返り値: 中断/時刻注入の投函端＋join ハンドル（validation Issue 1 の解決）。
pub struct TalkHandle {
    /// Tick / Close の投函端（Start は spawn_talk が投函済み）。
    pub inbox: std::sync::mpsc::Sender<SakuraMsg>,
    /// 非 RAII join ハンドル（テストの終了同期・本番は kanade 裁量）。
    pub actor: areka_actor::ActorHandle,
}

// ── kanade との授受（正本=areka-talk・re-export） ──

pub use areka_talk::{StartTalk, TalkDone, TalkEndReason, TalkId};

// ── 出力契約（cue ドメイン・写像の正本は本仕様） ──

/// 1 発火（両 sink 共通形）。requirements の SurfaceCommand級/TextCommand級 の実現形。
#[derive(Clone, Debug, PartialEq)]
pub struct TalkCue {
    /// talk 起点からの相対秒（R2.1・f64 秒＝dola ドメイン）。
    pub at: f64,
    /// 話者スコープの転写（scope n → ActorKey(n.to_string())・既定 "0"・R5）。
    pub actor: ActorKey,
    /// 演出コマンド（dola 正本・Emote は SurfaceArg の不透明転写・R3.2）。
    pub command: CueCommand,
}

/// CueCommand → 配送先スロットの分類（写像の正本・R3.3 の 2 系統分離）。
///
/// `Emote` / `EntityRef` / `BalloonSurface` → [`CueTarget::Shell`]、
/// `Text` / `NewLine` / `Clear` / `ClearAll` / `Choice` → [`CueTarget::Balloon`]。
/// 分類不能（`Custom` 等・M-boot compile は生成しない）と、**どの表現者の担当でもない
/// `Wait`**（action を持たず duration のみ）は `None`。
///
/// バルーン面切替（`BalloonSurface`）はサーフェス消費系＝表示系（SurfaceSink/seriko 行き）
/// ゆえ [`CueTarget::Shell`] へ分類する。文字状態機械（`CueTarget::Balloon`＝TextSink/
/// emo-text）へ流すのは誤配線（R3.2）。全域写像で `None`（配送不能）には落ちない（R3.3）。
///
/// 明示的な variant ごとの match により、dola が将来 variant を追加した際に
/// コンパイラが再検討を強制する（catch-all を置かない）。
pub fn cue_target_of(command: &CueCommand) -> Option<CueTarget> {
    match command {
        CueCommand::Emote { .. } => Some(CueTarget::Shell),
        CueCommand::EntityRef(..) => Some(CueTarget::Shell),
        CueCommand::BalloonSurface { .. } => Some(CueTarget::Shell), // 表示系＝SurfaceSink/seriko（3.2）
        CueCommand::Text(..) => Some(CueTarget::Balloon),
        CueCommand::NewLine { .. } => Some(CueTarget::Balloon),
        CueCommand::Clear => Some(CueTarget::Balloon),
        // 全スコープ消去はテキスト表現者（バルーン）の担当（Clear と同系）。
        CueCommand::ClearAll => Some(CueTarget::Balloon),
        CueCommand::Choice { .. } => Some(CueTarget::Balloon),
        CueCommand::Custom { .. } => None,
        // Wait は action を持たない純粋な待ち＝どの表現者の担当でもない（全員が
        // action を無視し duration のみ honor する）。分類不能（Custom）とは別理由で
        // `None` だが、「action する表現者がいない」点で帰結は同じ。
        CueCommand::Wait => None,
    }
}

// ── 再輸出（二重定義しない） ──
pub use areka_parsers::sakura::{NewLineRatio, SurfaceArg};
pub use dola::cue::{ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSheet, CueTarget};

#[cfg(test)]
mod tests {
    use super::*;
    use dola::DynamicValue;

    /// 全 CueCommand variant を [`cue_target_of`] に通し、写像表どおりの配送先へ
    /// 分類されることを検証する（R3.3・「全 variant の分類テスト」）。
    #[test]
    fn cue_target_of_classifies_every_variant() {
        // Shell 系
        assert_eq!(
            cue_target_of(&CueCommand::Emote {
                key: "smile".into()
            }),
            Some(CueTarget::Shell)
        );
        assert_eq!(
            cue_target_of(&CueCommand::EntityRef(42)),
            Some(CueTarget::Shell)
        );
        // バルーン面切替＝サーフェス消費系ゆえ表示系（Shell）へ。文字状態機械
        // （Balloon）へ流すのは誤配線（R3.2）・配送不能 None には落ちない（R3.3）。
        assert_eq!(
            cue_target_of(&CueCommand::BalloonSurface { key: "2".into() }),
            Some(CueTarget::Shell)
        );

        // Balloon 系
        assert_eq!(
            cue_target_of(&CueCommand::Text("hello".into())),
            Some(CueTarget::Balloon)
        );
        assert_eq!(
            cue_target_of(&CueCommand::NewLine { ratio: 1.0 }),
            Some(CueTarget::Balloon)
        );
        assert_eq!(
            cue_target_of(&CueCommand::Clear),
            Some(CueTarget::Balloon)
        );
        // 全スコープ消去もテキスト表現者の担当（対象スコープのみの Clear と同分類）。
        assert_eq!(
            cue_target_of(&CueCommand::ClearAll),
            Some(CueTarget::Balloon)
        );
        assert_eq!(
            cue_target_of(&CueCommand::Choice {
                id: "yes".into(),
                text: "はい".into()
            }),
            Some(CueTarget::Balloon)
        );

        // 分類不能
        assert_eq!(
            cue_target_of(&CueCommand::Custom {
                command: "fade".into(),
                params: DynamicValue::Null,
            }),
            None
        );

        // Wait は action を持たない＝担当する表現者がいない（全員が duration のみ honor）。
        assert_eq!(cue_target_of(&CueCommand::Wait), None);
    }
}
