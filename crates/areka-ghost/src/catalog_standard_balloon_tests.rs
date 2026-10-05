//! ゴーストの標準のバルーンの指定の読み手のテスト（spec: areka-P0-ghost-standard-balloon・
//! 要件 1.1〜1.6・1.14・7.1・7.5）。
//!
//! 同梱の最初の 1 個（`install.txt` の無印 → `balloon0`）の、番号付きの場面だけを足す。
//! 無印だけ・無印と `balloon0` の両方・`install.txt` が無い・読めないは `catalog_tests.rs` が担う。
//! 一時フォルダは `temp-path-kit` の下（ワークツリーの `target\`・要件 7.7）。

use super::test_support::put;
use super::*;
use crate::test_log_capture::capture;
use temp_path_kit::TempPath;

/// `install.txt` に `body` を置いたゴーストで `companion_balloon` を呼び、戻り値を返す。
/// 記録は 0 件であること（要件 1.5・1.14）まで判定する。
fn companion_of(label: &str, body: &str) -> Option<String> {
    let tmp = TempPath::new(label);
    let ghost = tmp.path().join("g");
    put(&ghost.join("install.txt"), body.as_bytes());
    let mut companion = Some("<未呼出>".to_owned());
    let events = capture(|| companion = companion_balloon(&ghost));
    assert!(events.is_empty(), "記録は 0 件: {events:?}");
    companion
}

/// 番号付きだけ → 最初の 1 個は `balloon0`（要件 1.1・1.4・1.10・7.1）。
#[test]
fn numbered_only_yields_balloon0() {
    let got = companion_of(
        "catalog-companion-numbered",
        "charset,UTF-8\nballoon0.directory,claudia\nballoon1.directory,claudia_vertical\n",
    );
    assert_eq!(got.as_deref(), Some("claudia"));
}

/// `balloon0` が欠番で `balloon1` だけ → 無し（探索は最初に無い番号で止まる・要件 1.5・7.1）。
#[test]
fn balloon1_only_yields_none() {
    let got = companion_of(
        "catalog-companion-gap",
        "charset,UTF-8\nballoon1.directory,claudia_vertical\n",
    );
    assert_eq!(got, None);
}

/// 先頭に 0 を付けた綴りは番号に数えない → 無し（要件 1.6・7.1）。
#[test]
fn zero_padded_number_yields_none() {
    let got = companion_of(
        "catalog-companion-padded",
        "charset,UTF-8\nballoon00.directory,claudia\n",
    );
    assert_eq!(got, None);
}

/// `*.directory` の行が無く同じ接頭辞の他の鍵だけ → 無し（要件 1.2・7.1）。
#[test]
fn other_keys_with_same_prefix_yield_none() {
    let got = companion_of(
        "catalog-companion-other-keys",
        "charset,UTF-8\nballoon.source.directory,claudia\nballoon0.source.directory,claudia\n",
    );
    assert_eq!(got, None);
}

/// 空の値の無印も「見つかった」に数え、`balloon0` へ進まない（要件 1.14・7.5）。
#[test]
fn empty_unnumbered_stops_before_balloon0() {
    let got = companion_of(
        "catalog-companion-empty-plain",
        "charset,UTF-8\nballoon.directory,\nballoon0.directory,claudia\n",
    );
    assert_eq!(got, None);
}

/// 空の値の `balloon0` も「見つかった」に数え、`balloon1` へ繰り下げない（要件 1.14・7.5）。
#[test]
fn empty_balloon0_stops_before_balloon1() {
    let got = companion_of(
        "catalog-companion-empty-zero",
        "charset,UTF-8\nballoon0.directory, \nballoon1.directory,claudia_vertical\n",
    );
    assert_eq!(got, None);
}
