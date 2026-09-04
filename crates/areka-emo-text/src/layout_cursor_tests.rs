use super::test_support::{IMAGE, inline_positions, model};
use super::{CursorWarnGuard, FixedMetrics, LayoutEngine, WrapPlan};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;
use areka_sakura::contract::ActorKey;
use log_capture_kit::count_levels;

// ── 配線層の検証（`\_l` の行区切り・軸上書き・末尾蒸発・両軸 no-op） ──
//
// 解決そのもの（基点＋値×係数・縮退の分類）の純関数テストは解決層の兄弟ファイル
// （`cursor_tag_tests.rs`／`cursor_tag_resolve_tests.rs`）が持つ。本ファイルが見るのは
// **配線**だけである——到着時の解決をどう保留し、いつ実体化し、行をどう分割するか。
//
// 共通前提は既存 layout テストと同じ FixedMetrics・font 10（全角 'あ' advance 10・pitch 13）。
// origin(0,0)・wordwrappoint None ゆえ文字描画開始点は (0, 0)・折返し閾値＝画像右辺 400。

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

// ── 単位の係数が実配置を駆動すること＋フラッシュ順序の per-axis 合成 ──
//
// 係数そのものの正しさは解決層のテストが見る。ここで見るのは「解決値が本当に次グリフの
// 配置座標へ乗るか」（配線が解決値を落としていないこと）と、保留改行との per-axis 合成
// （一方をカーソル上書き・他方は改行進行値）である。

/// 解決表の em/lh の係数がレイアウト配置を実際に駆動する。`\_l[2em, 3lh]`
/// （font 10・pitch 13）は inline=文字描画開始点(0)+2×10=20・block=文字描画開始点(0)+3×13=39 へ
/// 次グリフを載せる。単位ごとに異なる係数（em＝font_height・lh＝line_pitch）が配置座標に
/// 現れることを固定する（配線が軸ごとの解決値を取り違えず載せていることの証跡）。
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

/// **正典値（4.1 の語彙解禁による）**: かつて両軸とも縮退していた形（負値絶対＋`%`）が、
/// いまは両軸とも実導出される（縮退表「負値絶対＝実導出」「`%`＝実導出」の行・R5.2）。
///
/// `[あ, \_l[-1px, 50%], あ]`（origin (0,0)・font 10）:
/// - x = 文字描画開始点(0) + (−1) = −1（文字描画範囲 0〜400 の左外＝字義どおり・寄せない）
/// - y = 文字描画開始点(0) + 50 × (10/100) = 5
///
/// 移動が成立するので `\_l` は行の分割点になり、行は 1 → 2 本へ増える。
/// 書き換え前の現行値は「両軸とも縮退＝完全 no-op ゆえ 1 行 [あ@0, あ@10]」だった。
/// **原点の切替（1.2）に由来する差分ではない**——横書きの原点は切替の前後とも (0, 0) である。
#[test]
fn cursor_negative_and_percent_axes_now_resolve_literally() {
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
            }, // 負値絶対 → origin(0) + (−1) = −1
            y: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Percent,
            }, // % → origin(0) + 50 × (10/100) = 5
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
        2,
        "正典値（4.1 の語彙解禁による）: 両軸とも移動が成立するので \\_l は行の分割点になる\
         （書き換え前は完全 no-op ゆえ 1 行だった）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(
        inline_positions(&lines[1]),
        vec![-1.0],
        "負値絶対は字義どおり文字描画範囲の左外へ出る（内側へ寄せない・R2.6）"
    );
    assert_eq!(
        lines[1].rect.top, 5.0,
        "`%` の係数は font_height / 100（50% ＝ 5px）"
    );
}

// ── `\_l` 縮退 2 分岐のキャラクターごと warn-once（layout_with_cursor_warn・R5.3） ──

/// `\_l` の 2 縮退分岐（解釈不能／中央指定の軸取り違え）はキャラクターごと・分岐ごとに
/// 厳密 1 回だけ `warn!` する（R5.3）。同一キャラクターの再訪では追加警告なし、別キャラクターでは
/// 再び全分岐が警告される（持続 guard による per-actor once）。x 軸に縮退座標・y は Omitted
/// （Omitted は正常形で無音）とし、後続グリフで CursorMove を確実に処理させる。
///
/// **分岐は 4 → 2 へ減った**（タスク 4.1）: 負値絶対・`%`・`@` 相対は縮退ではなく実導出へ
/// 移ったので、縮退表に残るのは「解釈不能」と「中央指定の軸取り違え」の 2 行だけである
/// （design.md 縮退表・R5.2）。
#[test]
fn cursor_degrade_warns_once_per_actor_per_branch() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 2 縮退分岐（各 x 軸・y は Omitted）。
    let branches = [
        // 解釈不能（CursorDegrade::Unparsable）。
        CursorCoord::Invalid,
        // 中央指定の軸取り違え（CursorDegrade::CenterAxisMismatch）＝`centery` を X 軸に書いた。
        CursorCoord::CenterY,
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

    // キャラクター "0" 初回: 2 分岐がそれぞれ 1 回警告 → 計 2。
    let ((), counts) = count_levels(|| {
        for c in branches {
            run(&a0, c, &mut guard);
        }
    });
    warns += counts.warn;
    assert_eq!(warns, 2, "キャラクター 0 の初回は 2 分岐×1 回＝2 警告");

    // キャラクター "0" 再訪: 同一 (キャラクター, 分岐) は既出＝追加警告なし。
    let ((), counts) = count_levels(|| {
        for c in branches {
            run(&a0, c, &mut guard);
        }
    });
    warns += counts.warn;
    assert_eq!(
        warns, 2,
        "キャラクター 0 の再訪では追加警告なし（per-actor once）"
    );

    // キャラクター "1": 別キャラクターは guard の鍵が独立＝再び 2 分岐が警告 → 計 4。
    let ((), counts) = count_levels(|| {
        for c in branches {
            run(&a1, c, &mut guard);
        }
    });
    warns += counts.warn;
    assert_eq!(
        warns, 4,
        "別キャラクターでは再び全 2 分岐が警告される（キャラクターごと once）"
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
