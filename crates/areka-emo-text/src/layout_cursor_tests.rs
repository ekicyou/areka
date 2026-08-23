use super::test_support::{IMAGE, inline_positions, model};
use super::{CursorWarnGuard, FixedMetrics, LayoutEngine, WrapPlan, cursor_to_image_px};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;
use areka_sakura::contract::ActorKey;
use log_capture_kit::count_levels;

// ── R2.1/2.2/2.4: `\_l` カーソル座標 → image px 換算（cursor_to_image_px・タスク 4.1） ──
//
// 換算式（design §`\_l 換算式`）: Px＝恒等・Em＝×font_height・Lh＝×line_pitch、最終座標は
// origin（当該軸 validrect 原点）加算。実導出は絶対 Px/Em/Lh の非負値のみ Some、
// Percent／Relative(@)／負値絶対／Invalid／Omitted は None（縮退＝当該軸スキップ）。

/// 絶対 Px/Em/Lh の非負値は正典式どおり `origin + value × factor` を返す
/// （factor: Px=1・Em=font_height・Lh=line_pitch）。単位ごとに異なる factor を檻化。
#[test]
fn cursor_to_image_px_converts_absolute_units_with_origin() {
    // Px: image_px = value（恒等）→ origin(10) + 5 = 15。font_height/line_pitch は無関与。
    assert_eq!(
        cursor_to_image_px(
            CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Px,
            },
            10.0,
            20.0,
            25.0,
        ),
        Some(15.0)
    );
    // Em: image_px = value × font_height → 2 × 20 = 40 → origin(10) + 40 = 50。
    assert_eq!(
        cursor_to_image_px(
            CursorCoord::Absolute {
                value: 2.0,
                unit: CursorUnit::Em,
            },
            10.0,
            20.0,
            25.0,
        ),
        Some(50.0)
    );
    // Lh: image_px = value × line_pitch → 3 × 25 = 75 → origin(10) + 75 = 85。
    assert_eq!(
        cursor_to_image_px(
            CursorCoord::Absolute {
                value: 3.0,
                unit: CursorUnit::Lh,
            },
            10.0,
            20.0,
            25.0,
        ),
        Some(85.0)
    );
}

/// 非負境界 value=0.0 は Some(origin)（≥0 ゲートは 0 を含む＝原点そのもの・境界檻）。
#[test]
fn cursor_to_image_px_zero_value_maps_to_origin() {
    for unit in [CursorUnit::Px, CursorUnit::Em, CursorUnit::Lh] {
        assert_eq!(
            cursor_to_image_px(CursorCoord::Absolute { value: 0.0, unit }, 7.0, 20.0, 25.0),
            Some(7.0),
            "{unit:?}: value 0 は origin へ写る（≥0 ゲート内）"
        );
    }
}

/// 縮退全形は None（当該軸スキップ・warn-once）: 負値絶対・Percent・Relative(@)・
/// Invalid・Omitted。origin/font_height/line_pitch に依らず None を返す。
#[test]
fn cursor_to_image_px_degenerate_forms_return_none() {
    // 負値絶対（Px/Em/Lh いずれも）: 非負ゲート外＝None。
    for unit in [CursorUnit::Px, CursorUnit::Em, CursorUnit::Lh] {
        assert_eq!(
            cursor_to_image_px(
                CursorCoord::Absolute { value: -1.0, unit },
                10.0,
                20.0,
                25.0
            ),
            None,
            "{unit:?}: 負値絶対は None"
        );
    }
    // Percent（縮退保持・非負でも実導出しない）。
    assert_eq!(
        cursor_to_image_px(
            CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Percent,
            },
            10.0,
            20.0,
            25.0,
        ),
        None
    );
    // Relative（@ 接頭）: 単位が Px/Em/Lh でも M1 は None。
    assert_eq!(
        cursor_to_image_px(
            CursorCoord::Relative {
                value: 5.0,
                unit: CursorUnit::Px,
            },
            10.0,
            20.0,
            25.0,
        ),
        None
    );
    // Invalid（パース不能）。
    assert_eq!(
        cursor_to_image_px(CursorCoord::Invalid, 10.0, 20.0, 25.0),
        None
    );
    // Omitted（当該軸省略）。
    assert_eq!(
        cursor_to_image_px(CursorCoord::Omitted, 10.0, 20.0, 25.0),
        None
    );
}

