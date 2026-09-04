//! 適合一周走行の支援層（areka-P0-emo2-conformance-e2e・design D3／D6）。
//!
//! 本ファイルは**期待値を持たない**。段の駆動（注入と有界待ちの組）と、記録の突合に使う
//! 投影関数、そして進行状態の記録（第 2 系統）の型と取り出し口の本体を置く。台本と期待列は
//! `spine_conformance_script.rs`、判定は `spine_conformance_lap_tests.rs` が持つ。
//!
//! # 進行状態の記録が要る理由（R3.8・design D3「進行状態の台帳が要る理由」）
//!
//! 会話中であることは毎秒の変化通知の別（照会か片道か）と Ref3 で既に読める。**しかし選択待ちは
//! Ref3 では会話中と区別できない**——選択待ちの間も会話の枠は占有されたままで、`talk_active` と
//! `choice_active` が同時に真になり複合値 `talking,choosing` を成す
//! （`crates/areka-kanade/src/status.rs:211-216`）。Ref3 の源は `talk_active` だけ
//! （`crates/areka-kanade/src/schedule/events.rs:171-180`）ゆえ両者で同一値になる。
//! よって進行状態そのもの（組み立て済みのヘッダ値）を記録しなければ選択待ちは観測できない。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::conformance_script::{LapStage, TICK_STEP_MS};
use super::{
    ExitKind, LoopDriver, RecordedCall, SPIN_WAIT, ScriptedShioriBackend, ScriptedShioriHandle,
    ShioriBackend, SpineHarness, shell_target, spin_wait_until,
};
use areka_emo_present::PresentCommand;
use areka_ghost::dispatcher::DispatcherMsg;
use areka_kanade::{
    ChoiceInput, CloseReason, ExecutionSnapshot, KanadeMsg, MonotonicMs, MouseInput, ShioriCall,
};

/// 進行状態の記録 1 件（呼出 id と、その呼出に載った**組み立て済み**の進行状態の対）。
///
/// `status` は kanade が `ExecutionStatus::render()` 済みの wire 値をそのまま持つ
/// （`crates/areka-kanade/src/shiori/real.rs:136`／`:151`）。`None` は「`Status` ヘッダ行を
/// 出さない」ことを表す値であって、記録の欠落ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordedStatus {
    /// 呼出のイベント id（wire 形の逐語）。
    pub(super) id: String,
    /// 組み立て済みの進行状態。`None`＝ヘッダ行を出さない。
    pub(super) status: Option<String>,
}

/// 進行状態の記録の台帳（受け口本体と観測ハンドルが `Arc` で共有する）。
pub(super) type StatusLedger = Arc<Mutex<Vec<RecordedStatus>>>;

/// 台本受け口の書き込み点の本体（`spine.rs` の照会・片道の 2 か所から呼ばれる）。
///
/// **書き込み専用**である。既存の呼出記録とは別の台帳へ積むだけで、既存の読み手を 1 つも
/// 増やさない（design D3「追補の形（挙動を変えない）」）。
pub(super) fn record_status(ledger: &StatusLedger, id: &str, status: Option<&str>) {
    ledger
        .lock()
        .expect("status ledger mutex poisoned")
        .push(RecordedStatus {
            id: id.to_string(),
            status: status.map(str::to_string),
        });
}

/// 観測ハンドルの取り出し口の本体（進行状態の記録のスナップショットを呼出順で返す）。
pub(super) fn snapshot_status_calls(ledger: &StatusLedger) -> Vec<RecordedStatus> {
    ledger.lock().expect("status ledger mutex poisoned").clone()
}

// ===========================================================================
// 進行状態の記録（第 2 系統）の受け入れ確認（task 2.1・R3.8）
// ===========================================================================

