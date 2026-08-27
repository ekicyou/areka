use super::*;
use std::path::PathBuf;
use temp_path_kit::TempPath;

/// task 8.3: 退役した `default_system_vars()` の忠実な代役スタンドイン。
///
/// `{"username": DEFAULT_USERNAME}` のみを充填した凍結スナップショットを毎回新規構築して
/// 返す（退役前 provider と同一挙動）。既存テストの意図——既定 username 前提の凍結像刻印——を
/// [`SystemVarWiring::Custom`] 経由で無改変のまま保つための in-crate 共有ヘルパ。sylphya 読み口
/// （[`SystemVarWiring::FromSylphya`]）差替後も、これら既存テストは従来どおりの直接注入
/// セマンティクスを `Custom` で維持する（R7.1・R9.1・design「テスト呼出面の一括更新」）。
fn test_system_vars() -> SystemVarSource {
    Box::new(|| {
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", areka_sakura::sysvar::DEFAULT_USERNAME);
        snapshot
    })
}

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

    // 呼び出し側が `Box<dyn std::error::Error>` 等で一律に扱えることの確認。
    let as_std_error: &dyn std::error::Error = &err;
    assert!(as_std_error.source().is_none());
}

// ---- boot 統合テスト（task 3.1） ----

use areka_kanade::MonotonicMs;
use areka_sakura::contract::{CueSink, TalkCue};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

/// このテスト専用の一時ディレクトリ。共通窓口 `temp-path-kit` 経由で組むので、
/// 名前にプロセス識別子と連番が入り**プロセス間でも一意**（同じテストを同時に
/// 複数プロセスで走らせても互いの一時ファイルを消し合わない）。
///
/// 返り値が生きている間だけ実体が存在し、破棄で中身ごと消える。
fn unique_temp_dir(tag: &str) -> TempPath {
    TempPath::new(&format!("ghost-runtime-{tag}"))
}

/// `root` 直下に最小限の解決可能なゴーストツリー（`ghost/master/descript.txt`＋
/// `shell/master/descript.txt`）を構築する（`boot` が内部で `resolve` を通す
/// ための happy-path fixture）。
fn write_minimal_resolvable_ghost_fixture(root: &std::path::Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
    )
    .expect("write ghost descript.txt");

    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,TestShell\n",
    )
    .expect("write shell descript.txt");
}

/// テスト専用の最小 `ShioriBackend` fake（`get`/`notify` は無害な既定応答・`unload`
/// は Clean・`status` は Running を返すのみ・task 4.1 の `ScriptedShioriBackend` の
/// ような台本化はしない——boot 組み上げの結線成立だけを確認すれば足りる）。
struct FakeShioriBackend;

impl ShioriBackend for FakeShioriBackend {
    fn get(
        &mut self,
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        Ok(None)
    }

    fn notify(
        &mut self,
        _id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<(), RequestError> {
        Ok(())
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        Ok(ExitKind::Clean)
    }

    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }
}

/// テスト専用の `Clone` 可能な no-op sink（dispatcher の per-talk 注入
/// （`S: Clone`/`T: Clone`）を満たすためだけの最小実装・`dispatcher.rs` の
/// `RecordingSink` 流儀に倣うが、本テストは発火内容を検査しないため蓄積しない）。
#[derive(Clone)]
struct NoopSink;

impl CueSink for NoopSink {
    fn emit(&mut self, _cue: TalkCue) {}
}

