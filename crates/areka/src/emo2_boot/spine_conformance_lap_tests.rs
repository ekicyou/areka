//! 適合一周走行の本体（areka-P0-emo2-conformance-e2e・task 3.1・design D1「一周テスト」）。
//!
//! 起動 → 装着 → 自発会話 → 会話中の抑止 → 撫で → メニュー → 選択確定 → サブメニューと戻り →
//! 位置調整 → 終了、の全段を **1 本の走行**で辿る（R1.1・R2.2）。実 SHIORI にも実時計にも依存せず、
//! 注入した時刻と注入した入力だけで進む（R2.1）。明示実行の門（`#[ignore]`・環境変数）は持たない
//! （R2.8）。注入口は既存の公開送信端だけで、製品コードへ口を新設しない（R2.9）。
//!
//! 台本と期待列は `spine_conformance_script.rs`（design D2）、段の駆動器は
//! `spine_conformance_support.rs`（design D6）にあり、本ファイルは**逐語の期待値を 1 つも持たない**。
//!
//! # 段の完了条件（design D1 の段表に対する本走行の実現形）
//!
//! 各段は自分の完了条件を持ち、成立しなければその時点で走行を不合格とする（後段の結果で
//! 埋め合わせない＝R1.6）。段表どおりに書けなかった 4 点は、実測で裏を取ったうえで次のように置いた。
//!
//! 1. **自発会話段の「表示指令が届く」は成立しない**。自発会話の応答 `\0\s[0]…` が指す面は起動挨拶が
//!    既に表示した面と同一であり、面切替が起きないため表示指令が 1 件も出ない（実測: 40 反復・
//!    600 反復のいずれでも 0 件）。よって完了条件は「毎秒の変化通知が**照会**として出る」ことだけを
//!    見る。一周を通じて表示指令が出るのは**起動挨拶（本体・面 0）と撫での相方側（`\1\s[10]`＝
//!    面 10）の 2 件だけ**である。
//! 2. **自発会話段と会話中の抑止段は、待ちに何も注入しない**（`WaitInjection::Idle`）。会話中の抑止段が
//!    読む「会話の枠が占有されたまま」は**再生が終わると解ける**が、占有区間は実測 0.25〜0.65 秒で
//!    注入 1 本（`TICK_STEP_MS`＝1 秒）が丸ごと跨ぐ。この 2 段は再生を **1 ミリ秒も進めない**ので
//!    その競争が構造的に存在しない——会話の開始は kanade の運行状態で決まり、再生の進み具合に
//!    依らないからである。
//! 3. **再生し切ったことを直接問う口が無い**。`TalkDone` は kanade の内部状態を変えるだけで交信も
//!    表示指令も生まないため、テストからは観測できない。代わりに、表示側が報告する**占有終端**
//!    （[`TalkLifecycleSignal::DisplayEndAt`]・製品が出す値）を読み、注入時刻がその終端を越えた
//!    ことを「再生は終端の先へ進んだ」の代理とし（[`LapObserved::played_out_since`]）、そこから
//!    さらに [`SETTLE_ROUNDS`] 回、**注入せず観測だけ**を続けて連鎖の着地を待つ（[`settled`]）。
//!    終端の逐語はテストが持たない——台本を書き換えれば自動で追随する。
//! 4. **「サブメニューと戻り」段は注入の窓を 2 つ使う**。選択肢 ID は**直前に再生中の台本の `\q` 帳簿**に
//!    含まれていなければ弾かれる（`crates/areka-kanade/src/schedule/steady.rs:262`）ため、
//!    「もどる」と「エモの位置調整」を連続反復で投函すると 2 つ目が必ず棄却される。段名は同じまま、
//!    区間だけを 2 つに割って各々を再生し切らせる。
//!
//! # 決定論一周が「閉じる」を選ばない理由（design D1 段表・適合検証項目表 項目 8 との辻褄）
//!
//! 段表の「サブメニューと戻り」は「もどる・閉じる」と書くが、**閉じるを選ぶとメニューが消えて
//! 次段（位置調整）が到達不能になる**。よって決定論層は「もどる → エモの位置調整」の鎖を辿り、
//! 「閉じる」は**実機層の項目として残る**（適合検証項目表 項目 8）。この置き換えは台本
//! （`spine_conformance_script.rs`）の連鎖と一致している。
//!
//! # 後片付け（design D1「後片付け」・設計討議 #1 裁定・2026-09-02）
//!
//! 終了段の完了条件が **kanade の自己終了の観測**（送信端への投函が `Err`）を含むため、
//! [`SpineHarness::shutdown_bounded`] が踏む強制終了経路の届く先は既に無い。解放を出せる主体は
//! kanade の終了系列だけであり、kanade が居なければ 2 度目の解放も台本に無い通知も**構造的に**
//! 起きない。「結果を捨てるから安全」ではなく、この順序が根拠である。
//!
//! # 判定
//!
//! 判定は段の駆動をすべて終えた**後にまとめて**行う（段の途中で部分照合しない・design D1）。
//! 本ファイルは駆動と段ごとの完了条件までを持ち、3 つの列（[`LapLedgers`]）の**等値照合**は
//! 兄弟ファイル `spine_conformance_judge.rs` が持つ（R2.11 の主題単位の分割・末尾から接続する）。

use std::cell::RefCell;
use std::rc::Rc;

use super::conformance_script::{
    CHOICE_MAIN_MENU, CHOICE_MOVE_APPLY, CHOICE_MOVE_MENU, CHOICE_TALK_INTERVAL_MENU,
    LABEL_MAIN_MENU, LABEL_MOVE_APPLY, LABEL_MOVE_MENU, LABEL_TALK_INTERVAL_MENU, LAP_STAGES,
    LapStage, MENU_CLICK, MouseProbe, SECOND_CHANGE_BUSY, SECOND_CHANGE_PLAYABLE, STROKE_KERO,
    STROKE_SAKURA, TICK_STEP_MS, lap_backend,
};
use super::conformance_support::{
    CollectedCommand, Inbox, Injection, LapDriver, StageFailure, StageObservation, StagePlan,
    StageSink, WaitInjection,
};
use super::{
    ActorKey, CloseReason, Entity, GhostWindows, HINSTANCE, HWND, LoopDriver, Point,
    PresentCommand, RecordedCall, RecordedStatus, ScriptedShioriHandle, SpineHarness,
    TalkLifecycleSignal, WindowHandle, WindowPos, World, capture_logs, count_level,
    run_attach_phase, run_move_drain_phase,
};
use areka_kanade::{ChoiceInput, MouseButton, MouseEventKind, MouseInput};

