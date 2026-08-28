//! balloon モデル型（`model`）の単体テスト。
//!
//! 本モジュールは `model` とは別モジュールであり、公開パス
//! `crate::balloon::{BalloonModel, WindowPosition, Origin, WordWrapPoint,
//! ValidRect, Font, FontColor}` 経由で型へアクセスする。これにより
//! 「下流 engine が公開面（accessor）のみで各値を読める」I/O 契約と、
//! 「未指定＝`None` が `Some(0)` と判別される」done 基準（R2.6/R3.4）を、
//! 別モジュール視点で固定する。

#![cfg(test)]

use std::collections::BTreeMap;

use crate::balloon::{
    BalloonCursor, BalloonModel, CursorColor, Font, FontColor, Origin, ValidRect, WindowPosition,
    WindowPositionRaw, WordWrapPoint, parse,
};

/// テスト用: `&[(k, v)]` からフラット KV `BTreeMap` を組む小ヘルパ（parse_tests 流儀）。
fn kv_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn window_position_accessors_read_components() {
    // 公開/クレートパスで構築し accessor が各成分を返す（R2.1）。
    let wp = WindowPosition::new(Some(-34), Some(56));
    assert_eq!(wp.x(), Some(-34));
    assert_eq!(wp.y(), Some(56));
}

#[test]
fn window_position_unspecified_is_none_distinct_from_some_zero() {
    // 未指定＝None は Some(0) と判別される（R2.6/R4.2/R4.3 の核心）。
    let unspecified = WindowPosition::new(None, None);
    let zero = WindowPosition::new(Some(0), Some(0));
    assert_eq!(unspecified.x(), None);
    assert_eq!(zero.x(), Some(0));
    assert_ne!(unspecified.x(), zero.x());
    // 型の等価も両者で異なる（部分欠落を欠落なく表現）。
    assert_ne!(unspecified, zero);
}

#[test]
fn window_position_partial_absence_is_representable() {
    // x のみ指定・y 未指定という部分欠落を欠落なく持てる（R2.6）。
    let wp = WindowPosition::new(Some(-34), None);
    assert_eq!(wp.x(), Some(-34));
    assert_eq!(wp.y(), None);
}

#[test]
fn origin_accessors_read_components() {
    let o = Origin::new(Some(12), Some(34));
    assert_eq!(o.x(), Some(12));
    assert_eq!(o.y(), Some(34));
    // 未指定は None（R2.2/R2.6）。
    let empty = Origin::new(None, None);
    assert_eq!(empty.x(), None);
    assert_ne!(empty.x(), Some(0));
}

#[test]
fn word_wrap_point_accessors_and_negative_sign() {
    // 負値＝反対辺基準（R4.1）を符号付きで保持する。
    let w = WordWrapPoint::new(Some(-34), Some(0));
    assert_eq!(w.x(), Some(-34));
    assert_eq!(w.y(), Some(0));
    // y 未指定（存在しない）は None、Some(0) と判別（R2.3/R2.6）。
    let no_y = WordWrapPoint::new(Some(-34), None);
    assert_eq!(no_y.y(), None);
    assert_ne!(no_y.y(), Some(0));
    assert_ne!(no_y, w);
}

#[test]
fn valid_rect_accessors_read_four_edges() {
    // top/bottom/left/right を各独立に保持（R2.4）。
    let r = ValidRect::new(Some(10), Some(-56), Some(20), Some(-34));
    assert_eq!(r.top(), Some(10));
    assert_eq!(r.bottom(), Some(-56));
    assert_eq!(r.left(), Some(20));
    assert_eq!(r.right(), Some(-34));
}

#[test]
fn valid_rect_partial_absence_per_edge() {
    // 一部の辺のみ指定・残りは未指定という部分欠落を欠落なく表現（R2.4/R2.6/R3.4）。
    let r = ValidRect::new(Some(0), None, None, Some(-34));
    assert_eq!(r.top(), Some(0));
    assert_eq!(r.bottom(), None);
    assert_eq!(r.left(), None);
    assert_eq!(r.right(), Some(-34));
    // top=Some(0) と bottom=None が判別される（0 と未指定の区別）。
    assert_ne!(r.top(), r.bottom());
}

