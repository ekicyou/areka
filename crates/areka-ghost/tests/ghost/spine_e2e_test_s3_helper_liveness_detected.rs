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
use areka_kanade::{KanadeMsg, MonotonicMs};
use areka_parsers::charset::DefaultEncoding;

use areka_actor::{ActorError, ActorHandle};
use temp_path_kit::TempPath;

/// このテスト専用の一時ディレクトリ。共通窓口 `temp-path-kit` 経由で組むので、
/// 名前にプロセス識別子と連番が入り**プロセス間でも一意**（同じテストを同時に
/// 複数プロセスで走らせても互いの一時ファイルを消し合わない）。
///
/// 返り値が生きている間だけ実体が存在し、破棄で中身ごと消える。
fn unique_temp_dir(tag: &str) -> TempPath {
    TempPath::new(&format!("ghost-spine-s3-{tag}"))
}

/// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
/// `shell/master/descript.txt`）を構築する。S1 の `write_ghost_fixture` と同旨だが、
/// sibling module から private item は参照できないためローカルに複製する。
fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,S3TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
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

/// `ActorHandle::join` を有界時間で観測する（S2 の `join_bounded` と同旨のローカルコピー）。
fn join_bounded(
    what: &str,
    timeout: std::time::Duration,
    handle: ActorHandle,
) -> Result<(), ActorError> {
    let (res_tx, res_rx) = std::sync::mpsc::sync_channel::<Result<(), ActorError>>(0);
    std::thread::spawn(move || {
        let _ = res_tx.send(handle.join());
    });
    match res_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
    }
}

const BOUND: std::time::Duration = super::E2E_BOUND;

/// S3: helper 死活検出——scripted `status()` をシナリオ途中で `Exited(Abnormal)` へ差し
/// 替え、`runtime.kanade()` へ Tick を 1 回注入するだけで（Steady pump が発行する
/// OnSecondChange が shiori actor へ到達し、到達時チェックが検出する）、この e2e からは
/// 一度も明示 Close/ForceQuit を送らずに kanade が自律的に Fault 系列（Unloading{Fault}
/// →best-effort Unload→Stopped→StopSelf）へ倒れて終了することを確認する（design「S3
/// helper 死活」・要件 7.4/7.5/7.6）。
#[test]
fn s3_helper_liveness_detected_mid_scenario_drives_autonomous_fault_termination() {
    const SHELL_NAME: &str = "S3LivenessShell";

    let temp = unique_temp_dir("mid-scenario-drives-autonomous-fault-termination");
    let root = temp.path().to_path_buf();
    write_ghost_fixture(&root, SHELL_NAME);

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
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
        // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
        // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
        .get("username", Ok(None))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
        .notify("basewareversion", Ok(()))
        .get("OnSecondChange", Ok(None))
        .notify("OnSecondChange", Ok(()))
        .unload(Ok(ExitKind::Abnormal(1)))
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

    // ---- boot talk を Steady{None} 到達まで駆動する（S1 と同一技法・sleep 不使用） ----
    // dispatcher へ Tick を送るたびに RecordingSink を確認する有界再送ループ（実時間
    // 待機なし・単調増加する now の注入のみ・`yield_now` で他スレッドに実行機会を譲る
    // だけ）。boot talk が dispatcher の active slot に載って発火し終えた時点で、
    // kanade 自身は（dispatcher Tick とは無関係な別チャンネル経由で）basewareversion
    // NOTIFY の応答往復のみで既に Steady{talk: None} へ完了している（boot.rs:
    // 「boot は常に Steady{talk: None} へ完了する」・BootVersion+Notified の遷移は
    // StartTalk 発行と独立に basewareversion の応答のみで確定するため、StartTalk が
    // start-relay→dispatcher の 2 hop を経て active slot に載り、さらに Tick で実際に
    // 発火するよりずっと早く完了している）。
    let mut now: u64 = 1;
    let mut fired = false;
    let deadline = std::time::Instant::now() + super::E2E_BOUND;
    while std::time::Instant::now() < deadline {
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher actor should still be alive while probing for the boot talk");
        now += 1;
        if !surface_records
            .lock()
            .expect("records mutex poisoned")
            .is_empty()
        {
            fired = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        fired,
        "S3: surface cue never fired after repeated Tick — boot talk did not reach \
         dispatcher's active slot within bound"
    );

    // boot 系列（OnInitialize→username prefetch→OnFirstBoot→OnBoot→basewareversion）の
    // 5 呼出が完了済みであること（＝kanade が Steady{None} へ既に到達済みであること）を
    // 裏付ける間接証跡（S1 と同旨・死活監視の Status ノイズは除外して数える・task 8.2 の
    // username prefetch GET が OnInitialize と OnFirstBoot の間に 1 件加わり 4→5 になる）。
    let calls_handle = handle.calls();
    let boot_prefix_len = calls_handle
        .lock()
        .expect("calls mutex poisoned")
        .iter()
        .filter(|c| !matches!(c, RecordedCall::Status))
        .count();
    assert_eq!(
        boot_prefix_len, 5,
        "S3: boot 系列 5 呼出（OnInitialize/username/OnFirstBoot/OnBoot/basewareversion）が \
         完了していない——kanade はまだ Steady に到達していないはず"
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
        BOUND,
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
        BOUND,
        shiori_handle,
    )
    .expect("shiori actor should terminate once kanade's fault sequence sends ShioriMsg::Close");

    // ---- 副観測: 残る全コンポーネントも有界時間内に後始末される（design「全 join」）----
    // dispatcher は自身の Sender を保持し自然終了しない（「アクター別の停止経路」表）
    // ため、明示的に Close を送出する（S2 と同旨）。
    let _ = dispatcher.send(DispatcherMsg::Close);
    join_bounded("dispatcher join after Close", BOUND, dispatcher_handle)
        .expect("dispatcher should terminate after Close");

    // start-relay／down-relay は上流（kanade 自身の start_tx／shiori 自身の down_tx）が
    // 既に drop 済み（kanade・shiori 双方のアクタースレッドが既に終了している）ため、
    // メッセージを送らずとも自然終了する。
    join_bounded("start-relay natural termination", BOUND, start_relay_handle)
        .expect("start-relay should terminate naturally once kanade's start_tx is dropped");
    join_bounded("down-relay natural termination", BOUND, down_relay_handle)
        .expect("down-relay should terminate naturally once shiori's down_tx is dropped");

    // ---- sticky-once の間接証跡 ----
    // ShioriDown の発火自体は kanade inbox 側のイベントであり calls() には現れないが、
    // Fault 系列が best-effort Unload を実際に発行したこと（＝ShioriDown が届いて
    // Unloading{Fault} へ倒れたことの直接証跡）と、このシナリオ全体が有界時間内に
    // 完走したこと（status flapping で shiori actor がループし続けるような壊れ方を
    // していないこと）の 2 点を確認する。sticky-once の不変量そのものは task 1.4 の
    // 単体テスト（death_detected_once_reports_shiori_down_and_only_once）が既に固定
    // しており、本 e2e の責務は配線がそれを最後まで届けることの証明に置く
    // （CONCERNS 参照）。
    let all_calls = calls_handle.lock().expect("calls mutex poisoned").clone();
    assert!(
        all_calls.iter().any(|c| matches!(c, RecordedCall::Unload)),
        "S3: Fault 系列は best-effort Unload を発行するはず: {all_calls:?}"
    );
}
