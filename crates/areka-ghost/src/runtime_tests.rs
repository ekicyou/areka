use super::*;
use std::path::PathBuf;
use temp_path_kit::TempPath;

use crate::test_support::{
    RecordingSink, fake_options, run_bounded, shutdown_bounded, write_ghost,
};

#[test]
fn mount_variant_constructs_and_displays() {
    let err = GhostBootError::Mount(MountError::StartPointMissing {
        expected: PathBuf::from("ghost/master/descript.txt"),
    });

    let rendered = err.to_string();
    assert!(
        rendered.contains("ghost mount resolution failed"),
        "unexpected Display output: {rendered}"
    );
    assert!(
        rendered.contains("StartPointMissing"),
        "Display should surface the underlying MountError variant: {rendered}"
    );
}

#[test]
fn mount_variant_is_a_std_error() {
    let err = GhostBootError::Mount(MountError::ShellDirMissing {
        expected: PathBuf::from("ghost/master/shell/master"),
    });

    let as_std_error: &dyn std::error::Error = &err;
    assert!(as_std_error.source().is_none());
}

// ---- boot 統合テスト ----

use dola::time::testing::at_ms;

/// `ghost_root` に `ghost/master/descript.txt` が無いと `boot` は
/// `Err(GhostBootError::Mount(_))` を返す。connect closure は呼ばれたら panic するものを仕込み、
/// マウント失敗より後に connect を評価する実装ならこのテスト自体が失敗する。
#[test]
fn boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring() {
    let temp = TempPath::new(
        "ghost-runtime-boot-returns-mount-error-and-short-circuits-before-touching-shiori-wiring",
    );
    let options = GhostBootOptions {
        shiori: ShioriWiring::Custom(Box::new(|| -> Result<Box<dyn ShioriBackend>, String> {
            panic!(
                "connect must never be invoked when mount resolution fails \
                 (boot must short-circuit before spawning anything)"
            );
        })),
        ..fake_options(temp.path().to_path_buf())
    };

    match boot(options) {
        Err(GhostBootError::Mount(_)) => {}
        Ok(_) => panic!(
            "boot must fail with GhostBootError::Mount when ghost_root has no \
             ghost/master/descript.txt"
        ),
    }
}

// ---- boot→shutdown 統合テスト ----

/// 解決可能な `ghost_root` で `boot` した `GhostRuntime` を `shutdown(CloseReason::System)` すると、
/// ForceQuit → kanade join → `DispatcherMsg::Close` → dispatcher join → shiori join →
/// relay 2 本の join の全段が完走し `Ok(())` を返す。`TickerMode::Disabled` なので ticker 段は
/// 飛ばされる。`shutdown` は別スレッドで呼び、有界 `recv_timeout` で観測する。
#[test]
fn boot_then_shutdown_joins_everything_and_returns_ok() {
    let temp = TempPath::new("ghost-runtime-boot-then-shutdown-joins-everything-and-returns-ok");
    let root = temp.path().to_path_buf();
    write_ghost(&root);

    let runtime =
        boot(fake_options(root)).expect("boot should succeed for a resolvable ghost_root");

    shutdown_bounded(runtime);
}