// ── Task 4.2: pending-cursor 遅延実体化（`\_l` の行区切り＋軸上書き・末尾蒸発・両軸 no-op） ──
//
// 共通前提は既存 layout 檻と同じ FixedMetrics・font 10（全角 'あ' advance 10・pitch 13）。
// origin(0,0)・wordwrappoint None ゆえ region.left()=0・region.top()=0・閾値=画像右辺 400。

/// 絶対 Px の `\_l` は現在行を確定し（`\_l` は行区切り・RN-3）、指定軸で次グリフの
/// inline/block を上書きする。`[あ, あ, \_l[100px,50px], あ]` → 行 0=[0,10]・
/// 3 個目が (inline 100, block 50) の行 1 頭へ載る。
#[test]
fn cursor_move_commits_line_and_overrides_next_glyph_axes() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 100.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Px,
            },
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "\\_l は現在行を確定する（行区切り・RN-3）");
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
    assert_eq!(
        inline_positions(&lines[1]),
        vec![100.0],
        "inline 軸が origin(0)+100 へ上書き"
    );
    assert_eq!(lines[1].rect.top, 50.0, "block 軸が origin(0)+50 へ上書き");
}

/// 保留改行と `\_l` が同一フラッシュに混在するとき、順序は 行確定→保留改行 Σratio→
/// カーソル軸上書き。`[あ, \n(1.0), \_l[10px,5px], あ]`（pitch 13）: 改行送り(block=13)を
/// 経てカーソル上書きが後勝ちで (inline 10, block 5) が最終値になる（順序 (2)→(3) の証左）。
#[test]
fn cursor_flush_orders_after_pending_newline_and_overrides_it() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Px,
            },
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].rect.top, 0.0);
    // 改行送りは block を 13 にするが、カーソル block 上書きが後勝ちで 5 が最終値。
    assert_eq!(
        lines[1].rect.top, 5.0,
        "カーソル block 上書きが保留改行送り(13)に勝つ（順序 (2)→(3)）"
    );
    // 改行は行内を 0 へ戻すが、カーソル inline 上書きが後勝ちで 10 が最終値。
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "カーソル inline 上書きが改行の行内リセットに勝つ"
    );
}

/// 後続可視グリフの無い末尾 `\_l` は保留のまま蒸発する（newline-defer と同一規則・2.5）。
/// `[あ, \_l[100,50]]` → 1 行 [あ@0]（`\_l` は行を開かず・位置も動かさない）。
#[test]
fn trailing_cursor_move_evaporates() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 100.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Px,
            },
        },
    ];
    let lines = LayoutEngine::layout(
        &items,
        1,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 1, "末尾 \\_l は蒸発（空行を開かない）");
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(lines[0].rect.top, 0.0);
}

/// 両軸 None（`\_l[,]`＝両軸省略）は完全 no-op——行区切りもしない
/// （正典「両方省略で無効果」・2.4）。`[あ, \_l[,], あ]` → 1 行 [あ@0, あ@10]（改行なし）。
#[test]
fn both_axes_omitted_cursor_move_is_complete_noop() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Omitted,
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(
        lines.len(),
        1,
        "両軸省略の \\_l は行区切りしない（完全 no-op）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
}

// ── Task 4.3: 換算表完全性（em/lh をレイアウト経由で配置）＋フラッシュ順序の per-axis 合成 ──
//
// 4.1 の `cursor_to_image_px_*` は換算を単体で檻化するが、em/lh 係数が実際に次グリフ配置を
// 駆動する経路（layout フラッシュ）は 4.2 が Px のみで檻化していた。4.3 は換算表の em/lh 分岐を
// レイアウト経由で・保留改行との per-axis 合成（一方をカーソル上書き・他方は改行進行値）を・
// 全縮退両軸（Omitted でなく実導出 None 経由）の完全 no-op を補完する。

