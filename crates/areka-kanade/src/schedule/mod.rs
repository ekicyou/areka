//! schedule — 純粋運行状態機械（`src/schedule/`）。
//!
//! 全運行判断を純粋関数 [`step`] として実装する層である（I/O・スレッド・channel
//! 非依存＝決定的単体テストの本体）。[`step`] は現在の [`State`] と [`Input`] から
//! 次の [`State`] と副作用指示 [`Action`] の列を返す唯一の遷移入口であり、`tracing`
//! によるログ発行以外の副作用を持たない（可観測性の側効果であり状態・出力の決定性に
//! 影響しない・DD-3）。DD-9 により本モジュールは `pub(crate)` に閉じる。
//!
//! # 責務分割（本タスク 2.1 の担当範囲）
//! 本 `mod.rs` は**由来・状態を問わない横断遷移**（[`Input::TalkDone`] の理由が
//! [`TalkEndReason::Quit`] の場合／[`Input::ForceQuit`]／[`Input::ShioriDown`]／[`Input::ShioriReply`] の
//! 失敗）と、Unload 完了・防御アーム（未知 talk_id・Idle 以外の Boot・応答待ちでない
//! ShioriReply）を実装する。フェーズ固有の遷移は [`boot`]／[`steady`]／[`close`] の
//! 各サブモジュールへ委譲する（後続タスク 2.3／2.4／2.5 が本体を実装する）。
//!
//! # ログ規律（steering: areka-log-first-no-silent-failure）
//! すべての失敗・防御アームは `tracing::error!`／`tracing::warn!` を発行する。沈黙の
//! 失敗経路は存在しない。panic は新規導入しない（回復不能はすべて
//! `Unloading{Fault}`→`Stopped` の正規遷移で表現する・Req 6.4）。

use crate::msg::{
    ChoiceInput, CloseReason, KanadeConfig, MonotonicMs, MouseInput, ShioriCall, ShioriOutcome,
};
use crate::status::ExecutionSnapshot;
use crate::talk::{StartTalk, TalkDone, TalkEndReason, TalkId};