/// `into_parts()` で取り出した `GhostParts` は、`kanade`／`dispatcher` の投函端が生きており、
/// `TickerMode::Disabled` なので `ticker` は `None`、`handles` に全 `ActorHandle` が揃う。
/// 取り出した部品だけで `shutdown()` と同等の手順を手作業で駆動できる。
#[test]
fn into_parts_exposes_live_senders_and_all_handles_for_manual_teardown() {
    let temp = TempPath::new(
        "ghost-runtime-into-parts-exposes-live-senders-and-all-handles-for-manual-teardown",
    );
    let root = temp.path().to_path_buf();
    write_ghost(&root);

    let runtime =
        boot(fake_options(root)).expect("boot should succeed for a resolvable ghost_root");
    let parts = runtime.into_parts();

    assert!(
        parts.ticker.is_none(),
        "ticker sender must be None when TickerMode::Disabled was used"
    );
    assert!(
        parts.handles.ticker.is_none(),
        "ticker handle must be None when TickerMode::Disabled was used"
    );

    parts
        .kanade
        .send(KanadeMsg::Tick { now: at_ms(1) })
        .expect("kanade sender from into_parts should still be alive");
    parts
        .dispatcher
        .send(DispatcherMsg::Tick { now: at_ms(1) })
        .expect("dispatcher sender from into_parts should still be alive");

    let GhostParts {
        kanade,
        dispatcher,
        ticker: _,
        sylphya,
        sylphya_reader: _,
        handles,
    } = parts;
    let GhostHandles {
        kanade: kanade_handle,
        dispatcher: dispatcher_handle,
        shiori: shiori_handle,
        start_relay: start_relay_handle,
        down_relay: down_relay_handle,
        choice_wake_relay,
        ticker: _,
        sylphya: sylphya_handle,
    } = handles;
    assert!(
        choice_wake_relay.is_none(),
        "choice_wake_relay must be None when TickerMode::Disabled was used"
    );

    // shutdown() と同等の手順を手作業で駆動する。
    run_bounded(
        "manual teardown driven from into_parts",
        std::time::Duration::from_secs(10),
        move || {
            kanade
                .send(KanadeMsg::ForceQuit {
                    reason: areka_kanade::CloseReason::System,
                })
                .expect("kanade should still accept ForceQuit");
            kanade_handle
                .join()
                .expect("kanade should terminate normally after ForceQuit");

            dispatcher
                .send(DispatcherMsg::Close)
                .expect("dispatcher should still accept Close");
            dispatcher_handle
                .join()
                .expect("dispatcher should terminate normally after Close");

            shiori_handle
                .join()
                .expect("shiori should terminate normally (shiori_tx dropped with kanade)");
            start_relay_handle
                .join()
                .expect("start-relay should terminate normally (natural disconnect)");
            down_relay_handle
                .join()
                .expect("down-relay should terminate normally (natural disconnect)");

            // sylphya 供給者停止＋join（shutdown() 最終段の手作業ミラー）。
            sylphya.close();
            sylphya_handle
                .join()
                .expect("sylphya should terminate normally after Close");
        },
    );
}

// ---- sylphya_publisher() アクセサ ----

/// [`GhostRuntime::sylphya_publisher`] が返す供給端は boot が据えた生きた sylphya アクターのもので、
/// `persist_put` の投函後に `barrier()` が `Ok(())` を返す（アクターが投函を処理した）ことで観測する。
/// `persist_put` 自体は戻り値を持たないので、直後の反映フェンスで生存を確かめる。
#[test]
fn sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put() {
    let temp = TempPath::new(
        "ghost-runtime-sylphya-publisher-accessor-yields-live-publisher-that-accepts-persist-put",
    );
    let root = temp.path().to_path_buf();
    write_ghost(&root);

    let runtime =
        boot(fake_options(root)).expect("boot should succeed for a resolvable ghost_root");

    let publisher: &areka_sylphya::SylphyaPublisher = runtime.sylphya_publisher();
    publisher.persist_put(
        areka_sylphya::PersistScope::Ghost,
        vec![(areka_sylphya::PersistKey::BootCount, "1".into())],
    );

    // 反映フェンスで、アクセサが返したのが生きた供給端であることを観測する。宙吊り防止に有界化。
    run_bounded(
        "barrier after persist_put via sylphya_publisher()",
        std::time::Duration::from_secs(10),
        move || {
            runtime
                .sylphya_publisher()
                .barrier()
                .expect("sylphya actor should process the persist_put and reflect it");
            runtime
                .shutdown(areka_kanade::CloseReason::System)
                .expect("clean shutdown");
        },
    );
}

// ---- shutdown() の barrier() 明示フラッシュ確認 ----

