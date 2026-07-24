//! env ゲート実 pasta 追験（task 4.8・design.md「real pasta 追験（env ゲート）」・
//! 要件 8.1/8.4）。
//!
//! 実 32bit helper＋実 pasta（emo2）DLL 越しに、ghost 結線層の `boot`
//! （`ShioriWiring::Helper`・`TickerMode::Real`）から起動〜正規終了までの一周が
//! 実運行として完走することを、`HOST32_PASTA_DLL` env が設定されている場合のみ
//! 追験する（要件 8.4: 常時 `cargo test --workspace`（x64）緑化の前提とせず、
//! x86/i686 成果物への常時依存を課さない・opt-in gate）。
//!
//! # env-gate（既存 host32 E2E／`crates/areka-kanade/tests/kanade/real_helper_test.rs`
//! の慣行の踏襲）
//! `HOST32_PASTA_DLL`（pasta DLL のフルパス）が未設定なら silent skip（`return`）。
//! 設定済みだが DLL 不在なら明示 fail（silent skip を認めない）。helper exe は
//! `HOST32_HELPER_EXE` 優先→ワークスペース
//! `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe` 探索。
//!
//! # ghost 結線モデルとの違い（kanade 自身の `real_helper_test.rs` との差分）
//! kanade 自身のテストは DLL の**既存の親 dir**をそのまま `load_dir` として使うが、
//! ghost の `ShioriMount.dir` は `ghost_root/ghost/master` に**固定**される
//! （`areka_parsers::package::resolve` の規約・`real_connect` の `load_dir` は
//! `shiori.dir` から直接取る）。そのため本テストは実 DLL をフィクスチャの
//! `ghost/master/` へコピーして持ち込む（DLL 側を動かすのではなく、フィクスチャ側を
//! DLL の所在へ合わせる、の逆）。
//!
//! # 検証水準（design.md 引用）
//! 「応答スクリプトの内容は実ブレイン依存ゆえ照合しない（完走のみ・既存
//! `real_helper_test` と同じ検証水準）」——本テストは OnBoot 一周で
//! `RecordingSink` に**何らかの**発火が記録されることのみを確認し、`shutdown`
//! が `Ok(())` を返すことをもって完走を確認する。

use std::path::PathBuf;
use std::time::{Duration, Instant};

use areka_ghost::ticker::TickerConfig;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_kanade::CloseReason;
use areka_parsers::charset::DefaultEncoding;

use crate::spine_e2e_test::RecordingSink;

/// OnBoot 一周が sink へ何らかの発火を記録するまでの有界待機上限（実プロセス・実
/// ブレインとの往復ゆえ、kanade 自身の `real_helper_test.rs` の生成物待機と同様に
/// 秒単位の余裕を持たせる）。
const OBSERVE_TIMEOUT: Duration = Duration::from_secs(60);

/// 有界待機の再試行間隔。`TickerMode::Real` が背後で実 wall-clock 駆動している
/// ため、この 1 テストに限りポーリング用の実 sleep は許容される（design「OnBoot
/// 一周（sink 発火の非空を有界待機で観測）」の文字どおりの意味）。決定論
/// spine e2e（S1〜S6・要件 7.4 の sleep 不使用）はこの limitation の対象外——
/// 本テストはそもそも決定論性を要求されない env ゲート追験である。
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// `shutdown()` 呼出自体を有界に観測する（`runtime.rs`／`spine_e2e_test.rs` 各所の
/// `run_bounded` と同旨のローカルコピー）。
const SHUTDOWN_BOUND: Duration = Duration::from_secs(30);

