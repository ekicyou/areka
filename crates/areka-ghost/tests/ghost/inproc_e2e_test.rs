//! I1 canonical 一周 e2e（task 5.1・design.md「inproc 決定論 e2e（I1〜I3）」シナリオ I1）。
//!
//! spine の S1（`spine_e2e_test.rs` の `s1_boot_success`）を **実 InProc DLL 境界越し**で再演する。
//! S1 は台本 fake backend（`ScriptedShioriBackend`）が `\s[0]hello\e` を返すのに対し、本 I1 は
//! `ShioriWiring::InProc` で実ビルド済み `shiori4_testdll.dll` をロードし、その OnBoot 応答（凍結
//! スナップショット由来の起動挨拶）が kanade→start-relay→dispatcher→sakura→`RecordingSink` へ
//! 届くまでを **注入時刻のみ**（時刻前進＝talk 進行は単調増加する `DispatcherMsg::Tick` の `now` 注入のみ・
//! 実時計は talk を進めない）で駆動し、演出 cue 列を凍結台本由来の期待列と **全順序**で照合する
//! （要件 5.1/5.5・design.md I1）。駆動ループは反復回数境界でなく **壁時計 deadline 境界**（task 2.4 方式）で
//! 括り、poll 周期は `yield_now()`（sleep 不使用・要件 1.3）——deadline は宙吊り防止の上限にすぎず sim 時刻は
//! 注入 Tick の `now` のみが進める。
//!
//! さらに凍結応答が差し替えられた際の検出漏れを塞ぐため、期待列と凍結スナップショット
//! （`shiori4_testdll::snapshot_for("OnBoot")`）の整合を **ドリフト検出 assert** で固定する（要件 5.2）。
//! 正常終了の握手は `shutdown(CloseReason::System)==Ok(())` で観測する（要件 5.3）。観測は演出配送の
//! **受領レベル**（`RecordingSink`）に留め、実描画（サーフェス合成・画素読戻し）は要求しない（要件 5.5）。
//!
//! # 兄弟 spine テストとの共存（tasks.md「[4.2 重大・5.1/5.2必読]」＋ task 5.0）
//! 本テストは同一 `ghost` テストバイナリ内で spine の協調ループと並走する。自前 assemble をせず
//! `inproc_fixture::shared_test_ghost()`（OnceLock・assemble 一度・hardlink 優先）を再利用して新規バイト
//! 書き込み由来の Defender 再スキャン＝spine starvation を避け、駆動ループは反復回数境界でなく **壁時計
//! deadline 境界**（task 2.4 の runtime.rs InProc 統合テスト方式）で括る。spine のプローブループは task 5.0 で
//! 同じ壁時計 deadline へ硬化済みゆえ、本テストの並走による CPU 競合は spine の cue 到達を deadline 内で遅らせる
//! だけで偽陽性を生まない（5.0 前は spine が反復回数境界の空 spin で早合点し飢餓したため 2ms sleep で緩和して
//! いたが、その応急措置は 5.0 完了で撤去した）。残る低頻度残存はこの並列マルチエージェント build セッション
//! 特有の兄弟コンパイル burst 下でのみ稀に spine settle ループが飢餓するもので、本テスト自体は無関係。

use std::time::{Duration, Instant};

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostBootOptions, ShioriWiring, TickerMode, boot};
use areka_kanade::{CloseReason, MonotonicMs};
use areka_parsers::charset::DefaultEncoding;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

use crate::inproc_fixture::shared_test_ghost;
use crate::spine_e2e_test::RecordingSink;

/// 凍結 OnBoot スナップショットの Value（起動挨拶さくらスクリプト・tasks.md「[1.3 提供データ]」）。
///
/// 期待 cue 列（下記）とドリフト検出 assert の **単一の値源**。task 6.2 が実採取で PROVISIONAL を
/// 置換する際は、この定数と期待列を凍結後の実データへ更新する（tasks.md 6.2・要件 2.6）。
const EXPECTED_ONBOOT_VALUE: &str = r"\0\s[0]おはようございますわ（暫定）\e";

