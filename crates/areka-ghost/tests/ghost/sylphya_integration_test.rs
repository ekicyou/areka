//! sylphya × ghost 結線 統合テスト（task 8.4・design「ghost 結線 Validation」・
//! Testing Strategy → Integration Tests 2（ghost 結線）＋4（prefetch→初回 talk 順序））。
//!
//! 8.2 で sylphya（統一プロパティシステム）が ghost boot 系列へ結線された。本ファイルは
//! その end-to-end 結線を実 boot（mock shiori backend）で駆動し、下記 3 檻で固定する:
//!
//! 1. **boot 後の reader 実値解決**（R4.3/4.4/4.5/5.1/9.4）: descript 由来の selfname 系
//!    （`selfname`／`selfname2`／`keroname`）と baseware（`baseware.name`＝実値 `"areka"`／
//!    `baseware.version`＝areka-ghost の `CARGO_PKG_VERSION`）が、boot が据えた sylphya reader
//!    ＋ゴースト自身の `AskerId` から実値解決される（Degraded／NotFound でない）。
//! 2. **shutdown 全段成功（sylphya 段含む）**（R6.1/6.4/6.5）: boot→shutdown が全段
//!    （kanade→dispatcher→shiori→relay 2 本→sylphya Close＋join）を完走し `Ok(())` を返す。
//! 3. **username=Value → 初回 talk スナップショットに必ず入る（barrier フェンス・レース非依存）**
//!    （R2.1/2.2/4.2 ＋ prefetch→初回 talk 順序）: mock shiori が resource `username` へ
//!    `Value("SomeUser")` を返す構成で boot すると、prefetch sink の barrier が publish を
//!    初回 talk 前にフェンスするため、初回 talk が発火した時点の `talk_snapshot`（＝
//!    `from_sylphya_provider` が刻印する像そのもの）に `username = "SomeUser"` が**必ず**含まれる
//!    （sleep／retry なしで決定論成立）。companion で 204（NoContent）→ snapshot 非包含も固定する。
//!
//! ## ゴーストの AskerContext の入手（CONCERNS）
//! reader 照会は問い合わせ元 [`AskerContext`] を第一級で要する（R2.6）。本番 boot は
//! `ghost_asker_id(&mount.shiori.dir)` で構築し、`mount.shiori.dir` は `resolve` が
//! `ghost_root.join("ghost/master")` として合成する（正準化しない・`resolve.rs`）。ゆえに
//! テストは同じ導出——`areka_ghost::sylphya_wiring::ghost_asker_id(&root.join("ghost/master"))`
//! ——で production と**同一** asker を本番同型に再構築できる（GhostParts は asker を露出しない）。
//!
//! ## mock shiori の username=Value 台本化（CONCERNS）
//! username prefetch は kanade boot 系列が resource id `"username"`（`resource_username`・
//! `ALLOWED_RESOURCE_IDS`）へ GET する。mock backend が `get("username")` へ `Ok(Some("SomeUser"))`
//! を返すと、boot が kanade へ注入した実 `ResourceSink`（`make_username_resource_sink`）が
//! `publish_shiori(ghost_asker, "username", Some("SomeUser"))`＋`barrier()` を実行し、初回 talk 前に
//! 反映をフェンスする。

use std::collections::HashMap;

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::sylphya_wiring::ghost_asker_id;
use areka_ghost::{
    GhostBootOptions, GhostHandles, GhostParts, ShioriWiring, SystemVarWiring, TickerMode, boot,
};
use areka_kanade::{MonotonicMs, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_sylphya::{AskerContext, DottedResolution, FlatResolution};
use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};
use temp_path_kit::TempPath;

use crate::spine_e2e_test::RecordingSink;

/// 壁時計安全弁（兄弟 e2e と整合＝60s）。意味論 deadline は MonotonicMs 仮想時間で注入されるため
/// この壁時計値はテスト意味論に影響せず、並列負荷の飢餓による偽赤のみを防ぐ。
const E2E_BOUND: std::time::Duration = std::time::Duration::from_secs(60);