pub(crate) mod boot;
pub(crate) mod choice;
pub(crate) mod close;
/// ukadoc Reference 表の実装正本（純粋関数群）。DD-9 の例外として `pub`。
/// クレート公開面への露出は [`crate::events`] ファサード経由（[`crate::lib`] 参照）。
pub mod events;
/// タスク 6.1: 純粋 step 層の失敗・防御アームのログ発火検証（テスト専用）。
#[cfg(test)]
pub(crate) mod log_capture;
/// SHIORI Resource 照会の許可集合（イベント檻とは別族・Req4.1）。DD-9 の例外として `pub`。
/// submit ガードはイベント許可 ∨ リソース許可で判定する（`crate::actor` の egress チョークポイント）。
pub mod resources;
pub(crate) mod steady;

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
        }
    }

    /// 送出時点の実行状態スナップショット（運行フェーズ＋選択帳簿から導出・DD-IT-3／設計 C5）。
    ///
    /// [`snapshot_of`] は `Phase` しか読めず選択待ちを知れないため、**供給側の署名を `State`
    /// 全体へ広げた**形である（`status.rs` の NOTE どおり）。広げるのは内部シグネチャだけで
    /// あり、wire 送出契約（連結順序・区切り・空集合→ヘッダ行省略）は無改変である（Req6.3）。
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
    pub(crate) fn snapshot_with_choice(&self, choice_active: bool) -> ExecutionSnapshot {
        ExecutionSnapshot {
            talk_active: snapshot_of(&self.phase).talk_active,
            choice_active,
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

/// 選択帳簿が **choice in-flight**（選択由来の SHIORI 呼出の応答待ち）かを判定する（DD-12）。
///
/// [`choice_phase_active`]（＝選択待ち継続中・`choosing` の源）とは別軸である: `Waiting` は
/// 選択待ち継続中だが in-flight ではない。横断 `Failed`→`Unloading{Fault}` の免除
/// （[`on_shiori_reply`] の先行アーム・C4 規則 8）は in-flight のときだけ効かせる——選択待ち中に
/// 届く `Failed` は pump／マウス由来であり、免除すれば SHIORI 失敗の終了規律（Req6.1）が壊れる。
///
/// wildcard を置かないため、段フェーズの追加時は本表での判断がコンパイル時に要求される。
fn choice_in_flight(phase: &ChoicePhase) -> bool {
    match phase {
        ChoicePhase::Cascading { .. } | ChoicePhase::TimeoutInFlight => true,
        ChoicePhase::Waiting => false,
    }
}

/// 選択帳簿を消去する単一の掃除ヘルパ（C4 規則 7・Req1.3／6.2）。
///
/// 規則 7 の不変条件は「帳簿の対象 talk ≠ 現行 talk なら即 `None`」である。その不変条件が破れる
/// 遷移点——**対象トークの完了・slot 置換（マウス由来を含む）・close 系遷移**——で本関数を呼び、
/// 帳簿の対象と現行トークが食い違う状態を残さない。掃除は状態遷移であって失敗ではないため
/// `trace!` で観測する（沈黙で捨てる経路は作らない・log-first）。`at` は掃除点の識別子である。
///
/// 棄却経路（規則 1 の受領検証）からは呼ばない——棄却の定義は**状態不変**であり、既存帳簿を
/// 含めて一切書き換えないことが規則 1 の要求だからである。
pub(super) fn clear_choice_ledger(state: &mut State, at: &'static str) {
    if let Some(ledger) = state.choice.take() {
        tracing::trace!(
            target: "kanade",
            event = "choice_ledger_cleared",
            at = at,
            talk_id = ledger.talk_id.0,
            stage = steady::choice_phase_label(&ledger.phase),
            "選択帳簿を消去——帳簿の対象と現行トークの食い違いを残さない（C4 規則 7）"
        );
    }
}

/// 終了系列の起因（ログ語彙・遷移は共通）。
pub(crate) enum TermCause {
    Quit,
    Forced,
    CloseSilent,
    DeadlineExceeded,
    Fault,
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
    /// [`steady`] の選択調停（未対応カテゴリの即時解決・カスケード終端）に単一化されている
    /// （1 選択＝高々 1 解決・Req5.4）。
    ResolveChoice {
        talk_id: TalkId,
        id: String,
    },
    /// 選択待ちの解除＋トーク終了指示（→ [`TalkCommand::CancelChoice`](crate::talk::TalkCommand)）。
    ///
    /// タイムアウト後に SHIORI が応答を返さなかった場合の解除（Req7.5）。発行点はタスク 4.5。
    CancelChoice {
        talk_id: TalkId,
    },
}

/// 唯一の遷移入口。現在の [`State`] と [`Input`] から次の [`State`] と副作用指示
/// [`Action`] の列を返す純粋関数（`tracing` ログ発行のみ側効果として許容）。
///
/// 本タスク 2.1 は**横断遷移**（由来・状態を問わず終了系列へ進む共通ロジック）と
/// 防御アームを実装し、フェーズ固有の遷移は各サブモジュールへ委譲する。処理順は
/// 「横断遷移を先に判定 → 該当しなければフェーズ分岐」である。
pub(crate) fn step(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        // --- 横断遷移: 由来・状態を問わず終了系列へ進む共通ロジック ---

        // ForceQuit（全 Phase・DD-10）: best-effort OnClose NOTIFY を Action 先頭に積み、
        // quit ゲートを迂回して Unloading{Forced} へ直行する（Req 4.4）。
        Input::ForceQuit { reason } => force_quit(state, reason),

        // 死活報告（暫定 seam・DD-4）: error! 記録の上 Unloading{Fault} へ（Req 5.4）。
        Input::ShioriDown { reason } => {
            tracing::error!(target: "kanade", event = "shiori_down", reason = %reason, "SHIORI 死活報告を受領——終了系列（Fault）へ");
            to_unloading_fault(state)
        }

        // TalkDone: reason（3 値）・talk_id 突合を横断的に判定する（Req 2.5・4.3・6.2）。
        Input::TalkDone(done) => on_talk_done(state, done, config),

        // ShioriReply: Unload 完了・呼出失敗（Failed）の横断判定を先に行い、
        // それ以外は応答待ちフェーズへ委譲する（Req 6.1）。origin は応答の出所（DD-IE-3）。
        Input::ShioriReply { outcome, origin } => on_shiori_reply(state, outcome, origin, config),

        // Mouse（DD-IE-8）: Steady でのみ受理し steady::on_mouse へ委譲する。他フェーズ
        // （boot／close／terminate 後）では状態を変えず安全に無視する。boot 時のマウス移動は
        // 正常な環境入力ゆえ warn ではなく trace で観測する（沈黙の無視経路は作らない）。
        Input::Mouse(m) => match state.phase {
            Phase::Steady { .. } => steady::on_mouse(state, m),
            _ => {
                tracing::trace!(
                    target: "kanade",
                    event = "mouse_input_ignored",
                    phase = phase_label(&state.phase),
                    "非 Steady フェーズのマウス入力——状態を変えず無視（DD-IE-8）"
                );
                (state, Vec::new())
            }
        },

        // ChoiceWaiting（C4 規則 4・Req7.1）: Steady でのみ受理し steady::on_choice_waiting へ
        // 委譲する（現行トークとの識別子突合・DD-8 の期限写像・帳簿確立は委譲先の責務）。
        // 非 Steady フェーズ（boot 中・close 握手以降・終了系列）には受理すべき選択待ちが
        // 構造上存在しないため、帳簿を確立せず warn 記録の上で棄却する（状態不変）。
        Input::ChoiceWaiting {
            talk_id,
            choice_ids,
            display_end,
            timeout_directive_secs,
        } => match state.phase {
            Phase::Steady { .. } => steady::on_choice_waiting(
                state,
                talk_id,
                choice_ids,
                display_end,
                timeout_directive_secs,
                config,
            ),
            _ => {
                tracing::warn!(
                    target: "kanade",
                    event = "choice_waiting_stale",
                    reason = "non_steady_phase",
                    talk_id = talk_id.0,
                    choice_count = choice_ids.len(),
                    phase = phase_label(&state.phase),
                    "非 Steady フェーズの選択待ち通知——帳簿を確立せず棄却（C4 規則 4）"
                );
                (state, Vec::new())
            }
        },

        // Choice（C4 規則 1・Req1.1／1.3）: Steady でのみ受理し steady::on_choice へ委譲する
        // （帳簿突合・候補照合・段列決定は委譲先の責務）。非 Steady フェーズ（boot 中・close
        // 握手以降・終了系列）には受理すべき選択待ちが構造上存在しないため、状態を一切変えず
        // warn 記録の上で棄却して処理を継続する（沈黙の棄却経路は作らない）。
        Input::Choice(c) => match state.phase {
            Phase::Steady { .. } => steady::on_choice(state, c),
            _ => {
                tracing::warn!(
                    target: "kanade",
                    event = "choice_rejected_no_wait",
                    reason = "non_steady_phase",
                    choice_id = %c.id,
                    scope = c.scope,
                    reference_count = c.references.len(),
                    phase = phase_label(&state.phase),
                    "非 Steady フェーズの選択確定——状態不変で棄却（C4 規則 1）"
                );
                (state, Vec::new())
            }
        },

        // --- 防御アーム・フェーズ固有遷移への委譲 ---

        // Idle 以外での Boot は不整合（warn!＋現 Phase 維持・Req 6.2）。Idle のみ boot へ委譲。
        Input::Boot => match state.phase {
            Phase::Idle => boot::step(state, Input::Boot, config),
            _ => {
                tracing::warn!(target: "kanade", event = "boot_ignored", "Idle 以外での Boot 指示を無視");
                (state, Vec::new())
            }
        },

        // Tick・CloseRequest はフェーズ固有遷移（後続タスクが本体を実装）。
        Input::Tick { now } => dispatch_phase(state, Input::Tick { now }, config),
        Input::CloseRequest { reason } => {
            dispatch_phase(state, Input::CloseRequest { reason }, config)
        }
    }
}

/// 送出時点の運行フェーズから実行状態スナップショットを導出する（DD-IT-3）。
/// アクティブな talk を運ぶ phase のみ talk_active=true。
///
/// 選択待ち（`choice_active`）の源は `Phase` の外（[`State::choice`]）にあるため本関数からは
/// 知れず、常に false を返す。選択待ちを知る必要がある呼出点は [`State::snapshot`]／
/// [`State::snapshot_with_choice`] を使う（設計 C5）。残る本関数の利用点は選択待ちが構造上
/// 存在しない場面のみである——boot 系列（`BootVersion` の起動挨拶）と、`Unloading` へ遷移
/// **後**に採る `force_quit` の best-effort NOTIFY（＝INACTIVE・DD-IT-4）。
pub(crate) fn snapshot_of(phase: &Phase) -> ExecutionSnapshot {
    match phase {
        // アクティブな talk を運ぶ phase＝Steady{Some} と（挨拶追跡中の）BootVersion{Some}（DD-IT-12）。
        Phase::Steady { talk: Some(_) } | Phase::BootVersion { talk: Some(_) } => {
            ExecutionSnapshot {
                talk_active: true,
                choice_active: false,
            }
        }
        _ => ExecutionSnapshot::INACTIVE,
    }
}

/// フェーズの静的ラベル（ログ観測用）。`Phase` は Debug を持たないため、可観測性ログ
/// （例 `mouse_input_ignored`）に添える人間可読な識別子をここで与える。
fn phase_label(phase: &Phase) -> &'static str {
    match phase {
        Phase::Idle => "Idle",
        Phase::BootInit => "BootInit",
        Phase::BootPrefetch => "BootPrefetch",
        Phase::BootType => "BootType",
        Phase::BootMain => "BootMain",
        Phase::BootVersion { .. } => "BootVersion",
        Phase::Steady { .. } => "Steady",
        Phase::ClosePending { .. } => "ClosePending",
        Phase::CloseTalkWait { .. } => "CloseTalkWait",
        Phase::Unloading { .. } => "Unloading",
        Phase::Stopped => "Stopped",
    }
}