/// 換算表の em/lh 分岐がレイアウト配置を実際に駆動する（4.2 は Px のみ）。`\_l[2em, 3lh]`
/// （font 10・pitch 13）は inline=origin(0)+2×10=20・block=origin(0)+3×13=39 へ次グリフを載せる。
/// 単位ごとに異なる係数（em＝font_height・lh＝line_pitch）が配置座標に現れることを檻化する。
#[test]
fn cursor_move_em_and_lh_units_place_next_glyph_through_layout() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 2.0,
                unit: CursorUnit::Em,
            },
            y: CursorCoord::Absolute {
                value: 3.0,
                unit: CursorUnit::Lh,
            },
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "\\_l は現在行を確定する（行区切り）");
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
    assert_eq!(
        inline_positions(&lines[1]),
        vec![20.0],
        "em: inline = origin(0) + 2×font_height(10) = 20"
    );
    assert_eq!(
        lines[1].rect.top, 39.0,
        "lh: block = origin(0) + 3×line_pitch(13) = 39"
    );
}

/// 保留改行なしの単軸 `\_l` は指定軸のみ上書きし、省略軸は走査中の実行位置のまま据え置く
/// （軸別に独立・R2.4）。x のみ `\_l[10px,]` → inline=10・block は改行がないため 0 のまま。
/// y のみ `\_l[,50px]` → block=50・inline は直前グリフ送り終端(10)のまま（リセットされない）。
#[test]
fn cursor_move_single_axis_leaves_other_axis_unchanged() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // x 軸のみ上書き（y Omitted）: block は改行がないため据え置き（0）、inline のみ 10 へ。
    let x_only = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &x_only,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "\\_l は単軸でも行区切りする");
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "inline のみ上書き（origin+10）"
    );
    assert_eq!(
        lines[1].rect.top, 0.0,
        "block 軸は指定なし＝据え置き（改行なしゆえ 0 のまま）"
    );

    // y 軸のみ上書き（x Omitted）: inline は直前送り終端(10)のまま・block のみ 50 へ。
    let y_only = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Px,
            },
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &y_only,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].rect.top, 50.0, "block のみ上書き（origin+50）");
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "inline 軸は指定なし＝直前グリフ送り終端(10)のまま（改行リセットも上書きもされない）"
    );
}

/// 保留改行と単軸 `\_l` が同一フラッシュに混在するとき、上書き軸はカーソル値・省略軸は改行進行値を
/// 取る（all-or-nothing でなく per-axis 合成の証左・R2.2/2.4）。font 10・pitch 13:
/// - x のみ `\_l[10px,]`＋`\n(1.0)`: inline=カーソル 10・block=改行進行 13。
/// - y のみ `\_l[,5px]`＋`\n(1.0)`: block=カーソル 5・inline=改行リセット 0。
#[test]
fn cursor_and_pending_newline_compose_per_axis() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // inline 上書き・block は改行進行値を取る。
    let x_over = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &x_over,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "inline: カーソル上書きが後勝ち（改行リセット 0 を上書き）"
    );
    assert_eq!(
        lines[1].rect.top, 13.0,
        "block: カーソル省略ゆえ改行進行値 13 が残る（per-axis 合成）"
    );

    // block 上書き・inline は改行リセット値を取る（vice-versa）。
    let y_over = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Px,
            },
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &y_over,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[1].rect.top, 5.0,
        "block: カーソル上書きが後勝ち（改行進行 13 を上書き）"
    );
    assert_eq!(
        inline_positions(&lines[1]),
        vec![0.0],
        "inline: カーソル省略ゆえ改行の行内リセット値 0 が残る（per-axis 合成）"
    );
}

/// 両軸が実導出 None へ縮退（Omitted でなく負値絶対＋`%`）した `\_l` も完全 no-op——行区切りしない
/// （縮退表「全縮退」row・2.4）。`both_axes_omitted` の Omitted 経路と別の到達路（縮退分岐）で
/// 同一 no-op を檻化する。`[あ, \_l[-1px, 50%], あ]` → 1 行 [あ@0, あ@10]（改行なし）。
#[test]
fn cursor_all_axes_degraded_is_complete_noop() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: -1.0,
                unit: CursorUnit::Px,
            }, // 負値絶対 → None
            y: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Percent,
            }, // % → None
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(
        lines.len(),
        1,
        "両軸全縮退の \\_l は行区切りしない（完全 no-op・Omitted 経路と同一結果）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
}

