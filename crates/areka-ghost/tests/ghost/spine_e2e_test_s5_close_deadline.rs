// ===================== S5: close deadline 超過シナリオ（task 4.6） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S5:
// 「close deadline: close talk を意図的に完了させず、`KanadeMsg::Tick` の `now` を deadline
// 超過まで注入（既定 30_000ms を数値的に跨ぐ `now` を投函するだけ・実時間ゼロ・短縮構成
// 不要）→`Unloading{DeadlineExceeded}`→`Unload`→全 join。」を、S4（close 握手・正規の
// Quit 経路）と対をなす「close talk が自然完了しない」経路として駆動する（要件
// 7.4/7.5/7.6）。
//
// close talk の script は `\w[999999]this-never-completes\-`（`\w[999999]`＝約13.9時間
// 相当の待ち・drive.rs のコメントが示すとおり `\w[N]`＝N×50ms）にする——先頭に待ちを置く
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

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriCall, events};
use areka_parsers::charset::DefaultEncoding;

/// このテスト専用の一意な一時ディレクトリ（S1〜S4 の流儀を踏襲）。
fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_ghost_spine_e2e_s5_tests_{tag}"));
    dir
}

/// `root` 直下に最小限の解決可能なゴーストツリーを構築する（S1/S3/S4 の
/// `write_ghost_fixture` と同旨だが、sibling module から private item は参照できない
/// ためローカルに複製する）。
fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,S5TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
    )
    .expect("write ghost descript.txt");

    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        format!("charset,UTF-8\nname,{shell_name}\n").as_bytes(),
    )
    .expect("write shell descript.txt");
}

/// events 表由来の [`ShioriCall`] をこのファイル固有の [`RecordedCall`] へ変換する
/// （S1/S4 の `expected_from_shiori_call` と同旨のローカル複製・Req 7.1）。
fn expected_from_shiori_call(call: ShioriCall) -> RecordedCall {
    match call {
        ShioriCall::Get { id, references, .. } => RecordedCall::Get {
            id: id.to_string(),
            references,
        },
        ShioriCall::Notify { id, references, .. } => RecordedCall::Notify {
            id: id.to_string(),
            references,
        },
    }
}