#[test]
fn font_color_accessors_and_none_vs_some_zero() {
    // r/g/b それぞれ 0–255・各独立 None（R2.5/R2.6）。
    let c = FontColor::new(Some(255), Some(0), None);
    assert_eq!(c.r(), Some(255));
    assert_eq!(c.g(), Some(0));
    assert_eq!(c.b(), None);
    // g=Some(0)（黒成分）と b=None（未指定）が判別される。
    assert_ne!(c.g(), c.b());
}

#[test]
fn font_accessors_name_height_color() {
    let color = FontColor::new(Some(1), Some(2), Some(3));
    let font = Font::new(Some("さざなみゴシック".to_string()), Some(12), color);
    assert_eq!(font.name(), Some("さざなみゴシック"));
    assert_eq!(font.height(), Some(12));
    assert_eq!(font.color(), color);
    assert_eq!(font.color().r(), Some(1));
}

#[test]
fn font_unspecified_components_are_none() {
    // name/height 未指定は None、height の Some(0) とは判別（R2.5/R2.6）。
    let font = Font::new(None, None, FontColor::new(None, None, None));
    assert_eq!(font.name(), None);
    assert_eq!(font.height(), None);
    assert_ne!(font.height(), Some(0));
    assert_eq!(font.color(), FontColor::new(None, None, None));
}

#[test]
fn window_position_raw_accessors_read_raw_strings() {
    // 生値 2 項を公開/クレートパスで構築し accessor が借用で読める（要件 1.1/4.1・C2）。
    let raw = WindowPositionRaw::new(Some("center".to_string()), Some("0".to_string()));
    assert_eq!(raw.x_raw(), Some("center"));
    assert_eq!(raw.limit_raw(), Some("0"));
}

#[test]
fn window_position_raw_unspecified_is_none() {
    // 未指定＝None（「値なし」を型で表す・要件 1.1/4.1）。空文字 Some("") とも判別される。
    let unspecified = WindowPositionRaw::new(None, None);
    assert_eq!(unspecified.x_raw(), None);
    assert_eq!(unspecified.limit_raw(), None);
    let empty = WindowPositionRaw::new(Some(String::new()), Some(String::new()));
    assert_ne!(unspecified, empty);
}

