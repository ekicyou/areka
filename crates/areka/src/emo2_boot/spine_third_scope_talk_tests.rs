//! 窓もバルーンも無い scope を名指す台本（`\p[2]`）の会話が止まらずに最後まで進み、`\-` で終わる
//! 終了の台本の後に「終了」で終わる（spec: areka-P0-emily-ghost-verification 要件 4.1・4.2・4.5・
//! 6.7・タスク 3.2・設計「会話が止まらないテスト」）。
//!
//! えみりの形の固定入力（`char2.name`・`char2.*`〜`char4.*` を持つ）から配置の設定が返す scope は
//! 0・1 だけになった（タスク 3.1）。窓と資産はその scope だけで組まれ、台本の `\p[2]` は「窓も
//! バルーンも無い scope」を指す。本檻はその形の結線（scripted SHIORI・実 sink・実 presenter）に、
//! scope 2 を挟む起動の台本と終了の台本を流し、次を確かめる:
//!
//! 1. scope 2 の合図は areka の側まで届き（shell 表示対象 `shell_target(2)` への面の指令）、
//!    窓の無い対象として適用されても、その後ろの scope 0 の合図（目印の面 2100）が届く。
//! 2. 再生の終わりの知らせ（`TalkEnded`）が来る＝待つ側が止まらない。
//! 3. 終了の台本の後に解放（`Unload`）へ進み、最初の終了の出所が `KanadeStopped(Quit)`
//!    （告知なし・終了コード 0）になる。期限超過（`DeadlineExceeded`）や強制終了ではない。
//!
//! 時刻は注入 Tick だけが進める。1 つの会話の注入時刻は [`TALK_SPAN_CAP_MS`] で頭打ちにし、
//! 運行の終了挨拶の期限（30 秒）へ注入時刻だけで届かないようにする（注入時刻が観測を追い越さない）。
//! 頭打ちのまま進みが止まれば待ちの芯が `［止まった］` で落とす＝会話が止まったことの赤になる。

use std::cell::{Cell, RefCell};

use areka_kanade::{KanadeMsg, KanadeStopCause};
use areka_sakura::TalkId;
use shiori_host32_host::ExitKind;

use super::{
    CloseReason, Duration, GhostWindows, Instant, LoopDriver, PresentCommand, Progress,
    RecordedCall, SPIN_WAIT, ScriptedShioriBackend, SpineHarness, TalkLifecycleSignal,
    capture_logs, run_attach_phase, shell_target, wait_until,
};
use crate::app_exit::{ExitOrigin, FirstExit, fault_of};
use crate::emo2_boot::frame::run_ghost_quit_phase;
use crate::placement::shared_test_support::emily_shaped_scopes;

/// 起動の台本。scope 0・1 の台詞の間に scope 2 の面と台詞を挟み、その後ろへ scope 0 の目印の面
/// （2100）を置く。目印が届けば、再生は scope 2 の区間を越えて最後まで進んでいる。
const BOOT_TALK: &str = r"\0\s[0]ひとりめ。\1ふたりめ。\p[2]\s[0]さんにんめ。\0\s[2100]おわり。\e";

/// 終了の台本（`OnClose` の応答）。scope 2 を挟み、えみりの別れの台詞と同じく `\-` で終わる。
/// scope 2 の面は起動の台本と変える（同じ面の指令は SERIKO が重ねて出さないため・同値は発行しない）。
const CLOSE_TALK: &str = r"\0\s[0]またね。\p[2]\s[2100]さようなら。\0\s[2100]ばいばい。\-";

/// 目印の面（scope 0 の最後の合図）。
const MARKER_SURFACE: u32 = 2100;

/// 1 反復あたりの注入時刻の進み幅（ms）。
const TICK_STEP_MS: u64 = 100;

/// 1 つの会話に注入する時刻の上限（ms）。台本の再生は 2 秒に届かないので十分に長く、終了挨拶の
/// 期限 30 秒（`KanadeConfig::close_talk_deadline_ms`）より十分に短い。
const TALK_SPAN_CAP_MS: u64 = 10_000;

/// ポーリングの間隔（spine の「ハイブリッド待ち」の poll-backoff・R7.9）。
const BACKOFF: Duration = Duration::from_micros(200);

