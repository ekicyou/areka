//! `parse`/`parse_str` 公開 facade の単体テスト（タスク 2.2・2 層マージ＋KV→型写像）。
//!
//! 検証対象は公開エントリ `areka_parsers::balloon::{parse, parse_str}`:
//! - 2 層マージの優先度（画像別優先・R3.2／画像別欠落時 descript 継承・R3.3／descript のみ・R3.5）。
//! - 負値保持（`validrect.bottom,-56` → `Some(-56)`、`wordwrappoint.x,-34` → `Some(-34)`・R4.1）。
//! - 非負保持（R4.5）。ピクセル解決は行わない（R4.4）。
//! - 寛容（未知キー無視・R1.3／非数値 → `None`・R1.4／空入力 → 全 `None`・R1.5）。
//! - 非モデル化 distractor キー（`anchor.font.color.r` 等）が正規キーへ漏れないこと（R2.7）。
//! - RGB 部分欠落が個別 `None`（`font.color.r` のみ存在・R2.6）。

use super::parse::{parse, parse_str};
use std::collections::BTreeMap;

/// テスト用: `&[(k, v)]` からフラット KV `BTreeMap` を組む小ヘルパ。
fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// 空 descript ＋画像別層 None → 全スカラ `None`（R1.5/R3.5・空入力）。
#[test]
fn empty_descript_yields_all_none() {
    let got = parse(&BTreeMap::new(), None);

    assert_eq!(got.windowposition().x(), None);
    assert_eq!(got.windowposition().y(), None);
    assert_eq!(got.origin().x(), None);
    assert_eq!(got.origin().y(), None);
    assert_eq!(got.wordwrappoint().x(), None);
    assert_eq!(got.wordwrappoint().y(), None);
    assert_eq!(got.validrect().top(), None);
    assert_eq!(got.validrect().bottom(), None);
    assert_eq!(got.validrect().left(), None);
    assert_eq!(got.validrect().right(), None);
    assert_eq!(got.font().name(), None);
    assert_eq!(got.font().height(), None);
    assert_eq!(got.font().color().r(), None);
    assert_eq!(got.font().color().g(), None);
    assert_eq!(got.font().color().b(), None);
}

/// descript のみ（画像別層 None）→ descript 値を反映し画像別由来値は付かない（R3.5）。
#[test]
fn descript_only_reflects_descript_values() {
    let descript = map(&[
        ("origin.x", "0"),
        ("origin.y", "0"),
        ("wordwrappoint.x", "-34"),
        ("font.name", "Yu Gothic UI"),
        ("font.height", "28"),
        ("font.color.r", "0"),
        ("font.color.g", "0"),
        ("font.color.b", "0"),
    ]);

    let got = parse(&descript, None);

    assert_eq!(got.origin().x(), Some(0));
    assert_eq!(got.origin().y(), Some(0));
    assert_eq!(got.wordwrappoint().x(), Some(-34));
    assert_eq!(got.font().name(), Some("Yu Gothic UI"));
    assert_eq!(got.font().height(), Some(28));
    assert_eq!(got.font().color().r(), Some(0));
    // descript に無いキー由来のスカラは None（R3.4/R3.5）。
    assert_eq!(got.windowposition().x(), None);
    assert_eq!(got.windowposition().y(), None);
    assert_eq!(got.validrect().top(), None);
}

/// 画像別層が同一キーを上書き（後勝ち・画像別優先・R3.2）。
#[test]
fn image_layer_overrides_same_key() {
    let descript = map(&[("windowposition.x", "10"), ("windowposition.y", "20")]);
    let image = map(&[("windowposition.x", "266"), ("windowposition.y", "-129")]);

    let got = parse(&descript, Some(&image));

    // 同一キーは画像別層が勝つ（R3.2）。
    assert_eq!(got.windowposition().x(), Some(266));
    assert_eq!(got.windowposition().y(), Some(-129));
}

/// 画像別層に無いキーは descript を継承（R3.3）。
#[test]
fn image_missing_key_inherits_descript() {
    let descript = map(&[("wordwrappoint.x", "-34"), ("font.height", "28")]);
    // 画像別層は windowposition のみ持ち、wordwrappoint/font は持たない。
    let image = map(&[("windowposition.x", "266")]);

    let got = parse(&descript, Some(&image));

    // 画像別上書き。
    assert_eq!(got.windowposition().x(), Some(266));
    // 画像別に無いキーは descript 継承（R3.3）。
    assert_eq!(got.wordwrappoint().x(), Some(-34));
    assert_eq!(got.font().height(), Some(28));
}

