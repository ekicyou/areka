//! 運行表の型の塊（`mod.rs` から出した入力・相・帳簿・状態・終了の起因・副作用指示）。
//!
//! `mod.rs` が `pub(crate) use types::{…}` で同じ名前で見せるため、子のモジュールと
//! テストは `super::State` などのまま参照できる。兄弟のモジュールの型は `super::` から引く。

use super::{
    anchor, balloon_events, change, events, resources, sstp, talk_active_of, talk_gap, translate,
};
use crate::msg::{
    ChoiceInput, CloseReason, MonotonicMs, MouseInput, ShioriCall, ShioriFault, ShioriOutcome,
};
use crate::status::{ExecutionSnapshot, ExecutionStateUpdate, ExternalStates};
use crate::talk::{StartTalk, TalkDone, TalkId};

/// 状態機械への入力。`KanadeMsg`（外部入力）＋シェルが同期往復で得た SHIORI 応答。
/// `ShioriReply` が `KanadeMsg` に存在しないため、応答注入経路はシェル内部に閉じる。
pub(crate) enum Input {
    Boot,
    Tick {
        now: MonotonicMs,
    },
    TalkDone(TalkDone),
    CloseRequest {
        reason: CloseReason,
    },
    ForceQuit {
        reason: CloseReason,
    },
    ShioriDown {
        kind: crate::msg::ShioriDownKind,
        reason: String,
    },
    /// マウス入力（移動／ダブルクリック）。Steady のみ `steady::on_mouse` へ委譲し、
    /// 他フェーズでは状態を変えず安全に無視する（DD-IE-8）。
    Mouse(MouseInput),
    /// 直前の Action::ShioriRequest／ShioriUnload の結果（シェルが即時再投入）。
    ///
    /// `origin` は再投入元の呼出イベント ID（シェルが送出した call の id を転記・DD-IE-3）。
    /// 後続処理（マウス GET の origin 別 reply 政策・タスク 2.2）が応答の出所を識別するための
    /// 内部情報。unload の応答には出所イベントが無いため `"Unload"` を転記する。
    ShioriReply {
        outcome: ShioriOutcome,
        origin: &'static str,
    },
    /// 選択確定（バルーン上で確定した選択肢・UI 配線層 → kanade）。additive 増分（Req 4.4）。
    ///
    /// 受領検証（選択待ちの有無・talk_id 突合・候補集合照合）とカスケード駆動は
    /// [`steady::on_choice`]（C4 規則 1／2）が持つ。Steady のみ委譲し、他フェーズは状態を
    /// 変えず warn 記録の上で棄却する。
    Choice(ChoiceInput),
    /// アンカーの選択（UI 配線層 → kanade）。受理の規則と段の記憶は [`anchor::on_anchor`] が持つ。
    Anchor(crate::anchor_input::AnchorInput),
    /// 選択待ち成立の通知（talk → dispatcher → kanade）。additive 増分（Req 4.4）。
    ///
    /// `display_end` は dispatcher が `base_now` で単調 ms へ換算済み（DD-9・時間基準を新設しない）。
    /// `timeout_directive_secs` から期限への写像は [`choice::choice_deadline`]（DD-8）が担い、
    /// 帳簿確立は [`steady::on_choice_waiting`]（C4 規則 4）が行う。
    ChoiceWaiting {
        talk_id: TalkId,
        choice_ids: Vec<String>,
        display_end: MonotonicMs,
        timeout_directive_secs: Option<f64>,
    },
    /// 利用者の中断（バルーンの左ダブルクリック・UI 配線層 → kanade）。
    ///
    /// `scope` は「どのバルーンで起きたか」を運ぶだけで、受理の判断には使わない。
    /// 受理の規則は [`user_break::on_user_break`] が持ち、場面を問わず横断の腕から呼ぶ。
    UserBreak {
        scope: u32,
    },
    /// ゴーストの切替の要求（UI → kanade）。受理の判断は [`change::on_change_ghost`] が持つ。
    ChangeGhost(crate::change::ChangeRequest),
    /// 汎用の通知の入口（UI → kanade）。判断は [`change::on_raise_event`] が持つ。
    RaiseEvent {
        id: String,
        references: Vec<String>,
        method: crate::change::ShioriMethod,
    },
    /// 台詞の切れ目の口（UI → kanade）。始め方は [`talk_gap::begin`] が持ち、結果は毎 `step` の
    /// 後の [`talk_gap::observe`] が決める。
    AwaitTalkGap {
        raise: Option<crate::change::GapRaise>,
    },
    /// 外から届いた実行状態の知らせ（UI → kanade）。相を問わず写し（[`State::external`]）を
    /// 更新するだけで、運行は変えない（行動を返さない）。
    ExecutionState(ExecutionStateUpdate),
    /// 殻が [`Action::Translate`] を実行した結果（殻が即時再投入）。判断は [`translate::on_done`] が持つ。
    TranslateDone(translate::TranslateResult),
    /// 時間切れでバルーンを隠した知らせ（表示の側 → kanade）。判断は [`balloon_events::on_timeout_notice`] が持つ。
    BalloonTimeout {
        talk_id: TalkId,
    },
    /// 外からの台本（MCP の `sakurascript`・殻 → 運行表）。判断は [`sstp::on_send`] が持つ。
    SstpSend {
        script: String,
        /// 呼び手の strict（真・偽・省略）。始めたトークの番号と結んで控える。
        strict: Option<bool>,
    },
    /// 外から頼まれたイベント（MCP の `raise_event`・殻 → 運行表）。判断は [`sstp::on_notify`] が持つ。
    SstpNotify {
        event: String,
        /// 渡された Reference の列（補わない・そのまま送る）。
        references: Vec<String>,
        /// 呼び手の strict（真・偽・省略）。返った台本のトークの番号と結んで控える。
        strict: Option<bool>,
    },
}