/// 有界待機ヘルパ（S1〜S4 と同旨のローカルコピー）。
fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: std::time::Duration, f: F) {
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

/// S5: close deadline 超過——close talk を意図的に完了させず（`\w[999999]`＝約13.9時間
/// 相当の待ちで恒久的に止める）、`runtime.kanade()` へ `Tick` を 2 回注入するだけで
/// （1本目で deadline を確定・2本目で超過を跨ぐ）、`Unloading{DeadlineExceeded}`→
/// scripted `Ok(ExitKind::Clean)` の `Unload`→`Unloaded` 観測→`StopSelf` へ完走する
/// ことを、`runtime.shutdown()` の全スレッド join 成功をもって確認する（design「S5
/// close deadline」・要件 7.4/7.5/7.6）。
#[test]
fn s5_close_deadline_exceeded_forces_termination_via_tick_injection() {
    const SHELL_NAME: &str = "S5DeadlineShell";

    let root = unique_temp_dir("s5_close_deadline_exceeded_forces_termination_via_tick_injection");
    let _ = std::fs::remove_dir_all(&root);
    write_ghost_fixture(&root, SHELL_NAME);

    let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

    // boot 系列一式（S1/S4 と同旨）＋ OnClose（close talk を恒久的に止める待ち script）＋
    // unload（DeadlineExceeded 系列が発行する ShioriUnload の応答・Ok(Clean)）を台本化する。
    // OnSecondChange は台本化しない——本シナリオは kanade へ boot 完了後、CloseRequest と
    // deadline 用 Tick 2 本しか送らないため steady pump は起こらない。
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
        // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
        // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
        .get("username", Ok(None))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
        .notify("basewareversion", Ok(()))
        .get(
            "OnClose",
            Ok(Some(r"\w[999999]this-never-completes\-".to_string())),
        )
        .unload(Ok(ExitKind::Clean))
        .build();

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();

    let options = GhostBootOptions {
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

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    // ---- boot talk を Steady 到達まで駆動する（S1/S3/S4 と同一技法・sleep 不使用) ----
    // dispatcher への Tick は kanade 自身の last_now を更新しない（別チャンネル・別
    // 帳簿）ため、この loop を通しても kanade の last_now は None のまま維持される
    // （CONCERNS 参照）。
    let mut now: u64 = 1;
    let dispatcher = runtime.dispatcher();
    let tick = |n: u64| {
        dispatcher
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(n),
            })
            .expect("dispatcher actor should still be alive while driving the boot talk");
    };
    super::spin_pumping_ticks(
        "S5: surface cue never fired after repeated Tick — boot talk did not reach \
         dispatcher's active slot",
        &mut now,
        tick,
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
            reason: CloseReason::User,
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
        tick,
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
    // （close.rs の deadline=None 分岐・CONCERNS 参照）。
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
    let mut deadline_settled = false;
    let deadline = std::time::Instant::now() + super::E2E_BOUND;
    while std::time::Instant::now() < deadline {
        let has_unload = handle
            .calls()
            .lock()
            .expect("calls mutex poisoned")
            .iter()
            .any(|c| matches!(c, RecordedCall::Unload));
        if has_unload {
            deadline_settled = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        deadline_settled,
        "S5: Unload was never observed after the deadline-crossing Tick — forced \
         termination (CloseTalkWait deadline exceeded → Unload) did not complete within bound"
    );

    // ---- (a) 起動系列＋close 開始＋強制終了系列が正典順序で発火 ----
    // 死活監視ノイズ（RecordedCall::Status）を除外して比較する（S1/S3/S4 と同旨）。
    let expected_sequence = vec![
        expected_from_shiori_call(events::on_initialize(
            &areka_kanade::ExecutionSnapshot::INACTIVE,
        )),
        // username prefetch GET（OnInitialize 後・OnFirstBoot 前・R4.1・DD-9 の唯一の期待値導出経路）。
        expected_from_shiori_call(areka_kanade::resources::resource_username(
            &areka_kanade::ExecutionSnapshot::INACTIVE,
        )),
        expected_from_shiori_call(events::on_first_boot(
            &areka_kanade::ExecutionSnapshot::INACTIVE,
            // fixture ghost に永続ファイル無し＝vanish 不在ゆえ Ref0="0"（従来値同値）。
            0,
        )),
        expected_from_shiori_call(events::on_boot(
            &config,
            &areka_kanade::ExecutionSnapshot::INACTIVE,
        )),
        expected_from_shiori_call(events::baseware_version(
            &config,
            &areka_kanade::ExecutionSnapshot::INACTIVE,
        )),
        expected_from_shiori_call(events::on_close(
            CloseReason::User,
            &areka_kanade::ExecutionSnapshot::INACTIVE,
        )),
        RecordedCall::Unload,
    ];
    let calls_without_status: Vec<RecordedCall> = handle
        .calls()
        .lock()
        .expect("calls mutex poisoned")
        .iter()
        .filter(|c| !matches!(c, RecordedCall::Status))
        .cloned()
        .collect();
    assert_eq!(
        calls_without_status, expected_sequence,
        "起動系列＋close 開始＋強制終了系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→\
         basewareversion→OnClose→Unload）が正典順序で発火していない"
    );

    // ---- 主観測: shutdown() が全スレッド join を有界時間内に完走する（要件 7.3) ----
    // deadline 超過による強制終了が既に Unload まで完走済み（上の有界待機で確認済み）
    // であるため、ここでの `ForceQuit` 送出は kanade が既に自発停止済みであることの
    // 冪等パスを実地で運動させる（S4 と同旨）。close talk（`\w[999999]` で止まった
    // まま）は dispatcher の active slot に残っているはずだが、dispatcher への Close
    // 送出は稼働中 active talk へ `SakuraMsg::Close` を送って即座に中断させてから
    // join する（`close_active_if_any`・dispatcher.rs）ため、恒久的に止まった close
    // talk があっても shutdown は有界時間内に完走する（CONCERNS 参照）。
    run_bounded(
        "shutdown after S5 close deadline exceeded",
        super::E2E_BOUND,
        move || {
            let result = runtime.shutdown(CloseReason::System);
            assert!(
                result.is_ok(),
                "shutdown should return Ok(()) after the deadline-exceeded forced \
                 termination completes, got {result:?}"
            );
        },
    );

    let _ = std::fs::remove_dir_all(&root);
}
