// ===================== S3: helper 死活検出シナリオ（task 4.4） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S3:
// 「helper 死活: scripted `status` を `Exited(Abnormal)` へ遷移させ、`runtime.kanade()` へ
// `Tick{now}` を注入→Steady pump の OnSecondChange が shiori actor へ到達→到達時 status
// 確認で検出→ShioriDown→Fault 系列→全 join（有界・駆動は本番と同一経路・実時間ゼロ）。」
//
// S1（boot→Steady 到達確認の retry ループ技法）と S2（`into_parts()` ベースの直接 join に
// よる自律終了の証明技法）を組み合わせ、さらに「シナリオ途中で status を差し替える」
// （task 4.1 `status_transitions_from_running_to_exited_when_mutated_externally_mid_scenario`
// で証明済みの capability）を実際の e2e 経路へ初めて適用する。

use super::*;

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{
    GhostBootOptions, GhostHandles, GhostParts, ShioriWiring, SystemVarWiring, TickerMode, boot,
};
use areka_kanade::{KanadeConfig, KanadeMsg, MonotonicMs};
use areka_parsers::charset::DefaultEncoding;

/// S3: helper 死活検出——scripted `status()` をシナリオ途中で `Exited(Abnormal)` へ差し
/// 替え、`runtime.kanade()` へ Tick を 1 回注入するだけで（Steady pump が発行する
/// OnSecondChange が shiori actor へ到達し、到達時チェックが検出する）、この e2e からは
/// 一度も明示 Close/ForceQuit を送らずに kanade が自律的に Fault 系列（Unloading{Fault}
/// →best-effort Unload→Stopped→StopSelf）へ倒れて終了することを確認する（design「S3
/// helper 死活」・要件 7.4/7.5/7.6）。
#[test]
fn s3_helper_liveness_detected_mid_scenario_drives_autonomous_fault_termination() {
    const SHELL_NAME: &str = "S3LivenessShell";

    let temp = write_ghost_fixture("S3TestGhost", SHELL_NAME);
    let root = temp.path().to_path_buf();

    // boot 系列一式（S1 と同旨）＋ OnSecondChange（Steady pump の 1 発）＋ unload
    // （Fault 系列が発行する ShioriUnload の応答・best-effort ゆえ Abnormal でも Stopped へ
    // 収束する）を台本化する。OnClose は台本化しない——S3 は Fault 経路のため kanade 自身が
    // OnClose NOTIFY を発行することはない（正規 close 握手は S4/S5 の担当領域）。
    //
    // DD-IT-12: boot は挨拶 talk を追跡し `Steady{talk: Some(greeting)}` へ完了する。ゆえに
    // 下で注入する単一の `KanadeMsg::Tick` が pump する OnSecondChange は、挨拶 talk の
    // TalkDone が kanade に届いて `Steady{talk: None}` へ戻った後なら GET（Ref3=1）、まだ
    // 挨拶再生中なら NOTIFY（Ref3=0・`Status: talking`）になる（この 2 経路の別は挨拶
    // TalkDone の到達と注入 Tick の到達順というスレッド間タイミング次第・S3 の観測点は
    // 死活検出であり GET/NOTIFY の別に依存しない）。どちらの方式でも OnSecondChange は
    // shiori actor へ到達し、到達時 status() 確認が Exited を検出する（Req2.2/DD-IT-12）——
    // ゆえに GET/NOTIFY 双方を台本化し、レースが選んだ側だけが消費される（他方は未消費で
    // 無害）。
    let (backend, handle) = boot_script()
        .get("OnSecondChange", Ok(None))
        .notify("OnSecondChange", Ok(()))
        .unload(Ok(ExitKind::Abnormal(1)))
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

    // ---- boot talk を Steady 到達まで駆動する（S1 と同一技法・sleep 不使用） ----
    // dispatcher へ Tick を送るたびに RecordingSink を確認する有界再送ループ（実時間
    // 待機なし・単調増加する now の注入のみ・`yield_now` で他スレッドに実行機会を譲る
    // だけ）。kanade 自身は（dispatcher Tick とは無関係な別チャンネル経由で）
    // basewareversion NOTIFY の応答往復のみで Steady へ完了する（DD-IT-12: 挨拶 talk を
    // 追跡したまま `Steady{talk: Some(greeting)}` へ・BootVersion+Notified の遷移は
    // StartTalk 発行と独立に basewareversion の応答のみで確定する）。
    // ⚠ ただし cue の発火が boot 系列の完了を**含意しない**——`schedule/boot.rs` の
    // `to_baseware_version` は `Action::StartTalk` を basewareversion より先に積むので、cue（start-relay
    // →dispatcher の 2 hop ＋ Tick）と 起動の列の最後の記録（basewareversion・6 本目）（shiori アクタースレッド 1 hop）は
    // 別スレッドの競走であり、順序は保証されていない。実測では cue の側が遅いことが
    // ほとんどだが構造的な保証ではないため、件数は下の有界待ちで待ってから数える
    // （areka-P0-emo2-conformance-e2e R9.2・詳細は下の有界待ちの注記）。
    let mut now: u64 = 1;
    super::spin_pumping_ticks(
        "S3: surface cue never fired after repeated Tick — boot talk did not reach dispatcher's active slot",
        &mut now,
        dispatcher_tick(runtime.dispatcher()),
        || {
            !surface_records
                .lock()
                .expect("records mutex poisoned")
                .is_empty()
        },
    );

    // 起動系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→OnTranslate→basewareversion）が
    // 届き終えたこと（＝kanade が Steady へ既に到達済みであること）を裏付ける間接証跡（S1 と同旨・
    // 死活監視の Status は除いて数える）。件数は親の `expected_boot_prefix` の長さから取る。
    //
    // **数える前に起動系列の到達そのものを有界に待つ**（task 4.1・要件 9.2）。直前の待ちが抜ける
    // 条件は「surface cue が 1 件でも載ったこと」だけで、起動系列の完了を含意しない——
    // `to_baseware_version`（`areka-kanade/src/schedule/boot.rs`）は `Action::StartTalk` を
    // `basewareversion` の ShioriRequest より**先に**積むので、cue と最後の記録は別スレッドの
    // 競走になる。判定（Status を除いてちょうど期待の長さ）は変えず、数える前に待つだけである。
    let boot_prefix_len =
        expected_boot_prefix(&KanadeConfig::new(SHELL_NAME, env!("CARGO_PKG_VERSION"))).len();
    wait_until(
        "S3: 起動系列が揃わなかった——kanade はまだ Steady に到達していないはず",
        || non_status_calls(&handle).len() >= boot_prefix_len,
    );
    assert_eq!(
        non_status_calls(&handle).len(),
        boot_prefix_len,
        "S3: 起動系列（OnInitialize/username/OnFirstBoot/OnBoot/OnTranslate/basewareversion）の ほかに呼び出しが来ている"
    );

    // ---- helper がシナリオ途中で異常終了する様子を、backend の外側（テスト自身の
    // スレッド）から駆動する（task 4.1 の capability・design「S3 helper 死活」）。----
    handle.set_status(HelperStatus::Exited(ExitKind::Abnormal(1)));

    // ---- kanade へ Tick を 1 回だけ注入する（Steady pump の唯一の駆動源）。----
    // Steady{talk: None} + Tick → OnSecondChange GET が shiori actor へ届く
    // （steady.rs on_tick）。run_shiori_loop はメッセージ到達の冒頭で必ず
    // backend.status() を確認するため（親モジュール rustdoc 参照）、この 1 通の
    // Tick 到達だけで死活検出（Exited 初回観測→ShioriDown 送出）と OnSecondChange
    // 応答処理の両方が起こる。ShioriDown は down-relay 経由で kanade 自身の inbox
    // へ届き、次にそのメッセージを処理する際に横断アーム（Unloading{Fault}）へ
    // 倒れる——この e2e からは以後一切のメッセージを送らない。
    runtime
        .kanade()
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000_000),
        })
        .expect("kanade actor should still be alive to receive the liveness-detecting Tick");

    // ---- 主観測: kanade の自律終了（外部からの Close/ForceQuit を一切送らない）----
    // S2 と同じ into_parts() ベースの直接 join 技法——このテストは kanade へ Tick を
    // 1 回送った後、一度も Close/ForceQuit を送っていない。
    let parts = runtime.into_parts();
    let GhostParts {
        dispatcher,
        handles,
        ..
    } = parts;
    let GhostHandles {
        kanade: kanade_handle,
        dispatcher: dispatcher_handle,
        shiori: shiori_handle,
        start_relay: start_relay_handle,
        down_relay: down_relay_handle,
        ticker: _,
        sylphya: _,
    } = handles;

    join_bounded(
        "kanade autonomous fault termination after mid-scenario status transition",
        kanade_handle,
    )
    .expect(
        "kanade should autonomously terminate once the OnSecondChange-triggered \
         status() check detects Exited(Abnormal) and drives ShioriDown through the \
         real down_tx→down-relay→kanade_tx wiring — no external Close/ForceQuit should \
         be necessary",
    );

    // shiori actor: kanade の Fault 系列は Unloading{Fault} 到達時に ShioriUnload
    // action を発行し、その応答受領後に必ず shiori へ ShioriMsg::Close を送出して
    // から StopSelf する（「アクター別の停止経路」表・kanade 正本）ため、shiori
    // actor も有界時間内に終了するはず。
    join_bounded(
        "shiori actor termination after kanade's fault sequence closes it",
        shiori_handle,
    )
    .expect("shiori actor should terminate once kanade's fault sequence sends ShioriMsg::Close");

    // ---- 副観測: 残る全コンポーネントも有界時間内に後始末される（design「全 join」）----
    // dispatcher は自身の Sender を保持し自然終了しない（「アクター別の停止経路」表）
    // ため、明示的に Close を送出する（S2 と同旨）。
    let _ = dispatcher.send(DispatcherMsg::Close);
    join_bounded("dispatcher join after Close", dispatcher_handle)
        .expect("dispatcher should terminate after Close");

    // start-relay／down-relay は上流（kanade 自身の start_tx／shiori 自身の down_tx）が
    // 既に drop 済み（kanade・shiori 双方のアクタースレッドが既に終了している）ため、
    // メッセージを送らずとも自然終了する。
    join_bounded("start-relay natural termination", start_relay_handle)
        .expect("start-relay should terminate naturally once kanade's start_tx is dropped");
    join_bounded("down-relay natural termination", down_relay_handle)
        .expect("down-relay should terminate naturally once shiori's down_tx is dropped");

    // ---- sticky-once の間接証跡 ----
    // ShioriDown の発火自体は kanade inbox 側のイベントであり calls() には現れないが、
    // Fault 系列が best-effort Unload を実際に発行したこと（＝ShioriDown が届いて
    // Unloading{Fault} へ倒れたことの直接証跡）と、このシナリオ全体が有界時間内に
    // 完走したこと（status flapping で shiori actor がループし続けるような壊れ方を
    // していないこと）の 2 点を確認する。sticky-once の不変量そのものは task 1.4 の
    // 単体テスト（death_detected_once_reports_shiori_down_and_only_once）が既に固定
    // しており、本 e2e の責務は配線がそれを最後まで届けることの証明に置く。
    let all_calls = handle.calls().lock().expect("calls mutex poisoned").clone();
    assert!(
        all_calls.iter().any(|c| matches!(c, RecordedCall::Unload)),
        "S3: Fault 系列は best-effort Unload を発行するはず: {all_calls:?}"
    );
}