/// 運行フェーズ（可視化は System Flows の状態機械図）。各待ち点は「直前に発行した
/// 呼出の応答待ち」を表す（in-flight ≤ 1 ゆえ相関 id 不要）。
pub(crate) enum Phase {
    Idle,
    BootInit,
    /// username リソース照会（prefetch）応答待ち（OnInitialize 後・OnFirstBoot 前・R4.1）。
    /// 応答（Value/NoContent/Failed）は [`resources::ResourceOutcome`] へ写像され sink へ渡される
    /// （talk は生成しない・Invariant）。照会失敗でも boot は殺さず OnFirstBoot へ続行する。
    BootPrefetch,
    BootType,
    BootMain,
    /// basewareversion 応答待ち。起動挨拶を追跡する場合は `talk: Some(_)`（DD-IT-12）。
    /// 挨拶が無い（204）boot は `talk: None`＝従来どおり `Steady{talk: None}` へ完了する。
    BootVersion {
        talk: Option<ActiveTalk>,
    },
    Steady {
        talk: Option<ActiveTalk>,
    },
    ClosePending {
        reason: CloseReason,
    },
    CloseTalkWait {
        talk_id: TalkId,
        deadline: Option<MonotonicMs>,
    },
    /// 切替の `OnGhostChanging` の応答待ち（[`change`]）。
    ChangePending,
    /// `OnGhostChanging` の台詞の完了待ち／期限判定。
    ChangeTalkWait {
        talk_id: TalkId,
        deadline: Option<MonotonicMs>,
    },
    /// 切替の `OnClose` の応答待ち。
    ChangeClosePending,
    /// 切替の `OnClose` の別れの台詞の完了待ち／期限判定。
    ChangeCloseTalkWait {
        talk_id: TalkId,
        deadline: Option<MonotonicMs>,
    },
    Unloading {
        cause: TermCause,
    },
    Stopped,
}

/// 現在再生中の talk（origin は起動由来ラベル・ログ用）。
pub(crate) struct ActiveTalk {
    pub talk_id: TalkId,
    pub origin: &'static str,
    /// 当該 talk の起動スクリプト（`OnChoiceTimeout` の Reference0 供給源・DD-10）。
    ///
    /// **kanade が [`StartTalk`] で自ら作った値**の保持であり、新しい情報源ではない
    /// （通知同梱にせず kanade 内で完結させる・Req3.4）。Ref0 への割付はタスク 4.5。
    pub script: String,
}