/// 起動の台本と、`\-` で終わる終了の台本を返す台本を組む（`OnClose` は正規の握手の GET）。
fn third_scope_backend() -> SpineHarness {
    let (backend, shiori_handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(BOOT_TALK.to_string())))
        .notify("basewareversion", Ok(()))
        .get("OnClose", Ok(Some(CLOSE_TALK.to_string())))
        .unload(Ok(ExitKind::Clean))
        .build();
    SpineHarness::boot_with(backend, shiori_handle, LoopDriver::Inert)
}

/// 1 つの会話の観測。面の指令は `(対象, 面)` で、来た順に残す。
#[derive(Default)]
struct TalkSeen {
    shows: Vec<(areka_emo_present::TargetId, u32)>,
    signals: Vec<TalkLifecycleSignal>,
}

impl TalkSeen {
    /// この観測の中で最初に始まった会話の番号と、その会話が終わったか。
    fn first_talk(&self) -> Option<(Option<TalkId>, bool)> {
        let start = self
            .signals
            .iter()
            .position(|s| matches!(s, TalkLifecycleSignal::TalkStarted { .. }))?;
        let TalkLifecycleSignal::TalkStarted { talk_id } = self.signals[start] else {
            unreachable!("position で TalkStarted を選別済み");
        };
        let ended = self.signals[start..]
            .iter()
            .any(|s| matches!(s, TalkLifecycleSignal::TalkEnded { .. }));
        Some((talk_id, ended))
    }

    fn position(&self, target: areka_emo_present::TargetId, surface: u32) -> Option<usize> {
        self.shows
            .iter()
            .position(|&(t, s)| t == target && s == surface)
    }
}

/// 注入 Tick で会話を運び、`done` が真になるまで面の指令と表示の知らせを集める。
///
/// 受けた面の指令は本番の取り出しと同じく presenter へ適用する（窓の無い scope 2 の対象も含む）。
/// 注入時刻は `*now` から [`TALK_SPAN_CAP_MS`] で頭打ち。進みの目印は SHIORI の呼び出しの数と
/// 受けた指令・知らせの数の和で、30 秒進まなければ待ちの芯が `［止まった］` で落とす。
fn drive_talk(
    harness: &mut SpineHarness,
    now: &mut u64,
    what: &str,
    mut done: impl FnMut(&TalkSeen, &[RecordedCall]) -> bool,
) -> TalkSeen {
    let cap = *now + TALK_SPAN_CAP_MS;
    let seen = RefCell::new(TalkSeen::default());
    let received = Cell::new(0u64);
    let shiori = harness.shiori_handle.clone();
    let probe = || shiori.call_count() + received.get();
    let reached = wait_until(what, Progress::Count(&probe), || {
        *now = (*now + TICK_STEP_MS).min(cap);
        harness.inject_dispatcher_tick(*now);
        let mut seen = seen.borrow_mut();
        for cmd in harness.wiring.drain_received() {
            if let PresentCommand::ShowSurface {
                target, surface_id, ..
            } = &cmd
            {
                seen.shows.push((*target, *surface_id));
            }
            harness.wiring.apply_present(&mut harness.world, cmd);
            received.set(received.get() + 1);
        }
        for signal in harness.wiring.drain_lifecycle_signals() {
            seen.signals.push(signal);
            received.set(received.get() + 1);
        }
        harness.pump_text();
        if done(&seen, &harness.shiori_handle.non_status_calls()) {
            return true;
        }
        std::thread::sleep(BACKOFF);
        false
    });
    if let Err(failure) = reached {
        panic!("{failure}／{what}: 会話が止まった（再生の終わりまで進まない）");
    }
    seen.into_inner()
}

/// 1 つの会話が scope 2 の区間を越えて最後まで進んだことを確かめる。
fn assert_ran_past_scope_two(seen: &TalkSeen, third_surface: u32, what: &str) {
    let third = seen.position(shell_target(2), third_surface);
    let marker = seen.position(shell_target(0), MARKER_SURFACE);
    assert!(
        third.is_some(),
        "{what}: scope 2 の面の指令（shell_target(2)・面 {third_surface}）が areka の側へ届く（`\\p[2]` の道を通った）: {:?}",
        seen.shows
    );
    assert!(
        marker.is_some() && marker > third,
        "{what}: scope 2 の後ろの scope 0 の目印（面 {MARKER_SURFACE}）が届く＝再生が scope 2 の区間で止まらない: {:?}",
        seen.shows
    );
    assert!(
        matches!(seen.first_talk(), Some((Some(_), true))),
        "{what}: 会話が始まって終わる（TalkStarted → TalkEnded）: {:?}",
        seen.signals
    );
}

