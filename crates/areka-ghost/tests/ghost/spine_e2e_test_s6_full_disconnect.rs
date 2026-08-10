// ===================== S6: 全断線（段階的解体）シナリオ（task 4.7） =====================
//
// design.md「アクター別の停止経路（正本）」表・「シナリオ網羅（要件 7.5）」節・S6:
// 「全断線（段階的解体）: `into_parts` で分解し、①`DispatcherMsg::Close` 送出→dispatcher
// join（Close-only アクターの正規停止）②`KanadeMsg::Close` 送出→kanade join（運行意味論を
// 経ない素の停止）③残る senders を全 drop→shiori actor（kanade の `shiori_tx` drop による
// inbox 切断）・down-relay（shiori 停止による `down_tx` drop）・start-relay（kanade 停止に
// よる `start_tx` drop）が切断伝播だけで有界時間内に正常終了することを join で確認する。
// 純粋な「全 Sender drop 一斉解放」は Sender 環（停止経路マトリクス参照）ゆえ構造的に
// 成立しない——本シナリオはマトリクスの全行（Close 経路×2・切断経路×3）を 1 シナリオで
// 検証する再定義である。」
//
// `GhostRuntime`/`GhostParts` は `shiori_tx` を保持しない（design「GhostRuntime は
// shiori_tx を保持しない」・runtime.rs 3.1/3.2・`into_parts` の rustdoc）。`shiori_tx` は
// kanade 自身のアクタースレッドが `spawn_kanade(config, shiori_tx, start_tx)` の引数として
// **内部に**保持し続ける（`run_inbox` のクロージャが `shiori`/`sakura` を move キャプチャ
// する・`areka-kanade/src/actor.rs::spawn_kanade`）。ゆえに kanade スレッドが（`Close` によ
// る即時 `Break` で）終了しクロージャが return するとき、`shiori_tx`・`start_tx`（＝
// kanade にとっての `sakura` パラメータ）はその関数フレームの終了と共に**自動的に**
// drop される——手動 `drop()` は一切不要（そもそもこのテストは `shiori_tx`/`start_tx` を
// 握っていないため不可能でもある）。同様に `down_tx` は shiori actor 自身が受信ループの
// 全生涯にわたり保持する（task 1.4 の設計・`on_down` 保持）ため、shiori スレッドが終了
// すればそれも自動的に drop される。`ActorHandle::join` はスレッド関数の完全な終了
// （＝これらの drop が既に起こった後）を待ってから返るため、各 join の成功はその
// drop が既に起きたことの直接証跡になる。

use super::*;

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{
    GhostBootOptions, GhostHandles, GhostParts, ShioriWiring, SystemVarWiring, TickerMode, boot,
};
use areka_kanade::KanadeMsg;
use areka_parsers::charset::DefaultEncoding;

use areka_actor::{ActorError, ActorHandle};

/// このテスト専用の一意な一時ディレクトリ（S1〜S5 の流儀を踏襲）。
fn unique_temp_dir(tag: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_ghost_spine_e2e_s6_tests_{tag}"));
    dir
}

/// `root` 直下に最小限の解決可能なゴーストツリーを構築する（S1/S3/S4/S5 の
/// `write_ghost_fixture` と同旨だが、sibling module から private item は参照できない
/// ためローカルに複製する）。
fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,S6TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
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

/// `ActorHandle::join` を有界時間で観測する（S2〜S3 の `join_bounded` と同旨の
/// ローカルコピー）。
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

