//! 解決層 `cursor_tag` の決定論テスト（`areka-P0-cursor-tag-canon`）。
//!
//! design.md「Testing Strategy ／ Unit Tests（`cursor_tag_tests.rs`・純関数）」の住処である。
//! いまここに在るのは、単位の係数、別の基点束での絶対、値 0 の着地と、記録の 2 口
//! （範囲外の `debug!`・縮退警告の一回化）の**判断分岐そのもの**である。解決表の各行を
//! 1 本ずつ通すテスト（タスク 3.1）は、姉妹モジュール `cursor_tag_resolve_tests.rs` の全表
//! `resolution_table_holds_on_every_row_and_axis` に値ごと含まれていたので消した
//! （`audit-emo-text-layout-draw` 指摘 28）。全網羅（解決表の全行 × 両軸 × 境界値 × 正典の
//! 記述例 × ログ件数）はあちらが持つ。共通前提は `cursor_tag_test_support.rs` が持つ。
//!
//! 共通前提は design.md「Unit Tests」の逐語:
//! `font_height = 10`・`line_pitch = 12`・`image_size = (400, 224)`・
//! `origin`＝宣言例 `(50, 20)`・`current = (200, 30)`。
//!
//! **期待値は正典（design.md 解決表）の式から書く**——実装が返した値を書き写さない。基点 3 種
//! （`origin`・`current`・画像原寸の半分）と係数 4 種（1・`font_height`・`line_pitch`・
//! `font_height / 100`）はいずれも互いに異なる値になるよう選んであるので、基点や係数を
//! 取り違えた実装はどれか 1 本で必ず赤になる。

use super::test_support::{
    FONT_HEIGHT, LINE_PITCH, ORIGIN, VALID_BOTTOM, VALID_LEFT, VALID_RIGHT, VALID_TOP, basis,
    out_of_range_region,
};
use super::{
    CursorAxis, CursorBasis, CursorDegrade, CursorWarnGuard, note_out_of_range,
    resolve_cursor_axis, unit_coefficient, warn_cursor_degrade,
};
use crate::state::{CursorCoord, CursorUnit};
use areka_sakura::contract::ActorKey;
use log_capture_kit::{capture, count_levels};

/// 単位の係数は正典どおりのスカラーで、**軸に依らない**（R1.3/1.4）。
///
/// 係数そのもの（Px=1・Em=font_height・Lh=line_pitch・%=font_height/100）を檻化したうえで、
/// 同じ `1lh` を X と Y に与えたときの**基点からの移動量**が等しいことを見る。基点は
/// 軸ごとに異なる（`origin = (50, 20)`）ので、移動量で比較しないと軸非依存性は測れない。
#[test]
fn unit_coefficient_is_a_scalar_that_does_not_depend_on_the_axis() {
    assert_eq!(
        unit_coefficient(CursorUnit::Px, FONT_HEIGHT, LINE_PITCH),
        1.0
    );
    assert_eq!(
        unit_coefficient(CursorUnit::Em, FONT_HEIGHT, LINE_PITCH),
        FONT_HEIGHT
    );
    assert_eq!(
        unit_coefficient(CursorUnit::Lh, FONT_HEIGHT, LINE_PITCH),
        LINE_PITCH
    );
    assert_eq!(
        unit_coefficient(CursorUnit::Percent, FONT_HEIGHT, LINE_PITCH),
        FONT_HEIGHT / 100.0
    );

    let coord = CursorCoord::Absolute {
        value: 2.0,
        unit: CursorUnit::Lh,
    };
    let x = resolve_cursor_axis(coord, CursorAxis::X, &basis())
        .expect("実導出（縮退しない）")
        .expect("移動が成立する");
    let y = resolve_cursor_axis(coord, CursorAxis::Y, &basis())
        .expect("実導出（縮退しない）")
        .expect("移動が成立する");
    assert_eq!(x - ORIGIN.0, 2.0 * LINE_PITCH);
    assert_eq!(y - ORIGIN.1, 2.0 * LINE_PITCH);
    assert_eq!(x - ORIGIN.0, y - ORIGIN.1);
}