/// `boot` した `GhostRuntime` を `shutdown(CloseReason::System)` すると、sylphya `close()` の**直前**に
/// `barrier()` が呼ばれ、成功時に固定 info ログ `persist flush confirmed`（target `ghost-shutdown`）を出す。
///
/// write-through（FIFO close）が保証の正本で、この `barrier()` は best-effort な終了時フラッシュの安全網。
/// ログは「barrier が Ok を返し、その位置が close の直前である」ことの証跡になる。
///
/// `capture` はスレッドローカルに subscriber を差すため、`shutdown` を走らせる同一スレッド
/// （`run_bounded` の spawn 先）で出た info ログを捕捉する。join の宙吊りに備え有界化する。
#[test]
fn shutdown_confirms_persist_flush_via_barrier_before_close() {
    use crate::test_log_capture::{assert_logged, capture};

    let temp =
        TempPath::new("ghost-runtime-shutdown-confirms-persist-flush-via-barrier-before-close");
    let root = temp.path().to_path_buf();
    write_ghost(&root);

    let runtime =
        boot(fake_options(root)).expect("boot should succeed for a resolvable ghost_root");

    run_bounded(
        "shutdown with barrier flush confirmation",
        std::time::Duration::from_secs(10),
        move || {
            let events = capture(|| {
                let result = runtime.shutdown(areka_kanade::CloseReason::System);
                // barrier がアクター既死で Err でも panic せず続行するのが正で、
                // 生きた sylphya アクターに対するこの経路は必ず Ok を返す。
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) with a live sylphya actor, got {result:?}"
                );
            });
            assert_logged(
                &events,
                tracing::Level::INFO,
                "ghost-shutdown",
                "persist flush confirmed",
            );
        },
    );
}

// ---- InProc 結線の生成〜駆動〜終了 統合テスト ----

use std::sync::Arc;

/// 実ビルド済み x64 テスト DLL（`shiori4_testdll.dll`）を fixture の `ghost/master/` へ置き、
/// `ShioriWiring::InProc` で boot する。boot が送る `KanadeMsg::Boot` を起点に、実 DLL の OnBoot 応答
/// （挨拶 Value）が kanade→start-relay→dispatcher→sakura→`RecordingSink` へ届くまでを Tick 注入のみ
/// （sleep 不使用・注入時刻のみで前進）で駆動し、`RecordingSink` に `TalkCue` が 1 件以上捕捉されることと、
/// `shutdown(CloseReason::System)` が `Ok(())` を返すことを有界待機で確認する。
///
/// 結線の smoke に留める。OnBoot cue 列の全順序照合は別のテストが受け持つ。
#[test]
fn inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll() {
    let temp = crate::shiori_inproc::tests::dll_ghost("ghost-runtime-inproc-wiring");
    let root = temp.path().to_path_buf();

    let recording = RecordingSink::new();
    let records = recording.records();

    let options = GhostBootOptions {
        shiori: ShioriWiring::InProc,
        sinks: vec![Box::new(recording.clone()), Box::new(recording.clone())],
        ..fake_options(root)
    };

    let runtime = boot(options).expect("boot should succeed through ShioriWiring::InProc");

    // 駆動: dispatcher へ Tick を注入し続け、`RecordingSink` が挨拶 cue を捕捉するまで待つ
    // （sleep 不使用・単調増加 `now` の注入と `yield_now` のみ）。
    //
    // 反復回数でなく壁時計デッドラインで括る: 最初の SHIORI 呼出が shiori アクタースレッド上で
    // 実 DLL の `LoadLibraryExW`＋`CreateInstance` を初めて走らせ、数十 ms を要するため。
    // デッドラインは宙吊り防止の上限にすぎず、時刻の前進は注入 Tick のみ。
    let mut now: i64 = 1;
    let mut fired = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick { now: at_ms(now) })
            .expect("dispatcher actor should still be alive while probing for the boot talk");
        now += 1;
        if !records.lock().expect("records mutex poisoned").is_empty() {
            fired = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        fired,
        "InProc: OnBoot 挨拶 cue が Tick 注入後も sink に届かなかった——実 DLL 境界を横断した \
         生成〜駆動が成立していない"
    );

    shutdown_bounded(runtime);

    assert!(
        !records.lock().expect("records mutex poisoned").is_empty(),
        "RecordingSink には少なくとも 1 件の TalkCue が捕捉されているべき（挨拶が InProc 境界を越えた）"
    );
}

// ---- apply_boot_record_gate 単体 ----

use areka_sylphya::persist::FakePersistIo;
use areka_sylphya::{
    PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, save_scope, spawn_sylphya,
};

