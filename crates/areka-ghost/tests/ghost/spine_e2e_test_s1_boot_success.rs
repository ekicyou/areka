// ===================== S1: boot 成功シナリオ（task 4.2） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S1:
// 「Boot→OnBoot GET が Value→StartTalk→sakura 再生→RecordingSink の発火列（at 昇順・
// 内容一致）→TalkDone{Ended} が kanade へ転送される」を、起動から実 ghost スタック
// （kanade→start-relay→dispatcher→sakura の実アクター一式）を通して駆動し、時刻注入
// （Tick）のみで確認する（sleep 不使用・要件 7.2/7.4/7.6・純 x64）。

use super::*;

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::{KanadeConfig, MonotonicMs, ShioriCall, events};
use areka_parsers::charset::DefaultEncoding;

/// このテスト専用の一意な一時ディレクトリ（`runtime.rs`/`config.rs` テストの流儀を踏襲）。
fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_ghost_spine_e2e_s1_tests_{tag}"));
    dir
}

/// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
/// `shell/master/descript.txt`）を構築する。shell descript の `name` は `shell_name`
/// （`OnBoot` Ref0・`KanadeConfig::shell_name` の値源と一致させるための既知値・task
/// 4.2 参照材料 4/5）。
fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,S1TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
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

/// events 表由来の [`ShioriCall`] を、このファイル固有の [`RecordedCall`]（task 4.1 の
/// [`ScriptedShioriBackend`] 記録型）へ変換する（fixture・assert・実装が単一の正本＝
/// events 表を共有する・Req 7.1）。kanade 自身の統合テストが使う `expected_call`/
/// `CallMethod` は kanade クレート専用の private 型であり本ファイルからは参照できない
/// ため、ここで同旨の変換を用意する（task 4.2 参照材料 6 の指示どおり）。
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

/// 有界待機ヘルパ（`runtime.rs`/`dispatcher.rs` テストモジュールと同旨のローカルコピー）。
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