/// 選択待ち〜choice 系 in-flight の帳簿（DD-3）。
///
/// **バリア状態の複製ではなく kanade 側の配送状態**である（再生層のバリアは sakura が所有し、
/// 解決は [`Action::ResolveChoice`] という正規入力経路でのみ行う・Req5.6）。[`Phase`] を一切
/// 触らず [`State`] へ置くのは `pending_close` と同型の扱いであり、Req4.4「既存の決定的状態機械の
/// 観測資産を変更しない」に最忠実な形である（DD-3）。
pub(crate) struct ChoiceState {
    /// 対象 talk（`ActiveTalk.talk_id` と一致することが不変条件）。
    pub talk_id: TalkId,
    /// 照合用の候補選択肢 ID 列（表示順を保存・DD-7）。
    pub candidates: Vec<String>,
    /// 選択待ちの期限（`None`＝無期限。DD-8 の写像済み値）。
    pub deadline: Option<MonotonicMs>,
    /// 帳簿の段フェーズ。
    pub phase: ChoicePhase,
}

/// 選択帳簿の段フェーズ（`Cascading`／`TimeoutInFlight` は drive 内で同期完結する応答待ち）。
pub(crate) enum ChoicePhase {
    /// 選択確定の入力待ち。
    Waiting,
    /// カスケード段の SHIORI 応答待ち（`next`＝残段）。
    Cascading {
        choice_id: String,
        next: Option<CascadeNext>,
    },
    /// `OnChoiceTimeout` の応答待ち。
    TimeoutInFlight,
}

/// カスケードの残段（M1 は正典形の無印 1 段のみ・裁定 2）。
pub(crate) enum CascadeNext {
    Select,
}

/// 運行状態の全体（[`step`] の唯一の被写体）。Phase 外の帳簿はここに置く。
pub(crate) struct State {
    pub phase: Phase,
    /// 直近 Tick の注入時刻（Tick 受領ごとに更新・close 期限計算の基準）。
    pub last_now: Option<MonotonicMs>,
    /// talk_id 採番カウンタ（単調増番・再利用しない・StartTalk 生成時にインクリメント）。
    pub next_talk_id: u64,
    /// boot 中・active talk 中に受領した close 指示の保留（System Flows 補足遷移）。
    pub pending_close: Option<CloseReason>,
    /// 選択待ち〜choice 系 in-flight の帳簿（`pending_close` と同型に Phase 外へ置く・DD-3）。
    pub choice: Option<ChoiceState>,
    /// choice 起因の slot 差替で失われた旧 talk_id を 1 世代だけ保持する枠（F1 残余レース対策）。
    ///
    /// 遅れて届く旧 talk の `TalkDone` を `unknown_talk_done`（error）ではなく
    /// `talk_done_stale_choice`（info）で捌くための照合先（[`on_talk_done`] の防御アーム）。
    ///
    /// 保持は**ちょうど 1 世代**である（C4 規則 9）: 書き込み点は choice 起因の slot 差替
    /// （カスケード Value・タイムアウト Value）、消去点は現 talk の `TalkDone` 到達と
    /// 次の slot 差替（マウス由来の置換を含む）である。
    pub choice_prev_talk: Option<TalkId>,
    /// アンカーのイベントの応答待ちの段（送る直前に置き、応答で必ず取り出す・[`anchor`]）。
    pub anchor: Option<anchor::AnchorStage>,
    /// 利用者の中断の控え（止めた相手と scope。完了通知を待っている間だけ `Some`）。
    pub user_break_talk: Option<balloon_events::BreakNote>,
    /// 最後に再生を始めたトークの控え（3 つのバルーンのイベントの Reference0 の源）。
    pub shown: Option<balloon_events::ShownTalk>,
    /// 受理した切替（要求と `OnGhostChanging` の台本）。停止通知の切替の中身の源。
    pub change: Option<change::ChangeState>,
    /// 台詞の再生中に受けた切替の要求の保留（`pending_close` と同型）。
    pub pending_change: Option<crate::change::ChangeRequest>,
    /// 台詞の切れ目の見張り（高々 1 つ・[`talk_gap`]）。
    pub talk_gap: Option<talk_gap::GapWatch>,
    /// 外から届いた実行状態の写し（中断の旗・通信中・見えているバルーンの組）。ゴーストごとに新品。
    pub external: ExternalStates,
    /// 翻訳の待ちの帳簿（高々 1 つ・[`translate::TranslateWait`]）。
    pub translate: Option<translate::TranslateWait>,
    /// 応答を待っている GET の元のイベント（一括の最後の往復が GET のときだけ `Some`）。
    ///
    /// 書くのは [`translate::after`]、SHIORI の応答の入力で 1 回だけ取り出すのは [`translate::before`]。
    pub reply_source: Option<events::SourceEvent>,
    /// 最後に始めた外からの台本のトークの strict（次の外からの台本で上書き・[`sstp::StrictNote`]）。
    pub sstp_strict: Option<sstp::StrictNote>,
    /// 任意の名前の GET の応答待ちの控え（送る直前に置き、応答で必ず取り出す・[`sstp::NotifyStage`]）。
    ///
    /// 往復は殻の 1 つの一括の中で閉じるので、知らせの切れ目では常に `None`。
    pub sstp_notify: Option<sstp::NotifyStage>,
}

