// ===================== S4: close 握手シナリオ（task 4.5） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S4:
// 「close 握手: `CloseRequest`→`OnClose` GET が close talk（`\-` 終端）→
// `TalkDone{Quit}`→`Unload` が呼ばれ scripted `Ok(ExitKind::Clean)`→`Unloaded` 観測
// →`StopSelf`→shutdown で全スレッド join（要件 7.3）。」を、S1（boot→Steady 到達確認の
// retry ループ技法）を踏まえたうえで、初めて「正規（canonical）の終了要求」（S2/S3 の
// Fault 駆動終了とは異なる、成功する close talk 駆動の Quit 経路）を駆動する。
//
// close talk の script は `\-`（先行 cue のない bare quit タグ）にする——
// `areka_sakura::drive` の `quit_only_script_ends_immediately_with_quit_not_ended` が
// 示すとおり、これは空 CueSheet＋`TalkEndReason::Quit` へ即時（Tick 不要）コンパイルされる
// （空 sheet 高速経路）。ゆえに close talk 自体の完了確認に Tick 注入は要らない——ただし
// OnClose GET（kanade↔shiori の同期往復）・StartTalk（start_tx→start-relay→dispatcher_tx
// の 2 hop）・TalkDone（dispatcher 自身の inbox 経由で kanade へ転送）は依然として実スレッド
// 境界を跨ぐため、有界のスピン待機（Tick 送出なし・sleep なし・`yield_now` のみ）で
// `handle.calls()` に `Unload` が現れるのを確認する。
//
// `handle.calls()` に `Unload` が現れた時点で、kanade 自身の thread は
// `round_trip_unload`（`areka-kanade/src/actor.rs`）内の `reply_rx.recv()` にまだ
// ブロック中か、既にその応答を消化して `Stopped`＋`StopSelf`（shiori へ `Close` を送り
// break）へ進んでいる——いずれの場合も kanade は「次のメッセージを inbox から取り出す」
// 前に完結するため、この時点より後で送る `runtime.shutdown()` の `ForceQuit` は
// （a) まだ処理されず thread 終了と共に破棄されるか、(b) 送出自体が失敗する
// （既に停止済み＝冪等）のいずれかであり、`unload()` が二度目に呼ばれることはない
// （`ScriptedShioriBackend::unload` は `Option::take()` で一度きり消費するため、二重呼出は
// 即座に panic するはずだが、上記の理由からこの経路には到達しない）。

use super::*;

use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, events};
use areka_parsers::charset::DefaultEncoding;

