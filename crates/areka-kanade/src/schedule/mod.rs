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
/// タスク 6.1: 純粋 step 層の失敗・防御アームのログ発火検証（テスト専用）。
#[cfg(test)]
pub(crate) mod log_capture;
/// ukadoc Reference 表の実装正本（純粋関数群）。DD-9 の例外として `pub`。
/// クレート公開面への露出は [`crate::events`] ファサード経由（[`crate::lib`] 参照）。
pub mod events;
/// SHIORI Resource 照会の許可集合（イベント檻とは別族・Req4.1）。DD-9 の例外として `pub`。
/// submit ガードはイベント許可 ∨ リソース許可で判定する（`crate::actor` の egress チョークポイント）。
pub mod resources;
pub(crate) mod steady;

/// 状態機械への入力。`KanadeMsg`（外部入力）＋シェルが同期往復で得た SHIORI 応答。
/// `ShioriReply` が `KanadeMsg` に存在しないため、応答注入経路はシェル内部に閉じる。
pub(crate) enum Input {
    Boot,
    Tick { now: MonotonicMs },
    TalkDone(TalkDone),
    CloseRequest { reason: CloseReason },
    ForceQuit { reason: CloseReason },
    ShioriDown { reason: String },
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
    BootVersion { talk: Option<ActiveTalk> },
    Steady { talk: Option<ActiveTalk> },
    ClosePending { reason: CloseReason },
    CloseTalkWait { talk_id: TalkId, deadline: Option<MonotonicMs> },
    Unloading { cause: TermCause },
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
    /// `talk_done_stale_choice`（info）で捌くための照合先（掃除規律はタスク 4.6）。
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
    ResolveChoice { talk_id: TalkId, id: String },
    /// 選択待ちの解除＋トーク終了指示（→ [`TalkCommand::CancelChoice`](crate::talk::TalkCommand)）。
    ///
    /// タイムアウト後に SHIORI が応答を返さなかった場合の解除（Req7.5）。発行点はタスク 4.5。
    CancelChoice { talk_id: TalkId },
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
pub(crate) fn snapshot_of(phase: &Phase) -> ExecutionSnapshot {
    match phase {
        // アクティブな talk を運ぶ phase＝Steady{Some} と（挨拶追跡中の）BootVersion{Some}（DD-IT-12）。
        Phase::Steady { talk: Some(_) } | Phase::BootVersion { talk: Some(_) } => {
            ExecutionSnapshot { talk_active: true }
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
    let notify =
        Action::ShioriRequest(events::on_close_notify(reason, &snapshot_of(&state.phase)));
    (state, vec![notify, Action::ShioriUnload])
}

/// 呼出失敗・死活報告の共通終端: Unloading{Fault}＋ShioriUnload（unload は best-effort）。
fn to_unloading_fault(mut state: State) -> (State, Vec<Action>) {
    state.phase = Phase::Unloading {
        cause: TermCause::Fault,
    };
    (state, vec![Action::ShioriUnload])
}

/// reason（3 値）・talk_id 突合。既知 talk の `TalkEndReason::Quit` は横断的に終了系列（Quit）へ。
///
/// `Ended` と `Interrupted` はいずれも非 quit としてフェーズ固有遷移（定常復帰・close 終了拒否）
/// へ委譲する（設計「kanade schedule の 3 値写像」）。M1 には user-interrupt 配線が無く、かつ
/// dispatcher の slot 差替に伴う `Interrupted` は dispatcher が stale として破棄するため、
/// `Interrupted` が kanade まで到達することは想定されない。到達した場合も専用状態は起こさず
/// `Ended` と同一経路（非 quit）へ防御的に委譲し、`info!` でどの reason だったかを観測する。
fn on_talk_done(state: State, done: TalkDone, config: &KanadeConfig) -> (State, Vec<Action>) {
    match current_talk_id(&state.phase) {
        Some(active) if active == done.talk_id => match done.reason {
            TalkEndReason::Quit => {
                // 既知 talk の Quit → 終了系列（Quit）へ直行（Req 4.3）。
                let mut state = state;
                tracing::info!(target: "kanade", event = "talk_done_quit", talk_id = done.talk_id.0, "reason=Quit——終了系列（Quit）へ");
                state.phase = Phase::Unloading {
                    cause: TermCause::Quit,
                };
                (state, vec![Action::ShioriUnload])
            }
            TalkEndReason::Interrupted => {
                // 防御的に非 quit 扱い（M1 では到達しない想定・観測用ログ）。
                tracing::info!(target: "kanade", event = "talk_done_interrupted_as_non_quit", talk_id = done.talk_id.0, "reason=Interrupted——防御的に非 quit 扱い・フェーズ固有遷移へ委譲");
                dispatch_phase(state, Input::TalkDone(done), config)
            }
            TalkEndReason::Ended => {
                // Ended（定常復帰・close talk 完了）はフェーズ固有遷移へ委譲。
                dispatch_phase(state, Input::TalkDone(done), config)
            }
        },
        Some(_) | None => {
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
mod tests {
    use super::*;
    use crate::msg::ShioriFailure;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    fn state_in(phase: Phase) -> State {
        State {
            phase,
            last_now: Some(MonotonicMs(1_000)),
            next_talk_id: 5,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        }
    }

    fn steady_with_talk(talk_id: TalkId) -> Phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id,
                origin: "steady",
                script: String::new(),
            }),
        }
    }

    // --- 1. TalkDone{reason:Quit} for a KNOWN talk → Unloading{Quit} + [ShioriUnload] ---

    #[test]
    fn known_quit_from_steady_goes_to_unloading_quit() {
        let phase = steady_with_talk(TalkId(5));
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                reason: TalkEndReason::Quit,
            }),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Quit }));
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], Action::ShioriUnload));
    }

    #[test]
    fn known_quit_from_close_talk_wait_goes_to_unloading_quit() {
        let phase = Phase::CloseTalkWait {
            talk_id: TalkId(9),
            deadline: None,
        };
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(9),
                reason: TalkEndReason::Quit,
            }),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Quit }));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    }

    // --- 1b. TalkDone{reason:Interrupted} for a KNOWN talk → 非 quit 扱い（Ended と同一経路） ---

    #[test]
    fn known_interrupted_from_steady_is_routed_as_non_quit() {
        // Interrupted は防御的に非 quit 扱い（Ended と同一経路）へ委譲される。
        // steady::on_talk_done は pending_close が無ければ Steady{None} へ復帰する。
        let phase = steady_with_talk(TalkId(5));
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                reason: TalkEndReason::Interrupted,
            }),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: None }),
            "Interrupted は Quit 系列へ進まず、Ended と同じ定常復帰へ"
        );
        assert!(actions.is_empty());
    }

    // --- 2. ForceQuit from any phase → [Notify OnClose, ShioriUnload] + Unloading{Forced} ---

    #[test]
    fn force_quit_emits_onclose_notify_then_unload() {
        let (next, actions) = step(
            state_in(Phase::BootMain),
            Input::ForceQuit {
                reason: CloseReason::System,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Forced }));
        assert_eq!(actions.len(), 2);
        match &actions[0] {
            Action::ShioriRequest(ShioriCall::Notify { id, references, .. }) => {
                assert_eq!(id.as_str(), "OnClose");
                assert_eq!(references, &vec!["system".to_string()]);
            }
            _ => panic!("expected first action to be Notify OnClose"),
        }
        assert!(matches!(actions[1], Action::ShioriUnload));
    }

    // --- 3. ShioriDown from any phase → Unloading{Fault} + [ShioriUnload] ---

    #[test]
    fn shiori_down_goes_to_unloading_fault() {
        let (next, actions) = step(
            state_in(steady_with_talk(TalkId(5))),
            Input::ShioriDown {
                reason: "helper crashed".to_string(),
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Fault }));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    }

    // --- 4. ShioriReply{Failed} from a waiting phase → Unloading{Fault} + [ShioriUnload] ---

    #[test]
    fn shiori_reply_failed_goes_to_unloading_fault() {
        let (next, actions) = step(
            state_in(Phase::BootType),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "test",
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Unloading { cause: TermCause::Fault }));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
    }

    // --- 5. Unknown talk_id TalkDone → phase unchanged, no actions ---

    #[test]
    fn unknown_talk_id_keeps_phase_and_emits_nothing() {
        let phase = steady_with_talk(TalkId(5));
        let (next, actions) = step(
            state_in(phase),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(999),
                reason: TalkEndReason::Quit,
            }),
            &config(),
        );
        // 未知 talk_id は reason=Quit でも横断遷移させず、現 Phase を維持する。
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(5)),
            _ => panic!("expected Steady phase to be preserved"),
        }
        assert!(actions.is_empty());
    }

    // --- 6. Non-Idle Boot → phase unchanged, no actions ---

    #[test]
    fn boot_in_non_idle_phase_is_ignored() {
        let (next, actions) = step(state_in(Phase::BootMain), Input::Boot, &config());
        assert!(matches!(next.phase, Phase::BootMain));
        assert!(actions.is_empty());
    }

    // --- 7. Defensive: ShioriReply when phase does not await a reply → unchanged ---

    #[test]
    fn shiori_reply_when_not_waiting_is_ignored() {
        let (next, actions) = step(
            state_in(Phase::Idle),
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Idle));
        assert!(actions.is_empty());
    }

    // --- 8. Unloading + ShioriReply{Unloaded|Failed} → Stopped + [StopSelf] ---

    #[test]
    fn unloading_reply_unloaded_goes_to_stopped() {
        let (next, actions) = step(
            state_in(Phase::Unloading {
                cause: TermCause::Quit,
            }),
            Input::ShioriReply {
                outcome: ShioriOutcome::Unloaded,
                origin: "test",
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Stopped));
        assert!(matches!(actions.as_slice(), [Action::StopSelf]));
    }

    #[test]
    fn unloading_reply_failed_still_goes_to_stopped() {
        let (next, actions) = step(
            state_in(Phase::Unloading {
                cause: TermCause::Forced,
            }),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
                origin: "test",
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Stopped));
        assert!(matches!(actions.as_slice(), [Action::StopSelf]));
    }

    // --- 9. State::initial() invariants ---

    #[test]
    fn initial_state_is_idle_with_monotonic_counter() {
        let s = State::initial();
        assert!(matches!(s.phase, Phase::Idle));
        assert_eq!(s.last_now, None);
        assert_eq!(s.next_talk_id, 1);
        assert!(s.pending_close.is_none());
    }

    // --- 10. Mouse 横断ルーティング（Task 1・DD-IE-8） ---

    use crate::msg::{MouseEventKind, MouseInput};

    fn mouse_move() -> MouseInput {
        MouseInput {
            scope: 0,
            x: 10,
            y: 20,
            region: Some("head".to_string()),
            kind: MouseEventKind::Move,
        }
    }

    // 非 Steady フェーズ（boot／close／terminate 後）への Mouse は状態不変・SHIORI 問い合わせ
    // 一切なし（DD-IE-8: マウスは Steady でのみ受理・他は安全に無視）。
    #[test]
    fn mouse_input_in_non_steady_phases_is_ignored() {
        for phase in [
            Phase::Idle,
            Phase::BootMain,
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Phase::Stopped,
        ] {
            let before = std::mem::discriminant(&phase);
            let (next, actions) = step(state_in(phase), Input::Mouse(mouse_move()), &config());
            assert_eq!(
                std::mem::discriminant(&next.phase),
                before,
                "非 Steady フェーズでは Mouse で phase が変わらない"
            );
            assert!(
                actions.is_empty(),
                "非 Steady フェーズでは Mouse で Action（SHIORI 問い合わせ含む）を発行しない"
            );
        }
    }

    /// 単一 Action が期待 GET（id・references・status）と厳密一致することを検証する。
    /// ShioriCall は PartialEq を持たないため field 単位で比較する。
    fn assert_get(action: &Action, expected: &ShioriCall) {
        match (action, expected) {
            (
                Action::ShioriRequest(ShioriCall::Get { id, references, status }),
                ShioriCall::Get { id: eid, references: erefs, status: estatus },
            ) => {
                assert_eq!(id, eid, "GET id 不一致");
                assert_eq!(references, erefs, "GET references 不一致");
                assert_eq!(status.render(), estatus.render(), "GET status 不一致");
            }
            _ => panic!("期待した GET でない Action"),
        }
    }

    // Steady では横断アームが steady::on_mouse へ委譲し、Task 2.2 の実装で GET を発行する
    // （seam の意図的充填）。step() 経由（横断アーム込み）で GET が 1 件出ることを固定する。
    #[test]
    fn mouse_input_in_steady_emits_get_via_crosscutting_arm() {
        // Steady{None} + Move → OnMouseMove GET（Status 行なし・INACTIVE）。phase 不変。
        let (next, actions) = step(
            state_in(Phase::Steady { talk: None }),
            Input::Mouse(mouse_move()),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }), "マウス GET は phase を変えない");
        assert_eq!(actions.len(), 1, "マウス入力で GET を 1 件発行する（Task 2.2）");
        assert_get(
            &actions[0],
            &events::on_mouse_move(10, 20, 0, Some("head"), &ExecutionSnapshot::INACTIVE),
        );

        // Steady{Some} + Move → GET のまま発行され Status: talking を帯びる（DD-IE-1）。phase 不変。
        let (next, actions) = step(
            state_in(steady_with_talk(TalkId(5))),
            Input::Mouse(mouse_move()),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: Some(_) }), "active talk は維持される");
        assert_eq!(actions.len(), 1);
        assert_get(
            &actions[0],
            &events::on_mouse_move(10, 20, 0, Some("head"), &ExecutionSnapshot { talk_active: true }),
        );
    }

    // close 保留中は Steady であってもマウス GET を発行しない防御（構造的シーム・DD-IE-8）。
    #[test]
    fn mouse_input_in_steady_with_pending_close_emits_no_get() {
        let mut s = state_in(Phase::Steady { talk: None });
        s.pending_close = Some(CloseReason::System);
        let (next, actions) = step(s, Input::Mouse(mouse_move()), &config());
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert!(
            matches!(next.pending_close, Some(CloseReason::System)),
            "guard は pending_close を消費しない"
        );
        assert!(actions.is_empty(), "close 保留中はマウス GET を発行しない");
    }

    // ============================================================
    // 11. 選択系の additive 追加（タスク 4.1・Req4.4・DD-3／DD-10）
    // ============================================================

    /// 檻用の選択確定入力（内容は本檻で load-bearing でない＝写像・帳簿の存在のみを見る）。
    fn choice_input() -> ChoiceInput {
        ChoiceInput {
            id: "OnMenu".to_string(),
            label: "メニュー".to_string(),
            scope: 0,
            references: vec!["a0".to_string()],
        }
    }

    /// 檻用の選択待ち通知入力（同上）。
    fn choice_waiting_input() -> Input {
        Input::ChoiceWaiting {
            talk_id: TalkId(5),
            choice_ids: vec!["OnMenu".to_string()],
            display_end: MonotonicMs(2_000),
            timeout_directive_secs: None,
        }
    }

    /// Req4.4: 既存 `Phase` の 11 variant が無改変であること。
    ///
    /// 本 match は wildcard を持たないため、variant の削除・改名・形（フィールド構成）の
    /// 変更はコンパイルを壊す。DD-3 が要求する「Phase を一切触らない」を構造で固定する
    /// （`State.choice` は Phase の外＝`pending_close` と同型に置かれる）。
    #[test]
    fn existing_phase_variants_are_unchanged() {
        fn tag(phase: &Phase) -> &'static str {
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
        assert_eq!(tag(&Phase::Idle), "Idle");
        assert_eq!(tag(&Phase::Steady { talk: None }), "Steady");
        assert_eq!(tag(&Phase::Stopped), "Stopped");
    }

    /// Req4.4: 既存 `Action` 5 variant が無改変で、選択系 2 variant が additive に増えたこと。
    ///
    /// wildcard なしの網羅 match ゆえ、既存 5 variant のいずれかが消える／改名される／
    /// 形が変わると本檻はコンパイルできない。
    #[test]
    fn action_variants_are_existing_five_plus_choice_two() {
        fn tag(action: &Action) -> &'static str {
            match action {
                Action::ShioriRequest(_) => "ShioriRequest",
                Action::ShioriUnload => "ShioriUnload",
                Action::StartTalk(_) => "StartTalk",
                Action::ResourceOutcome { .. } => "ResourceOutcome",
                Action::StopSelf => "StopSelf",
                Action::ResolveChoice { .. } => "ResolveChoice",
                Action::CancelChoice { .. } => "CancelChoice",
            }
        }
        assert_eq!(tag(&Action::ShioriUnload), "ShioriUnload");
        assert_eq!(tag(&Action::StopSelf), "StopSelf");
        assert_eq!(
            tag(&Action::ResolveChoice {
                talk_id: TalkId(5),
                id: "OnMenu".to_string(),
            }),
            "ResolveChoice"
        );
        assert_eq!(
            tag(&Action::CancelChoice { talk_id: TalkId(5) }),
            "CancelChoice"
        );
    }

    /// Req4.4: 既存 `Input` 8 variant が無改変で、選択系 2 variant が additive に増えたこと。
    #[test]
    fn input_variants_are_existing_eight_plus_choice_two() {
        fn tag(input: &Input) -> &'static str {
            match input {
                Input::Boot => "Boot",
                Input::Tick { .. } => "Tick",
                Input::TalkDone(_) => "TalkDone",
                Input::CloseRequest { .. } => "CloseRequest",
                Input::ForceQuit { .. } => "ForceQuit",
                Input::ShioriDown { .. } => "ShioriDown",
                Input::Mouse(_) => "Mouse",
                Input::ShioriReply { .. } => "ShioriReply",
                Input::Choice(_) => "Choice",
                Input::ChoiceWaiting { .. } => "ChoiceWaiting",
            }
        }
        assert_eq!(tag(&Input::Boot), "Boot");
        assert_eq!(tag(&Input::Mouse(mouse_move())), "Mouse");
        assert_eq!(tag(&Input::Choice(choice_input())), "Choice");
        assert_eq!(tag(&choice_waiting_input()), "ChoiceWaiting");
    }

    /// DD-3: 選択帳簿は `State`（Phase 外）に置かれ、初期値は両方とも空である。
    #[test]
    fn initial_state_has_empty_choice_ledger() {
        let s = State::initial();
        assert!(s.choice.is_none(), "初期状態に選択待ち帳簿は無い");
        assert!(
            s.choice_prev_talk.is_none(),
            "初期状態に 1 世代保持の旧 talk_id は無い"
        );
    }

    /// C4 規則 1・Req1.3: 非 Steady フェーズの選択確定は棄却する（状態不変・Action なし）。
    ///
    /// 受理すべき選択待ちが構造上存在しないフェーズ（boot 中・close 握手以降・終了系列）へ
    /// 届いた選択確定は、帳簿の有無に関わらず横断アームで棄却される。棄却は既存帳簿にも
    /// 触れない（棄却の定義＝状態不変）。
    #[test]
    fn choice_input_in_non_steady_phases_is_rejected_without_changing_state() {
        for phase in [
            Phase::Idle,
            Phase::BootMain,
            Phase::BootVersion {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(5),
                    origin: "boot",
                    script: String::new(),
                }),
            },
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Phase::Stopped,
        ] {
            let before = std::mem::discriminant(&phase);
            let mut s = state_in(phase);
            s.choice = Some(ChoiceState {
                talk_id: TalkId(5),
                candidates: vec!["OnMenu".to_string()],
                deadline: None,
                phase: ChoicePhase::Waiting,
            });
            let (next, actions) = step(s, Input::Choice(choice_input()), &config());
            assert_eq!(
                std::mem::discriminant(&next.phase),
                before,
                "棄却は phase を変えない"
            );
            let ledger = next.choice.expect("棄却は既存帳簿にも触れない");
            assert!(matches!(ledger.phase, ChoicePhase::Waiting));
            assert!(actions.is_empty(), "棄却は Action を発行しない");
        }
    }

    // ============================================================
    // 12. 選択待ち通知の受領と帳簿確立（タスク 4.2・Req7.1／7.6／7.7・C4 規則 4・DD-8）
    // ============================================================

    /// 檻用の選択待ち通知（候補列・起点・指令を明示して組む）。
    fn choice_waiting_of(
        talk_id: TalkId,
        choice_ids: &[&str],
        display_end: MonotonicMs,
        timeout_directive_secs: Option<f64>,
    ) -> Input {
        Input::ChoiceWaiting {
            talk_id,
            choice_ids: choice_ids.iter().map(|s| s.to_string()).collect(),
            display_end,
            timeout_directive_secs,
        }
    }

    /// C4 規則 4・Req7.1: 現行トークと識別子が一致する通知は帳簿を確立する。
    ///
    /// 確立された帳簿は通知の候補列を表示順のまま保持し、期限は DD-8 写像済みの値
    /// （未指定→起点＋既定 30_000ms）を持ち、段フェーズは `Waiting` である。`Phase` は
    /// 一切触らない（DD-3）。
    #[test]
    fn choice_waiting_matching_talk_id_establishes_waiting_ledger() {
        let (next, actions) = step(
            state_in(steady_with_talk(TalkId(5))),
            choice_waiting_of(
                TalkId(5),
                &["OnMenu", "choice1"],
                MonotonicMs(2_000),
                None,
            ),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: Some(_) }),
            "帳簿確立は Phase を触らない（DD-3）"
        );
        let ledger = next.choice.expect("一致通知は帳簿を確立する");
        assert_eq!(ledger.talk_id, TalkId(5));
        assert_eq!(
            ledger.candidates,
            vec!["OnMenu".to_string(), "choice1".to_string()],
            "候補列は通知どおり（表示順を保存・DD-7）"
        );
        assert_eq!(
            ledger.deadline,
            Some(MonotonicMs(32_000)),
            "未指定は既定 30_000ms を起点へ加算（DD-8）"
        );
        assert!(matches!(ledger.phase, ChoicePhase::Waiting));
        assert!(actions.is_empty(), "帳簿確立は Action を発行しない");
    }

    /// Req7.1: 通知受領後の帳簿が「選択確定を受理できる」前提を満たす。
    ///
    /// 受理判定そのものはタスク 4.3 の担当であり、本檻はその判定が読む 3 条件——段フェーズが
    /// `Waiting`・候補列が照合可能な形で保持されている・帳簿の `talk_id` が**現行 talk と一致**
    /// （`ChoiceState` の不変条件）——を状態で固定する。
    #[test]
    fn established_ledger_satisfies_preconditions_for_choice_acceptance() {
        let (next, _) = step(
            state_in(steady_with_talk(TalkId(5))),
            choice_waiting_of(TalkId(5), &["OnMenu", "choice1"], MonotonicMs(2_000), None),
            &config(),
        );
        let active_talk_id = current_talk_id(&next.phase).expect("active talk は維持される");
        let ledger = next.choice.expect("一致通知は帳簿を確立する");
        assert!(
            matches!(ledger.phase, ChoicePhase::Waiting),
            "受理可能な段フェーズ（入力待ち）である"
        );
        assert_eq!(
            ledger.talk_id, active_talk_id,
            "帳簿の talk_id は現行 talk と一致する（ChoiceState の不変条件）"
        );
        assert!(
            ledger.candidates.iter().any(|c| c == "choice1"),
            "候補 ID は照合可能な形で保持される（4.3 の候補集合照合の前提）"
        );
    }

    /// DD-8・Req7.6／7.7: 期限は指令 3 値語彙どおりに写る（実値で突合）。
    ///
    /// 既定値以外の明示秒指定も同一規律で写る（Req7.7）。無効化（0／-1）は期限なし＝無期限
    /// であり、帳簿自体は確立される（Req7.6：計測を開始しないだけで選択待ちは継続する）。
    #[test]
    fn choice_waiting_deadline_follows_dd8_directive_mapping() {
        let display_end = MonotonicMs(2_000);
        let cases: [(Option<f64>, Option<MonotonicMs>); 5] = [
            // 未指定＝既定へ委譲（config.choice_timeout_default_ms = 30_000）。
            (None, Some(MonotonicMs(32_000))),
            // 無効化＝無期限（Req7.6）。
            (Some(0.0), None),
            (Some(-1.0), None),
            // 明示秒指定（Req7.7・既定値は関与しない）。
            (Some(5.0), Some(MonotonicMs(7_000))),
            (Some(2.5), Some(MonotonicMs(4_500))),
        ];
        for (directive, expected) in cases {
            let (next, _) = step(
                state_in(steady_with_talk(TalkId(5))),
                choice_waiting_of(TalkId(5), &["OnMenu"], display_end, directive),
                &config(),
            );
            let ledger = next
                .choice
                .unwrap_or_else(|| panic!("指令 {directive:?} でも帳簿は確立される"));
            assert_eq!(
                ledger.deadline, expected,
                "指令 {directive:?} の期限写像が DD-8 と一致しない"
            );
        }
    }

    /// C4 規則 4・Req7.1: 識別子が現行トークと一致しない通知は帳簿を確立しない。
    #[test]
    fn choice_waiting_with_mismatched_talk_id_does_not_establish_ledger() {
        let (next, actions) = step(
            state_in(steady_with_talk(TalkId(5))),
            choice_waiting_of(TalkId(999), &["OnMenu"], MonotonicMs(2_000), None),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: Some(_) }),
            "棄却は Phase を触らない"
        );
        assert!(next.choice.is_none(), "不一致通知は帳簿を確立しない");
        assert!(actions.is_empty(), "棄却は Action を発行しない");
    }

    /// C4 規則 4: 再生中でない Steady（`Steady{None}`）の通知は受理しない。
    #[test]
    fn choice_waiting_without_active_talk_does_not_establish_ledger() {
        let (next, actions) = step(
            state_in(Phase::Steady { talk: None }),
            choice_waiting_of(TalkId(5), &["OnMenu"], MonotonicMs(2_000), None),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert!(next.choice.is_none(), "active talk 不在では帳簿を確立しない");
        assert!(actions.is_empty());
    }

    /// C4 規則 4: 非 Steady フェーズの通知は受理しない（挨拶追跡中の `BootVersion{Some}` も含む）。
    #[test]
    fn choice_waiting_in_non_steady_phase_does_not_establish_ledger() {
        for phase in [
            Phase::Idle,
            Phase::BootMain,
            Phase::BootVersion {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(5),
                    origin: "boot",
                    script: String::new(),
                }),
            },
            Phase::ClosePending {
                reason: CloseReason::User,
            },
        ] {
            let before = std::mem::discriminant(&phase);
            let (next, actions) = step(
                state_in(phase),
                choice_waiting_of(TalkId(5), &["OnMenu"], MonotonicMs(2_000), None),
                &config(),
            );
            assert_eq!(
                std::mem::discriminant(&next.phase),
                before,
                "棄却は phase を変えない"
            );
            assert!(next.choice.is_none(), "非 Steady では帳簿を確立しない");
            assert!(actions.is_empty());
        }
    }

    /// C4 規則 4: 棄却は**既存の帳簿にも触れない**（状態不変が棄却の定義）。
    #[test]
    fn stale_choice_waiting_leaves_existing_ledger_untouched() {
        let mut s = state_in(steady_with_talk(TalkId(5)));
        s.choice = Some(ChoiceState {
            talk_id: TalkId(5),
            candidates: vec!["existing".to_string()],
            deadline: Some(MonotonicMs(9_000)),
            phase: ChoicePhase::Waiting,
        });
        let (next, actions) = step(
            s,
            choice_waiting_of(TalkId(999), &["OnMenu"], MonotonicMs(2_000), None),
            &config(),
        );
        let ledger = next.choice.expect("既存帳簿は棄却で消えない");
        assert_eq!(ledger.talk_id, TalkId(5));
        assert_eq!(ledger.candidates, vec!["existing".to_string()]);
        assert_eq!(ledger.deadline, Some(MonotonicMs(9_000)));
        assert!(actions.is_empty());
    }
}

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
mod log_firing_tests {
    use super::log_capture::{assert_logged, capture, logged_once, CapturedEvent};
    use super::*;
    use crate::msg::{CloseReason, ShioriFailure};
    use crate::talk::TalkDone;
    use tracing::Level;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    fn state_in(phase: Phase) -> State {
        State {
            phase,
            last_now: Some(MonotonicMs(1_000)),
            next_talk_id: 5,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        }
    }