// ── 4.1 で配線層から移設した純関数テスト ──
//
// 旧配線 `layout.rs` は座標換算の純関数（絶対 Px/Em/Lh の**非負値のみ**を実導出し、
// 負値・`%`・`@` 相対・解釈不能・省略はすべて `None`）を自分で持っていた。タスク 4.1 で
// それを撤去して意味論を本モジュールへ委譲したので、当時の純関数テスト 3 本もここへ移した。
//
// 移設にあたり期待値は**正典の式**（design.md 解決表）から書き直してある。3 本目（旧実装が
// 一律 `None` を返していた 5 形のいまの行き先）は、5 形とも姉妹モジュールの全表
// `resolution_table_holds_on_every_row_and_axis` の行に含まれるので消した（指摘 28）。

/// 絶対は `origin[axis] + 値 × 係数`（移設元 1 本目）。
///
/// 共通前提とは**別の基点束**（原点 `(10, 10)`・文字高さ 20・行送り 25）で通す——共通前提の値に
/// 焼き付いた実装（原点や係数を定数で持ってしまった実装）をここで赤にする。移設元が
/// `origin = 10`・`font_height = 20`・`line_pitch = 25` を渡していたのと同じ数値である。
#[test]
fn absolute_units_resolve_from_the_origin_on_an_independent_basis() {
    let b = CursorBasis {
        origin: (10.0, 10.0),
        current: (77.0, 88.0),
        image_size: (360.0, 180.0),
        font_height: 20.0,
        line_pitch: 25.0,
    };
    for (unit, value) in [
        (CursorUnit::Px, 5.0f32),
        (CursorUnit::Em, 2.0),
        (CursorUnit::Lh, 3.0),
        (CursorUnit::Percent, 40.0),
    ] {
        let coef = unit_coefficient(unit, b.font_height, b.line_pitch);
        assert_eq!(
            resolve_cursor_axis(CursorCoord::Absolute { value, unit }, CursorAxis::X, &b),
            Ok(Some(b.origin.0 + value * coef)),
            "{unit:?}: X ＝ origin.0 + 値 × 係数"
        );
        assert_eq!(
            resolve_cursor_axis(CursorCoord::Absolute { value, unit }, CursorAxis::Y, &b),
            Ok(Some(b.origin.1 + value * coef)),
            "{unit:?}: Y ＝ origin.1 + 値 × 係数"
        );
    }
}

/// 値 0 は係数に依らず原点そのものへ着地する（移設元 2 本目）。
///
/// 移設元は Px/Em/Lh の 3 単位だけを見ていた。`%` が実導出へ移った（R5.2）ので **4 単位**に
/// 広げてある——`0 × 係数 = 0` が単位の別に依らないことが、単位を軸に依らないスカラーとして
/// 与えている設計（R1.4）の帰結だからである。
#[test]
fn zero_value_lands_exactly_on_the_origin_for_every_unit() {
    let b = basis();
    for unit in [
        CursorUnit::Px,
        CursorUnit::Em,
        CursorUnit::Lh,
        CursorUnit::Percent,
    ] {
        assert_eq!(
            resolve_cursor_axis(
                CursorCoord::Absolute { value: 0.0, unit },
                CursorAxis::X,
                &b
            ),
            Ok(Some(b.origin.0)),
            "{unit:?}: 値 0 は原点へ写る"
        );
    }
}

// ── 3.2: 範囲外の記録（R2.6）と縮退警告の一回化（R5.1/5.2/5.3） ──
//
// design.md「Unit Tests」4.・5. の住処。ここに置くのは新設 2 口の**判断分岐そのもの**——
// 閉区間の内外判定と、一回化の鍵に軸が含まれないこと——である。全網羅（解決表の全行 ×
// 両軸 × 境界値 × 正典の記述例 × ログ件数）は姉妹モジュール `cursor_tag_resolve_tests.rs`。
// validrect の 4 辺と `out_of_range_region` は `cursor_tag_test_support.rs` が持つ。

