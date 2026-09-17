//! `parse`/`parse_str` 公開 facade の単体テスト（タスク 2.2・2 層マージ＋KV→型写像）。
//!
//! 検証対象は公開エントリ `areka_parsers::balloon::{parse, parse_str}`:
//! - 2 層マージの優先度（画像別優先・R3.2／画像別欠落時 descript 継承・R3.3／descript のみ・R3.5）。
//! - 負値保持（`validrect.bottom,-56` → `Some(-56)`、`wordwrappoint.x,-34` → `Some(-34)`・R4.1）。
//! - 非負保持（R4.5）。ピクセル解決は行わない（R4.4）。
//! - 寛容（未知キー無視・R1.3／非数値 → `None`・R1.4／空入力 → 全 `None`・R1.5）。
//! - 非モデル化 distractor キー（`anchor.font.color.r` 等）が正規キーへ漏れないこと（R2.7）。
//! - RGB 部分欠落が個別 `None`（`font.color.r` のみ存在・R2.6）。
//! - `writing_mode` の生文字列転記 4 パターン（単層／画像別上書き後勝ち／未指定＝`None`／
//!   未知値素通し・解釈しない・emo-text-layer 要件 5.6）。
//! - SSP 正典キー `vertical` の生文字列転記 4 形（単層／面別上書き層の後勝ち／未指定＝`None`
//!   かつ宣言された `0` と区別／語彙外値素通し・balloon-vertical-canon 要件 1.4/1.5/10.4）。

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
        ("validrect.top", "abc"), // 非数値 → None
        ("font.height", "-5"),    // u32 に負値 → parse Err → None
        ("font.color.r", "300"),  // u8 範囲外 → parse Err → None
        ("origin.x", "12"),       // 正常キーは継続して反映
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

/// `windowposition.x`/`.limit` の生値転記: 2 層マージ後に勝った値が取れる（要件 1.1/4.1・C2）。
///
/// 画像別層が同一キーを後勝ちで上書きし、その勝った生文字列がそのまま取れる。
/// 併せて既存の数値アクセサが無改変であること（`center` は `i32` パース不能ゆえ従来どおり
/// `None` へ降格）も固定する（要件 5.1 の bit 同一）。
#[test]
fn windowposition_raw_two_layer_merge_takes_winner() {
    let descript = map(&[("windowposition.x", "10"), ("windowposition.limit", "1")]);
    let image = map(&[
        ("windowposition.x", "center"),
        ("windowposition.limit", "0"),
    ]);

    let got = parse(&descript, Some(&image));

    // 生値は画像別層が勝つ（後勝ち・要件 1.1/4.1）。
    assert_eq!(got.windowposition_raw().x_raw(), Some("center"));
    assert_eq!(got.windowposition_raw().limit_raw(), Some("0"));
    // 既存の数値アクセサは無改変: `center` は非数値ゆえ従来どおり None（要件 5.1）。
    assert_eq!(got.windowposition().x(), None);
    assert_eq!(got.windowposition().y(), None);
}

/// 画像別層に無い `windowposition.*` は descript 基層を継承する（要件 1.1/4.1・R3.3 と同型）。
#[test]
fn windowposition_raw_image_missing_key_inherits_descript() {
    let descript = map(&[
        ("windowposition.x", "bottom"),
        ("windowposition.limit", "0"),
    ]);
    // 画像別層は windowposition.* を持たない。
    let image = map(&[("font.height", "28")]);

    let got = parse(&descript, Some(&image));

    assert_eq!(got.windowposition_raw().x_raw(), Some("bottom"));
    assert_eq!(got.windowposition_raw().limit_raw(), Some("0"));
}

/// `windowposition.x`/`.limit` 未指定 → いずれも「値なし」（`None`）（要件 1.1/4.1）。
#[test]
fn windowposition_raw_unspecified_is_none() {
    let descript = map(&[("origin.x", "12")]);

    let got = parse(&descript, None);

    assert_eq!(got.windowposition_raw().x_raw(), None);
    assert_eq!(got.windowposition_raw().limit_raw(), None);
}