/// Ghost スコープへ `seed` を事前保存した実 sylphya を起動し、`(parts, ghost_asker)` を返す。
/// `parts.reader` は無待機で `resolve_dotted_str` により seed 値を観測できる。seed が空なら記録なしのケース。
fn spawn_reader_seeded(seed: Vec<(PersistKey, String)>) -> (SylphyaParts, AskerId) {
    let shared = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        ghost: Some(PathBuf::from("/gate-ghost")),
        ..ScopeRoots::default()
    };
    if !seed.is_empty() {
        save_scope(PersistScope::Ghost, &roots, &shared, seed);
    }
    let parts = spawn_sylphya(SylphyaInit {
        roots,
        io: Box::new(shared),
        runtime_sink: None,
    });
    (parts, AskerId::new("ghost/gate-asker"))
}

/// テスト用の素の config（gate が触る 3 フィールドのみが観測対象・他は既定）。
fn base_config() -> KanadeConfig {
    KanadeConfig::new("master", "0.0.0-test")
}

/// step 1＋3（記録なし）: `areka.boot.count` 不在 → `first_boot=true` かつ
/// `first_boot_epilogue` に `areka.prop.set`/`areka.boot.count`/"1" が 1 件添付される。
#[test]
fn gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue() {
    let (parts, asker) = spawn_reader_seeded(vec![]);

    let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

    assert!(
        gated.first_boot,
        "起動記録（areka.boot.count）不在 → 初回起動（first_boot=true）"
    );
    assert_eq!(
        gated.first_boot_epilogue,
        vec![EpilogueCommand {
            name: PROP_SET_CUE_NAME.to_string(),
            tokens: vec!["areka.boot.count".to_string(), "1".to_string()],
        }],
        "初回は areka.prop.set / areka.boot.count / \"1\" の SET epilogue を 1 件添付する"
    );

    parts.publisher.close();
    parts.handle.join().expect("clean close joins");
}

/// step 1＋3（記録あり）: `areka.boot.count` 存在 → `first_boot=false` かつ epilogue 非添付
/// （2 回目以降起動）。
#[test]
fn gate_present_boot_record_marks_returning_and_no_epilogue() {
    let (parts, asker) = spawn_reader_seeded(vec![(PersistKey::BootCount, "1".into())]);

    let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

    assert!(
        !gated.first_boot,
        "起動記録あり → 2 回目以降起動（first_boot=false）"
    );
    assert!(
        gated.first_boot_epilogue.is_empty(),
        "非初回は起動記録 epilogue を添付しない"
    );

    parts.publisher.close();
    parts.handle.join().expect("clean close joins");
}

/// step 1（存在ゲートは数値解釈しない）: `areka.boot.count="0"` でも「存在」ゆえ
/// `first_boot=false`（値の中身を問わない存在判定）。
#[test]
fn gate_boot_record_is_existence_not_value() {
    let (parts, asker) = spawn_reader_seeded(vec![(PersistKey::BootCount, "0".into())]);

    let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

    assert!(
        !gated.first_boot,
        "存在ゲート: boot.count の値（\"0\"）に関わらず、存在すれば非初回"
    );
    assert!(gated.first_boot_epilogue.is_empty());

    parts.publisher.close();
    parts.handle.join().expect("clean close joins");
}

/// step 2（vanish 数値 present）: `areka.vanish.count="7"` → `vanish_count==7`。
#[test]
fn gate_vanish_count_present_numeric_is_parsed() {
    let (parts, asker) = spawn_reader_seeded(vec![
        (PersistKey::BootCount, "1".into()),
        (PersistKey::VanishCount, "7".into()),
    ]);

    let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

    assert_eq!(
        gated.vanish_count, 7,
        "areka.vanish.count=\"7\" → 7 が parse される"
    );

    parts.publisher.close();
    parts.handle.join().expect("clean close joins");
}

/// step 2（vanish 不在）: `areka.vanish.count` 不在 → `vanish_count==0`（既定縮退）。
#[test]
fn gate_vanish_count_absent_defaults_zero() {
    let (parts, asker) = spawn_reader_seeded(vec![]);

    let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

    assert_eq!(gated.vanish_count, 0, "vanish.count 不在 → 0 縮退");

    parts.publisher.close();
    parts.handle.join().expect("clean close joins");
}