/// シナリオ1（happy path）: 解決可能な `ghost_root`・`ShioriWiring::Custom`（fake
/// backend）・`TickerMode::Disabled` で `boot` すると `Ok(GhostRuntime)` が返り、
/// `kanade()`／`dispatcher()` の両方の投函端が生きている（＝実際にアクタースレッドが
/// 起動し受信ループへ入っている）ことを send の成功で確認する（要件 2.1/2.2/2.4）。
///
/// 本テストは boot 単体の結線成立のみを見るため、意図的に `shutdown()` を呼ばず
/// `runtime` を drop する——`ActorHandle` は非 RAII（detached）であり、テストプロセス
/// 終了時にスレッドがブロックしたまま回収されるのは想定どおり（design.md「保持物」
/// 節）。boot→shutdown の一連の流れは下記の
/// `boot_then_shutdown_joins_everything_and_returns_ok`（task 3.2）で確認する。
#[test]
fn boot_happy_path_wires_all_components_and_kicks_off_boot_sequence() {
    let temp = unique_temp_dir("boot-happy-path-wires-all-components-and-kicks-off-boot-sequence");
    let root = temp.path().to_path_buf();
    write_minimal_resolvable_ghost_fixture(&root);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| {
            Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    // kanade actor が生存し受信ループに入っていることの直接証跡（send 成功）。
    runtime
        .kanade()
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1),
        })
        .expect("kanade actor thread should be alive and receiving after boot");

    // dispatcher actor が生存し受信ループに入っていることの直接証跡（send 成功）。
    runtime
        .dispatcher()
        .send(DispatcherMsg::Tick {
            now: MonotonicMs(1),
        })
        .expect("dispatcher actor thread should be alive and receiving after boot");
}

/// シナリオ2（mount 失敗の短絡）: `ghost_root` に `ghost/master/descript.txt` が
/// 存在しない場合、`boot` は `Err(GhostBootError::Mount(_))` を返す。`shiori`
/// フィールドの connect closure は「呼ばれたら panic する」ものを故意に仕込み、
/// もし実装がマウント失敗より後にも connect を評価してしまうバグがあればこの
/// テスト自体が panic で失敗する——マウント解決失敗時に他のいかなるコンポーネント
/// も spawn されない（短絡する）ことの直接証跡になる（要件 2.5）。
#[test]
fn boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring() {
    let temp = unique_temp_dir(
        "boot-returns-mount-error-and-short-circuits-before-touching-shiori-wiring",
    );
    let root = temp.path().to_path_buf();
    // 起点不在を保証する（ディレクトリごと未作成・ghost/master/descript.txt 無し）。
    let _ = std::fs::remove_dir_all(&root);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Ansi,
        shiori: ShioriWiring::Custom(Box::new(|| -> Result<Box<dyn ShioriBackend>, String> {
            panic!(
                "connect must never be invoked when mount resolution fails \
                 (boot must short-circuit before spawning anything)"
            );
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    match boot(options) {
        Err(GhostBootError::Mount(_)) => {}
        Ok(_) => panic!(
            "boot must fail with GhostBootError::Mount when ghost_root has no \
             ghost/master/descript.txt"
        ),
    }
}

// ---- boot→shutdown 統合テスト（task 3.2） ----

/// テスト用の有界待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければ
/// テストを失敗させる（`dispatcher.rs`／`ticker.rs` テストモジュールと同じ流儀の
/// ローカルコピー・仮に `shutdown` の join が宙吊りするバグがあってもテスト
/// スイート全体をハングさせない）。
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

/// シナリオ3（happy path・task 3.2）: 解決可能な `ghost_root` で `boot` した
/// `GhostRuntime` に対し `shutdown(CloseReason::System)` を呼ぶと、`ForceQuit` →
/// kanade join → `DispatcherMsg::Close` → dispatcher join → shiori join → relay
/// 2 本の join という全段が完走し `Ok(())` を返す（要件 6.1/6.4）。`TickerMode::Disabled`
/// で組むため ticker 段は完全にスキップされる。`shutdown` 呼出自体を別スレッドへ
/// 逃がし有界 `recv_timeout` で観測する（本テストの完了条件そのものが「正常な
/// 起動〜終了の一連の流れ」の直接証跡になる）。
#[test]
fn boot_then_shutdown_joins_everything_and_returns_ok() {
    let temp = unique_temp_dir("boot-then-shutdown-joins-everything-and-returns-ok");
    let root = temp.path().to_path_buf();
    write_minimal_resolvable_ghost_fixture(&root);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| {
            Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    run_bounded(
        "shutdown after boot",
        std::time::Duration::from_secs(10),
        move || {
            let result = runtime.shutdown(areka_kanade::CloseReason::System);
            assert!(
                result.is_ok(),
                "shutdown should return Ok(()) when every stage joins cleanly, got {result:?}"
            );
        },
    );
}

/// シナリオ4（`into_parts` 構造分解・task 3.2）: `boot` した `GhostRuntime` から
/// `into_parts()` で `GhostParts` を取り出すと、`kanade`／`dispatcher` の投函端が
/// 生きており（send 成功で確認）、`TickerMode::Disabled` に対応して `ticker` は
/// `None`、`handles` に全 `ActorHandle` が揃っている。取り出した部品だけを使って
/// `shutdown()` と同等の手順（ForceQuit→kanade join→Close→dispatcher join→
/// shiori join→relay 2 本 join）を手作業で駆動できることを示す——`into_parts` が
/// S6 全断線シナリオ等の分解結線に必要な全てを過不足なく提供している直接証跡。
#[test]
fn into_parts_exposes_live_senders_and_all_handles_for_manual_teardown() {
    let temp =
        unique_temp_dir("into-parts-exposes-live-senders-and-all-handles-for-manual-teardown");
    let root = temp.path().to_path_buf();
    write_minimal_resolvable_ghost_fixture(&root);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| {
            Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
    let parts = runtime.into_parts();

    // ticker は TickerMode::Disabled に対応して None（送出端・handle 両方）。
    assert!(
        parts.ticker.is_none(),
        "ticker sender must be None when TickerMode::Disabled was used"
    );
    assert!(
        parts.handles.ticker.is_none(),
        "ticker handle must be None when TickerMode::Disabled was used"
    );

    // kanade/dispatcher の投函端が生きていることの直接証跡（send 成功）。
    parts
        .kanade
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1),
        })
        .expect("kanade sender from into_parts should still be alive");
    parts
        .dispatcher
        .send(DispatcherMsg::Tick {
            now: MonotonicMs(1),
        })
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
        ticker: _,
        sylphya: sylphya_handle,
    } = handles;

    // shutdown() と同等の手順を手作業で駆動する（ForceQuit→join→Close→join→
    // shiori/relay join・design.md「終了（shutdown）シーケンス」）。
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

            // sylphya 供給者停止＋join（shutdown() 最終段の手作業ミラー・design「shutdown」step 10）。
            sylphya.close();
            sylphya_handle
                .join()
                .expect("sylphya should terminate normally after Close");
        },
    );
}

