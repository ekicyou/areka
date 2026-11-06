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
//! `PASTA_DEBUG=0` で回す（9276 を取り合う）。同じ実行ファイルの実 pasta のテストは
//! [`real_pasta_turn`] で順に走る。
//!
//! # フィクスチャ（`tests/real_pasta_online.rs` と同じ形）
//! pasta は辞書（`dic/`）・`scripts/`・`pasta.toml` を `ghost/master` から読むので、DLL 単体の
//! フィクスチャでは応答できない。検体 `emo2` の使い捨ての複製（`sample-ghost-kit`・起動記録の
//! 無い新品）を根にし、その `ghost/master/pasta.dll` を `HOST32_PASTA_DLL` で上書きする。
//!
//! # 検証水準（design.md 引用）
//! 「応答スクリプトの内容は実ブレイン依存ゆえ照合しない（完走のみ・既存
//! `real_helper_test` と同じ検証水準）」——本テストは OnBoot 一周で
//! `RecordingSink` に**何らかの**発火が記録されることのみを確認し、`shutdown`
//! が `Ok(())` を返すことをもって完走を確認する。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_ghost::ticker::TickerConfig;
use areka_ghost::{GhostBootOptions, ShioriWiring, SystemVarWiring, TickerMode, boot};
use areka_parsers::charset::DefaultEncoding;
use areka_sakura::contract::TalkCue;
use sample_ghost_kit::SampleRoot;

use crate::common::{real_pasta_turn, resolve_helper_exe, run_bounded_shutdown};
use crate::spine_e2e_test::RecordingSink;

/// OnBoot 一周が sink へ何らかの発火を記録するまでの有界待機上限（実プロセス・実
/// ブレインとの往復ゆえ、kanade 自身の `real_helper_test.rs` の生成物待機と同様に
/// 秒単位の余裕を持たせる）。
pub(crate) const OBSERVE_TIMEOUT: Duration = Duration::from_secs(60);

/// 有界待機の再試行間隔。`TickerMode::Real` が背後で実 wall-clock 駆動している
/// ため、実 pasta の env ゲート追験に限りポーリング用の実 sleep は許容される（design「OnBoot
/// 一周（sink 発火の非空を有界待機で観測）」の文字どおりの意味）。決定論
/// spine e2e（S1〜S7・要件 7.4 の sleep 不使用）はこの limitation の対象外——
/// 本テストはそもそも決定論性を要求されない env ゲート追験である。
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// `shutdown()` 呼出自体を有界に観測する上限。
pub(crate) const SHUTDOWN_BOUND: Duration = Duration::from_secs(30);

/// 2 つの sink のどちらかに発火が記録されるまで [`OBSERVE_TIMEOUT`] を上限に実時計で待ち、
/// 見えたら `true` を返す（`snapshot_capture_test.rs` と分け合う 1 つの待ちの輪）。
pub(crate) fn wait_for_a_cue(
    surface: &Arc<Mutex<Vec<TalkCue>>>,
    text: &Arc<Mutex<Vec<TalkCue>>>,
) -> bool {
    let deadline = Instant::now() + OBSERVE_TIMEOUT;
    while Instant::now() < deadline {
        let surface_non_empty = !surface
            .lock()
            .expect("surface records mutex poisoned")
            .is_empty();
        let text_non_empty = !text.lock().expect("text records mutex poisoned").is_empty();
        if surface_non_empty || text_non_empty {
            return true;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    false
}

/// env ゲート実 pasta 追験（design.md「real pasta 追験（env ゲート）」・要件 8.1/8.4）。
///
/// `HOST32_PASTA_DLL` 未設定なら silent skip（CI 必須ゲートにしない）。設定時は
/// 実 32bit helper＋実 pasta 越しに ghost 結線層の `boot`〜`shutdown` 一周を追験する。
#[test]
fn real_pasta_boot_observes_a_cue_then_shuts_down_cleanly() {
    let _turn = real_pasta_turn();
    // --- env `HOST32_PASTA_DLL` 未設定＝任意 gate ゆえ silent skip（既存 host32 E2E／
    //     kanade 自身の real_helper_test.rs と同じ語法）---
    // 空・空白だけの値は未設定と同じに skip する（snapshot_capture_test と同じ扱い）。
    let Some(pasta_dll) = std::env::var("HOST32_PASTA_DLL")
        .ok()
        .filter(|v| !v.trim().is_empty())
    else {
        eprintln!(
            "HOST32_PASTA_DLL 未設定（または空）のため実 pasta 追験を skip（任意 gate・要件 8.1/8.4）。\
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

    // 起動記録の無い新品の emo2（辞書込み）に、指定の DLL を持ち込む。
    let sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let root = sample.folder().to_path_buf();
    std::fs::copy(
        &dll_path,
        root.join("ghost").join("master").join("pasta.dll"),
    )
    .expect("HOST32_PASTA_DLL を emo2 の ghost/master へ写せませんでした");

    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

    let options = GhostBootOptions {
        strict_judges: None,
        ghost_root: root,
        default_encoding: DefaultEncoding::Ansi,
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
    assert!(
        wait_for_a_cue(&surface_records, &text_records),
        "real pasta 越しの OnBoot 一周が {OBSERVE_TIMEOUT:?} 以内に \
         RecordingSink へ何らの発火も記録しなかった（応答内容自体は照合しない・完走のみ）"
    );

    // --- 正規終了（shutdown）が完走すること（有界に観測・宙吊り防止） ---
    run_bounded_shutdown("real pasta shutdown", SHUTDOWN_BOUND, runtime);
    drop(sample);
}