/// 進行状態の記録が**選択待ちを会話中と区別して**読めることを固定する（R3.8）。
///
/// 檻に入れる判断分岐:
/// - **付随参照だけでは足りないこと**: 会話中のみと会話中かつ選択待ちの 2 つのスナップショットから
///   本番の構築関数が組む毎秒の変化通知は、**参照列が 1 バイトも違わない**（Ref3 の源は
///   `talk_active` だけ）。既存の呼出記録は参照列までしか持たないので、ここで区別が消える。
/// - **新しい取り出し口が区別を回復すること**: 同じ 3 呼出を台本受け口へ通すと、進行状態の記録には
///   組み立て済みのヘッダ値がそのまま残り、選択待ちの回だけが `talking,choosing` として読める。
/// - **ヘッダ行を出さない場合の表し方**: 全状態が非アクティブなら記録は `None`（＝ヘッダ行なし）。
/// - **既存の記録が変わらないこと**: 同じ走行で `non_status_calls()` が従来どおり 3 件を返す。
#[test]
fn status_ledger_reads_choosing_where_references_cannot() {
    // ── 実状態の 3 通り（いずれも本番で起こりうる組み合わせのみ） ──
    // 選択待ちの間も会話の枠は占有されたままゆえ、選択待ちは常に talk_active と同時に真になる。
    let idle = ExecutionSnapshot::INACTIVE;
    let talking = ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
    };
    let choosing = ExecutionSnapshot {
        talk_active: true,
        choice_active: true,
    };

    // ── (1) 付随参照は会話中と選択待ちを区別しない（＝記録の第 2 系統が要る理由・R3.8） ──
    let talking_call = areka_kanade::events::on_second_change(MonotonicMs(0), &talking);
    let choosing_call = areka_kanade::events::on_second_change(MonotonicMs(0), &choosing);
    assert_eq!(
        call_references(&talking_call),
        call_references(&choosing_call),
        "会話中と選択待ちで参照列が違うなら Ref3 で区別できてしまい、進行状態の記録は要らない"
    );

    // ── (2) 同じ 3 呼出を台本受け口へ通す ──
    let (mut backend, handle) = ScriptedShioriBackend::builder()
        .get("OnSecondChange", Ok(None))
        .notify("OnSecondChange", Ok(()))
        .notify("OnSecondChange", Ok(()))
        .build();
    let idle_call = areka_kanade::events::on_second_change(MonotonicMs(0), &idle);
    drive_call(&mut backend, &idle_call);
    drive_call(&mut backend, &talking_call);
    drive_call(&mut backend, &choosing_call);

    // ── (3) 新しい取り出し口から進行状態が読める（選択待ちが会話中と別物として現れる） ──
    assert_eq!(
        handle.status_calls(),
        vec![
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: None,
            },
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: Some("talking".to_string()),
            },
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: Some("talking,choosing".to_string()),
            },
        ],
        "進行状態の記録が組み立て済みのヘッダ値（無いときは None）を呼出順に保持していない"
    );

    // ── (4) 既存の呼出記録は素通し（追補は書き込みのみで既存の読み手を増やさない） ──
    let existing = handle.non_status_calls();
    assert_eq!(
        existing.len(),
        3,
        "既存の呼出記録が追補で変質している: {existing:?}"
    );
    assert!(
        matches!(existing[0], RecordedCall::Get { .. })
            && matches!(existing[1], RecordedCall::Notify { .. })
            && matches!(existing[2], RecordedCall::Notify { .. }),
        "既存の呼出記録の別（照会・片道）が従来どおりに残っていない: {existing:?}"
    );
}

/// [`ShioriCall`] の参照列を借りる（照会・片道のどちらでも同じ位置にある）。
fn call_references(call: &ShioriCall) -> &[String] {
    match call {
        ShioriCall::Get { references, .. } | ShioriCall::Notify { references, .. } => references,
    }
}

/// 本番の構築関数が組んだ [`ShioriCall`] を、別と id と参照列と**組み立て済み進行状態**の
/// まま台本受け口へ通す（`crates/areka-kanade/src/shiori/real.rs:136-151` と同じ渡し方）。
fn drive_call(backend: &mut ScriptedShioriBackend, call: &ShioriCall) {
    match call {
        ShioriCall::Get {
            id,
            references,
            status,
        } => {
            let wire = status.render();
            backend
                .get(id.as_str(), references, wire.as_deref())
                .expect("台本の照会応答は Ok");
        }
        ShioriCall::Notify {
            id,
            references,
            status,
        } => {
            let wire = status.render();
            backend
                .notify(id.as_str(), references, wire.as_deref())
                .expect("台本の片道応答は Ok");
        }
    }
}
// ===========================================================================
// 段の駆動器（task 2.3・design D6「段の駆動器」・R2.10・R3.2）
// ===========================================================================