// ---- sylphya_publisher() アクセサ（task 3.1・position-persist） ----

/// シナリオ（task 3.1・position-persist）: `boot` した `GhostRuntime` から
/// [`GhostRuntime::sylphya_publisher`] で sylphya 供給端の参照を取得し、その参照経由で
/// `persist_put`（永続 put の投函）が呼び出せることを確認する（requirements.md 6.2・
/// design.md「C5 GhostRuntime 増分」——main が `PersistWiring` を組むために公開する
/// `kanade()`／`dispatcher()` と同型の additive アクセサ）。
///
/// アクセサが返すのは boot が据えた生きた sylphya アクターの供給端であることを、
/// `persist_put` 投函後に `barrier()` が `Ok(())` を返す（＝アクターが投函を処理して
/// 反映を完了できた）ことで観測する（fire-and-forget な `persist_put` 自体は戻り値を
/// 持たないため、直後の反映フェンスで生存を証拠づける）。
#[test]
fn sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put() {
    let temp = unique_temp_dir(
        "sylphya-publisher-accessor-yields-live-publisher-that-accepts-persist-put",
    );
    let root = temp.path().to_path_buf();
    write_minimal_resolvable_ghost_fixture(&root);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| {
            Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    // アクセサ経由で取得した供給端で persist_put が呼び出せる（requirements.md 6.2）。
    let publisher: &areka_sylphya::SylphyaPublisher = runtime.sylphya_publisher();
    publisher.persist_put(
        areka_sylphya::PersistScope::Ghost,
        vec![(areka_sylphya::PersistKey::BootCount, "1".into())],
    );

    // 反映フェンスで、アクセサが返したのが生きた供給端であることを観測する
    // （投函が処理され反映が完了 → sylphya アクター生存）。宙吊り防止に有界化。
    run_bounded(
        "barrier after persist_put via sylphya_publisher()",
        std::time::Duration::from_secs(10),
        move || {
            runtime
                .sylphya_publisher()
                .barrier()
                .expect("sylphya actor should process the persist_put and reflect it");
            // アクター後片付け（宙吊りスレッド回避のため clean shutdown）。
            let _ = runtime.shutdown(areka_kanade::CloseReason::System);
        },
    );
}

// ---- shutdown() の barrier() 明示フラッシュ確認（task 3.2・position-persist） ----

/// シナリオ（task 3.2・position-persist）: `boot` した `GhostRuntime` を
/// `shutdown(CloseReason::System)` すると、sylphya `close()`（step 10）の**直前**に
/// `barrier()` が呼ばれ、成功時に固定 info ログ `persist flush confirmed`
/// （target `ghost-shutdown`）を発行する（requirements.md 1.2・design.md「C5 GhostRuntime
/// 増分」step 10 直前・Monitoring「persist flush confirmed（C5 info）」）。
///
/// write-through（FIFO close＝E1）が保証の正本で、この `barrier()` は best-effort な
/// 終了時フラッシュ安全網（E2-lite）。ログ発行は「barrier が Ok を返し、その位置が close の
/// 直前である」ことの直接証跡になる（barrier→info→close のコード配置で順序が担保される）。
///
/// `capture` はスレッドローカルに subscriber を差すため、`shutdown` を走らせる同一スレッド
/// （`run_bounded` の spawn 先）で発行された info ログを捕捉する。join の宙吊りに備え有界化する。
#[test]
fn shutdown_confirms_persist_flush_via_barrier_before_close() {
    use crate::test_log_capture::{assert_logged, capture};

    let temp = unique_temp_dir("shutdown-confirms-persist-flush-via-barrier-before-close");
    let root = temp.path().to_path_buf();
    write_minimal_resolvable_ghost_fixture(&root);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| {
            Ok(Box::new(FakeShioriBackend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(NoopSink), Box::new(NoopSink)],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    run_bounded(
        "shutdown with barrier flush confirmation",
        std::time::Duration::from_secs(10),
        move || {
            let events = capture(|| {
                let result = runtime.shutdown(areka_kanade::CloseReason::System);
                // Err パス（barrier がアクター既死で Err）でも panic せず続行するのが正で、
                // 生きた sylphya アクターに対する本 happy path は必ず Ok を返す（要件 6.1/6.4）。
                assert!(
                    result.is_ok(),
                    "shutdown should return Ok(()) with a live sylphya actor, got {result:?}"
                );
            });
            // barrier() が Ok を返し、その確認ログが close() の直前で発行されたことの直接証跡。
            assert_logged(
                &events,
                tracing::Level::INFO,
                "ghost-shutdown",
                "persist flush confirmed",
            );
        },
    );
}

// ---- InProc 結線の生成〜駆動〜終了 統合テスト（task 2.4） ----

use std::sync::{Arc, Mutex};

/// task 2.4 専用のローカル記録 sink（`dispatcher.rs`／spine e2e の `RecordingSink` と同型・
/// `Clone` 可能で全 cue を `Arc<Mutex<Vec<TalkCue>>>` へ蓄積する）。tests バイナリ側の
/// `RecordingSink` は runtime.rs の in-crate 檻からは import できないため、ここでローカルに
/// 定義し直す（`areka-bin-crate-internal-tests-in-crate` の流儀）。
#[derive(Clone)]
struct RecordingSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl RecordingSink {
    fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }
}

impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("records mutex poisoned")
            .push(cue);
    }
}