/// S6: 全断線（段階的解体）——`into_parts` で分解し、①dispatcher へ `Close`→join
/// （Close-only アクターの正規停止・「アクター別の停止経路」表の dispatcher 行）②
/// kanade へ raw `Close`→join（運行意味論——OnClose NOTIFY／Unload 等——を一切経ない
/// 「非常口」停止。S2〜S5 のいずれも駆動していない、design「停止規約の Close」の
/// bare な構造的停止・同表の kanade 行）③以降は手動 `drop()` も追加送信も一切行わずに
/// shiori／down-relay／start-relay を join し、②で kanade スレッドが終了したことに
/// 伴う自動 drop カスケードだけで全て有界時間内に自然終了することを確認する（同表の
/// shiori／down-relay／start-relay の 3 行）。合計 5 join で「アクター別の停止経路」
/// マトリクスの全 5 行（Close 経路×2・切断経路×3）を 1 シナリオで検証する（design が
/// 述べる「マトリクスの全行…を1シナリオで検証する再定義」・要件 7.4/7.5/7.6）。
#[test]
fn s6_full_disconnect_staged_teardown_terminates_all_five_actors_within_bound() {
    const SHELL_NAME: &str = "S6DisconnectShell";

    let root = unique_temp_dir(
        "s6_full_disconnect_staged_teardown_terminates_all_five_actors_within_bound",
    );
    let _ = std::fs::remove_dir_all(&root);
    write_ghost_fixture(&root, SHELL_NAME);

    // boot() 内部の同期 kanade 往復（OnInitialize NOTIFY→OnFirstBoot GET→OnBoot GET→
    // basewareversion NOTIFY）が panic しない最小限の台本のみ用意する——本シナリオの
    // 焦点は解体の「構造」であり、boot talk（`\s[0]hello\e`）の再生完了までは駆動
    // しない（dispatcher の active slot に乗ったまま未進行でも、後続①の
    // `DispatcherMsg::Close` が既存の Close funnel で安全に中断させる・dispatcher.rs
    // 自身の単体テストで既に確認済みの挙動・CONCERNS 参照）。OnClose／Unload は台本化
    // しない——本シナリオは kanade へ `CloseRequest`／`ForceQuit` のいずれも送らず、
    // 運行意味論を経ない raw `KanadeMsg::Close` のみで停止させるため、shiori backend
    // の `unload()` が呼ばれることはない。
    let (backend, _handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        // task 8.2 の username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が発行する
        // resource GET。既定 username 前提（sylphya 未供給＝no_content）を faithful に再現するため
        // Ok(None) を台本化する（default_system_vars 相当の「%username のみ既定」世界）。
        .get("username", Ok(None))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
        .notify("basewareversion", Ok(()))
        .build();

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![
            Box::new(RecordingSink::new()),
            Box::new(RecordingSink::new()),
        ],
        system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    let parts = runtime.into_parts();
    let GhostParts {
        kanade,
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

    // ---- ①: DispatcherMsg::Close → dispatcher join ----
    // dispatcher の唯一の停止経路（「アクター別の停止経路」表・Close-only。self-sender
    // 保持ゆえ切断では構造的に止まらない）。boot talk が active slot に乗ったまま
    // 未進行でも、Close funnel が中断させて正規停止することを直接運動させる。
    dispatcher
        .send(DispatcherMsg::Close)
        .expect("dispatcher actor should still be alive to receive Close");
    join_bounded("① dispatcher join after Close", BOUND, dispatcher_handle)
        .expect("dispatcher should terminate after its only stop path (Close)");

    // ---- ②: KanadeMsg::Close → kanade join ----
    // kanade の raw「非常口」停止（`KanadeMsg::Close` は step を経ず即時 Break・
    // areka-kanade/src/actor.rs::spawn_kanade）。OnClose NOTIFY も Unload も一切
    // 呼ばれない——S2〜S5 が駆動する運行意味論（ForceQuit／CloseRequest／Fault）とは
    // 異なる、この e2e で初めて運動させる経路。kanade スレッドが Break で return する
    // 時点で、その関数フレームが内部に保持していた `shiori_tx`（shiori actor 自身の
    // inbox 送信端）・`start_tx`（start-relay の上流送信端）が自動的に drop される。
    kanade
        .send(KanadeMsg::Close)
        .expect("kanade actor should still be alive to receive raw Close");
    join_bounded("② kanade join after raw Close", BOUND, kanade_handle).expect(
        "kanade should terminate on its bare Close stop path without running any \
         shutdown semantics (no OnClose NOTIFY, no Unload)",
    );

    // ---- ③: 以降は手動 drop も追加送信も一切行わず、自動 drop カスケードのみで
    // shiori／down-relay／start-relay を有界時間内に join する ----
    // shiori actor: ②で kanade スレッドが終了した時点で、kanade が内部に保持していた
    // shiori_tx が既に drop 済み——shiori actor の inbox 受信（blocking recv）はその
    // Sender 側が尽きた時点で Err を返し、受信ループが正常終了する。
    join_bounded(
        "③ shiori actor natural termination via shiori_tx drop cascading from ②",
        BOUND,
        shiori_handle,
    )
    .expect(
        "shiori actor should terminate naturally once kanade's internally-held shiori_tx \
         is dropped as a consequence of kanade's actor thread exiting in step ②",
    );

    // down-relay: shiori actor が終了した時点で、shiori が内部に保持していた down_tx が
    // 同様に drop される——down-relay の上流（down_rx）が切断され自然終了する（shiori
    // actor の終了は直前の join で既に観測済みなので、この時点で down_tx は既に
    // drop されている）。
    join_bounded(
        "③ down-relay natural termination via down_tx drop cascading from shiori's exit",
        BOUND,
        down_relay_handle,
    )
    .expect(
        "down-relay should terminate naturally once shiori's internally-held down_tx is \
         dropped as a consequence of shiori's actor thread exiting",
    );

    // start-relay: ②で kanade スレッドが終了した時点で start_tx も既に drop 済み
    // （shiori_tx と同じ根本原因・②の時点で既に成立しているため、shiori／down-relay
    // の後に join しても機能的な前後関係はない——宣言順序の都合でここに置く）。
    join_bounded(
        "③ start-relay natural termination via start_tx drop cascading from ②",
        BOUND,
        start_relay_handle,
    )
    .expect(
        "start-relay should terminate naturally once kanade's internally-held start_tx is \
         dropped as a consequence of kanade's actor thread exiting in step ②",
    );

    let _ = std::fs::remove_dir_all(&root);
}
