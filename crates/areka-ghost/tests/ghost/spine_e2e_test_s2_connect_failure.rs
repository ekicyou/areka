// ===================== S2: 接続失敗シナリオ（task 4.3） =====================
//
// design.md「spine e2e（決定論・純 x64）」の「シナリオ網羅（要件 7.5）」節・S2:
// 「接続失敗: connect が Err→ShioriDown→Unloading{Fault}→全 join（有界）。」を、
// 起動から実 ghost スタック（shiori actor→down_tx→down-relay→kanade_tx の実結線
// 一式）を通して駆動する。Tick 注入は一切不要——`KanadeMsg::ShioriDown` は
// `run_inbox`（areka-kanade/src/actor.rs）が受領のたびに step へ即座に投入する
// 横断メッセージであり、dispatcher の Tick ポンプに一切ゲートされない
// （要件 7.4 の確認材料・kanade 自身の受信ループを直接読んで確認済み）。

use super::*;

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{
    GhostBootOptions, GhostHandles, GhostParts, ShioriWiring, SystemVarWiring, TickerMode, boot,
};
use areka_parsers::charset::DefaultEncoding;

use areka_actor::{ActorError, ActorHandle};

/// このテスト専用の一意な一時ディレクトリ（S1 の流儀を踏襲）。
fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_ghost_spine_e2e_s2_tests_{tag}"));
    dir
}

/// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
/// `shell/master/descript.txt`）を構築する。`s1_boot_success::write_ghost_fixture`
/// と同旨だが、sibling module から private item は参照できないためローカルに
/// 複製する（本シナリオは connect が即 `Err` を返し実際の起動系列を一切発火しない
/// ため、shell 側の `name` の値そのものは load-bearing でない）。
fn write_ghost_fixture(root: &std::path::Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,S2TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
    )
    .expect("write ghost descript.txt");

    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,S2TestShell\n",
    )
    .expect("write shell descript.txt");
}

/// `ActorHandle::join` を有界時間で観測する（`areka-kanade` 統合テストの
/// `join_bounded` と同旨のローカルコピー——`ActorHandle::join` 自体は無期限
/// ブロックし得るため、別スレッドへ逃がし `recv_timeout` で宙吊りを防ぐ）。
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

/// S2: 接続失敗——connect が即 `Err` を返しても `boot()` 自体は成功する
/// （connect 失敗は shiori アクタースレッド**内部**で非同期に起こるため、`boot()`
/// 自身の同期的な返り値には影響しない。`GhostBootError::Mount` のみが `boot` を
/// 失敗させる・design「起動（boot）シーケンス」）。その後、実結線（shiori actor の
/// `on_down`→`down_tx`→down-relay→`kanade_tx`）が `ShioriDown` を kanade へ届け、
/// kanade は本テストから一切 `Close`/`ForceQuit` を送られることなく自律的に
/// Unloading{Fault}→best-effort Unload→Stopped→StopSelf へ倒れて終了する
/// （`into_parts()` で得た `handles.kanade` を直接 join して確認・design「S2 接続
/// 失敗」）。加えて残る全コンポーネント（shiori／dispatcher／両 relay）も有界時間内に
/// 後始末されることを確認する（design「全 join（有界）」の文字どおりの意味）。
#[test]
fn s2_connect_failure_drives_autonomous_kanade_termination_and_full_teardown() {
    let root = unique_temp_dir(
        "s2_connect_failure_drives_autonomous_kanade_termination_and_full_teardown",
    );
    let _ = std::fs::remove_dir_all(&root);
    write_ghost_fixture(&root);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| Err("simulated connect failure".to_string()))),
        sinks: vec![
            Box::new(RecordingSink::new()),
            Box::new(RecordingSink::new()),
        ],
        system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    // boot() 自体は connect の成否と無関係に成功する——connect 失敗は非同期に
    // shiori アクタースレッド内部で起こるため、これは「接続失敗は boot 失敗では
    // ない」ことの重要な、逆に取り違えやすい直接証跡になる。
    let runtime = boot(options).expect(
        "boot must succeed even though the SHIORI connect will fail asynchronously \
         inside the shiori actor thread — a connect failure is NOT a boot failure",
    );

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

    // ---- 主観測: kanade の自律終了（外部からの Close/ForceQuit を一切送らない）----
    // 実結線（shiori actor の on_down→down_tx→down-relay→kanade_tx）が ShioriDown を
    // 届け、kanade 自身の Fault 系列が完走したことの直接証跡——このテストは kanade
    // へ一度もメッセージを送っていない。
    join_bounded(
        "kanade autonomous termination on connect failure",
        BOUND,
        kanade_handle,
    )
    .expect(
        "kanade should autonomously terminate once the real down_tx→down-relay→kanade_tx \
         wiring delivers ShioriDown from a genuine connect failure — no external shutdown \
         trigger should be necessary",
    );

    // shiori actor は接続確立に失敗し受信ループへ一切入らないため、ほぼ即座に終了する
    // （`spawn_shiori_actor` の connect-failure 経路・real.rs 参照）。
    join_bounded(
        "shiori actor near-instant exit (never entered its recv loop)",
        BOUND,
        shiori_handle,
    )
    .expect("shiori actor should already be finished — it never entered run_shiori_loop");

    // ---- 副観測: 残る全コンポーネントも有界時間内に後始末される（design「全 join」）----
    // dispatcher は自身の Sender を保持し自然終了しない（「アクター別の停止経路」表）
    // ため、明示的に Close を送出する。
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

    let _ = std::fs::remove_dir_all(&root);
}
