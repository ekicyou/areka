//! 終了指示から窓が閉じるまでの一本道の檻（areka-P0-emo2-conformance-e2e タスク 6.9・R15.6 ⑷）。
//!
//! 症状 C（2026-09-07 走行 A の項目 13）は「終了操作が終了挨拶を素通りして即座に窓を消す」もので
//! あった。機序は 2 つの欠落——操作が `CloseRequest` を送らないことと、握手が終わっても窓を閉じる
//! 者がいないことである。本檻は**その 2 つを繋いだ後の一本道**を、実 ghost 結線（scripted SHIORI）で
//! 端から端まで通す:
//!
//! `CloseRequest{User}` → `OnClose` **GET** → `\-` で終わる終了挨拶の再生 → `Unload` → kanade の
//! 停止通知 → UI の終了相 → ゴースト窓 0。
//!
//! 併せて `OnClose` が **NOTIFY ではない**ことを固定する。強制終了（`ForceQuit`）経路は `OnClose` を
//! 片道 NOTIFY で消化して辞書の終了挨拶を照会しない——それが症状 C の見え方そのものであり、
//! Method の取り違えは「挨拶が出ない」と同義である（R15.5 の生ログ判定も同じ語で読む）。
//!
//! sleep は poll-backoff にのみ用い、時刻は注入 Tick だけが進める（spine の待ちの規律・R7.9）。

use std::time::Duration;

use areka_kanade::KanadeMsg;
use bevy_ecs::prelude::With;
use shiori_host32_host::ExitKind;

use super::{
    CloseReason, Entity, Instant, LoopDriver, RecordedCall, SPIN_WAIT, ScriptedShioriBackend,
    SpineHarness, spin_wait_until,
};
use crate::emo2_boot::frame::run_ghost_quit_phase;
use crate::placement::spawn::GhostWindowMarker;

/// 終了挨拶（`OnClose` の応答）。実物と同じく **`\-`（終了指令）で終わる**。
///
/// なお運行側は別れの台詞を末尾に `\-` が在るのと同じ結果として扱うため、`\e` で終わっても
/// 終了へ進む（`crates/areka-kanade/src/schedule/close.rs` の `fn on_close_talk_wait`）。
/// 本檻が `\-` を書くのは実物の逐語に合わせるためであって、解放の条件ではない。一周テストの台本
/// （`spine_conformance_script.rs` の `CLOSE_TALK`）と同一の逐語だが、**写して持つ**——
/// 一周テストの 3 台帳は 1 バイトも動かさない約束であり、そちらの定数へ依存を張ると本檻の
/// 都合が台帳側の改変理由になり得るためである。
const CLOSE_TALK: &str = r"\0\s[0]またね。\-";

/// 起動 talk（`OnBoot` の応答）。最小の 1 面表示で終わる（既存 spine の標準台本と同形）。
const BOOT_TALK: &str = r"\s[0]\e";

/// 1 反復あたりの注入時刻の進み幅（ms）。再生を確実に終端まで運ぶため大きめに取る。
const TICK_STEP_MS: u64 = 100;

/// ポーリングの間隔（spine の「ハイブリッド待ち」の poll-backoff・R7.9）。
const BACKOFF: Duration = Duration::from_micros(200);

/// `GhostWindowMarker` 窓の現数。
fn ghost_count(harness: &mut SpineHarness) -> usize {
    harness
        .world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(&harness.world)
        .count()
}

/// 終了握手の台本（boot 系列＋起動 talk＋`OnClose` **GET**＋解放）を組む。
fn close_handshake_backend() -> SpineHarness {
    let (backend, shiori_handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(BOOT_TALK.to_string())))
        .notify("basewareversion", Ok(()))
        // 正規の握手は OnClose を **GET** で照会し、応答スクリプト（終了挨拶）を受け取る。
        .get("OnClose", Ok(Some(CLOSE_TALK.to_string())))
        .unload(Ok(ExitKind::Clean))
        .build();
    SpineHarness::boot_with(backend, shiori_handle, LoopDriver::Inert)
}

