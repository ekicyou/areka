//! balloon モデル型（`model`）の単体テスト。
//!
//! 本モジュールは `model` とは別モジュールであり、公開パス
//! `crate::balloon::{BalloonModel, WindowPosition, Origin, WordWrapPoint,
//! ValidRect, Font, FontColor}` 経由で型へアクセスする。これにより
//! 「下流 engine が公開面（accessor）のみで各値を読める」I/O 契約と、
//! 「未指定＝`None` が `Some(0)` と判別される」done 基準（R2.6/R3.4）を、
//! 別モジュール視点で固定する。

#![cfg(test)]

use crate::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

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
    );
    assert_eq!(model.windowposition().x(), Some(-34));
    assert_eq!(model.origin().y(), Some(34));
    assert_eq!(model.wordwrappoint().x(), Some(-34));
    assert_eq!(model.wordwrappoint().y(), None);
    assert_eq!(model.validrect().bottom(), Some(-56));
    // font は参照返し（String を含むため）。
    assert_eq!(model.font().name(), Some("さざなみゴシック"));
    assert_eq!(model.font().color().r(), Some(255));
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
    );
    assert_eq!(unspecified.windowposition().x(), None);
    assert_eq!(unspecified.origin().x(), None);
    assert_eq!(unspecified.validrect().top(), None);
    assert_eq!(unspecified.font().height(), None);

    let zeros = BalloonModel::new(
        WindowPosition::new(Some(0), Some(0)),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(Some(0), Some(0)),
        ValidRect::new(Some(0), Some(0), Some(0), Some(0)),
        Font::new(None, Some(0), FontColor::new(Some(0), Some(0), Some(0))),
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