/// 起動挨拶のテキスト本文（`EXPECTED_ONBOOT_VALUE` の `\s[0]` と `\e` に挟まれた表示文字列）。
///
/// 期待 Text cue の内容をこの定数から導出し、値源との連結を明示する（task 5.1）。
const EXPECTED_GREETING_TEXT: &str = "おはようございますわ（暫定）";

/// 有界待機ヘルパ（spine S1／runtime.rs の `run_bounded` と同旨のローカルコピー）。`shutdown` を
/// 別スレッドへ逃がし `recv_timeout` で宙吊りを防ぐ（`ActorHandle::join` 由来のブロックを括る）。
fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
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

/// I1: 起動から終話までの一周を注入時刻のみで駆動し、起動挨拶の演出列が凍結台本由来の期待列と
/// 全順序で一致することを照合する（＋ドリフト検出＋正常終了握手・要件 5.1/5.2/5.3/5.4/5.5/1.1/1.3）。
#[test]
fn i1_inproc_one_lap_records_frozen_onboot_greeting_and_closes_cleanly() {
    // ---- ドリフト検出（要件 5.2）: 期待列の値源（EXPECTED_ONBOOT_VALUE）が実 DLL の凍結
    //      スナップショットと一致することを先に固定する。凍結応答が task 6.2 等で差し替えられた
    //      のに期待列が追随していない場合、この assert（および後段の cue 列 assert）が確実に fail し、
    //      検出漏れ（silent drift）を塞ぐ。加えて期待 Text 本文が値源に含まれることも連結固定する。 ----
    let snapshot = shiori4_testdll::snapshot_for("OnBoot")
        .expect("OnBoot は shiori4-testdll のスナップショット表に収載されていること（narrowing: GET）");
    assert!(
        snapshot.contains(EXPECTED_ONBOOT_VALUE),
        "ドリフト検出: 凍結 OnBoot スナップショットが期待 Value を含まない。\
         スナップショットが差し替えられたら期待列（EXPECTED_ONBOOT_VALUE）も更新すること（要件 5.2・tasks.md 6.2）。\n\
         expected Value = {EXPECTED_ONBOOT_VALUE:?}\n\
         snapshot       = {snapshot:?}"
    );
    assert!(
        EXPECTED_ONBOOT_VALUE.contains(EXPECTED_GREETING_TEXT),
        "期待 Text 本文（EXPECTED_GREETING_TEXT）は値源 EXPECTED_ONBOOT_VALUE から導出される連結であること"
    );

    // ---- 実 InProc 一周の駆動 ----
    // 自前 assemble せず共有 fixture を再利用（tasks.md「[4.2 重大]」・spine starvation 回避）。
    let ghost_root = shared_test_ghost().to_path_buf();

    // broadcast ゆえ surface/text の両 sink が同一の全 cue を受ける（S1 と同律）。boot 前に records
    // ハンドルを取得しておく。
    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

    let options = GhostBootOptions {
        ghost_root,
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::InProc,
        surface_sink,
        text_sink,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed through ShioriWiring::InProc（実テスト DLL）");

    // 期待 broadcast 列（S1 と同構造）: `\0\s[0]<greeting>\e` は
    //   ClearAll@0（#6 全消去・talk 冒頭前置）/ Emote{key:"0"}@0（\s[0]）/ Text(<greeting>)@0
    // へコンパイルされる（`\0` は既定 scope 0 の明示ゆえ actor は "0" のまま・後続 cue が無く先頭群に
    // 留まる）。値源 EXPECTED_GREETING_TEXT から Text 内容を導出する。
    let expected: Vec<CueCommand> = vec![
        CueCommand::ClearAll,
        CueCommand::Emote {
            key: "0".to_string(),
        },
        CueCommand::Text(EXPECTED_GREETING_TEXT.to_string()),
    ];

    // 駆動: dispatcher へ Tick を注入し、両 sink が期待列長へ整定するまで待つ。反復回数ではなく
    // **壁時計デッドライン**で括る（task 2.4 方式）——InProc 経路は最初の SHIORI 呼出で実 DLL の
    // `LoadLibraryW`＋`CreateInstance` を初めて走らせ数十 ms を要するため。デッドラインは宙吊り防止の上限に
    // すぎず、talk timeline の前進は依然として注入 Tick の `now` のみ（実時計で進めない・要件 5.1）。DLL
    // ロード完了後は最初の Tick で挨拶が全 broadcast されるため、捕捉の有無・cue 列・順序は決定論的。
    //
    // poll 周期は `yield_now()`（task 2.4 先例と同形・sleep 不使用）。兄弟 spine のプローブループは task 5.0 で
    // 壁時計 deadline へ硬化済みゆえ、本テストの並走による CPU 競合は spine の cue 到達を deadline 内で遅らせる
    // だけで偽陽性を生まない（5.0 前は spine が反復回数境界の空 spin で早合点し飢餓したため 2ms sleep で緩和して
    // いたが、その応急措置は 5.0 完了で不要）。sim 時刻は依然として注入 Tick の `now` のみが進める。
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut now: u64 = 1;
    let mut captured = false;
    while Instant::now() < deadline {
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher actor should still be alive while probing for the boot talk");
        now += 1;
        let s = surface_records.lock().expect("records mutex poisoned").len();
        let t = text_records.lock().expect("records mutex poisoned").len();
        if s >= expected.len() && t >= expected.len() {
            captured = true;
            break;
        }
        // hang-guard の poll 周期（talk pacing ではない）。sim 時刻は注入 Tick の `now` のみが進める。
        std::thread::yield_now();
    }
    assert!(
        captured,
        "I1: 起動挨拶の演出列が壁時計デッドライン内に両 sink へ届かなかった——実 InProc DLL 境界を \
         横断した一周が成立していない（hang guard）"
    );

    // ---- 演出列の全順序照合（要件 5.1・broadcast・at 昇順・内容一致）----
    let surface = surface_records
        .lock()
        .expect("records mutex poisoned")
        .clone();
    let text = text_records.lock().expect("records mutex poisoned").clone();

    let assert_broadcast = |cues: &[TalkCue], who: &str| {
        let commands: Vec<CueCommand> = cues.iter().map(|c| c.command.clone()).collect();
        assert_eq!(
            commands, expected,
            "{who} sink は broadcast で ClearAll/Emote{{0}}/Text(挨拶) を全順序で受けること \
             （partition は演者側 relevance・凍結台本由来）: {cues:?}"
        );
        for cue in cues {
            assert_eq!(cue.at, 0.0, "{who} 発火は全て at=0.0（先頭群・後続 cue なし）");
            assert_eq!(
                cue.actor,
                ActorKey::from("0"),
                "{who} 発火 actor は \\0 が明示する既定 scope 0"
            );
        }
        for pair in cues.windows(2) {
            assert!(pair[0].at <= pair[1].at, "{who} 発火列は at 昇順であるべき");
        }
    };
    assert_broadcast(&surface, "surface");
    assert_broadcast(&text, "text");

    // ---- 正常終了の握手（要件 5.3）----
    // shutdown（ForceQuit→OnClose NOTIFY→Unload）が clean に完走し `Ok(())` を返すこと。有界待機で
    // 宙吊りを防ぐ（`shutdown` 内部の全 actor join を括る）。
    run_bounded(
        "shutdown after I1 boot talk completion",
        Duration::from_secs(15),
        move || {
            let result = runtime.shutdown(CloseReason::System);
            assert!(
                result.is_ok(),
                "正常終了の握手: shutdown は I1 一周後に Ok(()) を返すこと（clean close）, got {result:?}"
            );
        },
    );

    // 共有 fixture（shared_test_ghost）はプロセス寿命 leak ゆえ本テストでは削除しない（意図的・
    // inproc_fixture.rs のドキュメント参照）。
}