/// S4: close 握手——`CloseRequest` が `OnClose` GET を発行させ、応答スクリプト
/// （bare quit `\-`）が close talk として再生起動され、空 sheet 高速経路で即座に
/// `TalkDone{Quit}` へ終端し、kanade が横断アームで `Unloading{Quit}`→
/// scripted `Ok(ExitKind::Clean)` の `Unload`→`Unloaded` 観測→`StopSelf` へ完走する
/// ことを、`runtime.shutdown()` の全スレッド join 成功をもって確認する（design「S4
/// close 握手」・要件 7.3/7.4/7.5/7.6）。
#[test]
fn s4_close_handshake_completes_regular_shutdown_via_quit_ending_close_talk() {
    const SHELL_NAME: &str = "S4CloseShell";

    let temp = write_ghost_fixture("S4TestGhost", SHELL_NAME);
    let root = temp.path().to_path_buf();

    let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

    // boot 系列一式（S1 と同旨）＋ OnClose（bare quit `\-` を返す・close talk の trigger）
    // ＋ unload（Quit 経路の ShioriUnload が消費する唯一のスクリプト・Ok(Clean)）を
    // 台本化する。OnSecondChange は台本化しない——本シナリオは kanade へ Tick を一切
    // 送らないため steady pump は起こらない。
    let (backend, handle) = boot_script()
        .get("OnClose", Ok(Some(r"\-".to_string())))
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

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    // ---- boot talk を Steady 到達まで駆動する（S1/S3 と同一技法・sleep 不使用) ----
    // 起動直後に CloseRequest を送ると kanade がまだ boot 系列途中（Idle〜BootVersion）
    // の可能性があり（boot 中の CloseRequest は pending_close 記録のみで即握手しない）、
    // Steady 到達を待たずに送るのは不要な不確実性を招く。boot talk が dispatcher の
    // active slot に載って発火したことを surface cue の到達で確認すれば、kanade は
    // 既に（boot talk の再生完了を待たず）Steady へ到達済みである
    // （boot.rs「boot は常に Steady{talk: None} へ完了する」・S3 と同じ論拠）。
    let mut now: u64 = 1;
    super::spin_pumping_ticks(
        "S4: surface cue never fired after repeated Tick — boot talk did not reach \
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
    // S1〜S3 のいずれも一度も送っていない、この e2e ファイル初の「正規の終了要求」
    // （Fault 駆動ではない・successful close-talk 駆動の Quit 経路）。
    runtime
        .kanade()
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("kanade actor should still be alive to receive the close request");

    // ---- close 握手の完走を有界スピン待機で確認する（dispatcher Tick を注入しつつ・sleep なし) ----
    // OnClose GET→close talk（bare quit `\-`・空 sheet 高速経路で即 `TalkDone{Quit}` 発行）→
    // 横断アーム Unloading{Quit}→ShioriUnload という cascade は複数の実スレッド境界
    // （kanade↔shiori 同期往復・start-relay・dispatcher・per-talk spawn_talk スレッド・
    // dispatcher 自身の inbox 経由の kanade 転送）を跨ぐため有界スピンで待つ。
    //
    // **dispatcher への Tick 注入を続けること**（2026-07-30 是正）: CloseRequest 受領時に
    // 挨拶 talk（`hello`＝0.25s）がまだ active なら kanade は DD-IT-12 により即握手せず
    // `pending_close` を記録して `Steady{Some}` を維持し、挨拶 talk の `TalkDone` 受領時に
    // 初めて握手を開始する。その `TalkDone` は仮想時刻が horizon を越えて初めて出るため、
    // Tick を止めて yield だけで待つ旧実装は「引き渡しが速く `now` が 0.25s 未満で止まった
    // 実行」で永久に握手へ進めず 60s 安全弁まで空転していた（実測フレーキー）。
    // kanade ではなく **dispatcher** へ送るのが要点（kanade 宛 Tick は `Steady` pump として
    // 消費され台本外の `OnSecondChange` NOTIFY を誘発する）。
    super::spin_pumping_ticks(
        "S4: Unload was never observed after CloseRequest — regular close handshake \
         (OnClose GET → close talk → TalkDone{Quit} → Unload) did not complete",
        &mut now,
        dispatcher_tick(runtime.dispatcher()),
        || {
            handle
                .calls()
                .lock()
                .expect("calls mutex poisoned")
                .iter()
                .any(|c| matches!(c, RecordedCall::Unload))
        },
    );

    // ---- (a) 起動系列＋close 握手系列が正典順序で発火 ----
    // 死活監視ノイズ（RecordedCall::Status）を除外して比較する（S1/S3 と同旨）。
    // 本シナリオは kanade へ Tick を一切送らないため OnSecondChange は発火しない。
    let mut expected_sequence = expected_boot_prefix(&config);
    expected_sequence.extend([
        expected_from_shiori_call(events::on_close(
            CloseReason::User { scope: 0 },
            &areka_kanade::ExecutionSnapshot::INACTIVE,
        )),
        super::expected_translate(
            r"\-",
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
        "起動系列＋close 握手系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→OnTranslate→basewareversion→\
         OnClose→OnTranslate→Unload）が正典順序で発火していない"
    );

    // ---- 主観測: shutdown() が全スレッド join を有界時間内に完走する（要件 7.3) ----
    // close 握手が既に Unload まで完走済み（上の有界待機で確認済み）であるため、
    // ここでの `ForceQuit` 送出は kanade が既に自発停止済み（もしくは自発停止処理の
    // 最終盤）であることの冪等パスを実地で運動させる——`shutdown()` 自身の
    // 「kanade already stopped before ForceQuit send」分岐（design.md「終了
    // （shutdown）シーケンス」・runtime.rs 3.2 の status report で code-reading のみで
    // 検証済みだった経路）を、本 e2e が初めて実地の回帰檻として固定する。
    crate::common::run_bounded_shutdown(
        "shutdown after S4 regular close handshake completion",
        super::BOUND,
        runtime,
    );
}
