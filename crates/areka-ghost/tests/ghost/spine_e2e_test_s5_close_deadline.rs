// ===================== S5: close deadline 超過シナリオ（task 4.6） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S5:
// 「close deadline: close talk を意図的に完了させず、`KanadeMsg::Tick` の `now` を deadline
// 超過まで注入（既定 30_000ms を数値的に跨ぐ `now` を投函するだけ・実時間ゼロ・短縮構成
// 不要）→`Unloading{DeadlineExceeded}`→`Unload`→全 join。」を、S4（close 握手・正規の
// Quit 経路）と対をなす「close talk が自然完了しない」経路として駆動する（要件
// 7.4/7.5/7.6）。
//
// close talk の script は `\_w[49999950]this-never-completes\-`（`\_w[49999950]`＝約13.9時間
// 相当の待ち・`\_w[ms]` は絶対ミリ秒）にする——先頭に待ちを置く
// ことで空 sheet 高速経路（bare quit `\-`・S4 参照）を踏まず、実際に「再生完了通知が来ない」
// 状態を作る。本シナリオは close talk 開始後、dispatcher へ一切 Tick を送らない（送れば
// `\w` の経過秒が進み得る）ため、close talk は spawn 直後の待ちで恒久的に止まったまま
// になる——kanade 側の deadline 判定だけが `runtime.kanade()` への直接 `Tick` 注入で駆動する。
//
// # deadline の起点計算（`close.rs::deadline_from`・close.rs モジュール doc 参照）
// 本シナリオは S1/S4 と同じ boot-settling 技法（dispatcher への Tick 注入のみ）を使う——
// dispatcher への Tick は kanade 自身の `last_now` を一切更新しない（`KanadeMsg::Tick` を
// 受けたときのみ更新される・`schedule/steady.rs`/`schedule/close.rs` の `last_now` 更新箇所を
// 直接確認済み）。ゆえに `CloseRequest` 送出時点で kanade の `last_now` は依然 `None` であり、
// `ClosePending`→`CloseTalkWait` 遷移時の `deadline_from(None, ..)` は `None`（未確定）で
// `CloseTalkWait` に入る（close.rs「握手入口で last_now が None だった場合は deadline を
// None のまま入り、CloseTalkWait 最初の Tick 受領時点を起点に上限を設定する」）。
//
// `kanade` は `run_inbox`（`areka-kanade/src/actor.rs`）で 1 メッセージずつ**完全に同期**
// 処理する（`drive()` が OnClose の同期往復・状態遷移まで完結させてから次の inbox
// メッセージを取り出す）ため、`CloseRequest`→(直後に送る)`Tick` の到達順序は mpsc の
// FIFO 保証と合わせて完全に決定論的である——`CloseRequest` が処理し終わる（＝
// `CloseTalkWait` へ遷移済み）前に後続の `Tick` が処理されることはない。ゆえに:
// - 1本目の `Tick{now: arm_now}` → `CloseTalkWait` に入って初めて受ける Tick ゆえ
//   deadline を `arm_now + close_talk_deadline_ms` へ**確定するだけ**（超過判定はしない・
//   close.rs の `None` 分岐は比較を行わない）。
// - 2本目の `Tick{now: arm_now + close_talk_deadline_ms}` → `now >= deadline` で確実に
//   超過 → `Unloading{DeadlineExceeded}` → `ShioriUnload`。
// 2 本の Tick 送出そのものは即座に返る（inbox への enqueue のみ）ため、その後の
// `Unload` 呼出の実際の発火（kanade スレッドが実際に処理し終わる時点）は有界スピン
// 待機（`yield_now` のみ・sleep も追加 Tick も伴わない）で確認する。

use super::*;

use areka_ghost::{
    GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot_with_kanade_stop,
};
use areka_kanade::{
    CloseReason, KanadeConfig, KanadeMsg, KanadeNotice, KanadeStopCause, KanadeStopped,
    MonotonicMs, events,
};
use areka_parsers::charset::DefaultEncoding;