/// このテスト専用の一時ディレクトリ。共通窓口 `temp-path-kit` 経由で組むので、
/// 名前にプロセス識別子と連番が入り**プロセス間でも一意**（同じテストを同時に
/// 複数プロセスで走らせても互いの一時ファイルを消し合わない）。
///
/// 返り値が生きている間だけ実体が存在し、破棄で中身ごと消える。
fn unique_temp_dir(tag: &str) -> TempPath {
    TempPath::new(&format!("ghost-sylphya-{tag}"))
}

/// `root` 直下に解決可能なゴーストツリーを構築する。ghost/master/descript.txt の
/// `sakura.name`／`sakura.name2`／`kero.name` を引数の既知値で書く（selfname 系実値解決の入力・
/// R4.3/4.4/4.5）。shell/master/descript.txt は shell dir 存在確認を通すための最小 descript。
fn write_named_ghost_fixture(
    root: &std::path::Path,
    sakura_name: &str,
    sakura_name2: Option<&str>,
    kero_name: Option<&str>,
) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    let mut descript = String::from("charset,UTF-8\nname,SylphyaIntegGhost\n");
    descript.push_str(&format!("sakura.name,{sakura_name}\n"));
    if let Some(n2) = sakura_name2 {
        descript.push_str(&format!("sakura.name2,{n2}\n"));
    }
    if let Some(k) = kero_name {
        descript.push_str(&format!("kero.name,{k}\n"));
    }
    descript.push_str("shiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n");
    std::fs::write(ghost_master.join("descript.txt"), descript.as_bytes())
        .expect("write ghost descript.txt");

    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,SylphyaIntegShell\n",
    )
    .expect("write shell descript.txt");
}

/// 有界待機ヘルパ（`runtime.rs`／`spine_e2e_test.rs` と同旨のローカルコピー・宙吊り検出）。
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

/// 寛容 mock SHIORI backend（id ごとの GET 応答を map で持ち、未登録 id は `Ok(None)` を返す）。
///
/// `ScriptedShioriBackend`（spine e2e）は未登録 id で panic する厳密台本だが、本檻は起動系列の
/// 全 GET/NOTIFY を網羅列挙せず「username／OnBoot の値だけ効かせ、残りは無害既定」で足りるため、
/// 未登録は panic せず `Ok(None)`（GET）／`Ok(())`（NOTIFY）へ縮退する寛容 fake を用いる。
/// `unload`＝Clean・`status`＝Running（正規 clean shutdown 経路を通す）。
struct MapShioriBackend {
    get_values: HashMap<String, String>,
}

impl MapShioriBackend {
    fn new() -> Self {
        Self {
            get_values: HashMap::new(),
        }
    }

    /// `id` の GET 応答値を `Ok(Some(value))` として登録する。
    fn with_get(mut self, id: &str, value: &str) -> Self {
        self.get_values.insert(id.to_string(), value.to_string());
        self
    }
}