/// 数値指定も生文字列としてそのまま転記され、既存の数値アクセサと併存する（要件 4.1/5.1）。
#[test]
fn windowposition_raw_transcribes_numeric_verbatim() {
    let descript = map(&[
        ("windowposition.x", "266"),
        ("windowposition.y", "-129"),
        ("windowposition.limit", "1"),
    ]);

    let got = parse(&descript, None);

    // 生値は解釈せず文字列のまま。
    assert_eq!(got.windowposition_raw().x_raw(), Some("266"));
    assert_eq!(got.windowposition_raw().limit_raw(), Some("1"));
    // 既存の数値経路は無改変（要件 5.1）。
    assert_eq!(got.windowposition().x(), Some(266));
    assert_eq!(got.windowposition().y(), Some(-129));
}

/// 語彙外の値も解釈せず・警告せず素通しで転記される（転記層契約・要件 1.1/4.1）。
///
/// 0/1 検証・キーワード判別・警告縮退は下流（placement 側の分類純関数）の責務であり、
/// parser はここで判断しない（steering「parser は転記層・解釈は下流」）。
#[test]
fn windowposition_raw_out_of_vocabulary_values_pass_through() {
    let descript = map(&[
        ("windowposition.x", "diagonal"),
        ("windowposition.limit", "2"),
    ]);

    let got = parse(&descript, None);

    assert_eq!(got.windowposition_raw().x_raw(), Some("diagonal"));
    assert_eq!(got.windowposition_raw().limit_raw(), Some("2"));
    // 非数値 x は既存どおり数値スカラでは None（要件 5.1）。
    assert_eq!(got.windowposition().x(), None);
}

/// 前後空白は kv 層の trim で既に除かれており、転記層は追加の整形をしない（C2 前提）。
#[test]
fn windowposition_raw_relies_on_kv_layer_trim() {
    let got = parse_str(
        "windowposition.limit,  0  \nwindowposition.x,  center  ",
        None,
    );

    assert_eq!(got.windowposition_raw().limit_raw(), Some("0"));
    assert_eq!(got.windowposition_raw().x_raw(), Some("center"));
}

/// `writing_mode` 単層指定（descript のみ）→ 生文字列がそのまま転記される（要件 5.6）。
#[test]
fn writing_mode_descript_single_layer_transcribed() {
    let descript = map(&[("writing_mode", "vertical_rl")]);

    let got = parse(&descript, None);

    assert_eq!(got.writing_mode(), Some("vertical_rl"));
}

/// `writing_mode` 画像別上書き（後勝ち）→ 画像別層の値が descript 基層を上書きする（要件 5.6）。
#[test]
fn writing_mode_image_layer_overrides_descript() {
    let descript = map(&[("writing_mode", "horizontal_tb")]);
    let image = map(&[("writing_mode", "vertical_rl")]);

    let got = parse(&descript, Some(&image));

    assert_eq!(got.writing_mode(), Some("vertical_rl"));
}

/// `writing_mode` 未指定 → `None`（転記フィールドの不在表現・要件 5.6）。
#[test]
fn writing_mode_unspecified_is_none() {
    let descript = map(&[("origin.x", "12")]);

    let got = parse(&descript, None);

    assert_eq!(got.writing_mode(), None);
}

/// `writing_mode` 未知値も素通しで転記される（値の検証・語彙判定・fallback は下流責務・
/// parser は転記に徹する・要件 5.6）。
#[test]
fn writing_mode_unknown_value_passes_through_raw() {
    let descript = map(&[("writing_mode", "diagonal_bt")]);

    let got = parse(&descript, None);

    // 未知語彙でも解釈せず生文字列のまま転記する。
    assert_eq!(got.writing_mode(), Some("diagonal_bt"));
}