/// 報告された占有終端を「越えた」と見なすのに要する注入時刻の余白（刻み何本ぶんか）。
///
/// 占有終端は talk 相対秒で報告されるので、それを 1 本ぶん越えれば再生は終端の先へ進んでいる。
/// ここは**注入時刻の話だけ**であり、再生完了が kanade の運行状態へ届くのを待つのは
/// [`SETTLE_ROUNDS`] の役目である。
const PLAYOUT_MARGIN_TICKS: u64 = 1;

/// 完了条件が成立してから、**注入をやめて観測だけを続ける**反復の回数。
///
/// 待っているのは「再生完了 → kanade の運行状態が次へ進む」という実スレッド 2〜3 段の連鎖で、
/// これは**実時間**でしか進まない。当初これを注入時刻の余白で表したところ高負荷で待ち切れた
/// ——注入時刻は走行ごとに 1 ビットも変わらないので、**注入時刻で測る限り負荷が上がっても余白は
/// 1 ミリ秒も増えない**。反復回数なら 1 反復ごとに駆動器が 200µs 眠るので、負荷が上がるほど
/// 実時間の余白も自然に伸びる。実測の必要量は反復 2 回以内で、300 反復は 2 桁の余裕である。
const SETTLE_ROUNDS: usize = 300;

/// 着地が揃った後、再生を進めるために注入時刻を動かす本数（**実測の標本ではなく式**で決める）。
///
/// 着地（時刻に依らない観測）が揃った時点で会話は再生側の手に在り、あとは占有終端の先まで
/// 時間を運ぶだけである。必要量は `占有終端 ÷ 刻み ＋ 余白` で、一周で最長の占有区間は 0.65 秒
/// （位置調整サブメニュー）だから `ceil(650/1000) + PLAYOUT_MARGIN_TICKS = 1 + 1 = 2` 本。
/// [`assert_playout_covers_horizon`] が製品の報告した占有終端の実測最大と突き合わせるので、
/// 台本が伸びたらこの定数は黙って陳腐化せず**檻が落ちる**。
const PLAYOUT_TICKS: u64 = 2;

/// 再生を伴う段が注入時刻を進めてよい本数の上限（[`assert_budget_not_exhausted`] の判定値）。
///
/// # なぜ実測からではなく式から決めるのか（レビュー指摘・2 度目の差し戻し）
///
/// 以前この値は「実測 1〜2 本の 3 倍」として 6 本に置いていた。ところが高負荷での健全な走行が
/// 撫で段で **9 本**を記録し、壊れた形（M5）は 19 本——隔たりは 2 倍しかなく**健全側の裾が
/// 境界を越えていた**。原因は当時の据え置きが会話境界だけを見ており、表示指令や窓の移動といった
/// **残りの着地を待つあいだ時計が走っていた**ことで、その本数は機械の速さで変わるから、定数を
/// いくら上げても崖が動くだけで消えない。
///
/// 現在は [`AdvanceGate`] が**時刻に依らない着地をすべて**待ってから時計を開け、開いた後も
/// [`PLAYOUT_TICKS`] 本ぶんで再び閉じるので、前進量は機械の速さに依らず定まる。上限に 2 倍を
/// 採るのは、前段から遅れて届いた会話境界が占有の起点を引き直すと再生の相が**1 度だけ**やり直しに
/// なるためで、2 度目が無いのは各段が余韻で自分の信号を出し切ってから次段へ渡すからである。
/// 実測は **0〜1 本**（着地だけで完了する撫で・位置調整は 0）、壊れた形（M5）は 19 本。
const MAX_CLOCK_ADVANCE_TICKS: u64 = 2 * PLAYOUT_TICKS;

/// 再生を伴う段の待ち方——完了条件が成立するまでは再生側 Tick を注入し、成立後は
/// [`SETTLE_ROUNDS`] 回、**注入をやめて観測だけ**を続ける（`WaitInjection::DispatcherTickThenObserve`）。
///
/// 余韻を呼び手の側で数えていた頃は、完了条件が偽であるあいだ駆動器が注入を続けるため、余韻の
/// あいだに注入時刻が段の上限まで走った。選択待ちのまま止まっているはずの台本へ再生が流し込まれ、
/// 次の段の選択確定が棄却される——[`assert_budget_not_exhausted`] がこの形の再発を檻に入れる。
const OBSERVE_AFTER_READY: WaitInjection = WaitInjection::DispatcherTickThenObserve {
    settle_rounds: SETTLE_ROUNDS,
};

/// 位置調整の着地位置（実 fixture の移動量 `-353` と `two_scope_placements` からの検算値）。
///
/// x' = 1483 + 434/2 − 353 − 278/2 = 1208・y は `Fix` ゆえ現状維持 1063
/// （`spine_move_cue_tests.rs` が同じ台本で同じ値を固定している）。
const MOVE_LANDING: Point = Point { x: 1208, y: 1063 };

// ===========================================================================
// 観測（段の完了条件が読む量）
// ===========================================================================

/// 走行中に採り続ける観測量。段の完了条件はここだけを読む。
#[derive(Debug, Default)]
struct LapObserved {
    /// 起動した talk の累計件数（[`TalkLifecycleSignal::TalkStarted`] の数）。
    talks_started: usize,
    /// 現在の talk を最初に観測した注入時刻（占有終端の起点）。
    talk_base_ms: Option<u64>,
    /// 現在の talk の占有終端（ms・報告された既知最大）。
    talk_horizon_ms: u64,
    /// 位置調整の対象窓の現在位置（`run_move_drain_phase` 適用後）。
    move_target_pos: Option<Point>,
    /// 採取した表示指令の累計件数（着地の判定に使う）。
    displays_seen: usize,
    /// 走行を通じて報告された占有終端の最大値（[`assert_playout_covers_horizon`] の検算材料）。
    max_horizon_ms: u64,
}