#[test]
fn balloon_model_windowposition_raw_defaults_to_unspecified_and_builder_overrides() {
    // `new` は既存署名のまま（additive）で、生値は未指定既定から始まる（要件 5.1）。
    let base = BalloonModel::new(
        WindowPosition::new(Some(-34), Some(56)),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    assert_eq!(base.windowposition_raw().x_raw(), None);
    assert_eq!(base.windowposition_raw().limit_raw(), None);

    // additive ビルダで相乗りさせても既存の数値アクセサは不変（要件 5.1）。
    let with_raw = base.clone().with_windowposition_raw(WindowPositionRaw::new(
        Some("bottom".to_string()),
        Some("1".to_string()),
    ));
    assert_eq!(with_raw.windowposition_raw().x_raw(), Some("bottom"));
    assert_eq!(with_raw.windowposition_raw().limit_raw(), Some("1"));
    assert_eq!(with_raw.windowposition(), base.windowposition());
}

#[test]
fn balloon_model_aggregates_sub_structs_via_accessors() {
    // 集約ルートを公開/クレートパスで構築し、各 accessor で sub-struct を読める。
    let model = BalloonModel::new(
        WindowPosition::new(Some(-34), Some(56)),
        Origin::new(Some(12), Some(34)),
        WordWrapPoint::new(Some(-34), None),
        ValidRect::new(Some(10), Some(-56), Some(20), Some(-34)),
        Font::new(
            Some("さざなみゴシック".to_string()),
            Some(12),
            FontColor::new(Some(255), Some(255), Some(255)),
        ),
        Some("vertical_rl".to_string()),
        Some("1".to_string()),
    );
    assert_eq!(model.windowposition().x(), Some(-34));
    assert_eq!(model.origin().y(), Some(34));
    assert_eq!(model.wordwrappoint().x(), Some(-34));
    assert_eq!(model.wordwrappoint().y(), None);
    assert_eq!(model.validrect().bottom(), Some(-56));
    // font は参照返し（String を含むため）。
    assert_eq!(model.font().name(), Some("さざなみゴシック"));
    assert_eq!(model.font().color().r(), Some(255));
    // writing_mode は借用で生文字列を読む（emo-text-layer 要件 5.6）。
    assert_eq!(model.writing_mode(), Some("vertical_rl"));
    // budoux_newline も借用で生文字列を読む（budoux-newline 要件 1.1）。
    assert_eq!(model.budoux_newline(), Some("1"));
}

#[test]
fn balloon_model_all_unspecified_is_none_distinct_from_zero() {
    // 全成分未指定のモデルは、各 accessor が None を返し 0 埋めと判別される（R2.6/R3.4）。
    let unspecified = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    assert_eq!(unspecified.windowposition().x(), None);
    assert_eq!(unspecified.origin().x(), None);
    assert_eq!(unspecified.validrect().top(), None);
    assert_eq!(unspecified.font().height(), None);
    // writing_mode 未指定は None（emo-text-layer 要件 5.6）。
    assert_eq!(unspecified.writing_mode(), None);
    // budoux_newline 未指定も None（budoux-newline 要件 1.1/1.5）。
    assert_eq!(unspecified.budoux_newline(), None);

    let zeros = BalloonModel::new(
        WindowPosition::new(Some(0), Some(0)),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(Some(0), Some(0)),
        ValidRect::new(Some(0), Some(0), Some(0), Some(0)),
        Font::new(None, Some(0), FontColor::new(Some(0), Some(0), Some(0))),
        None,
        None,
    );
    // 未指定モデルとゼロ埋めモデルは全体としても判別される。
    assert_ne!(unspecified, zeros);
    assert_ne!(unspecified.windowposition(), zeros.windowposition());
}

#[test]
fn types_derive_copy_and_clone_where_specified() {
    // 整数のみの型は Copy（accessor が値返しできる根拠）。
    let wp = WindowPosition::new(Some(1), Some(2));
    let copied = wp; // Copy
    assert_eq!(wp, copied);
    let color = FontColor::new(Some(1), Some(2), Some(3));
    let color_copied = color; // Copy
    assert_eq!(color, color_copied);
    // Font は String を含むため Clone のみ（Copy 不可）。
    let font = Font::new(Some("A".to_string()), Some(1), color);
    let font_cloned = font.clone();
    assert_eq!(font, font_cloned);
}

// ── budoux_newline 拡張キーの転記（要件 1.1/1.5・writing_mode 写経） ──

/// 基層/画像別上書き層の後勝ちマージで `budoux_newline` が転記される（要件 1.1）。
///
/// 基層のみ→基層値・画像別のみ→画像別値・両層→画像別が後勝ち、という 2 層マージ
/// （既存 `parse` の `merged` 機構）に `budoux_newline` が正しく乗ることを固定する。
#[test]
fn budoux_newline_two_layer_merge_image_wins() {
    // 両層に指定 → 画像別層が descript 基層を上書き（後勝ち・要件 1.1）。
    let descript = kv_map(&[("budoux_newline", "0")]);
    let image = kv_map(&[("budoux_newline", "1")]);
    let merged = parse(&descript, Some(&image));
    assert_eq!(merged.budoux_newline(), Some("1"));

    // 基層のみ（画像別層 None）→ 基層値がそのまま転記される。
    let base_only = parse(&descript, None);
    assert_eq!(base_only.budoux_newline(), Some("0"));

    // 画像別層のみキー保持（descript に無し）→ 画像別値を継承する。
    let image_only = parse(&BTreeMap::new(), Some(&image));
    assert_eq!(image_only.budoux_newline(), Some("1"));
}

/// `budoux_newline` は値を検証・解釈せず生文字列のまま転記する（要件 1.1）。
///
/// 受理語彙外の `abc` でも parser 層は素通し転記する（語彙判定・fallback は下流 emo
/// テキスト層の責務・[areka-parser-transcribes-tree-downstream]）。
#[test]
fn budoux_newline_raw_string_no_validation() {
    let descript = kv_map(&[("budoux_newline", "abc")]);
    let got = parse(&descript, None);
    // 未知語彙でも解釈せず生文字列のまま転記する。
    assert_eq!(got.budoux_newline(), Some("abc"));
}

/// `budoux_newline` を書かない既存ゴーストの挙動は不変（未知キー自然無視・要件 1.5）。
///
/// キー欠落 → `budoux_newline()` は `None`、かつ他のモデル化キー（origin・writing_mode）は
/// 影響を受けない（完全一致引きゆえ既存ゴースト無害）。
#[test]
fn budoux_newline_absent_is_none_and_other_keys_unaffected() {
    let descript = kv_map(&[("origin.x", "12"), ("writing_mode", "vertical_rl")]);
    let got = parse(&descript, None);
    assert_eq!(got.budoux_newline(), None);
    // 既存の他キーは budoux_newline 追加の影響を受けない（既存ゴースト挙動不変）。
    assert_eq!(got.origin().x(), Some(12));
    assert_eq!(got.writing_mode(), Some("vertical_rl"));
}

// ── cursor.* additive スタイルモデル（タスク 2.1・要件 4.2/6.2） ──

/// `CursorColor` は `FontColor` と同一表現をミラー（r/g/b 各独立 `Option<u8>`・
/// `None` と `Some(0)` の判別）（要件 4.2）。
#[test]
fn cursor_color_accessors_and_none_vs_some_zero() {
    let c = CursorColor::new(Some(105), Some(25), None);
    assert_eq!(c.r(), Some(105));
    assert_eq!(c.g(), Some(25));
    assert_eq!(c.b(), None);
    // g=Some(25) と b=None（未指定）が判別される。
    assert_ne!(c.g(), c.b());
    // 既定は全成分 None（未指定バルーン判定の素材）。
    assert_eq!(CursorColor::default(), CursorColor::new(None, None, None));
}

/// `BalloonCursor` の各 accessor が style/brush/pen/font/blendmethod を読める（要件 4.2）。
#[test]
fn balloon_cursor_accessors_read_all_fields() {
    let cursor = BalloonCursor::new(
        Some("square".to_string()),
        CursorColor::new(Some(105), Some(25), Some(25)),
        CursorColor::new(Some(200), Some(200), Some(200)),
        CursorColor::new(Some(255), Some(255), Some(255)),
        Some("none".to_string()),
    );
    assert_eq!(cursor.style(), Some("square"));
    assert_eq!(
        cursor.brush_color(),
        CursorColor::new(Some(105), Some(25), Some(25))
    );
    assert_eq!(cursor.pen_color().r(), Some(200));
    assert_eq!(
        cursor.font_color(),
        CursorColor::new(Some(255), Some(255), Some(255))
    );
    assert_eq!(cursor.blendmethod(), Some("none"));
}

/// `BalloonCursor::default()` は全キー未指定（`None`／全成分 `None`）を表す
/// （cursor.* 未指定バルーン判定の素材・要件 4.3/6.1）。
#[test]
fn balloon_cursor_default_is_all_unspecified() {
    let d = BalloonCursor::default();
    assert_eq!(d.style(), None);
    assert_eq!(d.blendmethod(), None);
    assert_eq!(d.brush_color(), CursorColor::default());
    assert_eq!(d.pen_color(), CursorColor::default());
    assert_eq!(d.font_color(), CursorColor::default());
}

/// cursor.* を含む descript をマージ解析すると `cursor` 各フィールドが値を持つ（要件 4.2）。
/// 2 層マージ（既存 `parse` の `merged` 機構）に cursor.* が相乗りする。
#[test]
fn cursor_keys_populate_cursor_model_via_merge() {
    let descript = kv_map(&[
        ("cursor.style", "square"),
        ("cursor.brush.color.r", "105"),
        ("cursor.brush.color.g", "25"),
        ("cursor.brush.color.b", "25"),
        ("cursor.pen.color.r", "200"),
        ("cursor.font.color.r", "255"),
        ("cursor.font.color.g", "255"),
        ("cursor.font.color.b", "255"),
        ("cursor.blendmethod", "none"),
    ]);
    let got = parse(&descript, None);
    let cursor = got.cursor();

    assert_eq!(cursor.style(), Some("square"));
    assert_eq!(cursor.brush_color().r(), Some(105));
    assert_eq!(cursor.brush_color().g(), Some(25));
    assert_eq!(cursor.brush_color().b(), Some(25));
    // pen.color は r のみ指定 → g/b は個別 None（部分欠落を欠落なく表現）。
    assert_eq!(cursor.pen_color().r(), Some(200));
    assert_eq!(cursor.pen_color().g(), None);
    assert_eq!(cursor.pen_color().b(), None);
    assert_eq!(cursor.font_color().r(), Some(255));
    assert_eq!(cursor.font_color().g(), Some(255));
    assert_eq!(cursor.font_color().b(), Some(255));
    assert_eq!(cursor.blendmethod(), Some("none"));
}

/// cursor.* 未記載の descript では `cursor` 全フィールドが未指定（`None`／既定）になる（要件 4.3/6.1）。
#[test]
fn cursor_absent_yields_all_unspecified() {
    let descript = kv_map(&[("origin.x", "0"), ("font.height", "28")]);
    let got = parse(&descript, None);
    let cursor = got.cursor();

    assert_eq!(cursor.style(), None);
    assert_eq!(cursor.blendmethod(), None);
    assert_eq!(cursor.brush_color(), CursorColor::default());
    assert_eq!(cursor.pen_color(), CursorColor::default());
    assert_eq!(cursor.font_color(), CursorColor::default());
    // 既存の他キーは cursor.* 追加の影響を受けない（既存ゴースト挙動不変・R2.7）。
    assert_eq!(got.origin().x(), Some(0));
    assert_eq!(got.font().height(), Some(28));
}

/// cursor.* の画像別上書きは既存 2 層後勝ちマージに相乗りする（要件 4.2・R3.2）。
#[test]
fn cursor_keys_image_layer_overrides_descript() {
    let descript = kv_map(&[("cursor.style", "square"), ("cursor.brush.color.r", "105")]);
    let image = kv_map(&[("cursor.brush.color.r", "10")]);
    let got = parse(&descript, Some(&image));

    // 画像別層が同一キーを後勝ち上書き（R3.2）。
    assert_eq!(got.cursor().brush_color().r(), Some(10));
    // 画像別層に無い style は descript 継承（R3.3）。
    assert_eq!(got.cursor().style(), Some("square"));
}

/// 未モデル化 cursor サブキー（shadowcolor/shadowstyle 等）はモデル化フィールドへ漏れず、
/// 寛容パス素通しのまま（KV 層が語彙を落とさない＝6.2 の語彙シーム）。
#[test]
fn cursor_unmodeled_subkeys_do_not_leak_into_modeled_fields() {
    let descript = kv_map(&[
        ("cursor.style", "square"),
        ("cursor.shadowcolor.r", "77"),
        ("cursor.shadowcolor.g", "77"),
        ("cursor.shadowstyle", "1"),
    ]);
    let got = parse(&descript, None);
    let cursor = got.cursor();

    // 未モデル化サブキーはモデル化フィールドへ折り込まれない（完全一致引き）。
    assert_eq!(cursor.style(), Some("square"));
    assert_eq!(cursor.brush_color(), CursorColor::default());
    assert_eq!(cursor.pen_color(), CursorColor::default());
    assert_eq!(cursor.font_color(), CursorColor::default());
    assert_eq!(cursor.blendmethod(), None);
}

/// cursor.font.color.* は cursor モデルへ入るが、既存 `font.color.*` を汚染しない
/// （「cursor キーを font へ巻き込まない」既存不変条件の分離側・要件 6.2）。
#[test]
fn cursor_font_color_does_not_fold_into_font_color() {
    let descript = kv_map(&[("font.color.r", "0"), ("cursor.font.color.r", "255")]);
    let got = parse(&descript, None);

    // font.color.r は descript の 0（cursor.font.color.r の 255 に汚染されない）。
    assert_eq!(got.font().color().r(), Some(0));
    // cursor.font.color.r は cursor モデルへ入る。
    assert_eq!(got.cursor().font_color().r(), Some(255));
}

/// cursor.* サブキーは 1 キー単位で個別に有無を持てる（部分欠落を欠落なく表現・要件 4.2/2.6）。
///
/// 2.1 の写像テストは「ほぼ全キー在り」か「全キー無し」を主に固定するが、本テストは
/// **フィールドを跨いだ疎な部分指定**——`cursor.brush.color.g` のみ・`cursor.pen.color.b` のみ・
/// `cursor.font.color.*` 全欠落・`cursor.style`／`cursor.blendmethod` 欠落——を与え、各サブキーの
/// 有無が**互いに独立**に反映されることを固定する（同色内の r/g/b 独立性、色間の独立性、
/// 文字列フィールドの独立欠落を 1 本で檻に入れる）。
#[test]
fn cursor_subkeys_populate_independently_per_key() {
    let descript = kv_map(&[
        // brush は g のみ、pen は b のみ、font は 1 成分も指定しない。
        ("cursor.brush.color.g", "40"),
        ("cursor.pen.color.b", "200"),
    ]);
    let got = parse(&descript, None);
    let cursor = got.cursor();

    // 文字列フィールドは未指定 → None（他サブキー在りでも巻き込まれない）。
    assert_eq!(cursor.style(), None);
    assert_eq!(cursor.blendmethod(), None);

    // brush.color: g のみ Some、r/b は個別 None（同色内 r/g/b 独立・Some(0) と None を判別）。
    assert_eq!(cursor.brush_color().r(), None);
    assert_eq!(cursor.brush_color().g(), Some(40));
    assert_eq!(cursor.brush_color().b(), None);

    // pen.color: b のみ Some、r/g は個別 None（色間も独立＝brush の指定に影響されない）。
    assert_eq!(cursor.pen_color().r(), None);
    assert_eq!(cursor.pen_color().g(), None);
    assert_eq!(cursor.pen_color().b(), Some(200));

    // font.color: 1 成分も指定なし → 全成分 None（既定）。
    assert_eq!(cursor.font_color(), CursorColor::default());
}

/// `cursor.style` は語彙を解釈せず生文字列で忠実転記する（square / underline / square+underline /
/// none／未知値のいずれも `Some(そのまま)`・要件 4.2/6.5）。
///
/// style の語彙判定・fallback は下流 `ResolvedChoiceStyle::resolve`（タスク 5.3）の責務であり、
/// parser 層は不透明転写に徹する（[areka-parser-transcribes-tree-downstream]）。2.1 は "square"
/// のみを固定していたため、複合値（`square+underline` の `+` 含み）・`none`・未知語彙を追加で檻に入れる。
#[test]
fn cursor_style_value_variants_transcribed_verbatim() {
    for style in ["square", "underline", "square+underline", "none", "wobble"] {
        let descript = kv_map(&[("cursor.style", style)]);
        let got = parse(&descript, None);
        // 未知語彙・複合語彙でも解釈せず生文字列のまま転記する（不透明転写）。
        assert_eq!(got.cursor().style(), Some(style), "style={style:?}");
    }
}

/// `cursor.blendmethod` も語彙を解釈せず生文字列で忠実転記する（none / notmaskpen／未知値の
/// いずれも `Some(そのまま)`・要件 6.5）。style と同一規律の不透明転写を別フィールドでも固定する。
#[test]
fn cursor_blendmethod_value_variants_transcribed_verbatim() {
    for blend in ["none", "notmaskpen", "alpha"] {
        let descript = kv_map(&[("cursor.blendmethod", blend)]);
        let got = parse(&descript, None);
        assert_eq!(got.cursor().blendmethod(), Some(blend), "blend={blend:?}");
    }
}

/// SSP 正典キー `vertical` の生値は additive 追加であり、`new` だけで組んだモデルでは
/// 未宣言（`None`）から始まる。相乗りさせても既存アクセサの戻り値は 1 つも変わらない
/// （要件 1.4／1.8・`with_windowposition_raw` 流儀）。
#[test]
fn balloon_model_vertical_raw_defaults_to_unspecified_and_builder_overrides() {
    let base = BalloonModel::new(
        WindowPosition::new(Some(-34), Some(56)),
        Origin::new(Some(12), Some(34)),
        WordWrapPoint::new(Some(-34), None),
        ValidRect::new(Some(10), Some(-56), Some(20), Some(-34)),
        Font::new(
            Some("さざなみゴシック".to_string()),
            Some(12),
            FontColor::new(Some(255), Some(255), Some(255)),
        ),
        Some("vertical_rl".to_string()),
        Some("1".to_string()),
    );
    // additive 既定は未宣言。
    assert_eq!(base.vertical_raw(), None);

    let with_vertical = base.clone().with_vertical_raw(Some("1".to_string()));
    assert_eq!(with_vertical.vertical_raw(), Some("1"));
    // 既存の解析結果は 1 つも変わらない（要件 1.8）。
    assert_eq!(with_vertical.windowposition(), base.windowposition());
    assert_eq!(with_vertical.origin(), base.origin());
    assert_eq!(with_vertical.wordwrappoint(), base.wordwrappoint());
    assert_eq!(with_vertical.validrect(), base.validrect());
    assert_eq!(with_vertical.font(), base.font());
    assert_eq!(with_vertical.writing_mode(), base.writing_mode());
    assert_eq!(with_vertical.budoux_newline(), base.budoux_newline());
    assert_eq!(with_vertical.cursor(), base.cursor());
    assert_eq!(with_vertical.windowposition_raw(), base.windowposition_raw());
}

/// 未宣言（`None`）と `vertical,0` の宣言（`Some("0")`）を潰さずに区別して保持する
/// （共存規則の判定に宣言の有無が要る・要件 1.4）。
#[test]
fn balloon_model_vertical_raw_absent_is_distinct_from_declared_zero() {
    let absent = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    let declared_zero = absent.clone().with_vertical_raw(Some("0".to_string()));

    assert_eq!(absent.vertical_raw(), None);
    assert_eq!(declared_zero.vertical_raw(), Some("0"));
    assert_ne!(absent.vertical_raw(), declared_zero.vertical_raw());
    // モデル全体としても両者は判別される。
    assert_ne!(absent, declared_zero);
}

/// 空文字列の宣言（`vertical,`）は `None` へ潰さない。値の解釈・語彙判定・縮退は
/// 下流（書字方向の解決）の責務であり、転記層は素通しで保持する（要件 1.4）。
#[test]
fn balloon_model_vertical_raw_empty_declaration_is_not_collapsed_to_none() {
    let base = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    let empty = base.clone().with_vertical_raw(Some(String::new()));
    assert_eq!(empty.vertical_raw(), Some(""));
    assert_ne!(empty.vertical_raw(), None);
    assert_ne!(empty, base);

    // 語彙外の値も解釈せず逐語で保持する（不透明転写・`cursor.style` 流儀）。
    for raw in ["2", "true", "01"] {
        let got = base.clone().with_vertical_raw(Some(raw.to_string()));
        assert_eq!(got.vertical_raw(), Some(raw), "vertical={raw:?}");
    }
}