/// i686 helper 実行ファイルのパスを解決する
/// （`crates/areka-kanade/tests/kanade/real_helper_test.rs::resolve_helper_exe` を
/// almost 逐語的に踏襲——env `HOST32_HELPER_EXE` 最優先 → ワークスペース
/// `target/i686-pc-windows-msvc/{debug,release}/shiori-host32-helper.exe`）。
///
/// `CARGO_MANIFEST_DIR` はこのクレート自身（`crates/areka-ghost`）を指すが、
/// ワークスペースルートまでの hop 数（`crates/areka-ghost` → `crates/` →
/// ワークスペースルート）は kanade 側（`crates/areka-kanade` → 同じ 2 hop）と
/// 同一であるため、探索パスの組み立てはそのまま流用できる。
fn resolve_helper_exe() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("HOST32_HELPER_EXE") {
        let p = PathBuf::from(&explicit);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "HOST32_HELPER_EXE={explicit:?} が指すファイルが存在しません（i686 helper を先にビルド）"
        ));
    }

    // CARGO_MANIFEST_DIR = crates/areka-ghost → ワークスペースルート。
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // ワークスペースルート
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.clone());
    let target_base = workspace_root.join("target").join("i686-pc-windows-msvc");

    for profile in ["debug", "release"] {
        let candidate = target_base.join(profile).join("shiori-host32-helper.exe");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "i686 helper exe が見つかりません（探索先: {}\\{{debug,release}}\\shiori-host32-helper.exe）。\
         PowerShell で先に i686 helper をビルドしてください: \
         cargo build -p shiori-host32-helper --target i686-pc-windows-msvc",
        target_base.display()
    ))
}

/// このテスト専用の一意な一時ディレクトリ（`spine_e2e_test.rs` 各シナリオの流儀を踏襲）。
fn unique_temp_dir(tag: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("areka_ghost_real_pasta_tests_{tag}"));
    dir
}

/// `root` 直下に、実 pasta DLL を**物理的に持ち込んだ** `ghost/master/` を含む
/// 解決可能なゴーストツリーを構築する（`ShioriMount.dir` は常に
/// `ghost_root/ghost/master` に固定されるため——モジュール doc 参照）。
///
/// - `ghost/master/descript.txt` の `shiori,` 行はコピー後のファイル名で書く。
/// - `shell/master/descript.txt` は他 e2e フィクスチャと同じ最小形（`resolve_kanade_config`
///   の前提を満たすためだけの存在）。
fn write_real_pasta_ghost_fixture(root: &std::path::Path, dll_path: &std::path::Path) {
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");

    let dll_file_name = dll_path
        .file_name()
        .expect("HOST32_PASTA_DLL にファイル名がない")
        .to_string_lossy()
        .into_owned();
    std::fs::copy(dll_path, ghost_master.join(&dll_file_name))
        .expect("実 pasta DLL を ghost/master へコピーできませんでした");

    std::fs::write(
        ghost_master.join("descript.txt"),
        format!(
            "charset,UTF-8\nname,RealPastaTestGhost\nshiori,{dll_file_name}\n\
             seriko.defaultsurfacedirectoryname,master\n"
        ),
    )
    .expect("write ghost descript.txt");

    let shell_dir = root.join("shell").join("master");
    std::fs::create_dir_all(&shell_dir).expect("create shell/master");
    std::fs::write(
        shell_dir.join("descript.txt"),
        b"charset,UTF-8\nname,RealPastaTestShell\n",
    )
    .expect("write shell descript.txt");
}