/// シナリオ5（task 2.4・第 3 の結線 `ShioriWiring::InProc` の生成〜駆動〜終了 一気通貫）:
/// 実ビルド済み x64 テスト DLL（`shiori4_testdll.dll`）を fixture の `ghost/master/` へ配置し、
/// `ShioriWiring::InProc`（他 2 方式＝`Helper`／`Custom` と同列に選べる第 3 の正規結線・
/// 要件 1.1/3.1/7.1）で boot する。boot が内部送出する `KanadeMsg::Boot` を起点に、実 DLL の
/// OnBoot 応答（挨拶 Value）が kanade→start-relay→dispatcher→sakura→`RecordingSink` へ届く
/// までを Tick 注入のみ（sleep 不使用・要件 7.3——注入時刻のみで前進）で駆動し、`RecordingSink`
/// に少なくとも 1 件の `TalkCue` が捕捉されること（＝実 InProc DLL 境界を横断して挨拶が sink まで
/// 到達した＝生成〜駆動）を確認する。続いて `shutdown(CloseReason::System)` が `Ok(())` を返す
/// こと（＝正規 clean close・終了）を有界待機で確認する。本番 main 結線・機種自動判別（要件
/// 7.2/7.3）には一切触れない——`boot()` の match へ `InProc` arm を 1 本足すのみ。
///
/// 本テストは**結線 smoke**（第 3 arm の end-to-end 成立の証明）に留める。OnBoot cue 列の
/// 全順序照合＋ドリフト検出の厳密 e2e は task 5.1 の担当であり、ここでは「≥1 cue 捕捉＋clean
/// shutdown」で足りる（重複させない）。
#[test]
fn inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll() {
    // 実ビルド済み cdylib を deps ディレクトリから locate する（`shiori_inproc.rs` の
    // happy_path 檻と同一導出・design.md D-1）。不在時は silent skip せず明示 panic。
    let test_exe = std::env::current_exe().expect("test executable path is available");
    let deps_dir = test_exe
        .parent()
        .expect("test executable resides in a deps directory");
    let built_dll = deps_dir.join(shiori4_testdll::DLL_FILE_NAME);
    assert!(
        built_dll.exists(),
        "built test DLL が正準位置に不在: {}\n\
         この cdylib は `cargo test --workspace` が自動ビルドし単一の正準位置（deps）へ出力する。\
         単独実行時は先に `cargo test --workspace`（または `cargo build -p shiori4-testdll`）を\
         実行すること（フォールバックは設けない・design.md D-1）。",
        built_dll.display()
    );

    let temp = unique_temp_dir("inproc-wiring-boots-drives-and-shuts-down-through-real-test-dll");
    let root = temp.path().to_path_buf();

    // fixture: ghost/master/descript.txt（`shiori,` 行＝テスト DLL 名）＋ shell/master/descript.txt
    // （shell dir 存在チェックを通すための最小 descript）。
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        format!(
            "charset,UTF-8\nname,Shiori4TestGhost\nshiori,{}\nseriko.defaultsurfacedirectoryname,master\n",
            shiori4_testdll::DLL_FILE_NAME
        )
        .as_bytes(),
    )
    .expect("write ghost descript.txt");

    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,Shiori4TestShell\n",
    )
    .expect("write shell descript.txt");

    // 実 cdylib を fixture の ghost/master/ へコピーする（`inproc_connect` が
    // `mount.shiori.dir.join(file)` でロードする位置・D-1）。
    std::fs::copy(
        &built_dll,
        ghost_master.join(shiori4_testdll::DLL_FILE_NAME),
    )
    .expect("copy built test DLL into ghost/master");

    let recording = RecordingSink::new();
    let records = recording.records();

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::InProc,
        sinks: vec![Box::new(recording.clone()), Box::new(recording.clone())],
        system_vars: SystemVarWiring::Custom(test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed through ShioriWiring::InProc");

    // 駆動: dispatcher へ Tick を注入し続け、`RecordingSink` が挨拶 cue を捕捉するまで待つ
    // （sleep 不使用・単調増加 `now` の注入と `yield_now` のみ・S1 spine 技法）。boot が内部で
    // 送った `KanadeMsg::Boot` により OnBoot GET が発火し、StartTalk が
    // start-relay→dispatcher の別スレッド 2 hop を渡り active slot に載った直後の Tick で
    // 挨拶が全 broadcast される。
    //
    // 反復回数ではなく**壁時計デッドライン**で括る点が S1 と異なる（S1 の scripted backend は
    // 即応するが、本 InProc 経路は最初の SHIORI 呼出が shiori アクタースレッド上で実 DLL の
    // `LoadLibraryW`＋`CreateInstance` を初めて走らせ、これに数十 ms を要する）。デッドラインは
    // `run_bounded` の `recv_timeout` と同じく**宙吊り防止の上限**にすぎず、シミュレーション時刻の
    // 前進は依然として注入 Tick のみ（sleep で talk timeline を進めない・要件 7.3）。DLL ロード
    // 完了後は最初の Tick で挨拶が必ず発火するため結果は決定論的。
    let mut now: u64 = 1;
    let mut fired = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
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

    // 終了: shutdown が clean に完走し `Ok(())` を返すこと（正規 close・有界待機で宙吊り防止）。
    run_bounded(
        "shutdown after InProc boot talk",
        std::time::Duration::from_secs(10),
        move || {
            let result = runtime.shutdown(areka_kanade::CloseReason::System);
            assert!(
                result.is_ok(),
                "shutdown should return Ok(()) after InProc boot talk, got {result:?}"
            );
        },
    );

    // ≥1 cue 捕捉の最終 assert（挨拶が実 InProc DLL 境界を越えて sink に届いた・task 2.4 の
    // 観測可能な完了条件）。
    assert!(
        !records.lock().expect("records mutex poisoned").is_empty(),
        "RecordingSink には少なくとも 1 件の TalkCue が捕捉されているべき（挨拶が InProc 境界を越えた）"
    );
}