/// 注入の投函先（`Err` のとき「どの受信端が既に閉じていたか」を名指しする）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Inbox {
    /// kanade の受信端。毎秒の変化通知・撫で・選択確定・終了指示はすべてここへ入る。
    Kanade,
    /// dispatcher の受信端。再生の時刻を進める Tick だけがここへ入る。
    Dispatcher,
}

/// 段の頭で**1 度ずつ**投函する注入 1 件（design D6 の入力「注入の種類」）。
pub(super) enum Injection {
    /// 毎秒の変化通知。kanade の送信端へ `KanadeMsg::Tick` を**直接**投函する（R3.2）。既存の駆動が
    /// 投げているのは dispatcher 側で、そちらは毎秒の変化通知を起こさない
    /// （`crates/areka-ghost/src/dispatcher.rs:126`）。そして **1 本につき `OnSecondChange` が
    /// ちょうど 1 件**発行される（`crates/areka-kanade/src/schedule/steady.rs:669-718`）。判定は
    /// 等値照合ゆえ（R2.3）投函本数がそのまま期待列の本数になる——だからこの注入は
    /// [`StagePlan::once`] にしか置けない（[`WaitInjection`] が型として受け付けない）。
    KanadeTick,
    /// 再生の時刻を進める Tick。dispatcher の送信端へ投函する。再生層への中継だけを行い SHIORI
    /// 呼出を **1 件も**起こさないため、待ちの繰り返しに使ってよい唯一の注入である。
    DispatcherTick,
    /// マウス入力（撫での移動・メニューの二重クリック）。kanade の送信端へ投函する。
    Mouse(MouseInput),
    /// 選択確定。kanade の送信端へ投函する。
    Choice(ChoiceInput),
    /// 終了指示（通常の握手）。kanade の送信端へ投函する。
    CloseRequest(CloseReason),
    /// kanade の受信端が**まだ開いているか**を見るためだけの探り。
    ///
    /// 投函するのは `KanadeMsg::Boot` で、Idle 以外の運行フェーズでは `warn!` を出して
    /// **副作用指示を 1 件も返さずに捨てられる**（`crates/areka-kanade/src/schedule/mod.rs:425-431`
    /// の防御アームが `(state, Vec::new())` を返す＝SHIORI 呼出は 0 件）。ゆえに交信の列を 1 件も
    /// 増やさない。終了段の完了条件は「kanade の送信端への送信が Err になる」ことを含むが
    /// （design D1 の段表）、`std::sync::mpsc::Sender` には開閉を問う口が無く**送ってみるほかに
    /// 観測手段が無い**——毎秒の変化通知で代用すると 1 本ごとに `OnSecondChange` が増えて等値照合が
    /// 壊れるため、副作用を持たないこの探りを別に用意する。
    ///
    /// **注入時刻を運ばない**ので、頭打ち（[`StagePlan::stage`] の上限）の対象外である。時刻を
    /// 進めない注入は、待っている観測を追い越しようがない。
    KanadeProbe,
}

