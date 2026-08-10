// ===================== S7: 2 回目起動（起動記録あり）シナリオ（task 8.6） =====================
//
// design.md「Testing Strategy Integration §5」（"spine e2e 追随 … 2 回目起動相当（BootCount plant）で
// OnFirstBoot 非発火の新檻"）・「初回ゲートと起動記録」フロー・要件 3.3/8.3。
//
// S1（`s1_boot_success`）は永続ファイル無しの初回起動（`first_boot=true`）で、OnFirstBoot GET が
// 発火する（204 で OnBoot へフォールスルー）。本 S7 は **隔離した一時ゴースト**の永続鏡像へ
// `areka.boot.count` を **boot 前に据え**（S1 との唯一の差＝2 回目起動相当）、boot()`apply_boot_record_gate`
// が `first_boot=false` を解決 → kanade boot 系列が OnFirstBoot を **飛ばし** OnBoot（BootMain）から
// 起動運行を始めることを、実 ghost スタックを通して固定する。共有 first-boot fixture（i1/i2/s1 が依存）は
// 一切汚さない（temp-dir ＋ Drop ガード）。
//
// 観測（要件 3.3）:
//   1. 記録した SHIORI GET 系列に **OnFirstBoot GET が現れない**（OnBoot GET は現れる）。
//   2. `boot_gate skip_first_boot`（target="kanade" INFO・kanade アクタースレッド発火）がログされる。

use super::*;

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::MonotonicMs;
use areka_parsers::charset::DefaultEncoding;

use tracing::Level;

/// S7 専用の隔離ゴーストルート（Drop ガード付き・共有 first-boot fixture を汚さない）。
struct TempGhost {
    root: std::path::PathBuf,
}

impl TempGhost {
    fn new(tag: &str) -> Self {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "areka_ghost_spine_e2e_s7_{tag}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        Self { root }
    }
}

impl Drop for TempGhost {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// `root` 直下に最小限の解決可能なゴーストツリーを構築する（S1 の `write_ghost_fixture` と同旨・
/// sibling module の private item は参照できないためローカル複製）。
fn write_ghost_fixture(root: &std::path::Path, shell_name: &str) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        b"charset,UTF-8\nname,S7TestGhost\nshiori,dummy.dll\nseriko.defaultsurfacedirectoryname,master\n",
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

/// 起動記録（`areka.boot.count`）を ghost スコープの永続鏡像へ **boot 前に**据える（要件 3.3/8.3）。
///
/// boot() の `apply_boot_record_gate` が読む先＝`<mount.shiori.dir>/profile/areka/sylphya.toml`
/// （`shiori.dir == <root>/ghost/master`・resolve 規約 / `sylphya_wiring::profile_areka_root`）。
/// TOML スキーマは persist 層（`areka-sylphya/src/persist`）の `[boot] count` 写像に一致させる
/// （`PersistKey::BootCount` → 正準 key `areka.boot.count`・存在すれば `first_boot=false`）。
fn plant_boot_record(root: &std::path::Path) {
    let profile_areka = root
        .join("ghost")
        .join("master")
        .join("profile")
        .join("areka");
    std::fs::create_dir_all(&profile_areka).expect("create profile/areka");
    std::fs::write(
        profile_areka.join("sylphya.toml"),
        b"format-version = 1\n[boot]\ncount = \"1\"\n",
    )
    .expect("write sylphya.toml boot record");
}

/// 有界待機ヘルパ（S1 の `run_bounded` と同旨のローカルコピー・宙吊り防止）。
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

/// S7: 起動記録あり → OnFirstBoot を飛ばして OnBoot から起動し、`boot_gate skip_first_boot` を
/// ログする（要件 3.3・注入時刻のみ・sleep 不使用・純 x64）。
#[test]
fn s7_record_present_skips_onfirstboot_and_logs_skip_first_boot() {
    const SHELL_NAME: &str = "S7BootShell";

    // 全スレッド横断のログ捕捉を常駐させ、boot 前のバッファ長を基準点にする（以後の追加分のみ照合）。
    let log_buffer = super::global_log_probe::install();
    let base_len = log_buffer
        .lock()
        .expect("log buffer mutex は毒化しない")
        .len();

    let ghost = TempGhost::new("record_present_skips_onfirstboot");
    write_ghost_fixture(&ghost.root, SHELL_NAME);
    // 起動記録を据える——これが S1（初回起動）との唯一の差＝2 回目起動相当（要件 3.3）。
    plant_boot_record(&ghost.root);

    // 2 回目起動の台本: OnFirstBoot は **据えない**（gate が飛ばすため一度も GET されない）。
    // 発火系列 = OnInitialize → username prefetch → OnBoot → basewareversion（prefetch 段は 3.5 で不変）。
    // 万一 gate が退行して OnFirstBoot が GET されると `ScriptedShioriBackend` が「no scripted
    // response」で panic し、挨拶 cue が発火せず下の `fired` 表明が有界時間内に赤くなる（退行を検出）。
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("username", Ok(None))
        .get("OnBoot", Ok(Some(r"\s[0]hello\e".to_string())))
        .notify("basewareversion", Ok(()))
        .notify("OnClose", Ok(()))
        .unload(Ok(ExitKind::Clean))
        .build();

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();

    let options = GhostBootOptions {
        ghost_root: ghost.root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Custom(Box::new(move || {
            Ok(Box::new(backend) as Box<dyn ShioriBackend>)
        })),
        sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
        system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options)
        .expect("boot should succeed for a resolvable ghost_root with a planted boot record");

    // OnBoot の挨拶が dispatcher の active slot に載って発火するまで、Tick 注入で橋渡し（S1 と同技法・
    // 単調増加する `now` の注入のみ・sleep 不使用・`yield_now` で他スレッドへ実行機会を譲る）。
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
        "S7: OnBoot 挨拶 cue が発火しない — 記録あり起動が OnBoot から始まっていない（\
         gate 退行で OnFirstBoot GET により backend が panic した可能性も含む）"
    );