impl LapObserved {
    /// **この段が起こすはずの** talk がすべて起動し、最後の 1 本を再生し切ったか。
    ///
    /// `target_talks` は「この段を終えた時点で観測されているべき talk 起動の累計」である。段に入る前の
    /// 累計を上回るだけでは足りない——**前段の会話境界が遅れて届くと、それだけで次段の門が開く**。
    /// 実測（高負荷）で撫での相方側の会話境界がメニュー段まで遅れて届き、台本が選択待ちへ達する前に
    /// メニュー段が完了して次段の選択確定が棄却された（内訳: `照会 0 件・talk 起動 累計 3 件`）。
    fn played_out_since(&self, target_talks: usize, now_ms: u64) -> bool {
        self.talks_started >= target_talks
            && match self.talk_base_ms {
                Some(base) => {
                    now_ms >= base + self.talk_horizon_ms + PLAYOUT_MARGIN_TICKS * TICK_STEP_MS
                }
                None => false,
            }
    }
}

/// 注入時刻を進めてよい条件（段ごとに差し替える）。
///
/// # なぜ「会話境界だけ」では足りないのか（レビュー指摘・2 度目の差し戻し）
///
/// 段の待ちには 2 種類の観測が混ざる——**時刻に依らない着地**（照会が記録された・会話が起動した・
/// 表示指令が届いた・窓が動いた）と、**時刻を要する再生**（占有終端の先まで運ぶ）である。据え置きが
/// 会話境界だけを見ていたときは、残りの着地（相方の面切替・窓の移動）を待つあいだ時計が走り続け、
/// 前進量が機械の速さで変わった（高負荷の健全な走行で 9 本＝当時の上限 6 本を越えて赤）。ゆえに門は
/// **その段の完了条件のうち時刻に依らない部分をそのまま写す**。写した部分が揃うまで時計は 1 ミリ秒も
/// 動かず、揃ってからちょうど [`PLAYOUT_TICKS`] 本だけ動いてまた閉じる。
enum AdvanceGate {
    /// 常に進めてよい（待ちに何も注入しない段で使う）。
    Always,
    /// 時刻に依らない着地がすべて揃うまで据え置き、揃ったら [`PLAYOUT_TICKS`] 本だけ進める。
    AfterLanding {
        /// 着地の判定。時刻に依らない観測だけを読む（注入時刻を引数に取らないのが要点）。
        landed: Box<dyn Fn(&LapObserved, &ScriptedShioriHandle) -> bool>,
    },
}

/// 段の駆動器へ渡す投函先。注入は [`SpineHarness`] の既存の公開送信端へそのまま委ね（R2.9）、
/// 採取のたびに表示側の観測（会話境界・占有終端・移動の反映）を [`LapObserved`] へ写す。
struct LapSink<'a> {
    /// 起動済みのハーネス（注入も採取もこの既存の口だけを使う）。
    harness: &'a mut SpineHarness,
    /// 位置調整の対象窓（相方＝scope1 のキャラ窓）。
    move_target: Entity,
    /// 直前に投函した注入の時刻（会話境界を観測した時刻の代理）。
    last_inject_ms: u64,
    /// 交信の記録の読み口（着地の判定が照会の件数を読む）。
    handle: ScriptedShioriHandle,
    /// 注入時刻を進めてよい条件（段ごとに差し替える）。
    advance_gate: AdvanceGate,
    /// 完了条件と共有する観測。
    observed: Rc<RefCell<LapObserved>>,
}

impl StageSink for LapSink<'_> {
    fn inject(&mut self, injection: &Injection, now_ms: u64) -> Result<(), Inbox> {
        self.last_inject_ms = now_ms;
        StageSink::inject(self.harness, injection, now_ms)
    }

    fn may_advance_clock(&self) -> bool {
        match &self.advance_gate {
            AdvanceGate::Always => true,
            AdvanceGate::AfterLanding { landed } => {
                let observed = self.observed.borrow();
                // 着地が揃い、かつ現在の会話をまだ [`PLAYOUT_TICKS`] 本ぶん進めていないあいだだけ
                // 開く。開いた後の前進量が機械の速さに依らずこの本数へ定まるのが要点である。
                landed(&observed, &self.handle)
                    && observed.talk_base_ms.is_some_and(|base| {
                        self.last_inject_ms < base + PLAYOUT_TICKS * TICK_STEP_MS
                    })
            }
        }
    }

    fn collect(&mut self) -> Vec<PresentCommand> {
        {
            let mut observed = self.observed.borrow_mut();
            for signal in self.harness.wiring.drain_lifecycle_signals() {
                match signal {
                    // 会話境界（talk 起動ごとに 1 回）。占有終端の計測をここでやり直す。
                    TalkLifecycleSignal::TalkStarted => {
                        observed.talks_started += 1;
                        observed.talk_base_ms = Some(self.last_inject_ms);
                        observed.talk_horizon_ms = 0;
                    }
                    // 占有終端（talk 相対秒・既知最大が更新されたときだけ届く）。
                    TalkLifecycleSignal::DisplayEndAt(seconds) => {
                        let reported = horizon_ms(seconds);
                        observed.talk_horizon_ms = observed.talk_horizon_ms.max(reported);
                        observed.max_horizon_ms = observed.max_horizon_ms.max(reported);
                    }
                }
            }
            // 移動指令の受信端を実 frame 相で drain し、対象窓の位置を読み直す（design D1 位置調整段）。
            run_move_drain_phase(&self.harness.wiring, &mut self.harness.world);
            observed.move_target_pos = window_position(&self.harness.world, self.move_target);
        }
        let collected = StageSink::collect(self.harness);
        self.observed.borrow_mut().displays_seen += collected.len();
        collected
    }
}

/// 占有終端（talk 相対秒）をミリ秒へ切り上げる。
fn horizon_ms(seconds: f64) -> u64 {
    if seconds <= 0.0 {
        0
    } else {
        (seconds * 1_000.0).ceil() as u64
    }
}

// ===========================================================================
// 採取した 3 つの列（task 3.2 の等値照合はこの上に足す）
// ===========================================================================