/// 完了条件が成立するまで**毎反復繰り返す**注入。
///
/// この型が表せるのは「繰り返しても SHIORI 呼出を 1 件も増やさない」注入だけである。とりわけ
/// [`Injection::KanadeTick`] を**この型では表せない**——駆動器を通す限り、待ちの最中に毎秒の変化
/// 通知が増えることはない（増えると等値照合の期待列が満たせなくなる＝design D3・R2.3）。封じて
/// いるのは駆動器が持つ経路であって、[`StageSink`] を直に呼ぶ経路までは縛らない。
pub(super) enum WaitInjection {
    /// 再生の時刻を進める（SHIORI 呼出を 1 件も起こさない）。
    DispatcherTick,
    /// 再生の時刻を進めつつ、毎反復 [`Injection::KanadeProbe`] で kanade の受信端の開閉を見る。
    ///
    /// 終了段が使う（design D1）。close 握手は OnClose の応答スクリプトを**再生し切ってから**
    /// 終了系列へ進む（`crates/areka-kanade/src/schedule/close.rs:15-17` の `CloseTalkWait` が
    /// 再生完了通知を待つ）ため、再生の時刻を進めない探りだけでは kanade は永久に終了しない。
    /// ゆえに 2 つを組にする。
    DispatcherTickAndKanadeProbe,
    /// 何も注入せず観測だけを待つ。
    Idle,
    /// 完了条件が**初めて成立するまで**は再生側 Tick を注入し、成立した後は `settle_rounds` 回、
    /// **何も注入せず観測だけ**を続けてから段を終える。
    ///
    /// # なぜこの待ち方が要るのか（task 3.1 のレビュー指摘・R2.10）
    ///
    /// 段の完了条件が読む観測の中には、成立した後さらに実スレッドを 2〜3 段跨いで別の状態
    /// （選択待ちの帳簿など）が整うのを待たねばならないものがある。呼び手の側で「成立してから N 回
    /// 回す」と数えても、駆動器から見れば完了条件は**まだ偽**なので注入が続き、その N 回のあいだに
    /// 注入時刻が段の上限まで走ってしまう。上限が広いほど再生が余計に進み、待っている状態が
    /// 壊れる——実測では、選択待ちのまま止まっているはずの台本へ約 98 秒ぶんの再生が流し込まれ、
    /// 次の段の選択確定が棄却された。
    ///
    /// ゆえに**待ちの終盤で注入をやめる**のは駆動器の側の責務である。呼び手の関数では、完了条件が
    /// 偽である以上どうやっても注入を止められない。
    DispatcherTickThenObserve {
        /// 完了条件が成立した後、注入せずに観測だけを続ける反復の回数。
        settle_rounds: usize,
    },
}

/// 段 1 つぶんの駆動計画（design D6 の入力: 段名・注入の種類・注入時刻の上限）。区間の逐語は
/// `spine_conformance_script.rs` が唯一の置き場であり、本ファイルは写しを持たない。
pub(super) struct StagePlan<'a> {
    /// 段名と注入時刻の区間。
    pub(super) stage: &'a LapStage,
    /// 段の頭で 1 反復 1 件ずつ投函する注入（順序どおりに消費される）。
    pub(super) once: Vec<Injection>,
    /// `once` を出し切った後、完了条件が成立するまで毎反復繰り返す注入。
    pub(super) waiting: WaitInjection,
}

/// 採取した表示指令 1 件（design D3「表示の記録の要素」の段名以外の 2 要素。段名は
/// [`StageObservation::stage`] が 1 つ持つ）。
pub(super) struct CollectedCommand {
    /// 採取した表示指令そのもの。
    pub(super) command: PresentCommand,
    /// 採取した時点の注入時刻。
    pub(super) collected_at_ms: u64,
}

/// 段の駆動中に観測した「受信端が既に閉じていた」事実。終了段の完了条件は kanade の自己終了の観測を
/// 含む（design D1 の段表）ため、駆動器は投函の `Err` を握り潰さずこの形で呼び手へ渡す。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ClosedInboxes {
    /// kanade の送信端への投函が `Err` を返した。
    pub(super) kanade: bool,
    /// dispatcher の送信端への投函が `Err` を返した。
    pub(super) dispatcher: bool,
}

/// 完了条件へ渡す途中経過（design D6 の入力「段の完了条件」が読む観測）。
pub(super) struct StageProgress<'a> {
    /// この段でここまでに採取した表示指令の列。
    pub(super) collected: &'a [CollectedCommand],
    /// 現在の注入時刻。
    pub(super) now_ms: u64,
    /// ここまでに投函した [`Injection::KanadeProbe`] の本数。
    pub(super) kanade_probes: usize,
    /// この段の投函で観測した「受信端が既に閉じていた」事実。
    pub(super) closed: ClosedInboxes,
}