    // ---- (1) OnFirstBoot GET 不発・OnBoot GET 発火（要件 3.3）----
    // 死活監視ノイズ（run_shiori_loop が各メッセージ冒頭で status() を確認）を除外して照合する。
    let calls = handle.calls();
    let calls_without_status: Vec<RecordedCall> = calls
        .lock()
        .expect("calls mutex poisoned")
        .iter()
        .filter(|c| !matches!(c, RecordedCall::Status))
        .cloned()
        .collect();
    let is_get = |c: &RecordedCall, id: &str| {
        matches!(c, RecordedCall::Get { id: got, .. } if got == id)
    };
    assert!(
        !calls_without_status.iter().any(|c| is_get(c, "OnFirstBoot")),
        "記録あり起動では OnFirstBoot GET を一度も発行しない（要件 3.3）: {calls_without_status:?}"
    );
    assert!(
        calls_without_status.iter().any(|c| is_get(c, "OnBoot")),
        "記録あり起動は OnBoot GET から起動運行を始める（要件 3.3）: {calls_without_status:?}"
    );

    // ---- (2) boot_gate skip_first_boot ログ（要件 3.3・kanade アクタースレッド発火）----
    // OnBoot 到達は skip 分岐の後段ゆえ挨拶発火時点で skip ログは既に積まれているはずだが、
    // スレッド境界の可視化ラグを吸収するため有界スピンで整定を待つ（sleep 不使用・yield のみ）。
    let deadline = std::time::Instant::now() + super::E2E_BOUND;
    let mut skip_logged = false;
    while std::time::Instant::now() < deadline {
        let hit = log_buffer.lock().expect("log buffer mutex は毒化しない")[base_len..]
            .iter()
            .any(|e| {
                e.target == "kanade"
                    && e.level == Level::INFO
                    && e.message.contains("boot_gate skip_first_boot")
            });
        if hit {
            skip_logged = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        skip_logged,
        "boot_gate skip_first_boot ログ（target=\"kanade\" INFO）が未検出 — 初回ゲートの \
         skip 分岐が発火していない（要件 3.3）"
    );

    // ---- 後片付け（正規 shutdown・S1 と同じ有界待機）----
    run_bounded(
        "shutdown after S7 second-boot talk completion",
        super::E2E_BOUND,
        move || {
            let result = runtime.shutdown(areka_kanade::CloseReason::System);
            assert!(
                result.is_ok(),
                "shutdown should return Ok(()) after S7 second-boot talk completes, got {result:?}"
            );
        },
    );
    // `ghost`（TempGhost）はここでスコープを抜け Drop ガードが temp-dir を除去する。
}
