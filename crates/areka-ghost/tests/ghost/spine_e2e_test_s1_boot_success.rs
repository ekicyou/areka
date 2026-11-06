// ===================== S1: boot 成功シナリオ（task 4.2） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S1:
// 「Boot→OnBoot GET が Value→StartTalk→sakura 再生→RecordingSink の発火列（at 昇順・
// 内容一致）」を、起動から実 ghost スタック（kanade→start-relay→dispatcher→sakura の
// 実アクター一式）を通して駆動し、時刻注入（Tick）のみで確認する（sleep 不使用・要件
// 7.2/7.4/7.6・純 x64）。TalkDone{Ended} の kanade への転送はここでは見ない（末尾の注記）。

use super::*;

use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::KanadeConfig;
use areka_parsers::charset::DefaultEncoding;

/// S1: boot 成功——boot→OnBoot(Value)→StartTalk→sakura 再生→RecordingSink の発火列
/// （at 昇順・内容一致）を、Tick 注入のみで決定論的に確認する（要件 7.2/7.4/7.6）。
#[test]
fn s1_boot_success_plays_greeting_and_records_expected_cue_sequence() {
    const SHELL_NAME: &str = "S1BootShell";

    let temp = write_ghost_fixture("S1TestGhost", SHELL_NAME);
    let root = temp.path().to_path_buf();

    // events 表と同一パラメタで期待値導出用 config を構築する（`resolve_kanade_config` が
    // 実際に組み立てる値と shell_name/baseware_version が一致する・task 4.2 参照材料 4）。
    let config = KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"));

    // boot 系列一式のみを台本化する（OnSecondChange は kanade へ Tick を一切送らないため
    // 不要・OnClose/Unload は本テスト末尾の shutdown() が消費する）。
    let (backend, handle) = boot_script()
        .notify("OnClose", Ok(()))
        .unload(Ok(ExitKind::Clean))
        .build();

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

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

    // boot() は内部で KanadeMsg::Boot を既に送出済み——boot 系列は kanade アクタースレッド
    // 上で同期往復（oneshot round trip）のみで完走するため、この時点で OnInitialize〜
    // basewareversion の起動系列はスケジューリング次第で既に発火し終えて**いることもある**
    // （⚠ 発火し終えている保証は無い。cue の発火は起動系列の完了を含意しない——理由と
    // 対処は下の有界待ちの注記を読むこと。task 5.6・要件 9.2）。しかし
    // StartTalk は start_tx→start-relay→dispatcher_tx の 2 hop（別スレッド）を経るため、
    // dispatcher の active slot に talk が実際に載るタイミングはスレッドスケジューリング
    // 依存であり、単一の Tick 送出が必ず間に合う保証はない。sleep は使わず、Tick を送る
    // たびに RecordingSink を確認する再送ループ（実時間待機なし・単調増加する `now` の
    // 注入のみ・`yield_now` で他スレッドに実行機会を譲るだけ）でこの橋渡しをする。
    let mut now: u64 = 1;
    super::spin_pumping_ticks(
        "S1: surface cue never fired after repeated Tick — boot talk did not reach \
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

    // ---- (a) 起動系列が正典順序で発火（NOTIFY／GET の別・Reference 構成込み） ----
    // real shiori アクター（run_shiori_loop）はメッセージ到達のたびに冒頭で
    // backend.status() を確認する（死活監視・親モジュール rustdoc 参照）ため、
    // calls() には Get/Notify の間に RecordedCall::Status が挟まる。起動系列の
    // 順序判定はこの死活監視ノイズと無関係なので除外して比較する。
    let expected_boot_prefix = expected_boot_prefix(&config);

    // **数える前に起動系列の到達そのものを有界に待つ**（task 5.6・要件 9.2〔2026-09-04 の
    // 開発者裁定で S1 まで対象を広げた改訂版〕）。上の `spin_pumping_ticks` が抜ける条件は
    // 「surface cue が 1 件でも載ったこと」だけで、起動系列の完了を**含意しない**——
    // `to_baseware_version`（`crates/areka-kanade/src/schedule/boot.rs`）は `Action::StartTalk` を
    // `basewareversion` の `Action::ShioriRequest` より**先に**積むので、cue（start-relay→dispatcher の
    // 2 hop）と最後の記録（shiori アクタースレッドの 1 hop）は別スレッドの競走になる。待ちの脱出条件は
    // 「期待の長さ以上」だが、直後の等値照合が上限も保つので 1 件多く来れば赤になる。
    wait_until(
        "S1: boot 系列（OnInitialize/username/OnFirstBoot/OnBoot/OnTranslate/basewareversion）が揃わなかった",
        || non_status_calls(&handle).len() >= expected_boot_prefix.len(),
    );
    let calls_without_status = non_status_calls(&handle);
    assert_eq!(
        calls_without_status, expected_boot_prefix,
        "起動系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→OnTranslate→basewareversion）が正典順序で発火していない"
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
        dispatcher_tick(runtime.dispatcher()),
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

    // ---- 後片付け ----
    // TalkDone{Ended} の dispatcher→kanade の転送は公開面から覗けない。ForceQuit は全 Phase から
    // Unloading{Forced} へ直行するので、この shutdown の成功も転送の証跡にはならない
    // （正規の終了の握手は S4 の担当）。
    crate::common::run_bounded_shutdown(
        "shutdown after S1 boot talk completion",
        super::BOUND,
        runtime,
    );
}