/// 段の駆動の成果（design D6 の出力）。
pub(super) struct StageObservation {
    /// 段名。
    pub(super) stage: &'static str,
    /// その段で採取した表示指令の列（採取時の注入時刻つき）。
    pub(super) collected: Vec<CollectedCommand>,
    /// 実際に投函した注入の時刻列（投函順）。
    pub(super) injected_at_ms: Vec<u64>,
    /// 完了時点で投函されずに残っていた `once` の件数。**0 でないことは沈黙の失敗の芽である**
    /// ——計画した注入が届かないまま完了条件が成立した、という意味になる。
    pub(super) once_pending: usize,
    /// 待ちの最中に投函した [`Injection::KanadeProbe`] の本数。
    ///
    /// 探りは交信の列を 1 件も増やさないので、「探りを N 本投げても記録が 1 件も増えない」ことを
    /// 呼び手が数で確かめられるように返す。
    pub(super) kanade_probes: usize,
    /// 段の駆動中に観測した「受信端が既に閉じていた」事実。
    pub(super) closed: ClosedInboxes,
}

/// 段の駆動の失敗（design「決定論一周テストの失敗の形」）。
///
/// いずれも**呼び手へ返す**。有界時間が尽きたことを「観測なし」として素通りさせない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StageFailure {
    /// 完了条件が成立しないまま有界時間が尽きた（R1.6）。注入の時刻列と採取件数を持つので、
    /// 「注入が届いていない」のか「注入は届いたが観測が成立しない」のかを読み分けられる。
    Timeout {
        stage: &'static str,
        injected_at_ms: Vec<u64>,
        collected: usize,
        now_ms: u64,
    },
    /// 事前条件違反——段の下限が、走行中の注入時刻より手前にある（段の順序か宣言の誤り）。
    StartsBeforeStage {
        stage: &'static str,
        arrived_at_ms: u64,
        begin_ms: u64,
    },
    /// 計画した `once` の本数が段の区間に収まらない（頭打ちが注入を黙って落とす形）。
    PlanExceedsInterval {
        stage: &'static str,
        planned: usize,
        capacity: usize,
    },
    /// **駆動器の自己検査の失敗**——採取時の注入時刻が段の宣言区間の外にある（design D3）。
    ///
    /// 製品の退行ではない。駆動器の不変条件が守られる限り必ず偽になるため、この失敗が出たら
    /// テスト自身の駆動か段の宣言（区間の逆転など）が壊れている。
    CollectedOutsideInterval {
        stage: &'static str,
        collected_at_ms: u64,
        begin_ms: u64,
        limit_ms: u64,
    },
}

impl std::fmt::Display for StageFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageFailure::Timeout {
                stage,
                injected_at_ms,
                collected,
                now_ms,
            } => write!(
                f,
                "段「{stage}」の完了条件が有界時間内に成立しない（注入 {injected_at_ms:?}・採取 {collected} 件・注入時刻 {now_ms}ms）"
            ),
            StageFailure::StartsBeforeStage {
                stage,
                arrived_at_ms,
                begin_ms,
            } => write!(
                f,
                "段「{stage}」の事前条件違反: 注入時刻 {arrived_at_ms}ms で到着したが段の下限は {begin_ms}ms"
            ),
            StageFailure::PlanExceedsInterval {
                stage,
                planned,
                capacity,
            } => write!(
                f,
                "段「{stage}」の計画が区間に収まらない: 注入 {planned} 本に対し区間の収容は {capacity} 本"
            ),
            StageFailure::CollectedOutsideInterval {
                stage,
                collected_at_ms,
                begin_ms,
                limit_ms,
            } => write!(
                f,
                "段「{stage}」で駆動器の自己検査が失敗: 採取時の注入時刻 {collected_at_ms}ms が宣言区間 [{begin_ms}, {limit_ms}]ms の外（製品の退行ではなく駆動が壊れている）"
            ),
        }
    }
}