impl ShioriBackend for MapShioriBackend {
    fn get(
        &mut self,
        id: &str,
        _references: &[String],
        _status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        Ok(self.get_values.get(id).cloned())
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

/// GhostParts を `shutdown()` 同等の手順で手作業解体する（into_parts 経由で reader を握った後の
/// 後片付け・`runtime.rs` の into_parts 檻の teardown を忠実にミラー）。全段を有界待機で join する。
fn manual_teardown(parts: GhostParts) {
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

    run_bounded(
        "manual teardown after reader inspection",
        E2E_BOUND,
        move || {
            let _ = kanade.send(areka_kanade::KanadeMsg::ForceQuit {
                reason: areka_kanade::CloseReason::System,
            });
            kanade_handle
                .join()
                .expect("kanade should terminate normally after ForceQuit");
            let _ = dispatcher.send(DispatcherMsg::Close);
            dispatcher_handle
                .join()
                .expect("dispatcher should terminate normally after Close");
            shiori_handle
                .join()
                .expect("shiori should terminate normally (shiori_tx dropped with kanade)");
            start_relay_handle
                .join()
                .expect("start-relay should terminate naturally");
            down_relay_handle
                .join()
                .expect("down-relay should terminate naturally");
            sylphya.close();
            sylphya_handle
                .join()
                .expect("sylphya should terminate normally after Close");
        },
    );
}

/// 初回 talk が発火するまで dispatcher へ Tick を注入し続ける（sleep 不使用・単調増加 `now` の
/// 注入と `yield_now` のみ・S1 spine 技法）。boot が内部送出した `KanadeMsg::Boot` を起点に
/// prefetch（username GET＋barrier）→ OnBoot GET(Value) → StartTalk → dispatcher active slot →
/// 全 broadcast、の順で進む。初回 cue が `records` に載れば return（happens-before 確立点:
/// prefetch は OnBoot より厳密に前段ゆえ、cue 観測は barrier 反映後を含意する）。
fn drive_until_first_cue(
    dispatcher: &std::sync::mpsc::Sender<DispatcherMsg>,
    records: &std::sync::Arc<std::sync::Mutex<Vec<areka_sakura::contract::TalkCue>>>,
) {
    let mut now: u64 = 1;
    let deadline = std::time::Instant::now() + E2E_BOUND;
    while std::time::Instant::now() < deadline {
        dispatcher
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher actor should still be alive while probing for the boot talk");
        now += 1;
        if !records.lock().expect("records mutex poisoned").is_empty() {
            return;
        }
        std::thread::yield_now();
    }
    panic!(
        "boot talk cue never fired after repeated Tick within bound — the prefetch→talk sequence \
         did not progress (cannot verify the barrier fence without a fired boot talk)"
    );
}

// ============================================================================
// 檻 1: boot → reader が selfname 系＋baseware を実値解決する（R4.3/4.4/4.5/5.1/9.4）
// ============================================================================

/// descript の `sakura.name`／`sakura.name2`／`kero.name` を既知値で与えて boot すると、boot が
/// 据えた sylphya reader が、ゴースト自身の AskerId 文脈で `selfname`／`selfname2`／`keroname` を
/// **実値** 解決し、baseware（`baseware.name`＝`"areka"`／`baseware.version`＝areka-ghost の
/// `CARGO_PKG_VERSION`）を大域点付きで実値解決する（いずれも Degraded／NotFound でない）。
///
/// 反映の決定論化: 静的構成 publish（`publish_ghost_statics`）は boot 内で kanade spawn 前に投函
/// されるため、into_parts 後に `sylphya.barrier()`（FIFO＋直列処理）を打てば、その時点で静的層は
/// 鏡像へ反映済みである（sleep／retry 不要）。
#[test]
fn boot_reader_resolves_selfname_family_and_baseware_real_values() {
    let temp = unique_temp_dir("boot-reader-resolves-selfname-family-and-baseware-real-values");
    let root = temp.path().to_path_buf();
    write_named_ghost_fixture(&root, "むらさき", Some("紫"), Some("エモ"));

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| {
            Ok(Box::new(MapShioriBackend::new()) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![
            Box::new(RecordingSink::new()),
            Box::new(RecordingSink::new()),
        ],
        // 本番既定 provider（sylphya 読み口由来）で結線する（8.4 は結線の end-to-end を見る）。
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
    let parts = runtime.into_parts();

    // 静的層の反映フェンス（投函済み publish_static を鏡像へ反映してから読む・決定論）。
    parts
        .sylphya
        .barrier()
        .expect("sylphya barrier should succeed while the actor is alive");

    // production と同一の asker（mount.shiori.dir = ghost_root/ghost/master・正準化なし）。
    let ctx = AskerContext {
        asker: ghost_asker_id(&root.join("ghost/master")),
    };

    // --- selfname 系（フラット per-asker・実値） ---
    assert_eq!(
        parts.sylphya_reader.resolve_flat(&ctx, "selfname"),
        FlatResolution::Value("むらさき".to_string()),
        "selfname は sakura.name の実値へ解決されるべき（R4.3）"
    );
    assert_eq!(
        parts.sylphya_reader.resolve_flat(&ctx, "selfname2"),
        FlatResolution::Value("紫".to_string()),
        "selfname2 は sakura.name2 の実値へ解決されるべき（R4.4）"
    );
    assert_eq!(
        parts.sylphya_reader.resolve_flat(&ctx, "keroname"),
        FlatResolution::Value("エモ".to_string()),
        "keroname は kero.name の実値へ解決されるべき（R4.5）"
    );

    // --- baseware（大域点付き・実値・R5.1） ---
    assert_eq!(
        parts
            .sylphya_reader
            .resolve_dotted_str(&ctx, "baseware.name"),
        DottedResolution::Value("areka".to_string()),
        "baseware.name は実値 \"areka\" へ解決されるべき（R5.1）"
    );
    assert_eq!(
        parts
            .sylphya_reader
            .resolve_dotted_str(&ctx, "baseware.version"),
        DottedResolution::Value(env!("CARGO_PKG_VERSION").to_string()),
        "baseware.version は areka-ghost の CARGO_PKG_VERSION へ解決されるべき（R5.1）"
    );

    manual_teardown(parts);
}

// ============================================================================
// 檻 2: shutdown 全段（sylphya 段含む）が成功する（R6.1/6.4/6.5）
// ============================================================================

/// boot した `GhostRuntime` に `shutdown(System)` を打つと、全段（kanade→dispatcher→ticker（無）→
/// shiori→start-relay→down-relay→**sylphya Close＋join**）が best-effort 完走し `Ok(())` を返す
/// （どの段も panic 観測なし＝失敗集合が空）。sylphya 段（shutdown step 10）が clean に join する
/// ことは `Ok(())` に含意される（失敗があれば `GhostShutdownError.failures` に段名つきで現れる）。
#[test]
fn boot_then_shutdown_all_stages_including_sylphya_succeed() {
    let temp = unique_temp_dir("boot-then-shutdown-all-stages-including-sylphya-succeed");
    let root = temp.path().to_path_buf();
    write_named_ghost_fixture(&root, "むらさき", None, None);

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(|| {
            Ok(Box::new(MapShioriBackend::new()) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![
            Box::new(RecordingSink::new()),
            Box::new(RecordingSink::new()),
        ],
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");

    run_bounded(
        "shutdown (all stages incl. sylphya)",
        E2E_BOUND,
        move || {
            let result = runtime.shutdown(areka_kanade::CloseReason::System);
            assert!(
                result.is_ok(),
                "shutdown should return Ok(()) when every stage (incl. sylphya Close+join) joins \
                 cleanly, got {result:?}"
            );
        },
    );
}

// ============================================================================
// 檻 3: username=Value → 初回 talk スナップショットに必ず入る（barrier フェンス・レース非依存）
//        （R2.1/2.2/4.2 ＋ prefetch→初回 talk 順序・Integration Test 4）
// ============================================================================

/// mock shiori が resource `username` へ `Value("SomeUser")` を返す構成で boot する。prefetch sink
/// （`make_username_resource_sink`）が `publish_shiori(ghost_asker, "username", Some("SomeUser"))`＋
/// `barrier()` を実行し、**初回 talk より前**に反映をフェンスする。よって初回 talk が発火した時点の
/// `talk_snapshot`（＝`from_sylphya_provider` が dispatcher へ刻印する像そのもの）には
/// `username = "SomeUser"` が**必ず**含まれる——初回 cue の観測（happens-before 確立）後、
/// snapshot を**一度だけ** assert する（snapshot 自体の retry／sleep は一切なし＝レース非依存）。
#[test]
fn username_value_is_in_boot_talk_snapshot_via_barrier_fence() {
    let temp = unique_temp_dir("username-value-is-in-boot-talk-snapshot-via-barrier-fence");
    let root = temp.path().to_path_buf();
    write_named_ghost_fixture(&root, "むらさき", None, None);

    // 初回 talk 発火の観測点（driving 用）。OnBoot に非待ちの挨拶を積み、必ず 1 件以上 broadcast させる。
    let probe = RecordingSink::new();
    let records = probe.records();

    let backend = MapShioriBackend::new()
        // username prefetch（OnInitialize 後・OnFirstBoot 前・R4.1）が GET する resource id。
        .with_get("username", "SomeUser")
        // OnBoot 挨拶（`\w` 無し＝active slot 装填直後の単一 Tick で全発火・自然終端）。
        .with_get("OnBoot", r"\s[0]hello\e");

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(probe), Box::new(RecordingSink::new())],
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
    let parts = runtime.into_parts();

    // 初回 talk 発火まで駆動（prefetch→talk 順序ゆえ、cue 観測は barrier 反映後を含意する）。
    drive_until_first_cue(&parts.dispatcher, &records);

    // production と同一 asker で talk_snapshot を取る（provider が刻印する像と同一経路）。
    let ctx = AskerContext {
        asker: ghost_asker_id(&root.join("ghost/master")),
    };
    let snapshot = parts.sylphya_reader.talk_snapshot(&ctx);

    // barrier フェンスにより、初回 talk 発火時点の snapshot には username=Value が**必ず**入る
    // （一度だけ assert・snapshot の retry/sleep なし＝レース非依存）。
    assert_eq!(
        snapshot.get("username").map(String::as_str),
        Some("SomeUser"),
        "username=Value は barrier フェンスにより初回 talk スナップショットへ必ず反映される \
         （prefetch→talk 順序・R2.1/2.2/4.2）: {snapshot:?}"
    );

    manual_teardown(parts);
}

/// companion: mock shiori が resource `username` へ 204（NoContent）を返す構成では、prefetch sink が
/// 不在を記録するのみ（鏡像へ書かない・既定値は sakura の唯一定義点に残置・R4.2）ゆえ、初回 talk
/// スナップショットに `username` は**含まれない**（default 適用は下流 sakura の責務）。selfname は
/// 静的層由来で依然実在することも併せて固定する（不在記録が他フラット値を巻き添えにしない）。
#[test]
fn username_no_content_is_absent_from_boot_talk_snapshot() {
    let temp = unique_temp_dir("username-no-content-is-absent-from-boot-talk-snapshot");
    let root = temp.path().to_path_buf();
    write_named_ghost_fixture(&root, "むらさき", None, None);

    let probe = RecordingSink::new();
    let records = probe.records();

    // username は登録しない → GET は Ok(None)（204/NoContent 相当）。OnBoot 挨拶のみ登録。
    let backend = MapShioriBackend::new().with_get("OnBoot", r"\s[0]hello\e");

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(probe), Box::new(RecordingSink::new())],
        system_vars: SystemVarWiring::FromSylphya,
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed for a resolvable ghost_root");
    let parts = runtime.into_parts();

    drive_until_first_cue(&parts.dispatcher, &records);

    let ctx = AskerContext {
        asker: ghost_asker_id(&root.join("ghost/master")),
    };
    let snapshot = parts.sylphya_reader.talk_snapshot(&ctx);

    assert!(
        !snapshot.contains_key("username"),
        "204（NoContent）は不在記録のみで鏡像へ書かない——snapshot に username は含まれない \
         （既定値縮退は下流 sakura の責務・R4.2）: {snapshot:?}"
    );
    // 巻き添え非発生の裏取り: 静的層由来の selfname は依然実在する。
    assert_eq!(
        snapshot.get("selfname").map(String::as_str),
        Some("むらさき"),
        "username 不在記録が他の静的フラット値（selfname）を巻き添えにしてはならない"
    );

    manual_teardown(parts);
}