/// env ゲート実 pasta 追験（design.md「real pasta 追験（env ゲート）」・要件 8.1/8.4）。
///
/// `HOST32_PASTA_DLL` 未設定なら silent skip（CI 必須ゲートにしない）。設定時は
/// 実 32bit helper＋実 pasta 越しに ghost 結線層の `boot`〜`shutdown` 一周を追験する。
#[test]
fn real_pasta_boot_observes_a_cue_then_shuts_down_cleanly() {
    // --- env `HOST32_PASTA_DLL` 未設定＝任意 gate ゆえ silent skip（既存 host32 E2E／
    //     kanade 自身の real_helper_test.rs と同じ語法）---
    let Ok(pasta_dll) = std::env::var("HOST32_PASTA_DLL") else {
        eprintln!(
            "HOST32_PASTA_DLL 未設定のため実 pasta 追験を skip（任意 gate・要件 8.1/8.4）。\
             実ブレイン越しの起動一周を追験するには env に pasta DLL のフルパスを設定してください。"
        );
        return;
    };

    // --- env 設定済みだが DLL 不在なら明示 fail（silent skip を認めない）---
    let dll_path = PathBuf::from(&pasta_dll);
    assert!(
        dll_path.is_file(),
        "HOST32_PASTA_DLL={pasta_dll:?} が指す DLL が存在しません（指定 DLL 不在は明示 fail）"
    );

    // helper 不在も接続失敗へ倒すのではなく、追験の前提として明示 fail させる
    // （`real_helper_test.rs` と同じ哲学: 「helper 不在も connect 失敗
    // （ShioriDown）へ倒すのではなく、追験の前提として明示 fail させる」）。
    let helper_exe = resolve_helper_exe().expect("i686 helper exe の解決に失敗");

    let root = unique_temp_dir("real_pasta_boot_observes_a_cue_then_shuts_down_cleanly");
    let _ = std::fs::remove_dir_all(&root);
    write_real_pasta_ghost_fixture(&root, &dll_path);

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

    let options = GhostBootOptions {
        ghost_root: root.clone(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::Helper { helper_exe },
        sinks: vec![Box::new(surface_sink), Box::new(text_sink)],
        system_vars: SystemVarWiring::Custom(crate::common::test_system_vars()),
        app_profile_dir: None,
        ticker: TickerMode::Real(TickerConfig::default()),
    };

    // 起動自体が失敗するのはここでは環境ゲート条件ではなく genuine failure
    // （DLL/helper の実在は既に確認済み——mount 解決も含め boot が Err を返すのは
    // フィクスチャ／環境の不備であり、gate の対象ではない）。
    let runtime =
        boot(options).expect("real pasta 越しの boot は DLL/helper 実在確認済みなら成功するはず");

    // --- OnBoot 一周で sink に何らかの発火が記録されるまでの有界待機 ---
    // `TickerMode::Real` が実 wall-clock で駆動しているため、ここでの実 sleep-based
    // ポーリングは design の「OnBoot 一周（sink 発火の非空を有界待機で観測）」が
    // 明示的に許容する（決定論 spine e2e とは異なりこのテストは要件 7.4 の対象外）。
    let deadline = Instant::now() + OBSERVE_TIMEOUT;
    let mut observed = false;
    while Instant::now() < deadline {
        let surface_non_empty = !surface_records
            .lock()
            .expect("surface records mutex poisoned")
            .is_empty();
        let text_non_empty = !text_records
            .lock()
            .expect("text records mutex poisoned")
            .is_empty();
        if surface_non_empty || text_non_empty {
            observed = true;
            break;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    assert!(
        observed,
        "real pasta 越しの OnBoot 一周が {OBSERVE_TIMEOUT:?} 以内に \
         RecordingSink へ何らの発火も記録しなかった（応答内容自体は照合しない・完走のみ）"
    );

    // --- 正規終了（shutdown）が完走すること（有界に観測・宙吊り防止） ---
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<Result<(), String>>(0);
    std::thread::spawn(move || {
        let result = runtime
            .shutdown(CloseReason::System)
            .map_err(|err| format!("{err:?}"));
        let _ = done_tx.send(result);
    });
    match done_rx.recv_timeout(SHUTDOWN_BOUND) {
        Ok(result) => assert!(
            result.is_ok(),
            "real pasta 越しの shutdown は Ok(()) を返すはず（起動から正規終了までの \
             一周完走）: {result:?}"
        ),
        Err(_) => panic!("shutdown が {SHUTDOWN_BOUND:?} 以内に完了しなかった（宙吊りの疑い）"),
    }

    let _ = std::fs::remove_dir_all(&root);
}