/// `vertical` 単層指定（descript のみ）→ 生文字列がそのまま転記される（要件 1.4）。
#[test]
fn vertical_descript_single_layer_transcribed() {
    let descript = map(&[("vertical", "1")]);

    let got = parse(&descript, None);

    assert_eq!(got.vertical_raw(), Some("1"));
}

/// `vertical` 面別上書き層の後勝ち → 画像別層の値が descript 基層を上書きする（要件 1.5/10.4）。
///
/// 2 層マージはキー非依存の既存実装であり、`vertical` のためのマージ改変は 0 行である
/// （＝追加コード 0 で後勝ちが成立することの証跡）。
#[test]
fn vertical_image_layer_overrides_descript() {
    let descript = map(&[("vertical", "0")]);
    let image = map(&[("vertical", "1")]);

    let got = parse(&descript, Some(&image));

    assert_eq!(got.vertical_raw(), Some("1"));
}

/// `vertical` 未指定 → `None`。宣言された `0` とは区別して保持する（要件 1.4）。
///
/// 「未宣言」と「`0` の宣言」を潰さないことが共存規則（下流の書字方向の解決）の前提であり、
/// 両者を同一のモデル上で対比して固定する。
#[test]
fn vertical_unspecified_is_none_and_distinct_from_declared_zero() {
    let unspecified = parse(&map(&[("origin.x", "12")]), None);
    let declared_zero = parse(&map(&[("vertical", "0")]), None);

    assert_eq!(unspecified.vertical_raw(), None);
    // 宣言された `0` は `None` へ潰れない（未宣言との区別が保たれる）。
    assert_eq!(declared_zero.vertical_raw(), Some("0"));
}

/// `vertical` の語彙外値も解釈せず・警告せず素通しで転記される（転記層の無警告契約・要件 1.4）。
///
/// `0`/`1` の検証・語彙外値の警告付き縮退は下流（書字方向の解決層）の責務であり、
/// parser はここで判断しない（steering「parser は転記層・解釈は下流」）。
#[test]
fn vertical_out_of_vocabulary_value_passes_through_raw() {
    let got_numeric = parse(&map(&[("vertical", "2")]), None);
    let got_word = parse(&map(&[("vertical", "true")]), None);

    assert_eq!(got_numeric.vertical_raw(), Some("2"));
    assert_eq!(got_word.vertical_raw(), Some("true"));
}

/// T1: 書体の飾り 5 本はそれぞれ宣言値がそのまま読める（balloon-font-descript-keys 要件 2.1/9.1）。
///
/// キーごとに互いに異なる値を入れ、取り違え（別キーへの写像）も赤にする。
#[test]
fn font_decoration_raw_five_keys_transcribed_verbatim() {
    let got = parse_str(
        "font.bold,1\nfont.italic,0\nfont.outline,11\nfont.strike,12\nfont.underline,13",
        None,
    );

    let d = got.font_decoration_raw();
    assert_eq!(d.bold(), Some("1"));
    assert_eq!(d.italic(), Some("0"));
    assert_eq!(d.outline(), Some("11"));
    assert_eq!(d.strike(), Some("12"));
    assert_eq!(d.underline(), Some("13"));
}

/// T2: 影色 3 成分＋形態はそれぞれ宣言値がそのまま読める（要件 2.2/2.3/9.1）。
#[test]
fn font_shadow_raw_four_keys_transcribed_verbatim() {
    let descript = map(&[
        ("font.shadowcolor.r", "10"),
        ("font.shadowcolor.g", "20"),
        ("font.shadowcolor.b", "30"),
        ("font.shadowstyle", "offset"),
    ]);

    let got = parse(&descript, None);

    let s = got.font_shadow_raw();
    assert_eq!(s.color_r(), Some("10"));
    assert_eq!(s.color_g(), Some("20"));
    assert_eq!(s.color_b(), Some("30"));
    assert_eq!(s.style(), Some("offset"));
}