/// ForceQuit の横断遷移（DD-10）: best-effort OnClose NOTIFY を先頭に積み Unloading{Forced} へ。
///
/// OnClose NOTIFY の構築は events.rs（[`events::on_close_notify`]）へ委譲する——events.rs が
/// `ShioriCall` 構築の単一列挙点であり、force_quit はもはや inline 構築しない（DD-IT-8）。
/// スナップショットは Unloading へ遷移**後**の [`snapshot_of`]（＝INACTIVE）を渡す（DD-IT-4）。
fn force_quit(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "force_quit", reason = reason.as_ref_str(), "強制終了指示——終了系列（Forced）へ直行");
    state.phase = Phase::Unloading {
        cause: TermCause::Forced,
    };
    // close 系遷移の掃除点（C4 規則 7）: 現行トークは失われるため選択帳簿を残さない。
    clear_choice_ledger(&mut state, "force_quit");
    let notify = Action::ShioriRequest(events::on_close_notify(reason, &snapshot_of(&state.phase)));
    (state, vec![notify, Action::ShioriUnload])
}

/// 呼出失敗・死活報告の共通終端: Unloading{Fault}＋ShioriUnload（unload は best-effort）。
fn to_unloading_fault(mut state: State) -> (State, Vec<Action>) {
    state.phase = Phase::Unloading {
        cause: TermCause::Fault,
    };
    // close 系遷移の掃除点（C4 規則 7）。なお choice in-flight の `Failed` は本経路へ来ない
    // ——[`on_shiori_reply`] の先行アーム（DD-12）が steady へ委譲するためである。
    clear_choice_ledger(&mut state, "unloading_fault");
    (state, vec![Action::ShioriUnload])
}