/// 駆動器が注入を投函し表示指令を採取する先。本番は [`SpineHarness`]（既存の公開送信端と既存の
/// 取り出し口だけを使う＝R2.6・R2.9）、檻は記録だけを行う偽物を差す。
pub(super) trait StageSink {
    /// 注入 1 件を投函する。`Err(inbox)` は当該受信端が既に閉じていることを表す（panic しない）。
    fn inject(&mut self, injection: &Injection, now_ms: u64) -> Result<(), Inbox>;
    /// 表示指令を非ブロックで全件取り出す。
    fn collect(&mut self) -> Vec<PresentCommand>;
    /// 注入時刻を進めてよいか（既定は「常に進めてよい」）。
    ///
    /// # なぜ据え置きが要るのか（task 3.1 のレビュー指摘・R2.10・design D6 の危険欄）
    ///
    /// 段の待ちには 2 つの相が混ざっている——⑴ 投函した入力が実スレッドを何段も渡って
    /// **着地する**のを待つ相と、⑵ 着地した再生を**時間ぶん進める**相である。⑵ だけが注入時刻を
    /// 要するのに、駆動器は 1 反復ごとに時刻を進めるので、⑴ が長引くと時刻だけが先に上限へ達する。
    /// 上限に達すると以後は注入されないため**再生が永久に凍り**、その段は必ず待ち切れになる
    /// （実測: 高負荷で予算 50 本を使い切ってから 30 秒空転し、装着段・選択確定段が赤くなった）。
    ///
    /// 据え置きはこれを構造で断つ。⑴ のあいだは**時刻を進めずに**再生側 Tick を投函し続けるので、
    /// 着地までに何反復かかっても予算は 1 本も減らない。時刻が動かない注入は、待っている観測を
    /// 追い越しようがない——頭打ちが在る理由（注入時刻が観測を追い越すと条件が壊れる）に照らして
    /// 据え置きは安全側であり、予算は ⑵ に必要なぶんだけで足りるようになる。
    fn may_advance_clock(&self) -> bool {
        true
    }
}

impl StageSink for SpineHarness {
    fn inject(&mut self, injection: &Injection, now_ms: u64) -> Result<(), Inbox> {
        let now = MonotonicMs(now_ms);
        let kanade = |msg| self.ghost.kanade().send(msg).map_err(|_| Inbox::Kanade);
        match injection {
            Injection::KanadeTick => kanade(KanadeMsg::Tick { now }),
            Injection::Mouse(input) => kanade(KanadeMsg::Mouse(input.clone())),
            Injection::Choice(input) => kanade(KanadeMsg::Choice(input.clone())),
            Injection::CloseRequest(reason) => kanade(KanadeMsg::CloseRequest { reason: *reason }),
            // 防御アームが warn! だけを出して副作用指示を返さない（schedule/mod.rs:425-431）。
            Injection::KanadeProbe => kanade(KanadeMsg::Boot),
            Injection::DispatcherTick => self
                .ghost
                .dispatcher()
                .send(DispatcherMsg::Tick { now })
                .map_err(|_| Inbox::Dispatcher),
        }
    }

    fn collect(&mut self) -> Vec<PresentCommand> {
        self.wiring.drain_received()
    }
}

/// 段を順に駆動する駆動器（design D6）。注入時刻を走行を通じて 1 つだけ持つ。
pub(super) struct LapDriver {
    /// 次に投函する注入の時刻。走行を通じて単調増加し、各段の上限を超えない。
    now_ms: u64,
}

impl LapDriver {
    /// 注入時刻 0ms から始める。
    pub(super) fn new() -> Self {
        LapDriver { now_ms: 0 }
    }

    /// 現在の注入時刻。
    pub(super) fn now_ms(&self) -> u64 {
        self.now_ms
    }