// ---- apply_boot_record_gate 単体（task 7.1・position-persist・design「C5 GhostRuntime 増分」step 1-3） ----

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, save_scope, spawn_sylphya,
};

/// 同一 [`FakePersistIo`] を `Arc` 共有する委譲 IO（prop_sink.rs 先例の再実装）。
///
/// `FakePersistIo` は内部 `Mutex` で Clone 不可ゆえ、seed（`save_scope`）と spawn（`build_initial_image`
/// のロード）が**同一 store** を観測できるよう Arc 共有ハンドルで委譲する。
#[derive(Clone)]
struct SharedGateIo(Arc<FakePersistIo>);
impl PersistIo for SharedGateIo {
    fn read(&self, path: &std::path::Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &std::path::Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// Ghost スコープへ `seed` を事前保存した実 sylphya を起動し、`(parts, ghost_asker)` を返す。
///
/// spawn 時の `build_initial_image` が seed を初期鏡像の大域点付き区画（正準 key）へ投影するため、
/// `parts.reader` は無待機で `resolve_dotted_str` により seed 値を観測できる（本番 boot と同型の
/// 「起動時に永続をロード」経路）。seed が空なら不在ケース（記録なし）を表す。
fn spawn_reader_seeded(seed: Vec<(PersistKey, String)>) -> (SylphyaParts, AskerId) {
    let shared = SharedGateIo(Arc::new(FakePersistIo::new()));
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
/// `first_boot_epilogue` に `areka.prop.set`/`areka.boot.count`/"1" が 1 件添付される
/// （design C5-1/C5-3・要件 3.1/3.4）。
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
/// （2 回目以降起動・design C5-1/C5-3・要件 3.4）。
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
/// `first_boot=false`（値の中身を問わない存在判定・design C5-1「数値解釈しない」）。
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

/// step 2（vanish 数値 present）: `areka.vanish.count="7"` → `vanish_count==7`（要件 4.1）。
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

/// step 2（vanish 不在）: `areka.vanish.count` 不在 → `vanish_count==0`（既定縮退・要件 4.2）。
#[test]
fn gate_vanish_count_absent_defaults_zero() {
    let (parts, asker) = spawn_reader_seeded(vec![]);

    let gated = apply_boot_record_gate(base_config(), &parts.reader, &asker);

    assert_eq!(gated.vanish_count, 0, "vanish.count 不在 → 0 縮退");

    parts.publisher.close();
    parts.handle.join().expect("clean close joins");
}

/// step 2（vanish 非数値）: `areka.vanish.count="abc"` → `vanish_count==0`（寛容縮退・panic せず
/// 起動を止めない・要件 4.2/6.3）。
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

// ---- 終了時フラッシュの統合檻（task 8.4・position-persist・design Testing Strategy「Integration
//      Tests §3」・要件 1.2/8.1・design 軸E: E1 write-through＋mpsc FIFO close／E2-lite 越境フェンス） ----

use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{Axis, load_scope};

/// シナリオ（task 8.4・position-persist）: 実 `FsPersistIo`（temp dir）上の実 sylphya アクターへ、
/// **`PersistWiring` の clone 送信端**（UI スレッド常駐端の代役）から `barrier` を挟まずに複数回
/// `persist_put` を投函し（n×DragEnd 相当・同一 scope の last-write-wins）、その後 **runtime 側の
/// publisher** から `barrier()`→`close()`→アクター `join` の終了系列（design「shutdown」step 10・
/// E2-lite）を駆動する。join 後にファイル（`<ghost root>/sylphya.toml`）を `load_scope` で読み戻し、
/// clone 投函の**最終値**が反映されていることを確認する。
///
/// これは要件 1.2（正常終了時フラッシュ＝ドラッグ確定時 write-through への安全網）と 8.1（往復値
/// 等価の決定論檻）の統合檻であり、design 軸E の二重証明を兼ねる:
/// - **E1**（write-through＋mpsc FIFO close）: clone と runtime 側 publisher は同一 mpsc 送信端を
///   共有する（`PersistWiring` は `{ publisher: SylphyaPublisher }` ゆえ clone した publisher が
///   同一 FIFO）。clone が投函した put は、後続の `Close` 処理（Stop で積み残し破棄）より FIFO 順で
///   **先に**処理されるため、close→join を経ればファイルへ確実に反映される。
/// - **E2-lite（越境フェンス）**: runtime 側 publisher から呼ぶ `barrier()` が、**別送信端**（clone）
///   経由で enqueue 済みの put も被覆する（単一 FIFO・shutdown 時点で UI 送信は静止済み・design
///   「軸E」バリデーション Issue 2 対応）。
///
/// 判別性: 投函値は非 96 倍数の一意値（1234→1777・841→907）とし、最終値のみがファイルに現れる
/// （中間値 1234/841 は上書き消滅）ことを確認する——ファイルの最終値が観測できるのは、終了系列が
/// enqueue 済み put を処理したからに他ならない（barrier なしの clone put が確かにフラッシュされた証跡）。
///
/// `barrier`/`join` の宙吊りに備え `run_bounded` で有界化し、temp dir は成功パスで掃除する
/// （`FsPersistIo.commit` は親ディレクトリを作らない＝`File::create` のみ・のため ghost scope root を
/// 事前作成する。本番では `profile/areka/` が既存の前提）。
#[test]
fn exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence() {
    let temp =
        unique_temp_dir("exit-flush-reflects-barrierless-clone-puts-after-shutdown-sequence");
    let root = temp.path().to_path_buf();
    // FsPersistIo.commit は親ディレクトリを作らない（`File::create` のみ）ため、ghost scope root を
    // 事前作成する（本番では profile/areka/ が既存の前提）。未作成だと commit が Degraded になり
    // ファイルが書かれず、この檻が意味を失う。
    std::fs::create_dir_all(&root).expect("create ghost scope root dir");

    let roots = ScopeRoots {
        ghost: Some(root.clone()),
        ..ScopeRoots::default()
    };

    // runtime 側 sylphya（実 FsPersistIo・実アクター・本番 boot と同一の spawn 経路）。
    let parts = crate::sylphya_wiring::spawn_ghost_sylphya(roots.clone());

    // `PersistWiring` の clone 送信端（UI スレッド常駐端の代役）。PersistWiring は
    // `{ publisher: SylphyaPublisher }` ゆえ、clone した publisher が同一 mpsc 送信端＝同一 FIFO。
    let ui_send_end = parts.publisher.clone();

    // n×DragEnd 相当の barrier なし put（同一 scope 0・last-write-wins・非 96 倍数の判別値）。
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
    // clone 送信端では barrier を一切呼ばない（＝終了時フラッシュ安全網に委ねる・要件 1.2）。
    drop(ui_send_end);

    // runtime 側 shutdown フェンス（design「shutdown」step 10 の barrier→close→join を同型で駆動）。
    let SylphyaParts {
        reader: _,
        publisher,
        handle,
    } = parts;
    run_bounded(
        "runtime-side barrier -> close -> join (exit flush fence)",
        std::time::Duration::from_secs(10),
        move || {
            // 越境フェンス: 別送信端（clone）経由で enqueue 済みの put も被覆する（単一 FIFO）。
            publisher
                .barrier()
                .expect("runtime-side barrier must reflect clone-enqueued puts (live actor)");
            publisher.close();
            handle
                .join()
                .expect("sylphya actor joins cleanly after close");
        },
    );

    // アクター join 後、ファイルの最終値が clone 投函の last-write-wins と値等価であること
    // （barrier なし clone put が終了系列で確実にフラッシュされた・要件 1.2/8.1・E1＋E2-lite）。
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
    // 中間値が残っていないこと（last-write-wins の確認＝最終値のみが観測できるのは終了系列が
    // enqueue 済み put を処理したから）。
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
