//! 先送りした正典語彙が areka 側の本番コードに入り込んでいないことを固定するテストの、本体の片割れ
//! （要件 8.1／8.3／8.4／8.5、重なり順のグループ機構の要件 10.4／11.4、spec: placement-crate-split
//! 要件 5.2・5.3）。
//!
//! # 片割れの分担
//!
//! 配置の crate の片割れ（`crates/areka-placement/src/spawn_zorder_pair_deferred_tests.rs`）が、
//! 配置の本番ファイル 4 本・窓の印・配置の `src` の木を見る。ここは本体の 3 本（グループ機構が
//! `emo2_boot` に新設したもの）・帯の語の持ち主のファイル・本体の `src` の木を見る。wintf 側の
//! 本番ファイル 8 本は `crates/wintf/src/ecs/window/zorder_pair_deferred_vocabulary_tests.rs` が持つ。
//! 語の表と走査器は 2 つの片割れが共に使う `areka_placement::test_support::zorder_pair_deferred` に
//! 1 つだけあり、ここでは本体の実物で較正する。
//!
//! # 走査の範囲・語の表・行コメントを除いてから探す理由
//!
//! 配置の片割れの頭の説明のとおり。要件は「**本フィーチャーにおいて**足さない」ことを言うので、
//! 走査は 2 つの機構が全部を書いた本番ファイルに限る。常に手前の帯の語（`BAND_NEEDLES`）を
//! areka 側で持つのは受け口（`emo2_boot/window_state_cue.rs`）と取り出しの相
//! （`emo2_boot/frame/window_state.rs`）だけと決めてあるので、その 2 本に語が在ることを対照として主張する。

use std::path::PathBuf;

use areka_placement::test_support::zorder_pair_deferred::{
    BAND_NEEDLES, DEFERRED_NEEDLES, code_only, deferred_hits, rust_sources_under,
};

/// 重なり順のグループ機構（`areka-P0-scope-zorder-pinning`）が本体に新設した本番ファイル
/// （`CARGO_MANIFEST_DIR` からの相対）。下の
/// `the_scanned_roster_covers_every_zorder_production_source_in_this_crate` が
/// 「本体に実在する `zorder_` 系の本番ファイルが 1 本残らずここに載っている」ことを機械で見張る。
const PRODUCTION_FILES: [&str; 3] = [
    "src/emo2_boot/zorder_cue.rs",
    "src/emo2_boot/frame/zorder_drain.rs",
    "src/emo2_boot/frame/zorder_descript.rs",
];

/// 帯の語を持つ本体の本番ファイル（`stay-on-top-directives` の受け口と取り出しの相）。
const BAND_FILES: [&str; 2] = [
    "src/emo2_boot/window_state_cue.rs",
    "src/emo2_boot/frame/window_state.rs",
];

fn source_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_source(relative: &str) -> String {
    let path = source_path(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("本番ファイルを読めなかった（{}）: {e}", path.display()))
}

/// 先送り語彙は、本フィーチャーの本体の本番ファイルのコードに 1 つも無い。
#[test]
fn deferred_window_state_vocabulary_is_absent_from_this_features_areka_sources() {
    let mut scanned_files = 0usize;
    let mut scanned_code_bytes = 0usize;

    for relative in PRODUCTION_FILES {
        let raw = read_source(relative);
        assert!(
            raw.len() > 1_000,
            "{relative}: 本番ファイルにしては短すぎる（読み違えの疑い）: {} バイト",
            raw.len()
        );
        let code = code_only(&raw);
        assert!(
            !code.trim().is_empty(),
            "{relative}: コメントを落としたら本文が空になった（走査が空振りしている）"
        );

        let hits = deferred_hits(&code);
        assert!(
            hits.is_empty(),
            "{relative}: 先送りの語か帯の語が本番コードに現れている（帯の語を持つのは {BAND_FILES:?} だけ）: {hits:?}"
        );

        scanned_files += 1;
        scanned_code_bytes += code.len();
    }

    assert_eq!(
        scanned_files,
        PRODUCTION_FILES.len(),
        "対象ファイルを全部読めていない"
    );
    assert!(
        scanned_code_bytes > 10_000,
        "読んだコードが少なすぎる（走査が実体に届いていない）: {scanned_code_bytes} バイト"
    );
}