/// step 2（vanish 非数値）: `areka.vanish.count="abc"` → `vanish_count==0`（寛容縮退・panic せず
/// 起動を止めない）。
#[test]
fn gate_vanish_count_non_numeric_degrades_zero() {
    let (parts, asker) = spawn_reader_seeded(vec![
        (PersistKey::BootCount, "1".into()),
        (PersistKey::VanishCount, "abc".into()),
    ]);

    let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

    assert_eq!(
        gated.vanish_count, 0,
        "非数値 vanish.count は 0 へ寛容縮退する（起動は止めない・要件 6.3）"
    );

    parts.publisher.close();
    parts.handle.join().expect("clean close joins");
}

// ---- 終了時フラッシュの統合檻 ----

use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{Axis, load_scope};

/// 実 `FsPersistIo`（temp dir）上の実 sylphya アクターへ、`PersistWiring` の clone 送信端（UI スレッド
/// 常駐端の代役）から `barrier` を挟まずに複数回 `persist_put` を投函し（同一 scope の last-write-wins）、
/// その後 runtime 側 publisher から `barrier()`→`close()`→アクター `join` を駆動する。join 後に
/// `<ghost root>/sylphya.toml` を `load_scope` で読み戻し、clone 投函の最終値が反映されていることを確認する。
///
/// - write-through＋mpsc FIFO close: clone と runtime 側 publisher は同一 mpsc 送信端を共有するので、
///   clone が投函した put は後続の `Close`（Stop で積み残し破棄）より先に処理され、close→join を経れば
///   ファイルへ確実に反映される。
/// - 越境フェンス: runtime 側 publisher の `barrier()` は別送信端（clone）経由で enqueue 済みの put も被覆する
///   （単一 FIFO・shutdown 時点で UI 送信は静止済み）。
///
/// 投函値は非 96 倍数の一意値（1234→1777・841→907）で、ファイルに最終値だけが現れる（中間値 1234/841 は
/// 上書き消滅）ことが、barrier なしの clone put がフラッシュされた証跡になる。
///
/// `barrier`/`join` の宙吊りに備え `run_bounded` で有界化する。
#[test]
fn exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence() {
    let temp = TempPath::new(
        "ghost-runtime-exit-flush-reflects-barrierless-clone-puts-after-shutdown-sequence",
    );
    let root = temp.path().to_path_buf();

    let roots = ScopeRoots {
        ghost: Some(root.clone()),
        ..ScopeRoots::default()
    };

    let parts = crate::sylphya_wiring::spawn_ghost_sylphya(roots.clone(), None);

    // `PersistWiring` の clone 送信端（UI スレッド常駐端の代役）。clone した publisher は同一 mpsc 送信端＝同一 FIFO。
    let ui_send_end = parts.publisher.clone();

    // barrier なし put（同一 scope 0・last-write-wins・非 96 倍数の判別値）。
    ui_send_end.persist_put(
        PersistScope::Ghost,
        vec![
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X,
                },
                "1234".into(),
            ),
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::Y,
                },
                "841".into(),
            ),
        ],
    );
    ui_send_end.persist_put(
        PersistScope::Ghost,
        vec![
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X,
                },
                "1777".into(),
            ),
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::Y,
                },
                "907".into(),
            ),
        ],
    );
    // clone 送信端では barrier を一切呼ばない（終了時フラッシュの安全網に委ねる）。
    drop(ui_send_end);

    let SylphyaParts {
        reader: _,
        publisher,
        handle,
    } = parts;
    run_bounded(
        "runtime-side barrier -> close -> join (exit flush fence)",
        std::time::Duration::from_secs(10),
        move || {
            publisher
                .barrier()
                .expect("runtime-side barrier must reflect clone-enqueued puts (live actor)");
            publisher.close();
            handle
                .join()
                .expect("sylphya actor joins cleanly after close");
        },
    );

    // ファイルの最終値が clone 投函の last-write-wins と等しい。
    let loaded = load_scope(PersistScope::Ghost, &roots, &FsPersistIo);
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X
            },
            "1777".to_string()
        )),
        "終了フラッシュ後、scope 0 の X はファイルへ clone 投函の最終値 1777 で反映されるべき: {loaded:?}"
    );
    assert!(
        loaded.contains(&(
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::Y
            },
            "907".to_string()
        )),
        "終了フラッシュ後、scope 0 の Y はファイルへ clone 投函の最終値 907 で反映されるべき: {loaded:?}"
    );
    // 中間値が残っていない。
    assert!(
        !loaded.iter().any(|(k, v)| matches!(
            k,
            PersistKey::WindowPos {
                scope: 0,
                axis: Axis::X
            }
        ) && v == "1234"),
        "中間値 1234 は最終値 1777 に上書きされているべき（last-write-wins）: {loaded:?}"
    );
}