// ── Task 4.2: `\_l` 縮退 4 分岐の actor ごと warn-once（layout_with_cursor_warn・6.5） ──

/// `\_l` 換算の 4 縮退分岐（負値絶対／`%`／`@` 相対／パース不能）は actor ごと・分岐ごとに
/// 厳密 1 回だけ `warn!` する（6.5）。同一 actor の再訪では追加警告なし、別 actor では再び
/// 全分岐が警告される（持続 guard による per-actor once）。x 軸に縮退座標・y は Omitted
/// （Omitted は正常形で無音）とし、後続グリフで CursorMove を確実に処理させる。
#[test]
fn cursor_degrade_warns_once_per_actor_per_branch() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 4 縮退分岐（各 x 軸・y は Omitted）。
    let branches = [
        CursorCoord::Absolute {
            value: -1.0,
            unit: CursorUnit::Px,
        }, // 負値絶対
        CursorCoord::Absolute {
            value: 5.0,
            unit: CursorUnit::Percent,
        }, // %
        CursorCoord::Relative {
            value: 3.0,
            unit: CursorUnit::Px,
        }, // @ 相対
        CursorCoord::Invalid, // パース不能
    ];
    let a0 = ActorKey::from("0");
    let a1 = ActorKey::from("1");

    // guard は 3 段を通して持ち越し（per-actor once の観測対象そのもの）、警告件数は
    // 段ごとの捕捉窓の合計で数える（窓の外から観測するため段を分ける・累計は同じ）。
    let mut guard = CursorWarnGuard::default();
    let run = |actor: &ActorKey, coord: CursorCoord, guard: &mut CursorWarnGuard| {
        let items = [
            TextItem::CursorMove {
                x: coord,
                y: CursorCoord::Omitted,
            },
            TextItem::Glyph { ch: 'あ' },
        ];
        LayoutEngine::layout_with_cursor_warn(
            &items,
            1,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
            actor,
            guard,
        );
    };
    let mut warns = 0usize;

    // actor "0" 初回: 4 分岐がそれぞれ 1 回警告 → 計 4。
    let ((), counts) = count_levels(|| {
        for c in branches {
            run(&a0, c, &mut guard);
        }
    });
    warns += counts.warn;
    assert_eq!(warns, 4, "actor0 初回は 4 分岐×1 回＝4 警告");

    // actor "0" 再訪: 同一 (actor, 分岐) は既出＝追加警告なし。
    let ((), counts) = count_levels(|| {
        for c in branches {
            run(&a0, c, &mut guard);
        }
    });
    warns += counts.warn;
    assert_eq!(warns, 4, "actor0 再訪では追加警告なし（per-actor once）");

    // actor "1": 別 actor は guard が独立＝再び 4 分岐が警告 → 計 8。
    let ((), counts) = count_levels(|| {
        for c in branches {
            run(&a1, c, &mut guard);
        }
    });
    warns += counts.warn;
    assert_eq!(
        warns, 8,
        "別 actor では再び全 4 分岐が警告される（actor ごと once）"
    );
}

/// `Omitted`（軸省略）・実導出成功（非負 Px/Em/Lh）は縮退でなく無音（warn しない・R2.4）。
/// `\_l[5px, ]`（x 実導出・y 省略）と `\_l[,]`（両省略）はいずれも警告 0。
#[test]
fn cursor_omitted_and_valid_axes_do_not_warn() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let a0 = ActorKey::from("0");
    let ((), counts) = count_levels(|| {
        let mut guard = CursorWarnGuard::default();
        let items = [
            TextItem::CursorMove {
                x: CursorCoord::Absolute {
                    value: 5.0,
                    unit: CursorUnit::Px,
                },
                y: CursorCoord::Omitted,
            },
            TextItem::CursorMove {
                x: CursorCoord::Omitted,
                y: CursorCoord::Omitted,
            },
            TextItem::Glyph { ch: 'あ' },
        ];
        LayoutEngine::layout_with_cursor_warn(
            &items,
            1,
            &region,
            WritingMode::HorizontalTb,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
            &a0,
            &mut guard,
        );
    });
    assert_eq!(
        counts.warn, 0,
        "軸省略・実導出成功は縮退でない（警告しない）"
    );
}
