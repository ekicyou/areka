//! ゴーストの標準のバルーンの指定の読み手のテスト（spec: areka-P0-ghost-standard-balloon・
//! 要件 1.1〜1.6・1.14・2.1・2.2・2.13・2.14・5.4・7.1・7.4・7.5）。
//!
//! 同梱の最初の 1 個（`install.txt` の無印 → `balloon0`）の、番号付きの場面だけを足す。
//! 無印だけ・無印と `balloon0` の両方・`install.txt` が無い・読めないは `catalog_tests.rs` が担う。
//! descript の 2 鍵の読み手（`standard_balloon_keys`）の場面もここに置く。
//! 一時フォルダは `temp-path-kit` の下（ワークツリーの `target\`・要件 7.7）。

use super::test_support::{hold_exclusive, put, put_ghost};
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

/// ゴーストの `ghost/master/descript.txt` に `descript` を置いて `standard_balloon_keys` を呼ぶ。
/// 記録は 0 件であること（要件 2.13）まで判定する。
fn keys_of(label: &str, descript: &str) -> StandardBalloonKeys {
    let tmp = TempPath::new(label);
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let ghost = put_ghost(&root, "g", descript);
    let mut keys = None;
    let events = capture(|| keys = Some(standard_balloon_keys(&ghost)));
    assert!(events.is_empty(), "記録は 0 件: {events:?}");
    keys.expect("呼ばれた")
}

fn keys(path: Option<&str>, balloon: Option<&str>) -> StandardBalloonKeys {
    StandardBalloonKeys {
        default_balloon_path: path.map(str::to_owned),
        balloon: balloon.map(str::to_owned),
    }
}

/// 2 鍵とも書かれている → 両方の値（前後の空白は落とし、中身は読み替えない・要件 2.1・7.4）。
/// `recommended.*` は引かない（要件 2.14）。
#[test]
fn standard_keys_reads_both_keys_verbatim() {
    let got = keys_of(
        "catalog-standard-both",
        "charset,UTF-8\ndefault.balloon.path, ../balloon/x \nballoon, しずく 縦 \n\
         recommended.balloon,rec\nrecommended.balloon.path,recpath\n",
    );
    assert_eq!(got, keys(Some("../balloon/x"), Some("しずく 縦")));
}

/// 片方だけ → 書かれた鍵だけ。`recommended.*` は書かれていない鍵の代わりにならない（要件 2.13・2.14）。
#[test]
fn standard_keys_reads_one_key_alone() {
    let path_only = keys_of(
        "catalog-standard-path-only",
        "charset,UTF-8\ndefault.balloon.path,claudia\nrecommended.balloon,rec\n",
    );
    assert_eq!(path_only, keys(Some("claudia"), None));
    let name_only = keys_of(
        "catalog-standard-name-only",
        "charset,UTF-8\nballoon,claudia\nrecommended.balloon.path,recpath\n",
    );
    assert_eq!(name_only, keys(None, Some("claudia")));
}

/// 鍵の大文字が混じっても読む（値の大文字はそのまま・要件 2.1）。
#[test]
fn standard_keys_match_keys_case_insensitively() {
    let got = keys_of(
        "catalog-standard-case",
        "charset,UTF-8\nDefault.Balloon.PATH,Claudia\nBALLOON,Vertical\n",
    );
    assert_eq!(got, keys(Some("Claudia"), Some("Vertical")));
}

/// 値が空・鍵が無い → 無し（要件 2.2・2.13）。
#[test]
fn standard_keys_treat_empty_values_as_absent() {
    let empty = keys_of(
        "catalog-standard-empty",
        "charset,UTF-8\ndefault.balloon.path,\nballoon, \n",
    );
    assert_eq!(empty, StandardBalloonKeys::default());
    let absent = keys_of("catalog-standard-absent", "charset,UTF-8\nname,G\n");
    assert_eq!(absent, StandardBalloonKeys::default());
}

/// descript.txt が読めない → 2 欄とも無しで、読めない記録がちょうど 1 件（要件 5.4）。
#[test]
fn standard_keys_unreadable_warns_exactly_once() {
    let tmp = TempPath::new("catalog-standard-unreadable");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let ghost = put_ghost(
        &root,
        "locked",
        "charset,UTF-8\ndefault.balloon.path,claudia\nballoon,claudia\n",
    );
    let _held = hold_exclusive(&ghost.join("ghost").join("master").join("descript.txt"));

    let mut got = None;
    let events = capture(|| got = Some(standard_balloon_keys(&ghost)));
    assert_eq!(got, Some(StandardBalloonKeys::default()));
    let names: Vec<_> = events.iter().map(|e| e.event.as_deref()).collect();
    assert_eq!(names, [Some("catalog_descript_unreadable")], "{events:?}");
    assert_eq!(events[0].level, tracing::Level::WARN);
}