impl State {
    /// 初期運行状態（[`Phase::Idle`]・Tick 未受領・採番カウンタ 1・保留 close なし・選択帳簿なし）。
    ///
    /// `next_talk_id` は 1 起点の単調増番であり、StartTalk 生成のたびにインクリメントし
    /// 再利用しない（[`crate::talk::TalkId`] の一意性契約）。
    pub(crate) fn initial() -> State {
        State {
            phase: Phase::Idle,
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
            anchor: None,
            user_break_talk: None,
            shown: None,
            change: None,
            pending_change: None,
            talk_gap: None,
            external: ExternalStates::default(),
            translate: None,
            reply_source: None,
            sstp_strict: None,
            sstp_notify: None,
        }
    }

    /// 送出時点の実行状態スナップショット（talk は相から・choice は選択帳簿から・nouserbreak・
    /// online・balloons は外から届いた写しから作る・DD-IT-3／設計 C5）。
    ///
    /// 本番の [`ExecutionSnapshot`] は本メソッド（と [`snapshot_with_choice`](State::snapshot_with_choice)）
    /// か [`snapshot_without_talk`](State::snapshot_without_talk) からしか作らない。wire 送出契約
    /// （連結順序・区切り・空集合→ヘッダ行省略）は無改変である（Req6.3）。
    pub(crate) fn snapshot(&self) -> ExecutionSnapshot {
        let choice_active = self
            .choice
            .as_ref()
            .is_some_and(|ledger| choice_phase_active(&ledger.phase));
        self.snapshot_with_choice(choice_active)
    }

    /// 選択待ち継続中かを**外から与える**スナップショット導出。
    ///
    /// [`steady::on_choice`] と [`steady::on_cascade_reply`] は検証・分解のために帳簿を
    /// [`Option::take`] してから呼出を組み立てるため、その最中は `self.choice` が空である。
    /// これらの呼出点は手元の帳簿から得た真偽値をここへ渡す——[`snapshot`](State::snapshot) を
    /// そのまま呼ぶとカスケード段の呼出から `choosing` が落ちる（設計 C5 の源は
    /// `Waiting|Cascading|TimeoutInFlight` の全段である）。
    ///
    /// nouserbreak は「再生中のトーク かつ 写しの旗」——写しは UI の旗を遅れて追う鏡であり
    /// トークの境界で下ろさないため、トークが無い間に旗が残っていても載せない（要件 3.3）。
    pub(crate) fn snapshot_with_choice(&self, choice_active: bool) -> ExecutionSnapshot {
        let talk_active = talk_active_of(&self.phase);
        ExecutionSnapshot {
            talk_active,
            choice_active,
            no_user_break: talk_active && self.external.no_user_break,
            online: self.external.online,
            balloons: self.external.balloons.clone(),
        }
    }