/// 負値は符号付きのまま保持しピクセル解決しない（R4.1/R4.4）。
#[test]
fn negative_values_preserved_signed() {
    let descript = map(&[
        ("validrect.bottom", "-56"),
        ("wordwrappoint.x", "-34"),
        ("windowposition.y", "-129"),
    ]);

    let got = parse(&descript, None);

    assert_eq!(got.validrect().bottom(), Some(-56));
    assert_eq!(got.wordwrappoint().x(), Some(-34));
    assert_eq!(got.windowposition().y(), Some(-129));
}

/// 非負値はそのまま保持（R4.5）。
#[test]
fn non_negative_values_preserved() {
    let descript = map(&[
        ("validrect.top", "46"),
        ("validrect.left", "36"),
        ("font.height", "28"),
        ("font.color.g", "128"),
    ]);

    let got = parse(&descript, None);

    assert_eq!(got.validrect().top(), Some(46));
    assert_eq!(got.validrect().left(), Some(36));
    assert_eq!(got.font().height(), Some(28));
    assert_eq!(got.font().color().g(), Some(128));
}

/// 非数値値・範囲外は当該スカラを `None` へ降格し継続（R1.4）。
#[test]
fn non_numeric_and_out_of_range_demote_to_none() {
    let descript = map(&[
        ("validrect.top", "abc"),      // 非数値 → None
        ("font.height", "-5"),         // u32 に負値 → parse Err → None
        ("font.color.r", "300"),       // u8 範囲外 → parse Err → None
        ("origin.x", "12"),            // 正常キーは継続して反映
    ]);

    let got = parse(&descript, None);

    assert_eq!(got.validrect().top(), None);
    assert_eq!(got.font().height(), None);
    assert_eq!(got.font().color().r(), None);
    // 不能キーがあっても他キーは反映される（局所降格・全域継続・R1.4）。
    assert_eq!(got.origin().x(), Some(12));
}

/// 未知キー・非モデル化 distractor キーは正規キーへ漏れず無視される（R1.3/R2.7）。
/// `anchor.font.color.r`・`number.font.height`・`sstpmessage.font.height`・`cursor.font.color.r`
/// は正規の `font.color.r`／`font.height` を汚染しない（完全一致キー引きゆえ）。
#[test]
fn distractor_keys_do_not_leak_into_modeled_scalars() {
    let descript = map(&[
        ("anchor.font.color.r", "180"),
        ("number.font.height", "24"),
        ("sstpmessage.font.height", "22"),
        ("cursor.font.color.r", "99"),
        ("arrow.x", "5"),
        ("onlinemarker.file", "online.png"),
        ("unknown.key.here", "42"),
    ]);

    let got = parse(&descript, None);

    // どの distractor も正規スカラへ漏れない（R2.7）。
    assert_eq!(got.font().color().r(), None);
    assert_eq!(got.font().height(), None);
}

/// RGB 部分欠落は個別 `None`（`font.color.r` のみ存在 → r=Some, g/b=None・R2.6）。
#[test]
fn rgb_partial_absence_is_independently_none() {
    let descript = map(&[("font.color.r", "12")]);

    let got = parse(&descript, None);

    assert_eq!(got.font().color().r(), Some(12));
    assert_eq!(got.font().color().g(), None);
    assert_eq!(got.font().color().b(), None);
}

/// `parse_str` はデコード済み文字列 2 層を内部で KV 化して `parse` へ委譲する（R1.1/R1.5）。
/// 画像別層文字列は同一キーを上書きする。
#[test]
fn parse_str_decodes_and_merges_two_layers() {
    let descript = "windowposition.x,10\nwordwrappoint.x,-34\nfont.height,28";
    let image = "windowposition.x,266";

    let got = parse_str(descript, Some(image));

    // 画像別文字列層が上書き（R3.2）。
    assert_eq!(got.windowposition().x(), Some(266));
    // 画像別に無いキーは descript 継承（R3.3）。
    assert_eq!(got.wordwrappoint().x(), Some(-34));
    assert_eq!(got.font().height(), Some(28));
}

/// `parse_str` の画像別層 None は descript 文字列のみを写像する（R3.5）。
#[test]
fn parse_str_descript_only() {
    let got = parse_str("validrect.bottom,-56", None);

    assert_eq!(got.validrect().bottom(), Some(-56));
    assert_eq!(got.windowposition().x(), None);
}