/// 範囲**内**と**境界上**は 1 件も記録しない（縮退表「解決後の位置が …［閉区間］の外」）。
///
/// 閉区間であることが正典の `vertical_rl` の `\_l[0,0]`（X ＝ `region.right()`）を沈黙させる
/// 規定なので、`== min`／`== max` の 4 点を明示で檻に入れる。検査対象は**点**であって
/// グリフの矩形ではない——`x = left` の列矩形 `[left − font_height, left]` は外へはみ出すが、
/// 点が境界上なので記録しない（矩形の可視性は描画側の責務・design.md Unit Tests 4）。
#[test]
fn note_out_of_range_is_silent_inside_and_on_the_closed_boundary() {
    let region = out_of_range_region();
    let ((), counts) = count_levels(|| {
        // 内側（両軸）。
        note_out_of_range(CursorAxis::X, (VALID_LEFT + VALID_RIGHT) / 2.0, &region);
        note_out_of_range(CursorAxis::Y, (VALID_TOP + VALID_BOTTOM) / 2.0, &region);
        // 境界上（閉区間＝範囲内）。
        note_out_of_range(CursorAxis::X, VALID_LEFT, &region);
        note_out_of_range(CursorAxis::X, VALID_RIGHT, &region);
        note_out_of_range(CursorAxis::Y, VALID_TOP, &region);
        note_out_of_range(CursorAxis::Y, VALID_BOTTOM, &region);
    });
    assert_eq!(
        counts.debug, 0,
        "範囲内・境界上は 1 件も記録しない（閉区間）"
    );
    assert_eq!(counts.warn, 0, "範囲外記録は warn を出さない");
    assert_eq!(counts.error, 0, "縮退も範囲外も致命扱いしない（R5.1）");
}

/// 範囲**外**は軸ごとに `debug!` 1 件だけを残し、軸・値・範囲を構造化フィールドで載せる
/// （design.md Monitoring: `axis`・`value`・`range_min`・`range_max`）。
///
/// 件数だけでなく「どの軸のどの値がどの範囲の外だったか」まで見る——件数だけの檻は、
/// 軸を取り違えて範囲を引いた実装（X の値を Y の範囲で見る等）を素通しさせる。
#[test]
fn note_out_of_range_records_one_debug_with_axis_value_and_range() {
    let region = out_of_range_region();

    // X の下外（min − 0.5）。値はいずれも 2 進で厳密な半整数なので Debug 表現が一意に定まる。
    let ((), events) = capture(|| {
        note_out_of_range(CursorAxis::X, VALID_LEFT - 0.5, &region);
    });
    assert_eq!(events.len(), 1, "範囲外は 1 件");
    assert_eq!(
        events[0].level,
        tracing::Level::DEBUG,
        "DEBUG レベル（R2.6）"
    );
    let fields = events[0].fields_map();
    assert_eq!(fields.get("axis").copied(), Some("X"));
    assert_eq!(fields.get("value").copied(), Some("39.5"));
    assert_eq!(fields.get("range_min").copied(), Some("40.0"));
    assert_eq!(fields.get("range_max").copied(), Some("360.0"));

    // Y の上外（max + 0.5）。範囲は Y 軸の [top, bottom] であって X の [left, right] ではない。
    let ((), events) = capture(|| {
        note_out_of_range(CursorAxis::Y, VALID_BOTTOM + 0.5, &region);
    });
    assert_eq!(events.len(), 1, "範囲外は 1 件");
    let fields = events[0].fields_map();
    assert_eq!(fields.get("axis").copied(), Some("Y"));
    assert_eq!(fields.get("value").copied(), Some("200.5"));
    assert_eq!(fields.get("range_min").copied(), Some("20.0"));
    assert_eq!(fields.get("range_max").copied(), Some("200.0"));
}

/// 範囲外は**両軸 × 両側の 4 方向**で記録される（`min − 0.5` と `max + 0.5`・design Unit Tests 4）。
///
/// 上の 2 本は X の下外と Y の上外しか見ていないので、残る 2 方向（X の上外・Y の下外）を
/// 含めて 4 件を 1 度に締める。片側だけを見る実装（`value < min` を落とした等）はここで赤になる。
#[test]
fn note_out_of_range_records_every_side_of_both_axes() {
    let region = out_of_range_region();
    let ((), counts) = count_levels(|| {
        note_out_of_range(CursorAxis::X, VALID_LEFT - 0.5, &region);
        note_out_of_range(CursorAxis::X, VALID_RIGHT + 0.5, &region);
        note_out_of_range(CursorAxis::Y, VALID_TOP - 0.5, &region);
        note_out_of_range(CursorAxis::Y, VALID_BOTTOM + 0.5, &region);
    });
    assert_eq!(counts.debug, 4, "4 方向それぞれが 1 件");
}

/// 範囲外記録は**一回化しない**（同じ値を 2 度渡せば 2 件残る）。
///
/// 一回化するのは `warn_cursor_degrade` だけである（縮退表「一回化しない」）。
#[test]
fn note_out_of_range_is_not_deduplicated() {
    let region = out_of_range_region();
    let ((), counts) = count_levels(|| {
        note_out_of_range(CursorAxis::X, VALID_RIGHT + 0.5, &region);
        note_out_of_range(CursorAxis::X, VALID_RIGHT + 0.5, &region);
    });
    assert_eq!(counts.debug, 2, "範囲外記録は一回化しない");
}