/// T3: 未指定は 9 本すべて `None`。宣言された `0` は `Some("0")` で未指定と区別される
/// （要件 2.4/3.1/9.2・既定値を代入しない）。
#[test]
fn font_base_keys_unspecified_are_none_and_distinct_from_declared_zero() {
    let unspecified = parse(&map(&[("origin.x", "12")]), None);

    let d = unspecified.font_decoration_raw();
    assert_eq!(d.bold(), None);
    assert_eq!(d.italic(), None);
    assert_eq!(d.outline(), None);
    assert_eq!(d.strike(), None);
    assert_eq!(d.underline(), None);
    let s = unspecified.font_shadow_raw();
    assert_eq!(s.color_r(), None);
    assert_eq!(s.color_g(), None);
    assert_eq!(s.color_b(), None);
    assert_eq!(s.style(), None);

    let declared_zero = parse(
        &map(&[("font.bold", "0"), ("font.shadowcolor.r", "0")]),
        None,
    );
    assert_eq!(declared_zero.font_decoration_raw().bold(), Some("0"));
    assert_eq!(declared_zero.font_shadow_raw().color_r(), Some("0"));
}

/// T4: 正典の語彙に無い値も落とさず解釈せず素通しする（要件 2.5/9.3）。
///
/// `font.shadowcolor.r,300` は `u8` 範囲外だが、既存の `font.color.r` と違い `None` へ降格しない。
#[test]
fn font_base_keys_out_of_vocabulary_values_pass_through() {
    let got_numeric = parse(
        &map(&[
            ("font.bold", "2"),
            ("font.shadowstyle", "blur"),
            ("font.shadowcolor.r", "300"),
        ]),
        None,
    );
    let got_word = parse(&map(&[("font.bold", "yes")]), None);

    assert_eq!(got_numeric.font_decoration_raw().bold(), Some("2"));
    assert_eq!(got_numeric.font_shadow_raw().style(), Some("blur"));
    assert_eq!(got_numeric.font_shadow_raw().color_r(), Some("300"));
    assert_eq!(got_word.font_decoration_raw().bold(), Some("yes"));
}

/// T5: 影色の「無効化の語 `none`」「数値」「未指定」は互いに異なる 3 状態として保たれる
/// （要件 2.6/9.4）。
#[test]
fn font_shadowcolor_none_numeric_unspecified_are_three_distinct_states() {
    let none_word = parse(&map(&[("font.shadowcolor.r", "none")]), None);
    let numeric = parse(&map(&[("font.shadowcolor.r", "64")]), None);
    let unspecified = parse(&map(&[("origin.x", "12")]), None);

    let a = none_word.font_shadow_raw().color_r();
    let b = numeric.font_shadow_raw().color_r();
    let c = unspecified.font_shadow_raw().color_r();
    assert_eq!(a, Some("none"));
    assert_eq!(b, Some("64"));
    assert_eq!(c, None);
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);
}

/// T6: 影色 3 成分の部分欠落は個別に `None`（`g` だけ宣言 → g=Some, r/b=None・要件 2.2/9.4）。
#[test]
fn font_shadowcolor_partial_absence_is_independently_none() {
    let got = parse(&map(&[("font.shadowcolor.g", "77")]), None);

    let s = got.font_shadow_raw();
    assert_eq!(s.color_r(), None);
    assert_eq!(s.color_g(), Some("77"));
    assert_eq!(s.color_b(), None);
}

/// 書体の飾り 5 本＋影 4 本（基底 9 本）を宣言順に並べて読む（T7〜T9 の比較用）。
fn base_nine(m: &super::BalloonModel) -> [Option<&str>; 9] {
    let d = m.font_decoration_raw();
    let s = m.font_shadow_raw();
    [
        d.bold(),
        d.italic(),
        d.outline(),
        d.strike(),
        d.underline(),
        s.color_r(),
        s.color_g(),
        s.color_b(),
        s.style(),
    ]
}

/// 基底 9 本のキー名（`base_nine` と同じ順）。
const BASE_NINE_KEYS: [&str; 9] = [
    "font.bold",
    "font.italic",
    "font.outline",
    "font.strike",
    "font.underline",
    "font.shadowcolor.r",
    "font.shadowcolor.g",
    "font.shadowcolor.b",
    "font.shadowstyle",
];