/// reason（3 値）・talk_id 突合。既知 talk の `TalkEndReason::Quit` は横断的に終了系列（Quit）へ。
///
/// `Ended` と `Interrupted` はいずれも非 quit としてフェーズ固有遷移（定常復帰・close 終了拒否）
/// へ委譲する（設計「kanade schedule の 3 値写像」）。M1 には user-interrupt 配線が無く、かつ
/// dispatcher の slot 差替に伴う `Interrupted` は dispatcher が stale として破棄するため、
/// `Interrupted` が kanade まで到達することは想定されない。到達した場合も専用状態は起こさず
/// `Ended` と同一経路（非 quit）へ防御的に委譲し、`info!` でどの reason だったかを観測する。
fn on_talk_done(mut state: State, done: TalkDone, config: &KanadeConfig) -> (State, Vec<Action>) {
    match current_talk_id(&state.phase) {
        Some(active) if active == done.talk_id => {
            // 現 talk の完了に到達した時点で 1 世代 stale 帳簿の役目は終わる（C4 規則 9）。
            // 保持を延長すると「1 世代のみ」の契約が壊れ、真に未知の id まで info へ降格し得る。
            state.choice_prev_talk = None;
            match done.reason {
                TalkEndReason::Quit => {
                    // 既知 talk の Quit → 終了系列（Quit）へ直行（Req 4.3）。
                    tracing::info!(target: "kanade", event = "talk_done_quit", talk_id = done.talk_id.0, "reason=Quit——終了系列（Quit）へ");
                    state.phase = Phase::Unloading {
                        cause: TermCause::Quit,
                    };
                    // 対象トークの完了かつ close 系遷移の掃除点（C4 規則 7）。
                    clear_choice_ledger(&mut state, "talk_done_quit");
                    (state, vec![Action::ShioriUnload])
                }
                TalkEndReason::Interrupted => {
                    // 非 quit 扱い（観測用ログ）。本アームは元々「M1 では到達しない想定」の防御で
                    // あったが、**選択タイムアウトの解除経路により正規の到達点になった**（DD-11）:
                    // タイムアウト 204 → [`Action::CancelChoice`] → dispatcher が slot を維持したまま
                    // `Close` を転送 → talk が `TalkDone{Interrupted}` を正規送出 → ここへ到達 →
                    // フェーズ固有遷移（steady）が `Steady{None}` へ復帰させる（Req7.5）。
                    // 遷移・ログ語彙・レベルはいずれも無改変である（意味づけのみが変わった）。
                    tracing::info!(target: "kanade", event = "talk_done_interrupted_as_non_quit", talk_id = done.talk_id.0, "reason=Interrupted——非 quit 扱い・フェーズ固有遷移へ委譲（選択解除の正規到達点・DD-11）");
                    dispatch_phase(state, Input::TalkDone(done), config)
                }
                TalkEndReason::Ended => {
                    // Ended（定常復帰・close talk 完了）はフェーズ固有遷移へ委譲。
                    dispatch_phase(state, Input::TalkDone(done), config)
                }
            }
        }
        Some(_) | None => {
            // 1 世代 stale 防御（C4 規則 9・F1 残余レース・Req1.6）: choice 起因の slot 差替直後は、
            // 旧 talk の即時 `Done{Ended}`（再生層の即 settle）が dispatcher の slot 差替より前に
            // 投函され得る——この遅延 `Done` は**欠陥ではなく既知の順序レース**である。よって
            // 1 世代保持した旧 talk_id と照合し、一致するものは info で棄却して
            // `unknown_talk_done`（error）を真に未知の id 専用に保つ（正常系で error を出さない）。
            // 保持は消さない——消去点は現 talk の `TalkDone` 到達と次の slot 差替である（規則 9）。
            if state.choice_prev_talk == Some(done.talk_id) {
                tracing::info!(
                    target: "kanade",
                    event = "talk_done_stale_choice",
                    talk_id = done.talk_id.0,
                    "選択差替で置き換えた旧 talk の遅延完了通知——状態を変えず棄却（C4 規則 9・F1）"
                );
                return (state, Vec::new());
            }
            // 未知 talk_id の TalkDone → error!＋現 Phase 維持（Req 2.5・6.2）。
            tracing::error!(target: "kanade", event = "unknown_talk_done", talk_id = done.talk_id.0, "未知 talk_id の再生完了通知——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// ShioriReply の横断判定（Unload 完了・Failed）＋応答待ちフェーズ委譲。
///
/// `origin`（応答の出所イベント ID・DD-IE-3）は横断判定では使わないが、応答待ちフェーズへ
/// 委譲する際に `Input::ShioriReply` として保持したまま流す（タスク 2.2 のマウス GET origin 別
/// reply 政策がフェーズ固有遷移側で参照する）。
fn on_shiori_reply(
    state: State,
    outcome: ShioriOutcome,
    origin: &'static str,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    // Unloading 中の応答は Unload 完了として扱う。Unloaded／Failed のいずれも Stopped へ
    // 進む（Failed は error! の上で終了系列を継続・Error Handling「Unload 失敗」行）。
    if matches!(state.phase, Phase::Unloading { .. }) {
        return unloading_reply(state, outcome);
    }

    // prefetch 応答（username GET）は横断 Failed→Fault 経路に載せない（起動を殺さない・R4.1）。
    // Value/NoContent/Failed の全てを boot::step が [`resources::ResourceOutcome`] へ写像し、sink 呼出
    // 指示＋完了固定ログを添えて OnFirstBoot へ続行する。Failed の Fault 化を封じるため横断判定より先に捌く。
    if matches!(state.phase, Phase::BootPrefetch) {
        return boot::step(state, Input::ShioriReply { outcome, origin }, config);
    }

    // choice in-flight（カスケード段／タイムアウト GET の応答待ち）の応答は横断 Failed→Fault 経路に
    // 載せない（選択の失敗でゴーストを終了させない・Req4.5／DD-12・C4 規則 8）。prefetch の先行アームと
    // **同型**に、横断判定より先に steady へ委譲する。委譲先（steady の choice 先行アーム）が Failed を
    // `choice_shiori_failed_as_204`（error）記録の上で 204 と同一に扱い、会話を止めずに継続させる。
    //
    // 条件を `Steady` かつ in-flight（`Cascading|TimeoutInFlight`）に限るのが要点である: 選択待ち
    // （`Waiting`）中に届く Failed は pump／マウス由来であり、免除すると SHIORI 失敗時の終了規律
    // （Req6.1）が非 choice 経路まで緩む。非 choice 経路は無改変で従来どおり Unloading{Fault} へ倒れる。
    if matches!(state.phase, Phase::Steady { .. })
        && state
            .choice
            .as_ref()
            .is_some_and(|ledger| choice_in_flight(&ledger.phase))
    {
        return steady::step(state, Input::ShioriReply { outcome, origin }, config);
    }

    // 応答待ちフェーズでの Failed は横断的に Unloading{Fault} へ（Req 6.1）。
    if let ShioriOutcome::Failed(ref failure) = outcome
        && awaits_reply(&state.phase)
    {
        tracing::error!(target: "kanade", event = "shiori_failed", error = %failure, "SHIORI 呼出失敗——終了系列（Fault）へ");
        return to_unloading_fault(state);
    }

    // 応答待ちでない Phase への ShioriReply は構造上発生しない（防御アーム・Req 6.2）。
    if !awaits_reply(&state.phase) {
        tracing::warn!(target: "kanade", event = "unexpected_reply", "応答待ちでない Phase への SHIORI 応答を無視");
        return (state, Vec::new());
    }

    // 正常応答（Value／NoContent／Notified）は応答待ちフェーズ固有遷移へ委譲（origin を保持）。
    dispatch_phase(state, Input::ShioriReply { outcome, origin }, config)
}

/// Unloading 中の ShioriReply: Unloaded／Failed とも Stopped＋StopSelf へ（Failed は error!）。
fn unloading_reply(mut state: State, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    if let ShioriOutcome::Failed(ref failure) = outcome {
        tracing::error!(target: "kanade", event = "unload_failed", error = %failure, "Unload 失敗——終了系列は継続し停止する");
    }
    state.phase = Phase::Stopped;
    (state, vec![Action::StopSelf])
}

/// フェーズ固有遷移への委譲（boot／steady／close・後続タスクが本体を実装）。
fn dispatch_phase(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Idle
        | Phase::BootInit
        | Phase::BootPrefetch
        | Phase::BootType
        | Phase::BootMain
        | Phase::BootVersion { .. } => boot::step(state, input, config),
        Phase::Steady { .. } => steady::step(state, input, config),
        Phase::ClosePending { .. } | Phase::CloseTalkWait { .. } => {
            close::step(state, input, config)
        }
        // 終了系列（Unloading／Stopped）に届いた非横断入力は防御的に無視する。
        Phase::Unloading { .. } | Phase::Stopped => {
            tracing::warn!(target: "kanade", event = "input_after_terminate", "終了系列で受領した入力を無視");
            (state, Vec::new())
        }
    }
}

/// 現フェーズが応答待ち（直前に GET/NOTIFY/unload を発行済み）かを判定する。
///
/// `Steady` も応答待ちに含める: Steady の Tick は `OnSecondChange`（GET/NOTIFY）を発行し、
/// その応答をなお `Steady` のまま受ける（in-flight ≤ 1・シェルが直後に `ShioriReply` を
/// 再投入する）。ここに Steady が無いと Steady の `ShioriReply` が防御アーム
/// （unexpected_reply）で握り潰され、OnSecondChange の Value/NoContent 処理が壊れる。
/// これにより Steady の Failed は `on_shiori_reply` 経由で `Unloading{Fault}` へ（Req 6.1・
/// pump 含む任意の SHIORI 失敗で M1 は放棄）、Value/NoContent/Notified は `dispatch_phase`
/// → `steady::step` へ正しく委譲される。
fn awaits_reply(phase: &Phase) -> bool {
    matches!(
        phase,
        Phase::BootInit
            | Phase::BootType
            | Phase::BootMain
            | Phase::BootVersion { .. }
            | Phase::Steady { .. }
            | Phase::ClosePending { .. }
    )
}

/// 現フェーズが突合対象とする active talk の talk_id（無ければ None）。
fn current_talk_id(phase: &Phase) -> Option<TalkId> {
    match phase {
        // 挨拶追跡中の BootVersion も突合対象に含める（TalkDone が BootVersion 中に届いた場合の
        // 防御・DD-IT-12）。主要な突合は BootVersion→Steady 完了後の Steady{Some} で成立する。
        Phase::Steady {
            talk: Some(active), ..
        }
        | Phase::BootVersion {
            talk: Some(active), ..
        } => Some(active.talk_id),
        Phase::CloseTalkWait { talk_id, .. } => Some(*talk_id),
        _ => None,
    }
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;

/// タスク 6.1: 純粋 step 層の失敗・防御アームがログを発火することの実行可能検証。
///
/// Req 6.3（ログ無しの失敗経路を持たない）・Req 6.1（区別語彙ごとにログ）を、コードレビュー
/// でなく**テスト**で担保する。各 `error!` / `warn!` アームを `step()`（または各サブモジュール
/// `step`）で駆動し、`log_capture` で捕捉したイベントに `target="kanade"`・所定の `event`・所定
/// レベル（ERROR/WARN）が存在することを表明する。ログが除去・語彙変更・レベル変更されると当該
/// テストが失敗する。
///
/// ルーティング上 `step()` からは到達不能な防御アーム（構造上発生しない系）は、当該サブモジュール
/// の `step` を直接駆動して検証する（各テストのコメントで明示）。これらは「あり得ない Phase/入力の
/// 組」に対する防御であり、直接駆動が唯一かつ正当な網羅手段である。
#[cfg(test)]
#[path = "schedule_log_firing_tests.rs"]
mod log_firing_tests;