/// S5: close deadline 超過——close talk を意図的に完了させず（`\_w[49999950]`＝約13.9時間
/// 相当の待ちで恒久的に止める）、`runtime.kanade()` へ `Tick` を 2 回注入するだけで
/// （1本目で deadline を確定・2本目で超過を跨ぐ）、`Unloading{DeadlineExceeded}`→
/// scripted `Ok(ExitKind::Clean)` の `Unload`→`Unloaded` 観測→`StopSelf` へ完走し、停止の通知の
/// 原因が `DeadlineExceeded` であること（呼び出しの列は通常の Quit と同じなので、締切の道を
/// 通ったかは原因でしか分からない）と、`runtime.shutdown()` の全スレッド join 成功を確認する
/// （design「S5 close deadline」・要件 7.4/7.5/7.6）。
#[test]
fn s5_close_deadline_exceeded_forces_termination_via_tick_injection() {
    const SHELL_NAME: &str = "S5DeadlineShell";

    let temp = write_ghost_fixture("S5TestGhost", SHELL_NAME);
    let root = temp.path().to_path_buf();

    let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

    // boot 系列一式（S1/S4 と同旨）＋ OnClose（close talk を恒久的に止める待ち script）＋
    // unload（DeadlineExceeded 系列が発行する ShioriUnload の応答・Ok(Clean)）を台本化する。
    // OnSecondChange は台本化しない——本シナリオは kanade へ boot 完了後、CloseRequest と
    // deadline 用 Tick 2 本しか送らないため steady pump は起こらない。
    let (backend, handle) = boot_script()
        .get(
            "OnClose",
            Ok(Some(r"\_w[49999950]this-never-completes\-".to_string())),
        )
        .unload(Ok(ExitKind::Clean))
        .build();

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();

    let options = GhostBootOptions {
        strict_judges: None,
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
        system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    // 停止の通知の投函端つきで起こす（締切の道を通ったことは停止の原因で見る）。
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<KanadeNotice>();
    let runtime = boot_with_kanade_stop(options, Some(stop_tx))
        .expect("boot should succeed for a resolvable ghost_root");

    // ---- boot talk を Steady 到達まで駆動する（S1/S3/S4 と同一技法・sleep 不使用) ----
    // dispatcher への Tick は kanade 自身の last_now を更新しない（別チャンネル・別
    // 帳簿）ため、この loop を通しても kanade の last_now は None のまま維持される。
    let mut now: u64 = 1;
    super::spin_pumping_ticks(
        "S5: surface cue never fired after repeated Tick — boot talk did not reach \
         dispatcher's active slot",
        &mut now,
        dispatcher_tick(runtime.dispatcher()),
        || {
            !surface_records
                .lock()
                .expect("records mutex poisoned")
                .is_empty()
        },
    );

    // ---- 終了要求（正規/canonical）: CloseRequest を kanade へ送る ----
    runtime
        .kanade()
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("kanade actor should still be alive to receive the close request");

    // ---- close 握手が Steady を抜けて OnClose GET を発行するまで有界スピン待機する ----
    // DD-IT-12: boot は挨拶 talk を追跡し `Steady{talk: Some(greeting)}` へ完了する。ゆえに
    // CloseRequest 受領時に挨拶 talk がまだ active なら kanade は即握手せず `pending_close`
    // に記録して `Steady{Some}` を維持し、挨拶 talk の TalkDone 受領時に初めて握手を開始する
    // （steady.rs `on_close_request` / `on_talk_done`）。**kanade 宛**の Tick をこの間に
    // 送ってしまうと `Steady{Some}` の pump として消費され台本外の OnSecondChange NOTIFY を
    // 発行してしまう（CloseTalkWait の deadline も進まない）。ゆえに下の deadline 用 kanade
    // Tick は OnClose GET の出現を確認してから注入する。OnClose GET が現れた後の kanade は
    // ClosePending か CloseTalkWait のいずれかにあり、どちらでも下の 2 Tick は last_now を
    // 起点に deadline を確定・超過させる（ClosePending の Tick は last_now 更新のみ→続く
    // Value 応答で `deadline_from(Some)` 確定／CloseTalkWait の Tick は deadline=None を
    // 起点確定・close.rs 参照）。
    //
    // **一方 dispatcher 宛の Tick は注入し続けなければならない**（2026-07-30 是正）:
    // 握手の起点である挨拶 talk の `TalkDone` は仮想時刻が `hello` の horizon（0.25s）を
    // 越えて初めて出る。dispatcher の帳簿は kanade の `last_now` と独立ゆえ、ここで
    // dispatcher へ Tick を送っても上記 pump 問題は起こらない。旧実装は `yield_now` だけで
    // 待っており、「引き渡しが速く `now` が 0.25s 未満で止まった実行」では TalkDone が永久に
    // 出ず 60s 安全弁まで空転していた（実測フレーキー）。
    super::spin_pumping_ticks(
        "S5: OnClose GET was never issued after CloseRequest — the greeting-tracking close \
         deferral (DD-IT-12) never resolved into a close handshake",
        &mut now,
        dispatcher_tick(runtime.dispatcher()),
        || {
            handle
                .calls()
                .lock()
                .expect("calls mutex poisoned")
                .iter()
                .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"))
        },
    );

    // ---- deadline 超過を Tick 2 本の注入だけで駆動する（sleep 不使用・要件 7.4) ----
    // 1本目: CloseTalkWait 入場後 初めて受ける Tick——deadline を
    // `arm_now + close_talk_deadline_ms` へ確定するだけで比較はしない
    // （close.rs の deadline=None 分岐）。
    let arm_now: u64 = 5_000;
    runtime
        .kanade()
        .send(KanadeMsg::Tick {
            now: MonotonicMs(arm_now),
        })
        .expect("kanade actor should still be alive to receive the deadline-arming Tick");

    // 2本目: `now >= deadline`（`arm_now + close_talk_deadline_ms`）を確実に跨ぐ値を
    // 注入する——生産既定 30_000ms を数値的に跨ぐだけで実時間はゼロ（要件 7.4）。
    let cross_now = arm_now + config.close_talk_deadline_ms;
    runtime
        .kanade()
        .send(KanadeMsg::Tick {
            now: MonotonicMs(cross_now),
        })
        .expect("kanade actor should still be alive to receive the deadline-crossing Tick");

    // ---- deadline 超過による強制終了系列の完走を有界スピン待機で確認する ----
    // （Tick 送出は上で完了済み・以降は追加 Tick も sleep も伴わない）。
    wait_until(
        "S5: Unload was never observed after the deadline-crossing Tick — forced termination (CloseTalkWait deadline exceeded → Unload) did not complete within bound",
        || {
            handle
                .calls()
                .lock()
                .expect("calls mutex poisoned")
                .iter()
                .any(|c| matches!(c, RecordedCall::Unload))
        },
    );

    // ---- 停止の原因が締切の超過であること（通常の Quit の道へ退行すると原因が変わる）----
    // 起動を終えた `Steady` の通知が先に届くので、停止の通知まで読み飛ばす。
    let stopped: KanadeStopped = loop {
        let notice = stop_rx
            .recv_timeout(super::BOUND)
            .expect("kanade should post a stop notice after the deadline-exceeded Unload");
        if let KanadeNotice::Stopped(stopped) = notice {
            break stopped;
        }
    };
    assert_eq!(
        stopped.cause,
        KanadeStopCause::DeadlineExceeded,
        "S5: the stop cause must be DeadlineExceeded, got {stopped:?}"
    );

    // ---- (a) 起動系列＋close 開始＋強制終了系列が正典順序で発火 ----
    // 死活監視ノイズ（RecordedCall::Status）を除外して比較する（S1/S3/S4 と同旨）。
    let mut expected_sequence = expected_boot_prefix(&config);
    expected_sequence.extend([
        expected_from_shiori_call(events::on_close(
            CloseReason::User { scope: 0 },
            &areka_kanade::ExecutionSnapshot::INACTIVE,
        )),
        super::expected_translate(
            r"\_w[49999950]this-never-completes\-",
            &events::on_close(
                CloseReason::User { scope: 0 },
                &areka_kanade::ExecutionSnapshot::INACTIVE,
            ),
        ),
        RecordedCall::Unload,
    ]);
    let calls_without_status = non_status_calls(&handle);
    assert_eq!(
        calls_without_status, expected_sequence,
        "起動系列＋close 開始＋強制終了系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→\
         OnTranslate→basewareversion→OnClose→OnTranslate→Unload）が正典順序で発火していない"
    );

    // ---- 主観測: shutdown() が全スレッド join を有界時間内に完走する（要件 7.3) ----
    // deadline 超過による強制終了が既に Unload まで完走済み（上の有界待機で確認済み）
    // であるため、ここでの `ForceQuit` 送出は kanade が既に自発停止済みであることの
    // 冪等パスを実地で運動させる（S4 と同旨）。close talk（`\_w[49999950]` で止まった
    // まま）は dispatcher の active slot に残っているはずだが、dispatcher への Close
    // 送出は稼働中 active talk へ `SakuraMsg::Close` を送って即座に中断させてから
    // join する（`close_active_if_any`・dispatcher.rs）ため、恒久的に止まった close
    // talk があっても shutdown は有界時間内に完走する。
    crate::common::run_bounded_shutdown(
        "shutdown after S5 close deadline exceeded",
        super::BOUND,
        runtime,
    );
}
