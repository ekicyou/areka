//! 契約層（contract）— kanade との授受型・出力契約・cue 写像の宣言。
//!
//! kanade（③）・seriko（⑤）・emo（⑥）・ghost-setup が消費するメッセージ／出力／
//! 終端型と、cue ドメインへの**写像**（[`TalkCue`]・[`cue_target_of`]）を所有する。
//!
//! - 出力の意味論は cue ドメイン（dola 正本の型）で表現し、本層は写像と kanade 授受型
//!   （暫定）を所有する。
//! - [`StartTalk`]/[`TalkDone`] は kanade が正本だが未実装ゆえ本層が**暫定所有**する
//!   （DD-1・不変）。kanade 完成時は re-export へ差し替え、下流の import パス
//!   （`areka_sakura::contract::*`）を不変に保つ。
//! - dola cue 型・parsers 値型は re-export し二重定義しない。

// ── kanade との授受（暫定所在・DD-1／kanade 完成時に移譲） ──

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

/// talk 起動契約（正本=kanade・暫定所在）。
pub struct StartTalk {
    /// 再生対象のさくらスクリプト本文。
    pub script: String,
    /// talk 相関 ID。
    pub talk_id: TalkId,
    /// TalkDone の返信端（oneshot 相当・move-consume が唯一の高々 1 回機構）。
    pub reply: areka_actor::ReplySender<TalkDone>,
}

/// talk 相関 ID（kanade が stale 終端信号の棄却に用いる・R6.6）。不透明 newtype。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TalkId(pub u64);

/// 終端信号（正本=kanade・暫定所在）。通算高々 1 回・reason 3 値（R6/R7）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TalkDone {
    /// 対応する talk の相関 ID。
    pub talk_id: TalkId,
    /// 終端理由（3 値）。
    pub reason: TalkEndReason,
}

/// 終端理由（従来の quit:bool を 3 値化・議題#1 確定）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkEndReason {
    /// `\e` / 末尾到達 / 空列（R6.1/6.3/1.4）。
    Ended,
    /// `\-`（R6.2）。
    Quit,
    /// Close による中断（R7.4・close 握手 ACK）。
    Interrupted,
}

/// spawn_talk の返り値: 中断/時刻注入の投函端＋join ハンドル（validation Issue 1 の解決）。
pub struct TalkHandle {
    /// Tick / Close の投函端（Start は spawn_talk が投函済み）。
    pub inbox: std::sync::mpsc::Sender<SakuraMsg>,
    /// 非 RAII join ハンドル（テストの終了同期・本番は kanade 裁量）。
    pub actor: areka_actor::ActorHandle,
}

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
/// `Emote` / `EntityRef` → [`CueTarget::Shell`]、
/// `Text` / `NewLine` / `Clear` / `Choice` → [`CueTarget::Balloon`]。
/// 分類不能（`Custom` 等・M-boot compile は生成しない）は `None`（呼び手が error ログ）。
///
/// 明示的な variant ごとの match により、dola が将来 variant を追加した際に
/// コンパイラが再検討を強制する（catch-all を置かない）。
pub fn cue_target_of(command: &CueCommand) -> Option<CueTarget> {
    match command {
        CueCommand::Emote { .. } => Some(CueTarget::Shell),
        CueCommand::EntityRef(..) => Some(CueTarget::Shell),
        CueCommand::Text(..) => Some(CueTarget::Balloon),
        CueCommand::NewLine { .. } => Some(CueTarget::Balloon),
        CueCommand::Clear => Some(CueTarget::Balloon),
        CueCommand::Choice { .. } => Some(CueTarget::Balloon),
        CueCommand::Custom { .. } => None,
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
    }
}