/// S1: boot 成功——boot→OnBoot(Value)→StartTalk→sakura 再生→RecordingSink の発火列
/// （at 昇順・内容一致）→TalkDone を、Tick 注入のみで決定論的に確認する
/// （要件 7.2/7.4/7.6）。
#[test]
fn s1_boot_success_plays_greeting_and_records_expected_cue_sequence() {
    const SHELL_NAME: &str = "S1BootShell";

    let root = unique_temp_dir("s1_boot_success_plays_greeting_and_records_expected_cue_sequence");
    let _ = std::fs::remove_dir_all(&root);
    write_ghost_fixture(&root, SHELL_NAME);

    // events 表と同一パラメタで期待値導出用 config を構築する（`resolve_kanade_config` が
    // 実際に組み立てる値と shell_name/baseware_version が一致する・task 4.2 参照材料 4）。
    let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

    // boot 系列一式のみを台本化する（OnSecondChange は kanade へ Tick を一切送らないため
    // 不要・OnClose/Unload は本テスト末尾の shutdown() が消費する）。
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
        // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
        // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
        .get("username", Ok(None))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
        .notify("basewareversion", Ok(()))
        .notify("OnClose", Ok(()))
        .unload(Ok(ExitKind::Clean))
        .build();

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

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

    // boot() は内部で KanadeMsg::Boot を既に送出済み——boot 系列は kanade アクタースレッド
    // 上で同期往復（oneshot round trip）のみで完走するため、この時点で OnInitialize〜
    // basewareversion の 4 呼出はスケジューリング次第で既に発火し終えている。しかし
    // StartTalk は start_tx→start-relay→dispatcher_tx の 2 hop（別スレッド）を経るため、
    // dispatcher の active slot に talk が実際に載るタイミングはスレッドスケジューリング
    // 依存であり、単一の Tick 送出が必ず間に合う保証はない。sleep は使わず、Tick を送る
    // たびに RecordingSink を確認する再送ループ（実時間待機なし・単調増加する `now` の
    // 注入のみ・`yield_now` で他スレッドに実行機会を譲るだけ）でこの橋渡しをする。
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
        "S1: surface cue never fired after repeated Tick — boot talk did not reach \
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

    // ---- (a) 起動系列が正典順序で発火（NOTIFY／GET の別・Reference 構成込み） ----
    // real shiori アクター（run_shiori_loop）はメッセージ到達のたびに冒頭で
    // backend.status() を確認する（死活監視・親モジュール rustdoc 参照）ため、
    // calls() には Get/Notify の間に RecordedCall::Status が挟まる。起動系列の
    // 順序判定はこの死活監視ノイズと無関係なので除外して比較する。
    let expected_boot_prefix = vec![
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
    ];
    let calls = handle.calls();
    let calls_without_status: Vec<RecordedCall> = calls
        .lock()
        .expect("calls mutex poisoned")
        .iter()
        .filter(|c| !matches!(c, RecordedCall::Status))
        .cloned()
        .collect();
    assert_eq!(
        calls_without_status, expected_boot_prefix,
        "起動系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→basewareversion）が正典順序で発火していない"
    );

    // ---- (b)(c) RecordingSink の発火列（broadcast・at 昇順・内容一致）----
    // broadcast ゆえ surface/text の両 sink が**同一の全 cue** を受ける（中央振り分け廃止・
    // どの action を演じるかは演者側 relevance の責務）。`\s[0]hello\e` の期待 broadcast 列:
    //   ClearAll@0（#6 全消去・task 5.2 冒頭前置）/ Emote{0}@0（\s[0]）/ Text(hello)@0 /
    //   **初回起動 epilogue**（`areka.prop.set` [BootCount 正準 key, "1"]）@`hello` の再生完了時刻。
    //
    // 4 件目は「初回起動なら起動記録を書く」epilogue（`runtime.rs` step 3・要件 3.4）であり、
    // 本 fixture は永続ファイルを持たない＝毎回 `first_boot=true` ゆえ**常に台本に載る**。
    // 期待値（cue 名・key・at）は本番権威から導出し、定数の直書きを避ける——`at` は
    // `text_playback_duration("hello")`（= 5 文字 × CHAR_NOMINAL_MS）そのものである。
    //
    // 整定は `spin_pumping_ticks` で行う——4 件目は仮想時刻が `hello` の horizon を越えて
    // 初めて発火するため、**Tick を注入し続けなければ永久に届かない**（旧実装は yield のみで
    // 待ち、horizon を越えるか否かがスケジューラ依存だった＝17.5% 偽赤の直接原因）。
    let expected: Vec<(f64, CueCommand)> = vec![
        (0.0, CueCommand::ClearAll),
        (
            0.0,
            CueCommand::Emote {
                key: "0".to_string(),
            },
        ),
        (0.0, CueCommand::Text("hello".to_string())),
        (
            areka_sakura::duration::text_playback_duration("hello"),
            CueCommand::command_carrier(
                areka_ghost::prop_sink::PROP_SET_CUE_NAME,
                vec![
                    areka_sylphya::persist::PersistKey::BootCount.to_canonical_key(),
                    "1".to_string(),
                ],
            ),
        ),
    ];
    super::spin_pumping_ticks(
        "S1: broadcast cue 列が期待長へ整定しなかった（初回起動 epilogue まで含む全 4 件）",
        &mut now,
        tick,
        || {
            surface_records
                .lock()
                .expect("records mutex poisoned")
                .len()
                >= expected.len()
                && text_records.lock().expect("records mutex poisoned").len() >= expected.len()
        },
    );
    let surface = surface_records
        .lock()
        .expect("records mutex poisoned")
        .clone();
    let text = text_records.lock().expect("records mutex poisoned").clone();
    let assert_broadcast = |cues: &[TalkCue], who: &str| {
        let observed: Vec<(f64, CueCommand)> =
            cues.iter().map(|c| (c.at, c.command.clone())).collect();
        assert_eq!(
            observed, expected,
            "{who} sink は broadcast で ClearAll/Emote/hello/初回起動 epilogue を \
             (at, command) ごと受ける（partition は演者側 relevance）: {cues:?}"
        );
        for cue in cues {
            assert_eq!(
                cue.actor,
                ActorKey::from("0"),
                "{who} 発火 actor は 既定 scope 0"
            );
        }
        for pair in cues.windows(2) {
            assert!(pair[0].at <= pair[1].at, "{who} 発火列は at 昇順であるべき");
        }
    };
    assert_broadcast(&surface, "surface");
    assert_broadcast(&text, "text");

    // ---- 後片付け兼 (c) の間接証跡 ----
    // TalkDone{Ended} が dispatcher→kanade へ転送済みであること（dispatcher の slot が
    // 解放され kanade が Steady{None} へ戻っていること）は、kanade inbox を直接覗く
    // 経路が公開面に無いため、後続の shutdown（ForceQuit→OnClose NOTIFY→Unload の順）
    // が台本どおり完走し Ok(()) を返すことをもって間接的に確認する——もし TalkDone が
    // 届かず kanade が Steady{Some} に取り残されていても ForceQuit は横断遷移で全 Phase
    // から Unloading{Forced} へ直行するため shutdown 自体は成立してしまうが、これは
    // 「正規終了握手」シナリオ（task 4.5）の担当範囲であり、本タスクの主眼は (a)(b) の
    // 発火列検証に置く（CONCERNS 参照）。
    run_bounded(
        "shutdown after S1 boot talk completion",
        super::E2E_BOUND,
        move || {
            let result = runtime.shutdown(areka_kanade::CloseReason::System);
            assert!(
                result.is_ok(),
                "shutdown should return Ok(()) after S1 boot talk completes, got {result:?}"
            );
        },
    );

    let _ = std::fs::remove_dir_all(&root);
}