    /// 会話も選択も無いと決めて送る場面（起動の各段・切替・終了の握手・強制終了）の
    /// スナップショット。talk・choice・nouserbreak は無し、online と balloons は写しのまま（要件 5.1）。
    pub(crate) fn snapshot_without_talk(&self) -> ExecutionSnapshot {
        ExecutionSnapshot {
            talk_active: false,
            choice_active: false,
            no_user_break: false,
            online: self.external.online,
            balloons: self.external.balloons.clone(),
        }
    }
}

/// 選択帳簿の段フェーズが「選択待ち継続中」かを判定する（設計 C5 の源の定義）。
///
/// 3 段すべてが継続中である——`Cascading`／`TimeoutInFlight` は SHIORI 応答待ちであって
/// 選択待ちの終了ではなく、選択肢は表示されたままである。選択待ちが終わるのは帳簿そのものが
/// 消えるとき（解決・タイムアウト解除・トーク差替）だけであり、それを
/// [`State::snapshot`] の `Option` 判定が表す（Req6.2）。
///
/// wildcard を置かないため、段フェーズの追加時は本表での判断がコンパイル時に要求される。
fn choice_phase_active(phase: &ChoicePhase) -> bool {
    match phase {
        ChoicePhase::Waiting | ChoicePhase::Cascading { .. } | ChoicePhase::TimeoutInFlight => true,
    }
}

/// 終了系列の起因（ログ語彙・遷移は共通）。
///
/// `Clone` は停止通知が原因を控えるため（`actor_stop::stop_cause_of`）。
#[derive(Clone)]
pub(crate) enum TermCause {
    Quit,
    Forced,
    CloseSilent,
    DeadlineExceeded,
    /// 失敗の種類と理由を運ぶ（停止通知へそのまま写す）。
    Fault(ShioriFault),
}

/// 状態機械が返す副作用指示（シェルが実行する）。
pub(crate) enum Action {
    /// GET／NOTIFY 発行（シェルが oneshot 往復し ShioriReply を再投入する）。
    ShioriRequest(ShioriCall),
    /// unload 発行（同上）。
    ShioriUnload,
    StartTalk(StartTalk),
    /// リソース照会結果を注入クロージャ（[`resources::ResourceSink`]）へ**同期的に**渡す
    /// （prefetch・R4.1）。シェルが sink を呼ぶ副作用指示であり、sink が返るまで次段（OnFirstBoot）
    /// へ進まない。リソース照会は talk を生成しない——結果を StartTalk へ流さず sink へ渡すのみ（Invariant）。
    ResourceOutcome {
        id: &'static str,
        outcome: resources::ResourceOutcome,
    },
    /// 終了系列完了（シェルは shiori へ Close を送り自身も Break する）。
    StopSelf,
    /// 選択待ちバリアの解決指示（→ [`TalkCommand::ResolveChoice`](crate::talk::TalkCommand)）。
    ///
    /// `talk_id` は再生層／dispatcher の stale ガード用・`id` は確定した選択肢 ID。発行点は
    /// [`steady`] の選択調停（`script:` の選択肢の解決・カスケード終端）に単一化されている
    /// （1 選択＝高々 1 解決・Req5.4）。
    ResolveChoice {
        talk_id: TalkId,
        id: String,
    },
    /// 選択待ちの解除＋トーク終了指示（→ [`TalkCommand::CancelChoice`](crate::talk::TalkCommand)）。
    ///
    /// 発行点は 2 つある。⑴ タイムアウト後に SHIORI が応答を返さなかった場合の解除（Req7.5）。
    /// ⑵ 利用者の中断の受理（[`user_break::on_user_break`]）——再生中のトークを止める唯一の指示で
    /// ある。dispatcher の受け口は選択の状態を見ず、現行の枠と `talk_id` が一致すれば閉じ指示を
    /// 転送するだけなので、選択待ちでないトークに使っても副作用は無い。名前が「選択」を指すのは
    /// ⑴ が先にあったためで、⑵ は同じ単一の閉じ口を再利用している。
    CancelChoice {
        talk_id: TalkId,
    },
    /// 運行の通知を UI へ送る（シェルは停止通知と同じ送出端へそのまま流す）。
    Notice(crate::change::KanadeNotice),
    /// 翻訳の依頼（殻が `OnTranslate` を往復させ、結果を [`Input::TranslateDone`] で再投入する）。
    Translate(translate::TranslateRequest),
}