/// 同一の走行から採った 3 つの列（design D3「3 つの列」）。
struct LapLedgers {
    /// 交信の列（呼出の別・id・参照列）。
    calls: Vec<RecordedCall>,
    /// 進行状態の列（呼出 id・組み立て済み進行状態）。
    statuses: Vec<RecordedStatus>,
    /// 〈段名・表示指令〉の列（採取時の注入時刻つき）。
    display: Vec<(&'static str, CollectedCommand)>,
}

// ===========================================================================
// 走行の道具
// ===========================================================================

/// 偽 HWND の `WindowHandle`（実窓なし・headless 決定論シーム・`spine_move_cue_tests.rs` と同形）。
fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// 各キャラ／バルーン窓へ偽 `WindowHandle` を付与する（これが無いと移動が warn＋no-op へ縮退する）。
fn attach_fake_window_handles(world: &mut World, windows: &GhostWindows) {
    let mut raw = 0x100usize;
    for scope in windows.scopes().collect::<Vec<_>>() {
        for entity in [
            windows.char_window(scope).expect("キャラ窓"),
            windows.balloon_window(scope).expect("バルーン窓"),
        ] {
            world.entity_mut(entity).insert(fake_handle(raw));
            raw += 0x10;
        }
    }
}

/// entity の `WindowPos.position` を読む（未設定は `None`）。
fn window_position(world: &World, entity: Entity) -> Option<Point> {
    world.get::<WindowPos>(entity).and_then(|pos| pos.position)
}

/// 台本の注入値からマウス入力を組む（座標・話者・当たり領域は台本の逐語をそのまま運ぶ）。
fn mouse(probe: &MouseProbe, kind: MouseEventKind) -> MouseInput {
    MouseInput {
        scope: probe.scope,
        x: probe.x,
        y: probe.y,
        region: Some(probe.region.to_string()),
        kind,
    }
}

/// 選択確定入力を組む（付随参照列は空＝実物のメニューと同じ形）。
fn choice(id: &str, label: &str) -> ChoiceInput {
    ChoiceInput {
        id: id.to_string(),
        label: label.to_string(),
        scope: 0,
        references: Vec::new(),
    }
}

/// 記録の中の照会（GET）の件数を id で数える。
fn get_calls(calls: &[RecordedCall], id: &str) -> usize {
    calls
        .iter()
        .filter(|call| matches!(call, RecordedCall::Get { id: got, .. } if got == id))
        .count()
}

/// 記録の中に、指定 id で末尾参照が `tail_ref` の照会（GET）が在るか。
fn has_get(calls: &[RecordedCall], id: &str, tail_ref: &str) -> bool {
    calls.iter().any(|call| {
        matches!(call, RecordedCall::Get { id: got, references }
            if got == id && references.last().map(String::as_str) == Some(tail_ref))
    })
}

/// 記録の中に、指定 id で末尾参照が `tail_ref` の片道（NOTIFY）が在るか。
fn has_notify(calls: &[RecordedCall], id: &str, tail_ref: &str) -> bool {
    calls.iter().any(|call| {
        matches!(call, RecordedCall::Notify { id: got, references }
            if got == id && references.last().map(String::as_str) == Some(tail_ref))
    })
}

/// 段の駆動の結果を受け取り、失敗なら**段名を名指しして**走行を落とす（沈黙の失敗を作らない）。
fn expect_stage(outcome: Result<StageObservation, StageFailure>) -> StageObservation {
    expect_stage_with(outcome, || String::new())
}

/// [`expect_stage`] に、その段が読んでいた観測の**内訳**を添える。
///
/// # なぜ内訳が要るか（R12.5・レビュー指摘）
///
/// 駆動器が返す待ち切れは「注入の時刻列・採取件数」までしか運ばない。ところが選択を投函する段は、
/// 表示指令を 1 件も出さないのが正常なので `採取 0 件` が何も語らず、**選択が棄却された**のか
/// **後続の観測がまだ届かない**のかを区別できない。区別できないと、移動経路の本物の退行
/// （変異 M3）と間欠的な赤が**同じ文面**になる。内訳はその 2 つを 1 行で切り分ける。
fn expect_stage_with(
    outcome: Result<StageObservation, StageFailure>,
    breakdown: impl FnOnce() -> String,
) -> StageObservation {
    match outcome {
        Ok(observed) => {
            assert_eq!(
                observed.once_pending, 0,
                "段「{}」: 計画した注入が {} 件届かないまま完了条件が成立した（沈黙の失敗）",
                observed.stage, observed.once_pending
            );
            observed
        }
        Err(failure) => panic!(
            "{failure}
  内訳: {}",
            breakdown()
        ),
    }
}

/// 選択を投函した段の内訳（選択が受理されたか・移動が着地したか）を 1 行に組む。
///
/// 照会が 0 件なら kanade が選択確定を**棄却**している——直前の段が残したはずの選択待ちが、
/// 投函の時点で失われていたという意味である（`crates/areka-kanade/src/schedule/steady.rs:219-263`
/// の 4 つの棄却アーム）。1 件以上あるのに段が待ち切れたなら、棄却ではなく後続の観測
/// （移動指令の受信端 → frame 相 drain → 対象窓）が届いていない。
fn choice_breakdown(
    handle: &ScriptedShioriHandle,
    id: &str,
    observed: &Rc<RefCell<LapObserved>>,
) -> String {
    let accepted = get_calls(&handle.non_status_calls(), id);
    let observed = observed.borrow();
    format!(
        "選択確定「{id}」の照会 {accepted} 件（0 件＝kanade が選択を棄却＝直前の段の選択待ちが失われている）・talk 起動 累計 {} 件・占有の起点 {:?}・占有終端 {}ms・対象窓 {:?}（着地予定 {MOVE_LANDING:?}）",
        observed.talks_started,
        observed.talk_base_ms,
        observed.talk_horizon_ms,
        observed.move_target_pos
    )
}

/// 余韻を持つ段が、**注入時刻を上限まで走らせずに**完了したことを確かめる（レビュー指摘の再発防止）。
///
/// 測るのは投函の**回数**ではない。着地待ちのあいだ据え置きで投函される再生側 Tick は注入時刻を
/// 1 ミリ秒も動かさないので、回数は高負荷で数百回にもなる（実測 1,353 回）——正常な姿である。
/// 壊れるのは**注入時刻が走った**ときで、そのとき段は「再生を進めきってしまった」状態で完了し、
/// 次の段が読む選択待ちが失われる。実測の前進は 0〜1 本、壊れた形（M5）は 19 本。
fn assert_budget_not_exhausted(observed: &StageObservation, stage: &LapStage) {
    let reached = observed
        .injected_at_ms
        .last()
        .copied()
        .unwrap_or(stage.begin_ms);
    let advanced = (reached - stage.begin_ms) / TICK_STEP_MS;
    let capacity = (stage.limit_ms - stage.begin_ms).div_ceil(TICK_STEP_MS);
    assert!(
        advanced <= MAX_CLOCK_ADVANCE_TICKS,
        "段「{}」が注入時刻を {} 本ぶん進めて完了した（上限は {} 本・区間の予算 {} 本・投函 {} 回）——余韻が注入を止めていないか、着地待ちが据え置かれていない",
        observed.stage,
        advanced,
        MAX_CLOCK_ADVANCE_TICKS,
        capacity,
        observed.injected_at_ms.len()
    );
    let _ = reached;
}

/// [`PLAYOUT_TICKS`] が、製品が報告した占有終端を実際に覆っていることを検算する。
///
/// `PLAYOUT_TICKS` は「一周で最長の占有区間は 0.65 秒」という前提から式で決めた定数である。台本が
/// 伸びてその前提が崩れると、再生を運びきる前に門が閉じて段が待ち切れになる——原因が
/// 「定数が足りない」ことだと分かる形で落とすために、走行のたびに実測の最大と突き合わせる。
fn assert_playout_covers_horizon(observed: &LapObserved) {
    let needed = observed.max_horizon_ms + PLAYOUT_MARGIN_TICKS * TICK_STEP_MS;
    let budget = PLAYOUT_TICKS * TICK_STEP_MS;
    assert!(
        needed <= budget,
        "PLAYOUT_TICKS（{PLAYOUT_TICKS} 本＝{budget}ms）が占有終端の実測最大 {}ms ＋余白 {}ms を覆っていない——台本が伸びたら本数を式から引き直すこと",
        observed.max_horizon_ms,
        PLAYOUT_MARGIN_TICKS * TICK_STEP_MS
    );
}

/// 段の採取を〈段名・表示指令〉の列へ写す。
fn collect_display(
    display: &mut Vec<(&'static str, CollectedCommand)>,
    observed: StageObservation,
) {
    let stage = observed.stage;
    display.extend(observed.collected.into_iter().map(|cmd| (stage, cmd)));
}

/// 段の区間宣言を [`LAP_STAGES`] から順に取り出す反復子。
///
/// 段の順序と区間は台本側の宣言が唯一の源であり、本ファイルは写しを持たない。取り出しの順序が
/// 段表とずれたら、その場で名指しして落とす。
struct LapStages(std::slice::Iter<'static, LapStage>);

impl LapStages {
    fn new() -> Self {
        LapStages(LAP_STAGES.iter())
    }

    /// 次の段を取り出し、期待した段名であることを確かめる。
    fn take(&mut self, expected: &str) -> &'static LapStage {
        let stage = self
            .0
            .next()
            .unwrap_or_else(|| panic!("段の宣言が尽きた（段「{expected}」を取り出せない）"));
        assert_eq!(
            stage.name, expected,
            "段の宣言の並びが駆動の並びと違う（台本の LAP_STAGES を確認すること）"
        );
        stage
    }

    /// 宣言をすべて使い切ったことを確かめる（駆動し忘れた段が無い）。
    fn assert_exhausted(mut self) {
        if let Some(stage) = self.0.next() {
            panic!("駆動されていない段が残っている: 「{}」", stage.name);
        }
    }
}

// ===========================================================================
// 据え置きの門の受け入れ確認（task 3.1 の再提出・design D6 の危険欄・R2.10）
// ===========================================================================

/// 据え置きの門だけを検査する投函先（起動を伴わずに駆動器の時刻の進め方を見る）。
struct HoldingSink {
    /// 投函の記録（注入時刻・投函順）。
    injected: Vec<u64>,
    /// 残りの据え置き回数（0 になったら注入時刻を進めてよい）。
    hold_rounds: std::cell::Cell<usize>,
}

impl StageSink for HoldingSink {
    fn inject(&mut self, _injection: &Injection, now_ms: u64) -> Result<(), Inbox> {
        self.injected.push(now_ms);
        Ok(())
    }

    fn collect(&mut self) -> Vec<PresentCommand> {
        Vec::new()
    }

    fn may_advance_clock(&self) -> bool {
        let left = self.hold_rounds.get();
        if left > 0 {
            self.hold_rounds.set(left - 1);
            false
        } else {
            true
        }
    }
}

/// 駆動器の 2 つの門——**着地待ちの据え置き**と**余韻での注入停止**——を 1 本の投函列で固定する。
///
/// 檻に入れる判断分岐:
/// - **据え置き中も投函は続くこと**: 再生側 Tick が止まると、待っている会話は永久に起動しない
///   （駆動器が注入をやめると再生が凍る＝design D6 の危険欄）。
/// - **据え置き中は時刻が 1 ミリ秒も動かないこと**: 動くと予算が減り、着地が遅いと上限へ達して
///   同じ凍り方をする。実測ではこれが高負荷で装着段・選択確定段を赤くしていた。
/// - **門が開いたら刻みどおり進むこと**: 据え置きが解けない実装は再生を進める相へ進めない。
/// - **完了条件が成立した後は 1 本も投函しないこと**: 余韻のあいだ注入が続くと、選択待ちのまま
///   止まっているはずの台本へ再生が流し込まれ、次段の選択確定が棄却される。
///
/// 期待する投函列は「据え置き 5 本（すべて下限）＋前進 4 本（刻みどおり）＋余韻 0 本」である。
/// 区間は 19 本ぶん取ってあるので、余韻で注入が続けば必ず 4 本ぶん余計に並ぶ。
#[test]
fn stage_driver_holds_the_injection_clock_until_the_landing_is_observed() {
    let stage = LapStage {
        name: "門の検査",
        begin_ms: 1_000,
        limit_ms: 20_000,
    };
    let mut sink = HoldingSink {
        injected: Vec::new(),
        hold_rounds: std::cell::Cell::new(5),
    };
    let mut driver = LapDriver::new();
    let mut rounds = 0usize;
    let observed = driver
        .run_stage(
            &mut sink,
            &StagePlan {
                stage: &stage,
                once: Vec::new(),
                waiting: WaitInjection::DispatcherTickThenObserve { settle_rounds: 4 },
            },
            |_| {
                rounds += 1;
                rounds > 9
            },
        )
        .expect("門は待ち切れを起こさない");

    assert_eq!(
        observed.injected_at_ms,
        vec![
            1_000, 1_000, 1_000, 1_000, 1_000, 1_000, 2_000, 3_000, 4_000
        ],
        "据え置き 5 本（下限に釘付け）→ 前進 4 本（刻みどおり）→ 余韻 0 本、になっていない"
    );
    assert!(
        observed
            .injected_at_ms
            .iter()
            .all(|at| (stage.begin_ms..=stage.limit_ms).contains(at)),
        "投函が段の区間の外へ出ている: {:?}",
        observed.injected_at_ms
    );
}

// ===========================================================================
// 一周の走行（design D1・R1.1／1.5／1.6・R2.1／2.2／2.6／2.7／2.8／2.9）
// ===========================================================================

/// 一周の全段を 1 本の走行で辿り、各段を設計の完了条件まで進めて全実行主体を有界時間で畳む。
///
/// 檻に入れる判断分岐（段ごとの完了条件がそのまま主張である）:
/// - **起動**: 起動系列 5 呼出が記録に揃う。
/// - **装着**: 装着の計画件数と実装着件数が一致し、起動挨拶が表示指令を出して再生し切る。
/// - **自発会話**: 毎秒の変化通知が**照会**として出る（Ref3="1"＝会話を始められる）。
/// - **会話中の抑止**: 同じ通知が**片道**になり Ref3 が "0" になる（会話の枠が占有されたまま）。
/// - **撫で**: 移動の照会が 2 件（本体・相方）記録され、相方側の面切替が表示指令として届く。
/// - **メニュー**: 二重クリックの照会が記録され、メニューの台本が選択待ちまで再生される。
/// - **選択確定**: 選択肢 ID と同名の照会が記録される。
/// - **サブメニューと戻り**: 「もどる」と「エモの位置調整」の同名照会が続けて記録される。
/// - **位置調整**: 移動指令が受信端へ届き、対象窓が算出位置へ動く。
/// - **終了**: 終了の照会 → 終了挨拶の再生 → 解放、が記録され、解放がちょうど 1 件で、かつ
///   kanade の送信端への投函が `Err` になる（＝受信側が閉じた＝自己終了の観測）。
#[test]
fn conformance_lap_walks_every_stage_to_its_completion() {
    // ── 起動: 一周用の台本で組立を起こす（GPU World・合成窓一式・実 emo2 資産・本番と同じ 4 受け口） ──
    let (backend, handle) = lap_backend();
    let mut harness = SpineHarness::boot_with(backend, handle.clone(), LoopDriver::Inert);

    let observed = Rc::new(RefCell::new(LapObserved::default()));
    let mut driver = LapDriver::new();
    let mut stages = LapStages::new();
    let mut display: Vec<(&'static str, CollectedCommand)> = Vec::new();

    // 移動の反映口（実窓生成前ゆえ handle 未付与）を埋め、位置調整段の対象窓を決める。
    let windows = harness
        .world
        .get_resource::<GhostWindows>()
        .expect("spine World には GhostWindows が挿入済み")
        .clone();
    attach_fake_window_handles(&mut harness.world, &windows);
    let move_target = windows.char_window(1).expect("相方（scope1）のキャラ窓");
    let move_origin = window_position(&harness.world, move_target);
    assert!(
        move_origin.is_some() && move_origin != Some(MOVE_LANDING),
        "前提: 位置調整の対象窓は初期位置を持ち、着地位置とは別である: {move_origin:?}"
    );

    let mut sink = LapSink {
        harness: &mut harness,
        move_target,
        last_inject_ms: 0,
        handle: handle.clone(),
        advance_gate: AdvanceGate::Always,
        observed: Rc::clone(&observed),
    };

    // ── 段 1: 起動（注入なし・組立が起こした起動系列 5 呼出が揃うのを待つ） ──
    let probe = handle.clone();
    sink.advance_gate = AdvanceGate::Always;
    let report = handle.clone();
    let boot = expect_stage_with(
        driver.run_stage(
            &mut sink,
            &StagePlan {
                stage: stages.take("起動"),
                once: Vec::new(),
                waiting: WaitInjection::Idle,
            },
            |_| probe.non_status_calls().len() >= 5,
        ),
        || {
            format!(
                "起動系列の呼出 {} 件（5 件で成立）。**この段は注入を 1 件も行わない**ので、0 件なら組立そのものが有界時間内に走っていない——一周の駆動ではなく、既存の兄弟テストが同じ待ちで共有する起動の飢餓（`spine_boot_smoke_tests.rs` と `spine_conformance_support_tests.rs` の`non_status_calls().len() >= 5`）と同じ系統である",
                report.non_status_calls().len()
            )
        },
    );
    collect_display(&mut display, boot);

    // ── 段 2: 装着（装着相を直接駆動し、起動挨拶を再生し切る） ──
    let logs = capture_logs(|| run_attach_phase(&mut sink.harness.wiring, &mut sink.harness.world));
    assert!(
        logs.iter()
            .any(|line| line.contains("planned=2") && line.contains("attached=2")),
        "段「装着」の完了条件が成立しない（計画件数と実装着件数が一致しない）: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "段「装着」で ERROR が出ている: {logs:?}"
    );
    let watch = Rc::clone(&observed);
    // この段が再生側へ届かせる会話は 1 本——起動挨拶（`OnBoot` の応答）である。
    let talks_target = observed.borrow().talks_started + 1;
    let attach_stage = stages.take("装着");
    // 起動挨拶が起動し、**その表示指令が届く**まで注入時刻を据え置く。会話境界だけで開けると、
    // 表示指令を待つあいだ時計が走って前進量が機械の速さに依存する（レビュー指摘）。
    let displays_target = observed.borrow().displays_seen + 1;
    sink.advance_gate = AdvanceGate::AfterLanding {
        landed: Box::new(move |observed, _| {
            observed.talks_started >= talks_target && observed.displays_seen >= displays_target
        }),
    };
    let attach = expect_stage(driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: attach_stage,
            once: Vec::new(),
            waiting: OBSERVE_AFTER_READY,
        },
        |progress| {
            !progress.collected.is_empty()
                && watch
                    .borrow()
                    .played_out_since(talks_target, progress.now_ms)
        },
    ));
    assert_budget_not_exhausted(&attach, attach_stage);
    collect_display(&mut display, attach);

    // ── 段 3: 自発会話（kanade へ毎秒の変化通知を 1 本だけ投函する） ──
    //
    //     1 本につき `OnSecondChange` がちょうど 1 件発行される（`schedule/steady.rs:669-718`）ため、
    //     等値照合（R2.3）が成り立つのは 1 本のときだけである。**待ちには何も注入しない**
    //     （`WaitInjection::Idle`）——次段が読む「会話中」を守るためである（モジュール doc の 2）。
    let probe = handle.clone();
    let idle_talk = expect_stage(driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: stages.take("自発会話"),
            once: vec![Injection::KanadeTick],
            waiting: WaitInjection::Idle,
        },
        |_| {
            has_get(
                &probe.non_status_calls(),
                "OnSecondChange",
                SECOND_CHANGE_PLAYABLE,
            )
        },
    ));
    collect_display(&mut display, idle_talk);

    // ── 段 4: 会話中の抑止（同じ通知が片道になり Ref3 が "0" になる） ──
    //
    //     直前の段が再生を進めすぎないことが前提である。自発会話段は「talk が起動した」ところで
    //     完了するので占有区間の内側に留まり、この段の投函は会話中の kanade へ届く。
    let probe = handle.clone();
    let suppressed = expect_stage(driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: stages.take("会話中の抑止"),
            once: vec![Injection::KanadeTick],
            waiting: WaitInjection::Idle,
        },
        |_| {
            has_notify(
                &probe.non_status_calls(),
                "OnSecondChange",
                SECOND_CHANGE_BUSY,
            )
        },
    ));
    collect_display(&mut display, suppressed);

    // ── 段 5: 撫で（本体・相方の 2 件。相方側は面 10 への切替ゆえ表示指令が届く） ──
    let probe = handle.clone();
    let watch = Rc::clone(&observed);
    // 本体・相方の 2 件を続けて投函するので、本体側の会話は再生側へ届く前に相方側へ差し替わる
    // ——再生側へ着地する会話は**1 本**（相方側）である。この段が余韻（観測だけの反復）を持つのが
    // 要点で、余韻が無かったときは会話境界が次段（メニュー）まで遅れて届き、その 1 件だけで
    // メニュー段の門が開いて台本が選択待ちへ達しないまま完了していた（実測・高負荷）。
    let talks_target = observed.borrow().talks_started + 1;
    let displays_target = observed.borrow().displays_seen + 1;
    let stroke_stage = stages.take("撫で");
    // この段の完了条件は**丸ごと時刻に依らない**（照会 2 件・会話の起動・相方の面切替の表示指令）。
    // ゆえに門へそのまま写すと、時計は 1 ミリ秒も動かずに段が終わる（前進 0 本）。写し漏らすと、
    // 漏らした観測を待つあいだ時計が走る——高負荷で 9 本まで走って赤くなったのがその形である。
    sink.advance_gate = AdvanceGate::AfterLanding {
        landed: Box::new(move |observed, handle| {
            get_calls(&handle.non_status_calls(), "OnMouseMove") >= 2
                && observed.talks_started >= talks_target
                && observed.displays_seen >= displays_target
        }),
    };
    let stroke = expect_stage(driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: stroke_stage,
            once: vec![
                Injection::Mouse(mouse(&STROKE_SAKURA, MouseEventKind::Move)),
                Injection::Mouse(mouse(&STROKE_KERO, MouseEventKind::Move)),
            ],
            waiting: OBSERVE_AFTER_READY,
        },
        |progress| {
            get_calls(&probe.non_status_calls(), "OnMouseMove") >= 2
                && !progress.collected.is_empty()
                && watch.borrow().talks_started >= talks_target
        },
    ));
    assert_budget_not_exhausted(&stroke, stroke_stage);
    collect_display(&mut display, stroke);

    // ── 段 6: メニュー（二重クリック → メインメニューの台本を選択待ちまで再生する） ──
    let probe = handle.clone();
    let watch = Rc::clone(&observed);
    // この段が起こす会話は 1 本（メインメニューの台本）。
    let talks_target = observed.borrow().talks_started + 1;
    let menu_stage = stages.take("メニュー");
    // 表示指令を出さない段なので、着地は「二重クリックの照会が記録され、応答が会話として起動した」
    // ことに尽きる。残りは選択待ちまで再生を運ぶ相で、そこだけが注入時刻を要する。
    sink.advance_gate = AdvanceGate::AfterLanding {
        landed: Box::new(move |observed, handle| {
            get_calls(&handle.non_status_calls(), "OnMouseDoubleClick") >= 1
                && observed.talks_started >= talks_target
        }),
    };
    let menu = expect_stage(driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: menu_stage,
            once: vec![Injection::Mouse(mouse(
                &MENU_CLICK,
                MouseEventKind::DoubleClick {
                    button: MouseButton::Left,
                },
            ))],
            waiting: OBSERVE_AFTER_READY,
        },
        |progress| {
            get_calls(&probe.non_status_calls(), "OnMouseDoubleClick") >= 1
                && watch
                    .borrow()
                    .played_out_since(talks_target, progress.now_ms)
        },
    ));
    assert_budget_not_exhausted(&menu, menu_stage);
    collect_display(&mut display, menu);

    // ── 段 6 の自己検査: 選択肢が**相方側（scope1）**のバルーンへ載ったこと（task 5.7） ──
    //
    //     3 つの列は scope を 1 ビットも運ばない——表示指令はキャラ窓の面切替だけ、選択の帳簿は
    //     ID の集合だけ（`crates/areka-kanade/src/schedule/steady.rs:262`）である。実物
    //     `menu.pasta:15` は選択肢を `エモ：`（scope1）へ置くので、台本がそれを scope0 へ写し
    //     違えても 3 列は緑のままになる。scope を持つ観測点は文字層の選択状態
    //     （`crates/areka-emo-text/src/actor.rs:539` の `choice_active`）だけなので、ここで 1 度
    //     読んで写しの正しさを固定する。`pump_text` は文字層 UI アクターの drain だけを行い、
    //     交信も表示指令も 1 件も増やさない（`spine.rs:862`）。
    sink.harness.pump_text();
    {
        let rt = sink.harness.runtime.borrow();
        let (on_sakura, on_kero) = (
            rt.choice_active(&ActorKey::from("0")),
            rt.choice_active(&ActorKey::from("1")),
        );
        assert!(
            !on_sakura && on_kero,
            "メニューの選択肢が相方側へ載っていない（本体側 {on_sakura}・相方側 {on_kero}）——台本の `\\1` が実物 `menu.pasta:15` と食い違っている"
        );
    }

    // ── 段 7〜9: 選択確定 → サブメニューと戻り（窓 2 つ） ──
    //
    //     選択肢 ID は直前に再生中の台本の `\q` 帳簿に含まれていなければ弾かれるため、1 つ選ぶ
    //     ごとに次の台本を選択待ちまで再生し切る。「サブメニューと戻り」は 2 つ選ぶので窓を 2 つ使う。
    for (stage_name, id, label) in [
        (
            "選択確定",
            CHOICE_TALK_INTERVAL_MENU,
            LABEL_TALK_INTERVAL_MENU,
        ),
        ("サブメニューと戻り", CHOICE_MAIN_MENU, LABEL_MAIN_MENU),
        ("サブメニューと戻り", CHOICE_MOVE_MENU, LABEL_MOVE_MENU),
    ] {
        let probe = handle.clone();
        let watch = Rc::clone(&observed);
        let report = handle.clone();
        let report_pos = Rc::clone(&observed);
        // 選択の確定ごとに起きる会話は 1 本（同名イベントの応答）。
        let talks_target = observed.borrow().talks_started + 1;
        let choice_stage = stages.take(stage_name);
        let landed_id = id.to_string();
        sink.advance_gate = AdvanceGate::AfterLanding {
            landed: Box::new(move |observed, handle| {
                get_calls(&handle.non_status_calls(), &landed_id) >= 1
                    && observed.talks_started >= talks_target
            }),
        };
        let chosen = expect_stage_with(
            driver.run_stage(
                &mut sink,
                &StagePlan {
                    stage: choice_stage,
                    once: vec![Injection::Choice(choice(id, label))],
                    waiting: OBSERVE_AFTER_READY,
                },
                |progress| {
                    get_calls(&probe.non_status_calls(), id) >= 1
                        && watch
                            .borrow()
                            .played_out_since(talks_target, progress.now_ms)
                },
            ),
            || choice_breakdown(&report, id, &report_pos),
        );
        assert_budget_not_exhausted(&chosen, choice_stage);
        collect_display(&mut display, chosen);
    }

    // ── 段 10: 位置調整（移動指令が受信端へ届き、対象窓が算出位置へ動く） ──
    let probe = handle.clone();
    let watch = Rc::clone(&observed);
    let report = handle.clone();
    let report_pos = Rc::clone(&observed);
    let talks_target = observed.borrow().talks_started + 1;
    // 位置調整も完了条件が丸ごと時刻に依らない（照会・会話の起動・窓が算出位置へ動いたこと）。
    // 窓の移動まで門に含めるのが要点で、含めないと移動の着地を待つあいだ時計が走る。
    sink.advance_gate = AdvanceGate::AfterLanding {
        landed: Box::new(move |observed, handle| {
            get_calls(&handle.non_status_calls(), CHOICE_MOVE_APPLY) >= 1
                && observed.talks_started >= talks_target
                && observed.move_target_pos == Some(MOVE_LANDING)
        }),
    };
    let move_stage = stages.take("位置調整");
    let moved = expect_stage_with(
        driver.run_stage(
            &mut sink,
            &StagePlan {
                stage: move_stage,
                once: vec![Injection::Choice(choice(
                    CHOICE_MOVE_APPLY,
                    LABEL_MOVE_APPLY,
                ))],
                waiting: WaitInjection::DispatcherTick,
            },
            |_| {
                get_calls(&probe.non_status_calls(), CHOICE_MOVE_APPLY) >= 1
                    && watch.borrow().move_target_pos == Some(MOVE_LANDING)
            },
        ),
        || choice_breakdown(&report, CHOICE_MOVE_APPLY, &report_pos),
    );
    assert_budget_not_exhausted(&moved, move_stage);
    collect_display(&mut display, moved);

    // ── 段 11: 終了（終了指示 → 終了挨拶の再生 → 解放 → kanade の自己終了の観測） ──
    //
    //     自己終了は「送ってみる」ほかに観測手段が無い（`std::sync::mpsc::Sender` に開閉を問う口が
    //     無い）。駆動器の探りは副作用指示 0 件の `KanadeMsg::Boot` を使うので交信の列を 1 件も
    //     増やさず、再生側 Tick と組で投函されるので握手の再生も同時に進む。
    let talks_target = observed.borrow().talks_started + 1;
    // 終了挨拶が起動するまで据え置き、起動したら終了指令へ届くだけ時計を進める。解放と自己終了を
    // 待つ相は探りが担うので時計は要らない（探りは据え置きにも頭打ちにも掛からない）。
    sink.advance_gate = AdvanceGate::AfterLanding {
        landed: Box::new(move |observed, handle| {
            get_calls(&handle.non_status_calls(), "OnClose") >= 1
                && observed.talks_started >= talks_target
        }),
    };
    let close_stage = stages.take("終了");
    let closing = expect_stage(driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: close_stage,
            once: vec![Injection::CloseRequest(CloseReason::User)],
            waiting: WaitInjection::DispatcherTickAndKanadeProbe,
        },
        |progress| progress.closed.kanade,
    ));
    assert!(
        closing.kanade_probes > 0,
        "段「終了」: 自己終了の探りが 1 本も投函されていない"
    );
    assert_budget_not_exhausted(&closing, close_stage);
    collect_display(&mut display, closing);
    stages.assert_exhausted();

    // ── 判定は段の駆動をすべて終えた後にまとめて行う（段の途中で部分照合しない・design D1） ──
    let ledgers = LapLedgers {
        calls: handle.non_status_calls(),
        statuses: handle.status_calls(),
        display,
    };

    // 再生を運ぶ本数（[`PLAYOUT_TICKS`]）が製品の報告した占有終端を覆っていたこと（自己検査）。
    assert_playout_covers_horizon(&observed.borrow());
    // 3 つの列を期待と等値で突き合わせる（design D3・R2.3／2.4／2.5・R3.6／3.7）。
    conformance_judge::judge_lap(&ledgers);

    // 後片付け（既存の有界な畳み方）。上の終了段が kanade の自己終了まで観測済みゆえ、強制終了の
    // 届く先はもう無い（設計討議 #1 裁定）。
    harness.shutdown_bounded();
}

// 主題単位の分割（R2.11）: 3 つの列の等値照合は兄弟ファイルへ置き、**本ファイルの末尾から**接続する。
// 本ファイルは 1 ファイル 1,000 行の見張りまで余白が僅かで、`spine.rs` の接続宣言は 3 本で確定済み
// ゆえ、経路をそちらへ増やさない（task 2.3 が支援層で踏んだ手順と同じ）。
#[cfg(test)]
#[path = "spine_conformance_judge.rs"]
mod conformance_judge;