/// えみりの形の配置（scope 0・1 の窓だけ）で、`\p[2]` を挟む起動の会話が最後まで進み、`\p[2]` を
/// 挟んで `\-` で終わる終了の会話の後に「終了」で終わる。
///
/// # 非空虚性
///
/// - 配置がえみりの形から 3 人目の窓を作れば（タスク 3.1 の後退）、最初の表明が落ちる。
/// - 台本が scope 2 を通らなければ、`shell_target(2)` の面の指令が無く落ちる。
/// - scope 2 の区間で再生が止まれば、目印の面も `TalkEnded` も来ず、頭打ちの時刻のまま待ちの芯が
///   `［止まった］` で落とす。
/// - 終了の会話が終わらず期限で畳まれれば、最初の終了の出所が `DeadlineExceeded` になり落ちる。
#[test]
fn spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits() {
    let mut harness = third_scope_backend();

    // ── 配置はえみりの形の固定入力が返す scope と同じ（0・1 だけ・窓もバルーンも scope 2 には無い） ──
    let fixture_scopes = emily_shaped_scopes();
    assert_eq!(
        fixture_scopes,
        vec![0, 1],
        "えみりの形の固定入力から配置の設定が返す scope は 0・1 だけ"
    );
    let window_scopes: Vec<usize> = harness.world.resource::<GhostWindows>().scopes().collect();
    assert_eq!(
        window_scopes, fixture_scopes,
        "結線の窓はえみりの形の配置と同じ scope で立っている"
    );
    let logs = capture_logs(|| run_attach_phase(&mut harness.wiring, &mut harness.world));
    assert!(
        logs.iter()
            .any(|l| l.contains("planned=2") && l.contains("attached=2")),
        "装着は scope 0・1 の 2 つだけ（planned=2 attached=2）: {logs:?}"
    );
    assert_eq!(
        harness.wiring.balloon_model_scopes(),
        vec![0, 1],
        "バルーンは scope 0・1 だけ（scope 2 には無い）"
    );

    // ── 起動の会話: scope 2 を越えて最後まで進み、終わりの知らせが来る ──
    let mut now = 0u64;
    let boot = drive_talk(&mut harness, &mut now, "起動の会話", |seen, _| {
        matches!(seen.first_talk(), Some((_, true)))
    });
    assert_ran_past_scope_two(&boot, 0, "起動の会話");

    // ── 終了指示（製品では右クリックメニューの「終了」が送るのと同じ 1 件） ──
    harness
        .ghost
        .kanade()
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("kanade は生存しており終了指示を受け取る");

    // ── 終了の会話: scope 2 を越えて `\-` まで進み、解放へ進む ──
    let close = drive_talk(&mut harness, &mut now, "終了の会話", |seen, calls| {
        matches!(seen.first_talk(), Some((_, true)))
            && calls.iter().any(|c| matches!(c, RecordedCall::Unload))
    });
    assert_ran_past_scope_two(&close, MARKER_SURFACE, "終了の会話");
    assert_ne!(
        close.first_talk().map(|(id, _)| id),
        boot.first_talk().map(|(id, _)| id),
        "終了の会話は起動の会話とは別の会話"
    );
    let calls = harness.shiori_handle.non_status_calls();
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose")),
        "終了の台本は OnClose の GET で照会される（正規の握手）: {calls:?}"
    );

    // ── 停止の知らせを終了相が読み、終わり方が「終了」になる ──
    let deadline = Instant::now() + SPIN_WAIT;
    let mut quit_consumed = false;
    while Instant::now() < deadline {
        if run_ghost_quit_phase(&mut harness.world) {
            quit_consumed = true;
            break;
        }
        std::thread::sleep(BACKOFF);
    }
    assert!(
        quit_consumed,
        "kanade の停止の知らせが有界内に UI へ届かない"
    );
    let origin = harness.world.resource::<FirstExit>().0.clone();
    assert_eq!(
        origin,
        ExitOrigin::KanadeStopped(KanadeStopCause::Quit),
        "終了の台本の後の終わり方は「終了」（期限超過・強制終了ではない）"
    );
    assert!(
        fault_of(&origin).is_none(),
        "きれいな終了＝告知なし・終了コード 0"
    );

    harness.shutdown_bounded();
}