    fn steady_with_talk(talk_id: TalkId) -> Phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id,
                origin: "steady",
                script: String::new(),
            }),
        }
    }

    /// `step()` を捕捉付きで駆動し、発行イベント列を返す（state は move）。
    fn run_step(phase: Phase, input: Input) -> Vec<CapturedEvent> {
        let cfg = config();
        capture(|| {
            let _ = step(state_in(phase), input, &cfg);
        })
    }

    // ============================================================
    // 失敗アーム（level = ERROR）
    // ============================================================

    #[test]
    fn error_shiori_down_logs() {
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::ShioriDown {
                reason: "helper crashed".to_string(),
            },
        );
        assert_logged(&ev, Level::ERROR, "shiori_down");
    }

    #[test]
    fn error_shiori_failed_logs() {
        // 応答待ちフェーズ（BootType）+ Failed → 横断アーム shiori_failed。
        let ev = run_step(
            Phase::BootType,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "test",
            },
        );
        assert_logged(&ev, Level::ERROR, "shiori_failed");
    }

    #[test]
    fn error_unknown_talk_done_logs() {
        // 突合対象 talk (5) と異なる talk_id (999) の TalkDone → unknown_talk_done。
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(999),
                reason: TalkEndReason::Ended,
            }),
        );
        assert_logged(&ev, Level::ERROR, "unknown_talk_done");
    }

    #[test]
    fn error_unload_failed_logs() {
        // Unloading 中の Failed 応答 → unload_failed（終了系列は継続）。
        let ev = run_step(
            Phase::Unloading {
                cause: TermCause::Quit,
            },
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
                origin: "test",
            },
        );
        assert_logged(&ev, Level::ERROR, "unload_failed");
    }

    #[test]
    fn error_close_deadline_exceeded_logs() {
        // CloseTalkWait で deadline 超過 Tick → close_deadline_exceeded。
        let cfg = config();
        let s = State {
            phase: Phase::CloseTalkWait {
                talk_id: TalkId(7),
                deadline: Some(MonotonicMs(1_000)),
            },
            last_now: Some(MonotonicMs(900)),
            next_talk_id: 8,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        };
        let ev = capture(|| {
            let _ = step(s, Input::Tick { now: MonotonicMs(2_000) }, &cfg);
        });
        assert_logged(&ev, Level::ERROR, "close_deadline_exceeded");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— mod.rs
    // ============================================================

    #[test]
    fn warn_force_quit_logs() {
        let ev = run_step(
            Phase::BootMain,
            Input::ForceQuit {
                reason: CloseReason::System,
            },
        );
        assert_logged(&ev, Level::WARN, "force_quit");
    }

    #[test]
    fn warn_boot_ignored_logs() {
        // 非 Idle での Boot → mod.rs boot_ignored。
        let ev = run_step(Phase::BootMain, Input::Boot);
        assert_logged(&ev, Level::WARN, "boot_ignored");
    }

    #[test]
    fn warn_unexpected_reply_logs() {
        // 応答待ちでない Phase（Idle）への ShioriReply → mod.rs unexpected_reply。
        let ev = run_step(
            Phase::Idle,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
        );
        assert_logged(&ev, Level::WARN, "unexpected_reply");
    }

    #[test]
    fn warn_input_after_terminate_logs() {
        // 終了系列（Stopped）で受領した非横断入力（Tick）→ dispatch_phase input_after_terminate。
        let ev = run_step(Phase::Stopped, Input::Tick { now: MonotonicMs(1_000) });
        assert_logged(&ev, Level::WARN, "input_after_terminate");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— boot.rs
    // ============================================================

    #[test]
    fn warn_boot_input_ignored_logs() {
        // boot フェーズ（BootInit）+ boot 無関係入力（Tick）→ dispatch_phase→boot::step _ アーム。
        let ev = run_step(Phase::BootInit, Input::Tick { now: MonotonicMs(1_000) });
        assert_logged(&ev, Level::WARN, "boot_input_ignored");
    }

    #[test]
    fn warn_boot_unexpected_reply_logs() {
        // BootInit（Notified 待ち）に Value → boot::on_reply unexpected_reply。
        let ev = run_step(
            Phase::BootInit,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("unexpected".to_string()),
                origin: "test",
            },
        );
        assert_logged(&ev, Level::WARN, "boot_unexpected_reply");
    }

    #[test]
    fn warn_boot_reply_ignored_logs() {
        // boot::on_reply の防御 `_` アーム（応答待ちでない boot Phase）。step() 経由では
        // Idle が awaits_reply=false ゆえ mod.rs 側で握り潰され到達しない。構造上発生しない
        // 防御アームゆえ boot::step を直接駆動して検証する（唯一の網羅手段）。
        let cfg = config();
        let ev = capture(|| {
            let _ = boot::step(
                state_in(Phase::Idle),
                Input::ShioriReply {
                    outcome: ShioriOutcome::Notified,
                    origin: "test",
                },
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "boot_reply_ignored");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— steady.rs
    // ============================================================

    #[test]
    fn warn_steady_value_during_talk_logs() {
        // Steady{Some} + 非マウス Value（DD-6 防御・narrowed）→ steady_value_during_talk。
        // origin は非マウス（OnSecondChange）——マウス origin は置換アームへ抜けて warn しない
        // ため、DD-6 破棄ログの発火には非マウス origin が必要（DD-IE-2 の意味の縮小）。
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("late".to_string()),
                origin: "OnSecondChange",
            },
        );
        assert_logged(&ev, Level::WARN, "steady_value_during_talk");
    }

    #[test]
    fn warn_steady_unexpected_reply_logs() {
        // Steady{None} + Unloaded（想定外の応答）→ steady_reply_unexpected。
        let ev = run_step(
            Phase::Steady { talk: None },
            Input::ShioriReply {
                outcome: ShioriOutcome::Unloaded,
                origin: "test",
            },
        );
        assert_logged(&ev, Level::WARN, "steady_unexpected_reply");
    }

    #[test]
    fn warn_steady_input_ignored_logs() {
        // steady::step の `_` アーム（Steady に無関係な入力）。step() 経由では Boot は mod.rs、
        // ForceQuit/ShioriDown は横断アームで捌かれ steady へ届かない。構造上発生しない防御
        // アームゆえ steady::step を直接駆動して検証する。
        let cfg = config();
        let ev = capture(|| {
            let _ = steady::step(
                state_in(Phase::Steady { talk: None }),
                Input::Boot,
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "steady_input_ignored");
    }

    #[test]
    fn warn_steady_phase_unexpected_logs() {
        // steady::step に非 Steady Phase（BootMain）が届いた場合の防御。ルーティング上
        // 到達不能ゆえ steady::step を直接駆動して検証する。
        let cfg = config();
        let ev = capture(|| {
            let _ = steady::step(
                state_in(Phase::BootMain),
                Input::Tick { now: MonotonicMs(1_000) },
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "steady_phase_unexpected");
    }

    // ============================================================
    // 防御 / 無視アーム（level = WARN）— close.rs
    // ============================================================

    #[test]
    fn warn_close_notified_unexpected_logs() {
        // ClosePending + Notified（OnClose は GET ゆえ構造上あり得ない）→ close_notified_unexpected。
        let ev = run_step(
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
        );
        assert_logged(&ev, Level::WARN, "close_notified_unexpected");
    }

    #[test]
    fn warn_close_reply_unexpected_logs() {
        // ClosePending + Unloaded（Value/NoContent/Notified 以外）→ close_reply_unexpected。
        let ev = run_step(
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Input::ShioriReply {
                outcome: ShioriOutcome::Unloaded,
                origin: "test",
            },
        );
        assert_logged(&ev, Level::WARN, "close_reply_unexpected");
    }

    #[test]
    fn warn_close_pending_input_ignored_logs() {
        // ClosePending + 無関係入力（CloseRequest）→ close_pending_input_ignored。
        let ev = run_step(
            Phase::ClosePending {
                reason: CloseReason::User,
            },
            Input::CloseRequest {
                reason: CloseReason::User,
            },
        );
        assert_logged(&ev, Level::WARN, "close_pending_input_ignored");
    }

    #[test]
    fn warn_close_talk_wait_input_ignored_logs() {
        // CloseTalkWait + 無関係入力（CloseRequest）→ close_talk_wait_input_ignored。
        let ev = run_step(
            Phase::CloseTalkWait {
                talk_id: TalkId(2),
                deadline: None,
            },
            Input::CloseRequest {
                reason: CloseReason::User,
            },
        );
        assert_logged(&ev, Level::WARN, "close_talk_wait_input_ignored");
    }

    #[test]
    fn warn_close_phase_unexpected_logs() {
        // close::step に非 close Phase（Steady）が届いた場合の防御アーム（上位 match `_`）。
        // ルーティング上到達不能ゆえ close::step を直接駆動して検証する。
        let cfg = config();
        let ev = capture(|| {
            let _ = close::step(
                state_in(Phase::Steady { talk: None }),
                Input::Tick { now: MonotonicMs(1_000) },
                &cfg,
            );
        });
        assert_logged(&ev, Level::WARN, "close_phase_unexpected");
    }

    // ============================================================
    // 観測用ログ（level = INFO）— TalkEndReason::Interrupted の防御的非 quit 扱い
    // ============================================================

    // ============================================================
    // 選択確定の受領検証とカスケード駆動（タスク 4.3・設計ログ語彙表）
    // ============================================================

    /// 任意の `State` を捕捉付きで駆動する（帳簿を直接構成する檻用）。
    fn run_step_state(state: State, input: Input) -> Vec<CapturedEvent> {
        let cfg = config();
        capture(|| {
            let _ = step(state, input, &cfg);
        })
    }

    /// 檻用の選択確定入力。
    fn choice_input_of(id: &str) -> Input {
        Input::Choice(crate::msg::ChoiceInput {
            id: id.to_string(),
            label: "メニュー".to_string(),
            scope: 0,
            references: Vec::new(),
        })
    }

    /// 帳簿つき `Steady{Some(5)}` を構成する。
    fn state_with_ledger(candidates: &[&str], phase: ChoicePhase) -> State {
        let mut s = state_in(steady_with_talk(TalkId(5)));
        s.choice = Some(ChoiceState {
            talk_id: TalkId(5),
            candidates: candidates.iter().map(|c| c.to_string()).collect(),
            deadline: None,
            phase,
        });
        s
    }

    /// Req1.3: 選択待ち不在・対象 talk 不一致・非 Steady の 3 経路とも warn で記録する。
    #[test]
    fn warn_choice_rejected_no_wait_logs() {
        // 選択待ち帳簿が無い（解決済み・未成立）。
        let ev = run_step(steady_with_talk(TalkId(5)), choice_input_of("OnMenu"));
        assert_logged(&ev, Level::WARN, "choice_rejected_no_wait");

        // 帳簿の対象 talk が現行 talk と食い違う。
        let mut s = state_in(steady_with_talk(TalkId(5)));
        s.choice = Some(ChoiceState {
            talk_id: TalkId(999),
            candidates: vec!["OnMenu".to_string()],
            deadline: None,
            phase: ChoicePhase::Waiting,
        });
        let ev = run_step_state(s, choice_input_of("OnMenu"));
        assert_logged(&ev, Level::WARN, "choice_rejected_no_wait");

        // 非 Steady フェーズ（横断アーム側）。
        let ev = run_step(Phase::BootMain, choice_input_of("OnMenu"));
        assert_logged(&ev, Level::WARN, "choice_rejected_no_wait");
    }

    /// Req1.4: 候補集合に無い ID の棄却は warn で記録する。
    #[test]
    fn warn_choice_rejected_unknown_id_logs() {
        let ev = run_step_state(
            state_with_ledger(&["OnMenu"], ChoicePhase::Waiting),
            choice_input_of("choice9"),
        );
        assert_logged(&ev, Level::WARN, "choice_rejected_unknown_id");
    }

    /// Req1.1: 段の進行中（`Cascading`／`TimeoutInFlight`）の二重確定は warn で記録する。
    #[test]
    fn warn_choice_rejected_busy_logs() {
        for phase in [
            ChoicePhase::Cascading {
                choice_id: "OnMenu".to_string(),
                next: None,
            },
            ChoicePhase::TimeoutInFlight,
        ] {
            let ev = run_step_state(state_with_ledger(&["OnMenu"], phase), choice_input_of("OnMenu"));
            assert_logged(&ev, Level::WARN, "choice_rejected_busy");
        }
    }

    /// Req2.7: `script:` 前置の明示縮退は warn 記録の上で選択解決のみを行う。
    #[test]
    fn warn_choice_unsupported_category_logs() {
        let ev = run_step_state(
            state_with_ledger(&["script:\\e"], ChoicePhase::Waiting),
            choice_input_of("script:\\e"),
        );
        assert_logged(&ev, Level::WARN, "choice_unsupported_category");
        // 未対応カテゴリでも選択解決は実行する（会話を止めない・Req2.7）。
        assert_logged(&ev, Level::INFO, "choice_resolved");
    }

    /// Req1.6: 受理は info で記録し、判定した段列をフィールドに載せる。
    #[test]
    fn info_choice_accepted_logs_plan() {
        let ev = run_step_state(
            state_with_ledger(&["OnMenu"], ChoicePhase::Waiting),
            choice_input_of("OnMenu"),
        );
        let accepted = logged_once(&ev, Level::INFO, "choice_accepted");
        assert_eq!(
            accepted.fields.get("choice_id").map(String::as_str),
            Some("OnMenu"),
            "確定した選択肢 ID がログフィールドに載る。\n捕捉={accepted:#?}"
        );
        assert_eq!(
            accepted.fields.get("plan").map(String::as_str),
            Some("Named"),
            "判定した段列がログフィールドに載る。\n捕捉={accepted:#?}"
        );
    }

    /// 各段の GET 送出は trace で記録する（設計ログ語彙表）。
    #[test]
    fn trace_choice_cascade_stage_logs() {
        let ev = run_step_state(
            state_with_ledger(&["choice1"], ChoicePhase::Waiting),
            choice_input_of("choice1"),
        );
        assert_logged(&ev, Level::TRACE, "choice_cascade_stage");
    }

    /// Req5.1: `ResolveChoice` 発行は info で記録する。
    #[test]
    fn info_choice_resolved_logs() {
        let ev = run_step_state(
            state_with_ledger(
                &["choice1"],
                ChoicePhase::Cascading {
                    choice_id: "choice1".to_string(),
                    next: None,
                },
            ),
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceSelect",
            },
        );
        assert_logged(&ev, Level::INFO, "choice_resolved");
    }

    /// Req4.5: カスケード段の失敗は error で記録し 204 相当で継続する。
    ///
    /// mod.rs 横断 `Failed` アームの免除（DD-12）はタスク 4.6 の担当ゆえ、本タスク時点では
    /// `step()` 経由の Failed は横断アームに先取りされる。steady 側の 204 相当処理そのものを
    /// `steady::step` の直接駆動で固定する（既存檻流儀＝到達不能アームは直接駆動で検証）。
    #[test]
    fn error_choice_shiori_failed_as_204_logs() {
        let cfg = config();
        let s = state_with_ledger(
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: None,
            },
        );
        let ev = capture(|| {
            let _ = steady::step(
                s,
                Input::ShioriReply {
                    outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                    origin: "OnChoiceSelect",
                },
                &cfg,
            );
        });
        assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
    }

    // ============================================================
    // 選択待ち帳簿の確立・棄却（タスク 4.2・設計ログ語彙表）
    // ============================================================

    /// 帳簿確立は info で記録し、**候補数と期限をフィールドに載せる**（設計ログ語彙表）。
    ///
    /// 「ログに載っていること」自体が要求であるため、発火だけでなくフィールド実値まで突合する。
    #[test]
    fn info_choice_waiting_established_logs_candidate_count_and_deadline() {
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::ChoiceWaiting {
                talk_id: TalkId(5),
                choice_ids: vec!["OnMenu".to_string(), "choice1".to_string()],
                display_end: MonotonicMs(2_000),
                timeout_directive_secs: None,
            },
        );
        let established = logged_once(&ev, Level::INFO, "choice_waiting_established");
        assert_eq!(
            established.fields.get("choice_count").map(String::as_str),
            Some("2"),
            "候補数がログフィールドに載る。\n捕捉={established:#?}"
        );
        assert_eq!(
            established.fields.get("deadline_ms").map(String::as_str),
            Some("Some(32000)"),
            "写像済みの期限がログフィールドに載る。\n捕捉={established:#?}"
        );
    }

    /// Req7.6: 無効化指令で確立された帳簿は、期限フィールドが無期限（`None`）として観測できる。
    #[test]
    fn info_choice_waiting_established_logs_indefinite_deadline() {
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::ChoiceWaiting {
                talk_id: TalkId(5),
                choice_ids: vec!["OnMenu".to_string()],
                display_end: MonotonicMs(2_000),
                timeout_directive_secs: Some(-1.0),
            },
        );
        let established = logged_once(&ev, Level::INFO, "choice_waiting_established");
        assert_eq!(
            established.fields.get("deadline_ms").map(String::as_str),
            Some("None"),
            "無効化指令は無期限として記録される。\n捕捉={established:#?}"
        );
    }

    /// 通知の棄却は 3 経路（talk_id 不一致／active talk 不在／非 Steady）とも warn で記録する。
    #[test]
    fn warn_choice_waiting_stale_logs() {
        let cases = [
            // 現行トークと識別子が一致しない。
            (steady_with_talk(TalkId(5)), TalkId(999)),
            // 再生中でない（active talk 不在）。
            (Phase::Steady { talk: None }, TalkId(5)),
            // 非 Steady フェーズ。
            (Phase::BootMain, TalkId(5)),
        ];
        for (phase, talk_id) in cases {
            let ev = run_step(
                phase,
                Input::ChoiceWaiting {
                    talk_id,
                    choice_ids: vec!["OnMenu".to_string()],
                    display_end: MonotonicMs(2_000),
                    timeout_directive_secs: None,
                },
            );
            assert_logged(&ev, Level::WARN, "choice_waiting_stale");
        }
    }

    #[test]
    fn info_talk_done_interrupted_as_non_quit_logs() {
        // 既知 talk の reason=Interrupted → 防御的に非 quit 扱い（Ended と同一経路）へ委譲。
        // M1 では到達しない想定だが、到達した場合にどの reason だったか観測できることを検証する。
        let ev = run_step(
            steady_with_talk(TalkId(5)),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                reason: TalkEndReason::Interrupted,
            }),
        );
        assert_logged(&ev, Level::INFO, "talk_done_interrupted_as_non_quit");
    }
}
