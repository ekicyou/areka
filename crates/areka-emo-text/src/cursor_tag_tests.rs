//! 解決層 `cursor_tag` の決定論テスト（`areka-P0-cursor-tag-canon`）。
//!
//! design.md「Testing Strategy ／ Unit Tests（`cursor_tag_tests.rs`・純関数）」の住処である。
//! タスク 3.1 が置くのは**解決表の各行を 1 本ずつ通す最小限**であり、全網羅（両軸 × 全書式 ×
//! 境界値 × 縮退 × ログ件数）はタスク 3.3 が本ファイルへ追加する。
//!
//! 共通前提は design.md「Unit Tests」の逐語:
//! `font_height = 10`・`line_pitch = 13`・`image_size = (400, 224)`・
//! `origin`＝宣言例 `(50, 20)`・`current = (200, 30)`。
//!
//! **期待値は正典（design.md 解決表）の式から書く**——実装が返した値を書き写さない。基点 3 種
//! （`origin`・`current`・画像原寸の半分）と係数 4 種（1・`font_height`・`line_pitch`・
//! `font_height / 100`）はいずれも互いに異なる値になるよう選んであるので、基点や係数を
//! 取り違えた実装はどれか 1 本で必ず赤になる。

use super::{
    CursorAxis, CursorBasis, CursorDegrade, CursorWarnGuard, note_out_of_range,
    resolve_cursor_axis, unit_coefficient, warn_cursor_degrade,
};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit};
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::ActorKey;
use log_capture_kit::{capture, count_levels};

/// design.md「Unit Tests」共通前提の文字高さ（正典 `1em`＝タグ時点の文字高さ）。
const FONT_HEIGHT: f32 = 10.0;
/// 同・行送り（正典 `1lh`＝1em＋行間。`ceil(10 × 1.25) = 13`）。
const LINE_PITCH: f32 = 13.0;
/// 同・バルーン画像原寸（`centerx`／`centery` の基準）。
const IMAGE_SIZE: (f32, f32) = (400.0, 224.0);
/// 同・宣言された文字描画開始点（絶対座標の基点）。
const ORIGIN: (f32, f32) = (50.0, 20.0);
/// 同・現在の文字描画位置（`@` 相対の基点）。
const CURRENT: (f32, f32) = (200.0, 30.0);

fn basis() -> CursorBasis {
    CursorBasis {
        origin: ORIGIN,
        current: CURRENT,
        image_size: IMAGE_SIZE,
        font_height: FONT_HEIGHT,
        line_pitch: LINE_PITCH,
    }
}

/// 解決表「`""`（省略）→ `Ok(None)`＝動かさない・無音」（R1.6/5.5）。両軸とも同じ。
#[test]
fn omitted_axis_does_not_move() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Omitted, CursorAxis::X, &basis()),
        Ok(None)
    );
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Omitted, CursorAxis::Y, &basis()),
        Ok(None)
    );
}

/// 解決表「`N`（数値・負値・小数）→ `origin[axis] + N × coef`」（R2.1/1.3/2.3）。
///
/// 基点が `origin` であること（`current` でも 0 でもない）と、負値・小数がそのまま
/// 通る（内側へ寄せない）ことを、単位 Px／Em の 2 系統で檻化する。
#[test]
fn absolute_is_measured_from_the_origin() {
    // 正の小数 Px: origin.x + 12.5 × 1
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: 12.5,
                unit: CursorUnit::Px,
            },
            CursorAxis::X,
            &basis()
        ),
        Ok(Some(ORIGIN.0 + 12.5))
    );
    // 負値 Em（クランプせず素通し）: origin.y + (−3) × font_height
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Absolute {
                value: -3.0,
                unit: CursorUnit::Em,
            },
            CursorAxis::Y,
            &basis()
        ),
        Ok(Some(ORIGIN.1 + -3.0 * FONT_HEIGHT))
    );
}

/// 解決表「`@N`（単位付き可）→ `current[axis] + N × coef`」（R3.1/3.2/3.3）。
///
/// 基点が `current` であること（`origin` ではない）と、`%` の係数が
/// `font_height / 100` であることを檻化する。
#[test]
fn relative_is_measured_from_the_current_position() {
    // 正典の記述例 `@-1lh`（「1 列ぶん左の列の先頭へ」）: current.x + (−1) × line_pitch。
    // 値を束縛で置くのは「基点 + 値 × 係数」の式の形を崩さずに書くためである。
    let minus_one_lh = -1.0_f32;
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: minus_one_lh,
                unit: CursorUnit::Lh,
            },
            CursorAxis::X,
            &basis()
        ),
        Ok(Some(CURRENT.0 + minus_one_lh * LINE_PITCH))
    );
    // design.md「Unit Tests」の例 `@-1650%`: current.y + (−1650) × font_height / 100
    assert_eq!(
        resolve_cursor_axis(
            CursorCoord::Relative {
                value: -1650.0,
                unit: CursorUnit::Percent,
            },
            CursorAxis::Y,
            &basis()
        ),
        Ok(Some(CURRENT.1 + -1650.0 * (FONT_HEIGHT / 100.0)))
    );
}