/// T7: 9 本を両層に別の値で書くと、画像別の上書き層の値が勝つ（要件 4.1/9.5）。
#[test]
fn font_base_keys_image_layer_overrides_descript() {
    let descript: String = BASE_NINE_KEYS
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k},d{i}\n"))
        .collect();
    let image: String = BASE_NINE_KEYS
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k},i{i}\n"))
        .collect();

    let got = parse_str(&descript, Some(&image));

    assert_eq!(
        base_nine(&got),
        [
            Some("i0"),
            Some("i1"),
            Some("i2"),
            Some("i3"),
            Some("i4"),
            Some("i5"),
            Some("i6"),
            Some("i7"),
            Some("i8"),
        ]
    );
}

/// T8: 画像別層が 9 本を持たないとき、既定層の値を引き継ぐ（要件 4.2/9.5）。
#[test]
fn font_base_keys_image_missing_key_inherits_descript() {
    let descript: String = BASE_NINE_KEYS
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{k},d{i}\n"))
        .collect();

    let got = parse_str(&descript, Some("origin.x,3\nfont.height,20"));

    assert_eq!(
        base_nine(&got),
        [
            Some("d0"),
            Some("d1"),
            Some("d2"),
            Some("d3"),
            Some("d4"),
            Some("d5"),
            Some("d6"),
            Some("d7"),
            Some("d8"),
        ]
    );
    // 画像別層の別キーはちゃんと重なっている（層が読まれていないことによる偶然の緑を防ぐ）。
    assert_eq!(got.font().height(), Some(20));
}

/// T9: 画像別層そのものが無いとき、既定層だけが写る（要件 4.3/9.5）。
#[test]
fn font_base_keys_descript_only_when_image_layer_absent() {
    let descript = map(&[
        ("font.bold", "1"),
        ("font.italic", "0"),
        ("font.outline", "1"),
        ("font.strike", "0"),
        ("font.underline", "1"),
        ("font.shadowcolor.r", "10"),
        ("font.shadowcolor.g", "none"),
        ("font.shadowcolor.b", "30"),
        ("font.shadowstyle", "outline"),
    ]);

    let got = parse(&descript, None);

    assert_eq!(
        base_nine(&got),
        [
            Some("1"),
            Some("0"),
            Some("1"),
            Some("0"),
            Some("1"),
            Some("10"),
            Some("none"),
            Some("30"),
            Some("outline"),
        ]
    );
}

/// T10: 接頭辞付きのキー（影を含む・`disable.` を含む）を書いても、基底 14 本はすべて未指定のまま
/// （要件 4.4/4.5/9.6）。既存の `distractor_keys_do_not_leak_into_modeled_scalars` は別に残す。
#[test]
fn prefixed_font_keys_do_not_leak_into_base_font_keys() {
    let descript = map(&[
        ("anchor.font.shadowcolor.r", "11"),
        ("anchor.font.shadowstyle", "offset"),
        ("anchor.notselect.font.shadowcolor.r", "12"),
        ("anchor.visited.font.shadowstyle", "outline"),
        ("cursor.font.shadowcolor.g", "13"),
        ("cursor.font.shadowstyle", "offset"),
        ("cursor.notselect.font.shadowcolor.b", "14"),
        ("number.font.height", "24"),
        ("sstpmessage.font.name", "MS Gothic"),
        ("communicatebox.font.color.r", "15"),
        ("disable.font.bold", "1"),
        ("anchor.font.bold", "1"),
        ("cursor.font.underline", "1"),
    ]);

    let got = parse(&descript, None);

    assert_eq!(got.font().name(), None);
    assert_eq!(got.font().height(), None);
    assert_eq!(got.font().color().r(), None);
    assert_eq!(got.font().color().g(), None);
    assert_eq!(got.font().color().b(), None);
    assert_eq!(base_nine(&got), [None; 9]);
}