/// 走査の道具が実際に語を見つけられること、そしてコメントだけを落としていることを、本体の実物で固定する。
#[test]
fn the_areka_deferred_vocabulary_scan_can_actually_find_what_it_looks_for() {
    for (needle, why) in DEFERRED_NEEDLES.into_iter().chain(BAND_NEEDLES) {
        let sample = format!("let handle = SomeApi::{needle}(hwnd);");
        let hits = deferred_hits(&sample);
        assert!(
            hits.iter().any(|(n, _)| *n == needle),
            "語 `{needle}`（{why}）を検出できていない: {hits:?}"
        );
    }

    // 対照: 受け口の素の全文には最小化の語（読めない第 2 引数の例＝否定の説明）があり、
    // コードだけの本文には無い。
    let raw = read_source("src/emo2_boot/window_state_cue.rs");
    assert!(
        raw.to_lowercase().contains("minimize"),
        "前提が崩れている: 素の全文には最小化の語があるはず（別の対照へ差し替えること）"
    );
    let code = code_only(&raw);
    assert!(
        !code.to_lowercase().contains("minimize"),
        "最小化の語が説明文以外に現れている（要件 8.3）"
    );
    // 落とし過ぎていない（走査が中身を見ている）。
    assert!(
        code.contains("WindowStateDirective"),
        "コードだけの本文から受け口の実装が消えている"
    );
}

/// 帯の語を持つのは受け口と取り出しの相のファイルであり、そのコード（コメントを除く）で帯の語の
/// 走査が語を見つける（上の「無い」の主張の対照・持ち主が改名や移動をすれば赤くなる）。
#[test]
fn the_band_files_are_where_the_band_words_live() {
    for relative in BAND_FILES {
        let code = code_only(&read_source(relative));
        assert!(
            deferred_hits(&code)
                .iter()
                .any(|(n, _)| BAND_NEEDLES.iter().any(|(b, _)| b == n)),
            "{relative}: 帯の語が見つからない（帯の語の持ち主が動いた？）"
        );
        assert!(
            !PRODUCTION_FILES.contains(&relative),
            "{relative} が重なり順の機構の名簿に入っている"
        );
    }
}

/// 本体の `src` 配下の `.rs` を、`CARGO_MANIFEST_DIR` からの相対パス（`/` 区切り）で返す。
fn all_source_files() -> Vec<String> {
    rust_sources_under(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// 重なり順のグループ機構が全部を書いた本体の本番ファイル
/// ＝ファイル名が `zorder_` で始まり、テストでもテスト専用の道具立てでもないもの。
fn zorder_production_sources() -> Vec<String> {
    all_source_files()
        .into_iter()
        .filter(|relative| {
            let name = relative.rsplit('/').next().unwrap_or(relative);
            name.starts_with("zorder_")
                && !name.ends_with("_tests.rs")
                && !name.ends_with("_test_support.rs")
        })
        .collect()
}

/// 走査対象の一覧が、本体に実在する本番ファイルへ両方向で追随していることを固定する
/// （名簿がずれても「無い」の主張は緑のままなので、守りが静かに狭まらないように）。
#[test]
fn the_scanned_roster_covers_every_zorder_production_source_in_this_crate() {
    let actual = zorder_production_sources();

    // ① 道具の較正（本体の実物で）: 既知の正例が挙がり、木に在る既知の偽例
    //   （テスト・テスト専用の道具立て）は挙がらない。
    assert!(
        actual.contains(&"src/emo2_boot/zorder_cue.rs".to_string()),
        "実物の走査が効いていない（既知の本番ファイルを見つけられない）: {actual:?}"
    );
    let test_support = "src/emo2_boot/frame/zorder_drain_test_support.rs";
    assert!(
        all_source_files().contains(&test_support.to_string()),
        "較正の外れが失われた: {test_support} が木に無い（別の対照へ差し替えること）"
    );
    assert!(
        !actual.iter().any(|p| p == test_support),
        "テスト専用の道具立てを本番ファイルとして数えている: {actual:?}"
    );
    assert!(
        !actual.iter().any(|p| p.ends_with("_tests.rs")),
        "テストファイルを本番ファイルとして数えている: {actual:?}"
    );

    // ② 実物が名簿から漏れていない。
    let missing: Vec<&String> = actual
        .iter()
        .filter(|relative| !PRODUCTION_FILES.contains(&relative.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "本番ファイルが先送り語彙の走査対象に載っていない（名簿へ足すこと）: {missing:?}"
    );

    // ③ 名簿の項目が実在する（改名・移動で名簿だけが取り残されない）。
    for relative in PRODUCTION_FILES {
        assert!(
            source_path(relative).is_file(),
            "走査対象の一覧に実在しないファイルが載っている: {relative}"
        );
    }
}
