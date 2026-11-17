//! 先送りした正典語彙が areka 側の本番コードに入り込んでいないことを固定するテストの、`areka-runtime` の片割れ
//! （要件 8.1／8.3／8.4／8.5、重なり順のグループ機構の要件 10.4／11.4、spec: placement-crate-split
//! 要件 5.2・5.3）。
//!
//! # 片割れの分担
//!
//! 配置の crate の片割れ（`crates/areka-placement/src/spawn_zorder_pair_deferred_tests.rs`）が、
//! 配置の本番ファイル 4 本・窓の印・配置の `src` の木を見る。ここは`areka-runtime` の 3 本（グループ機構が
//! 新設したもの）・帯の語の持ち主のファイル・`areka-runtime` の `src` の木を見る。wintf 側の
//! 本番ファイル 8 本は `crates/wintf/src/ecs/window/zorder_pair_deferred_vocabulary_tests.rs` が持つ。
//! 語の表と走査器は 2 つの片割れが共に使う `areka_placement::test_support::zorder_pair_deferred` に
//! 1 つだけあり、ここでは`areka-runtime` の実物で較正する。
//!
//! # 走査の範囲・語の表・行コメントを除いてから探す理由
//!
//! 配置の片割れの頭の説明のとおり。重なり順の 2 機構（ペア機構＝`areka-P0-ghost-window-zorder`・
//! グループ機構＝`areka-P0-scope-zorder-pinning`）の要件は「**本フィーチャーにおいて**足さない」
//! ことを言う（要件の文の引用）ので、走査は 2 つの機構が全部を書いた本番ファイルに限る。常に手前の帯の語（`BAND_NEEDLES`）を
//! areka 側で持つのは受け口（`src/window_state_cue.rs`）と取り出しの相
//! （`src/frame/window_state.rs`）だけと決めてあるので、その 2 本に語が在ることを対照として主張する。

use std::path::Path;

use areka_placement::test_support::zorder_pair_deferred::{
    BAND_NEEDLES, assert_needles_are_detected, assert_roster_covers, assert_vocabulary_absent,
    code_only, deferred_hits, read_source, rust_sources_under, zorder_production_sources,
};

/// 重なり順のグループ機構（`areka-P0-scope-zorder-pinning`）が`areka-runtime` に新設した本番ファイル
/// （`CARGO_MANIFEST_DIR` からの相対）。下の
/// `the_scanned_roster_covers_every_zorder_production_source_in_this_crate` が
/// 「`areka-runtime` に実在する `zorder_` 系の本番ファイルが 1 本残らずここに載っている」ことを機械で見張る。
const PRODUCTION_FILES: [&str; 3] = [
    "src/zorder_cue.rs",
    "src/frame/zorder_drain.rs",
    "src/frame/zorder_descript.rs",
];

/// 帯の語を持つ`areka-runtime` の本番ファイル（`stay-on-top-directives` の受け口と取り出しの相）。
const BAND_FILES: [&str; 2] = ["src/window_state_cue.rs", "src/frame/window_state.rs"];

/// 先送り語彙は、重なり順の 2 機構が書いた`areka-runtime` の本番ファイルのコードに 1 つも無い。
#[test]
fn deferred_window_state_vocabulary_is_absent_from_this_features_areka_sources() {
    assert_vocabulary_absent(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &PRODUCTION_FILES,
        &format!("{BAND_FILES:?}"),
    );
}

/// 走査の道具が実際に語を見つけられること、そしてコメントだけを落としていることを、`areka-runtime` の実物で固定する。
#[test]
fn the_areka_deferred_vocabulary_scan_can_actually_find_what_it_looks_for() {
    assert_needles_are_detected();

    // 対照: 受け口の素の全文には最小化の語（読めない第 2 引数の例＝否定の説明）があり、
    // コードだけの本文には無い。
    let raw = read_source(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "src/window_state_cue.rs",
    );
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
        let code = code_only(&read_source(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            relative,
        ));
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

/// 走査対象の一覧が、`areka-runtime` に実在する本番ファイルへ両方向で追随していることを固定する
/// （名簿がずれても「無い」の主張は緑のままなので、守りが静かに狭まらないように）。
#[test]
fn the_scanned_roster_covers_every_zorder_production_source_in_this_crate() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let actual = zorder_production_sources(manifest);

    // ① 道具の較正（`areka-runtime` の実物で）: 既知の正例が挙がり、木に在る既知の偽例
    //   （テスト・テスト専用の道具立て）は挙がらない。
    assert!(
        actual.contains(&"src/zorder_cue.rs".to_string()),
        "実物の走査が効いていない（既知の本番ファイルを見つけられない）: {actual:?}"
    );
    let test_support = "src/frame/zorder_drain_test_support.rs";
    assert!(
        rust_sources_under(manifest).contains(&test_support.to_string()),
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

    // ②③ 実物が名簿から漏れず、名簿の項目が実在する（共有の支えが両方向を見る）。
    assert_roster_covers(manifest, &PRODUCTION_FILES, &actual);
}