/// 解決表「`centerx` on X → `image_size.0 / 2`」（R4.1/4.3）。
///
/// 共通前提の画像幅 400 は半分が 200 で `current.x` と同値になり基点の取り違えを弁別
/// できないため、`current`・`origin` のどちらとも異なる半分を持つ画像原寸を 1 件足す。
#[test]
fn centerx_on_x_resolves_to_half_the_image_width() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterX, CursorAxis::X, &basis()),
        Ok(Some(IMAGE_SIZE.0 / 2.0))
    );
    let discriminating = CursorBasis {
        image_size: (360.0, 180.0),
        ..basis()
    };
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterX, CursorAxis::X, &discriminating),
        Ok(Some(360.0 / 2.0))
    );
}

/// 解決表「`centery` on Y → `image_size.1 / 2`」（R4.2/4.3）。
#[test]
fn centery_on_y_resolves_to_half_the_image_height() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterY, CursorAxis::Y, &basis()),
        Ok(Some(IMAGE_SIZE.1 / 2.0))
    );
    let discriminating = CursorBasis {
        image_size: (360.0, 180.0),
        ..basis()
    };
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterY, CursorAxis::Y, &discriminating),
        Ok(Some(180.0 / 2.0))
    );
}

/// 解決表「`centerx` on Y・`centery` on X → `Err(CenterAxisMismatch)`」（R1.5・縮退表）。
#[test]
fn center_written_on_the_wrong_axis_degrades_to_axis_mismatch() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterX, CursorAxis::Y, &basis()),
        Err(CursorDegrade::CenterAxisMismatch)
    );
    assert_eq!(
        resolve_cursor_axis(CursorCoord::CenterY, CursorAxis::X, &basis()),
        Err(CursorDegrade::CenterAxisMismatch)
    );
}

/// 解決表「解釈不能・非有限 → `Err(Unparsable)`」（R1.5/5.1/5.2）。
#[test]
fn invalid_degrades_to_unparsable() {
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Invalid, CursorAxis::X, &basis()),
        Err(CursorDegrade::Unparsable)
    );
    assert_eq!(
        resolve_cursor_axis(CursorCoord::Invalid, CursorAxis::Y, &basis()),
        Err(CursorDegrade::Unparsable)
    );
}

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

// ── 3.2: 範囲外の記録（R2.6）と縮退警告の一回化（R5.1/5.2/5.3） ──
//
// design.md「Unit Tests」4.・5. の住処。全網羅（解決表の全行 × 両軸 × 境界値 × ログ件数）は
// タスク 3.3 が本ファイルへ追加する。ここに置くのは新設 2 口の**判断分岐そのもの**——
// 閉区間の内外判定と、一回化の鍵に軸が含まれないこと——だけである。

/// 範囲外記録の檻で使う文字描画範囲（validrect）の 4 辺。
///
/// 画像全域（`0..400 × 0..224`）の**部分矩形**にしてある——全域にすると「validrect の辺」と
/// 「バルーン画像の辺」を取り違えた実装が素通りしてしまう（`image_size` は `(400, 224)`）。
const VALID_LEFT: f32 = 40.0;
const VALID_TOP: f32 = 20.0;
const VALID_RIGHT: f32 = 360.0;
const VALID_BOTTOM: f32 = 200.0;

/// 上の 4 辺を持つ `TextRegion`。
///
/// 書字方向は `note_out_of_range` の判定に**関与しない**——当該関数が読むのは validrect の
/// 4 辺だけで、書字方向が効くのは `start()`／`wrap_threshold()` の側である。ここで
/// `HorizontalTb` を渡すのは `TextRegion::resolve` の引数を埋めるためにすぎない。
///
/// **捕捉窓の外で組むこと**——`TextRegion::resolve` は未宣言の `origin` 成分について
/// `debug!` を出すので、窓の中で組むと範囲外記録の件数に混ざる。
fn out_of_range_region() -> TextRegion {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(
            Some(VALID_TOP as i32),
            Some(VALID_BOTTOM as i32),
            Some(VALID_LEFT as i32),
            Some(VALID_RIGHT as i32),
        ),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    let region = TextRegion::resolve(
        &model,
        (IMAGE_SIZE.0 as u32, IMAGE_SIZE.1 as u32),
        WritingMode::HorizontalTb,
    );
    // 檻の前提を檻にする（fixture が意図した矩形になっていることの確認）。
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        (VALID_LEFT, VALID_TOP, VALID_RIGHT, VALID_BOTTOM)
    );
    region
}

/// 範囲**内**と**境界上**は 1 件も記録しない（縮退表「解決後の位置が …［閉区間］の外」）。
///
/// 閉区間であることが正典の `vertical_rl` の `\_l[0,0]`（X ＝ `region.right()`）を沈黙させる
/// 規定なので、`== min`／`== max` の 4 点を明示で檻に入れる。
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

/// 検査対象は**点**であって、その点に置かれるグリフの矩形ではない。
///
/// `x = left` の列矩形は `[left − font_height, left]` で validrect の左外へはみ出すが、
/// 点そのものは境界上なので記録しない（矩形の可視性は描画側の責務・design.md Unit Tests 4）。
#[test]
fn note_out_of_range_checks_the_point_not_the_glyph_rect() {
    let region = out_of_range_region();
    // 列矩形の左端は validrect の外（前提の確認）。
    assert!(VALID_LEFT - FONT_HEIGHT < region.left());
    let ((), counts) = count_levels(|| {
        note_out_of_range(CursorAxis::X, VALID_LEFT, &region);
    });
    assert_eq!(counts.debug, 0, "点が境界上なら矩形が外へ出ても記録しない");
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