/// 縮退警告はキャラクターごと・分岐ごとに初回 1 回だけ（R5.3）。**鍵に軸は含まれない**
/// （design.md 検証表 H5: `\_l[centery,centerx]` は軸が違っても同一キャラクターで 1 回）。
#[test]
fn warn_cursor_degrade_warns_once_per_actor_and_branch_regardless_of_axis() {
    let a0 = ActorKey::from("0");
    let a1 = ActorKey::from("1");
    let mut guard = CursorWarnGuard::default();

    // 初回（actor "0" × Unparsable）＝1 件。
    let ((), counts) = count_levels(|| {
        warn_cursor_degrade(
            &a0,
            CursorAxis::X,
            CursorCoord::Invalid,
            CursorDegrade::Unparsable,
            &mut guard,
        );
    });
    assert_eq!(counts.warn, 1, "初回は警告する");

    // 同一 (actor, degrade) の再訪＝0 件（軸が違っても鍵に含まれないので沈黙する）。
    let ((), counts) = count_levels(|| {
        warn_cursor_degrade(
            &a0,
            CursorAxis::X,
            CursorCoord::Invalid,
            CursorDegrade::Unparsable,
            &mut guard,
        );
        warn_cursor_degrade(
            &a0,
            CursorAxis::Y,
            CursorCoord::Invalid,
            CursorDegrade::Unparsable,
            &mut guard,
        );
    });
    assert_eq!(
        counts.warn, 0,
        "同一 (actor, 分岐) は軸が違っても再警告しない"
    );

    // 別の分岐（同一 actor）＝再び 1 件。
    let ((), counts) = count_levels(|| {
        warn_cursor_degrade(
            &a0,
            CursorAxis::Y,
            CursorCoord::CenterX,
            CursorDegrade::CenterAxisMismatch,
            &mut guard,
        );
        // 同じ分岐を軸だけ変えて再訪＝沈黙（H5 の逐語: 両軸取り違えでも warn は 1 件）。
        warn_cursor_degrade(
            &a0,
            CursorAxis::X,
            CursorCoord::CenterY,
            CursorDegrade::CenterAxisMismatch,
            &mut guard,
        );
    });
    assert_eq!(
        counts.warn, 1,
        "分岐が変われば再び 1 件・軸違いは追加しない"
    );

    // 別の actor＝分岐ごとに再び 1 件ずつ。
    let ((), counts) = count_levels(|| {
        warn_cursor_degrade(
            &a1,
            CursorAxis::X,
            CursorCoord::Invalid,
            CursorDegrade::Unparsable,
            &mut guard,
        );
        warn_cursor_degrade(
            &a1,
            CursorAxis::Y,
            CursorCoord::CenterX,
            CursorDegrade::CenterAxisMismatch,
            &mut guard,
        );
    });
    assert_eq!(counts.warn, 2, "別キャラクターでは分岐ごとに再び 1 件");
}

/// 縮退警告の中身（design.md Monitoring: `actor`・`axis`・`coord`・`degrade`）。
///
/// 件数だけの檻は「どのキャラクターのどの軸のどの書式がどう縮退したか」を言わないので、
/// 4 フィールドの値まで見る。レベルは `warn`（`error!` は使わない・R5.1）。
#[test]
fn warn_cursor_degrade_records_actor_axis_coord_and_degrade() {
    let actor = ActorKey::from("1");
    let mut guard = CursorWarnGuard::default();
    let ((), events) = capture(|| {
        warn_cursor_degrade(
            &actor,
            CursorAxis::Y,
            CursorCoord::CenterX,
            CursorDegrade::CenterAxisMismatch,
            &mut guard,
        );
    });
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].level,
        tracing::Level::WARN,
        "致命扱いしない（R5.1）"
    );
    let fields = events[0].fields_map();
    assert_eq!(fields.get("actor").copied(), Some("1"));
    assert_eq!(fields.get("axis").copied(), Some("Y"));
    assert_eq!(fields.get("coord").copied(), Some("CenterX"));
    assert_eq!(fields.get("degrade").copied(), Some("CenterAxisMismatch"));
}