    /// 実時計で 1 段を駆動する（本番の呼出点）。
    pub(super) fn run_stage<S: StageSink>(
        &mut self,
        sink: &mut S,
        plan: &StagePlan<'_>,
        complete: impl FnMut(&StageProgress<'_>) -> bool,
    ) -> Result<StageObservation, StageFailure> {
        self.run_stage_with(Instant::now, sink, plan, complete)
    }

    /// 時計を注入できる [`Self::run_stage`] の内側（檻専用の継ぎ目・`settle_bounded_with` と同旨）。
    /// 打ち切りの上限は `SPIN_WAIT`（30 秒）ゆえ、実時計で「尽きる側」を檻に入れると 1 本 30 秒
    /// かかる。反復の中身（注入・採取・完了判定）は実時計のときと 1 行も変わらない。
    pub(super) fn run_stage_with<S: StageSink>(
        &mut self,
        mut clock: impl FnMut() -> Instant,
        sink: &mut S,
        plan: &StagePlan<'_>,
        mut complete: impl FnMut(&StageProgress<'_>) -> bool,
    ) -> Result<StageObservation, StageFailure> {
        let LapStage {
            name,
            begin_ms,
            limit_ms,
        } = *plan.stage;

        // ── 事前条件: 注入時刻は前段の上限以上から始まる（巻き戻して駆動しない） ──
        if begin_ms < self.now_ms {
            return Err(StageFailure::StartsBeforeStage {
                stage: name,
                arrived_at_ms: self.now_ms,
                begin_ms,
            });
        }
        // ── 計画が区間に収まるか（収まらないと頭打ちが注入を黙って落とす＝沈黙の失敗） ──
        let capacity = injection_capacity(begin_ms, limit_ms);
        if plan.once.len() > capacity {
            return Err(StageFailure::PlanExceedsInterval {
                stage: name,
                planned: plan.once.len(),
                capacity,
            });
        }

        let deadline = clock() + SPIN_WAIT;
        self.now_ms = begin_ms;

        let mut collected: Vec<CollectedCommand> = Vec::new();
        let mut injected_at_ms: Vec<u64> = Vec::new();
        let mut once_next = 0usize;
        let mut kanade_probes = 0usize;
        let mut closed = ClosedInboxes::default();
        // 完了条件が成立してから観測だけで回した反復の回数（余韻を持つ待ち方でのみ使う）。
        let mut settled_rounds: Option<usize> = None;

        loop {
            // ── 採取（毎反復）＋駆動器の自己検査（design D3・製品の判定ではない） ──
            for command in sink.collect() {
                if self.now_ms < begin_ms || self.now_ms > limit_ms {
                    return Err(StageFailure::CollectedOutsideInterval {
                        stage: name,
                        collected_at_ms: self.now_ms,
                        begin_ms,
                        limit_ms,
                    });
                }
                collected.push(CollectedCommand {
                    command,
                    collected_at_ms: self.now_ms,
                });
            }

            let holds = complete(&StageProgress {
                collected: &collected,
                now_ms: self.now_ms,
                kanade_probes,
                closed,
            });
            // 余韻を持つ待ち方では、完了条件が成立してからさらに `settle_rounds` 回、**注入せずに**
            // 観測だけを続ける。成立が崩れたら数え直す（一瞬の成立を完了と誤認しない）。
            let settle_target = match plan.waiting {
                WaitInjection::DispatcherTickThenObserve { settle_rounds } => Some(settle_rounds),
                _ => None,
            };
            match settle_target {
                None => {
                    if holds {
                        return Ok(StageObservation {
                            stage: name,
                            collected,
                            injected_at_ms,
                            once_pending: plan.once.len() - once_next,
                            kanade_probes,
                            closed,
                        });
                    }
                }
                Some(target) => {
                    if holds {
                        let rounds = settled_rounds.map_or(1, |done: usize| done + 1);
                        settled_rounds = Some(rounds);
                        if rounds >= target {
                            return Ok(StageObservation {
                                stage: name,
                                collected,
                                injected_at_ms,
                                once_pending: plan.once.len() - once_next,
                                kanade_probes,
                                closed,
                            });
                        }
                    } else {
                        settled_rounds = None;
                    }
                }
            }

            // ── 有界時間が尽きたら必ず呼び手へ返す（素通りさせない） ──
            if clock() >= deadline {
                return Err(StageFailure::Timeout {
                    stage: name,
                    injected_at_ms,
                    collected: collected.len(),
                    now_ms: self.now_ms,
                });
            }

            // ── 受信端の開閉の探り（design D1 の終了段） ──
            //
            // 頭打ちの対象外である。頭打ちが在るのは注入時刻が観測を追い越すと待っている条件が
            // 壊れるためだが、探りは**注入時刻を運ばない**ので追い越しようがない。一方で
            // 「閉じた」と分かった後は投げ続ける意味が無いので止める。
            let probing = once_next >= plan.once.len()
                && matches!(plan.waiting, WaitInjection::DispatcherTickAndKanadeProbe);
            if probing && !closed.kanade {
                if sink.inject(&Injection::KanadeProbe, self.now_ms).is_err() {
                    closed.kanade = true;
                }
                kanade_probes += 1;
            }

            // ── 注入（上限に達したら以後は注入せず観測だけを待つ＝不変条件） ──
            // 投函の可否は 3 つの門で決まる。
            //  ⑴ 余韻に入ったら**何も注入しない**（呼び手が何回数えようと注入が続くと、待っている
            //     状態が余計な再生で壊れる）。
            //  ⑵ 実 async の着地待ち（[`StageSink::may_advance_clock`] が偽）のあいだは、**時刻を
            //     据え置いたまま**再生側 Tick を投函し続ける。予算は 1 本も減らない。
            //  ⑶ それ以外は上限まで通常どおり注入し、上限に達したら注入をやめる。
            let holding = !sink.may_advance_clock();
            let picked = if settled_rounds.is_some() {
                None
            } else {
                match plan.once.get(once_next) {
                    Some(injection) => {
                        once_next += 1;
                        Some(injection)
                    }
                    None if holding || self.now_ms < limit_ms => match plan.waiting {
                        WaitInjection::DispatcherTick
                        | WaitInjection::DispatcherTickAndKanadeProbe
                        | WaitInjection::DispatcherTickThenObserve { .. } => {
                            Some(&Injection::DispatcherTick)
                        }
                        WaitInjection::Idle => None,
                    },
                    None => None,
                }
            };
            if let Some(injection) = picked {
                if let Err(inbox) = sink.inject(injection, self.now_ms) {
                    match inbox {
                        Inbox::Kanade => closed.kanade = true,
                        Inbox::Dispatcher => closed.dispatcher = true,
                    }
                }
                injected_at_ms.push(self.now_ms);
                if !holding {
                    // 刻みが上限を跨ぐときは上限で止める（注入時刻は段の上限を超えない）。
                    self.now_ms = self.now_ms.saturating_add(TICK_STEP_MS).min(limit_ms);
                }
            }

            std::thread::sleep(Duration::from_micros(200));
        }
    }
}

/// 段の区間が収容できる注入の本数（下限から上限**未満**へ `TICK_STEP_MS` 刻みで並べた本数）。
fn injection_capacity(begin_ms: u64, limit_ms: u64) -> usize {
    if limit_ms <= begin_ms {
        return 0;
    }
    (limit_ms - begin_ms).div_ceil(TICK_STEP_MS) as usize
}

/// 注入の種別名（記録と失敗メッセージのための短い語）。
fn injection_kind(injection: &Injection) -> &'static str {
    match injection {
        Injection::KanadeTick => "kanade-tick",
        Injection::DispatcherTick => "dispatcher-tick",
        Injection::Mouse(_) => "mouse",
        Injection::Choice(_) => "choice",
        Injection::CloseRequest(_) => "close-request",
        Injection::KanadeProbe => "kanade-probe",
    }
}

// 主題単位の分割（R2.11）: 駆動器を縛る檻は兄弟ファイルへ置く。`spine.rs` は design が接続宣言
// 3 本に固定しているため、本ファイルの側から接続する。
#[cfg(test)]
#[path = "spine_conformance_support_tests.rs"]
mod driver_tests;