/// 終了指示 → `OnClose` GET → 終了挨拶 → 解放 → 停止通知 → 終了相 1 回で窓 0（R15.6 ⑷）。
///
/// # 非空虚性
///
/// - 終了指示が握手に入らなければ `OnClose` が 1 件も現れず、`Unload` にも到達しない（有界待ちで落ちる）。
/// - `OnClose` を NOTIFY で消化していれば GET の表明が落ちる（＝辞書の終了挨拶を照会していない）。
/// - 停止通知が出ない、または終了相がそれを読まなければ、窓が 4 枚残ったまま最後の表明が落ちる。
#[test]
fn spine_close_request_runs_the_farewell_then_the_quit_phase_closes_the_windows() {
    let mut harness = close_handshake_backend();
    let windows_before = ghost_count(&mut harness);
    assert!(
        windows_before > 0,
        "前提: 合成 GhostWindows でゴースト窓が立っている"
    );

    // ── 起動系列（非 Status 5 呼出）の完了を待つ。終了指示を boot の途中で差し込むと、
    //    再生完了通知が `BootVersion` 滞在中に届いて捨てられる狭い窓を踏み得る。 ──
    let mut boot_calls = Vec::new();
    spin_wait_until(|| {
        boot_calls = harness.shiori_handle.non_status_calls();
        boot_calls.len() >= 5
    });
    assert!(
        boot_calls.len() >= 5,
        "前提: 起動系列 5 呼出が有界内に発火する: {boot_calls:?}"
    );

    // ── 終了指示（製品では右クリックメニューの「終了」が送るのと同じ 1 件） ──
    harness
        .ghost
        .kanade()
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("kanade は生存しており終了指示を受け取る");

    // ── 再生を運ぶ（起動 talk の完了 → 保留していた終了指示の消化 → OnClose GET → 終了挨拶 →
    //    その再生完了 → 解放）。時刻は注入 Tick だけが進める。 ──
    let mut now = 0u64;
    let mut calls = Vec::new();
    let deadline = Instant::now() + SPIN_WAIT;
    let mut unloaded = false;
    while Instant::now() < deadline {
        now += TICK_STEP_MS;
        harness.inject_dispatcher_tick(now);
        calls = harness.shiori_handle.non_status_calls();
        if calls.iter().any(|c| matches!(c, RecordedCall::Unload)) {
            unloaded = true;
            break;
        }
        std::thread::sleep(BACKOFF);
    }
    assert!(
        unloaded,
        "終了指示 → 終了挨拶 → 解放 が有界内に完走しない（握手の配線が繋がっていない）: {calls:?}"
    );

    // ⑴ OnClose は **GET**（辞書へ終了挨拶を照会した証跡）。NOTIFY は強制終了経路であり、
    //    それが現れるなら挨拶を素通りしている（症状 C そのもの）。
    let get_index = calls
        .iter()
        .position(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"));
    assert!(
        get_index.is_some(),
        "OnClose は GET で照会される（正規の握手・R15.5）: {calls:?}"
    );
    assert!(
        !calls
            .iter()
            .any(|c| matches!(c, RecordedCall::Notify { id, .. } if id == "OnClose")),
        "OnClose の NOTIFY は現れない（強制終了経路を通っていない・R15.5）: {calls:?}"
    );

    // ⑵ 解放はちょうど 1 件で、OnClose GET より後に現れる。
    let unload_index = calls
        .iter()
        .position(|c| matches!(c, RecordedCall::Unload))
        .expect("上の有界待ちで Unload の存在は確定している");
    assert!(
        get_index < Some(unload_index),
        "OnClose GET → Unload の順（照会してから解放する）: {calls:?}"
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| matches!(c, RecordedCall::Unload))
            .count(),
        1,
        "解放は 1 件だけ: {calls:?}"
    );

    // ⑶ 停止通知を終了相が読み、ゴースト窓が 0 になる。通知は解放の記録の直後（マイクロ秒級）に
    //    投函されるため、相を有界に回して消化を待つ（時刻は進めない＝純粋ポーリング）。
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
        "kanade の停止通知が有界内に UI へ届かない（通知の配線が繋がっていない）"
    );
    assert_eq!(
        ghost_count(&mut harness),
        0,
        "終了相 1 回で全ゴースト窓が閉じる（R15.4）"
    );

    // 後片付け（既存の有界な畳み方）。kanade は自ら停止済みゆえ `ForceQuit` の送出は失敗し、
    // ghost 側は冪等に `debug!` で流して join を完走する（R15.4 の「以後の終了統括は冪等」）。
    harness.shutdown_bounded();
}