// ---- 起こしの中継 ----

/// 刻み役が本物のときだけ起こしの中継が起き、`into_parts` の束に `Some` で載る（`Disabled` で
/// `None` なのは `into_parts_exposes_live_senders_and_all_handles_for_manual_teardown` が見る）。
/// 取り出した部品で kanade → 下りの中継 → 起こしの中継 → 刻み役の Close の順に畳めることも見る。
#[test]
fn choice_wake_relay_exists_only_with_the_real_ticker() {
    let temp = TempPath::new("ghost-runtime-choice-wake-relay-exists-only-with-the-real-ticker");
    let root = temp.path().to_path_buf();
    write_ghost(&root);

    let mut options = fake_options(root);
    options.ticker = TickerMode::Real(crate::ticker::TickerConfig::default());
    let parts = boot(options).expect("boot (Real)").into_parts();
    let GhostParts {
        kanade,
        dispatcher,
        ticker,
        sylphya,
        sylphya_reader: _,
        handles,
    } = parts;
    let GhostHandles {
        kanade: kanade_handle,
        dispatcher: dispatcher_handle,
        shiori: shiori_handle,
        start_relay: start_relay_handle,
        down_relay: down_relay_handle,
        choice_wake_relay,
        ticker: ticker_handle,
        sylphya: sylphya_handle,
    } = handles;
    let choice_wake_relay =
        choice_wake_relay.expect("choice_wake_relay must be Some with TickerMode::Real");
    let ticker = ticker.expect("ticker sender must be Some with TickerMode::Real");
    let ticker_handle = ticker_handle.expect("ticker handle must be Some with TickerMode::Real");

    run_bounded(
        "manual teardown with the real ticker",
        std::time::Duration::from_secs(10),
        move || {
            kanade
                .send(KanadeMsg::ForceQuit {
                    reason: areka_kanade::CloseReason::System,
                })
                .expect("kanade should still accept ForceQuit");
            kanade_handle
                .join()
                .expect("kanade should terminate normally");
            dispatcher
                .send(DispatcherMsg::Close)
                .expect("dispatcher should still accept Close");
            dispatcher_handle
                .join()
                .expect("dispatcher should terminate normally");
            shiori_handle
                .join()
                .expect("shiori should terminate normally");
            start_relay_handle
                .join()
                .expect("start-relay should terminate normally");
            down_relay_handle
                .join()
                .expect("down-relay should terminate normally");
            // 刻み役がまだ動いていても、kanade の停止で起こしの線の送り端が落ちて中継は終わる。
            choice_wake_relay
                .join()
                .expect("choice-wake-relay should terminate before the ticker Close");
            ticker
                .send(TickerMsg::Close)
                .expect("ticker should still accept Close");
            ticker_handle
                .join()
                .expect("ticker should terminate normally after Close");
            sylphya.close();
            sylphya_handle
                .join()
                .expect("sylphya should terminate normally");
        },
    );
}

/// 刻み役が本物の構成でも正規の終了が詰まらずに `Ok` で終わる（kanade → 下りの中継 →
/// 起こしの中継 → 刻み役の Close の順・中継は kanade の停止で送り端が落ちて自然に終わる）。
#[test]
fn shutdown_with_the_real_ticker_joins_the_choice_wake_relay() {
    let temp =
        TempPath::new("ghost-runtime-shutdown-with-the-real-ticker-joins-the-choice-wake-relay");
    let root = temp.path().to_path_buf();
    write_ghost(&root);

    let mut options = fake_options(root);
    options.ticker = TickerMode::Real(crate::ticker::TickerConfig::default());
    let runtime = boot(options).expect("boot (Real)");
    shutdown_bounded(runtime);
}
