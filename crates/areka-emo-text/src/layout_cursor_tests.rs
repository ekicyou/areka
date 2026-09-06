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
// 共通前提は既存 layout テストと同じ FixedMetrics・font 10（全角 'あ' advance 10・pitch 12＝10 + 行間 2）。
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
/// カーソル軸上書き。`[あ, \n(1.0), \_l[10px,5px], あ]`（pitch 12）: 改行送り(block=12)を
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
    // 改行送りは block を 12 にするが、カーソル block 上書きが後勝ちで 5 が最終値。
    assert_eq!(
        lines[1].rect.top, 5.0,
        "カーソル block 上書きが保留改行送り(12)に勝つ（順序 (2)→(3)）"
    );
    // 改行は行内を 0 へ戻すが、カーソル inline 上書きが後勝ちで 10 が最終値。
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "カーソル inline 上書きが改行の行内リセットに勝つ"
    );
}

/// 後続可視グリフの無い末尾 `\_l` は保留のまま蒸発する（完了仕様 `areka-P0-newline-defer` R5.2／5.3 と同一規則）。
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
/// （正典「両方省略で無効果」・R1.6/5.4/6.2）。`[あ, \_l[,], あ]` → 1 行 [あ@0, あ@10]（改行なし）。
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
/// （font 10・pitch 12）は inline=文字描画開始点(0)+2×10=20・block=文字描画開始点(0)+3×12=36 へ
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
        lines[1].rect.top, 36.0,
        "lh: block = origin(0) + 3×line_pitch(12) = 36"
    );
}

/// 保留改行なしの単軸 `\_l` は指定軸のみ上書きし、省略軸は走査中の実行位置のまま据え置く
/// （軸別に独立・R1.2/1.6）。x のみ `\_l[10px,]` → inline=10・block は改行がないため 0 のまま。
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
/// 取る（all-or-nothing でなく per-axis 合成の証左・R1.2/1.6）。font 10・pitch 12:
/// - x のみ `\_l[10px,]`＋`\n(1.0)`: inline=カーソル 10・block=改行進行 12。
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
        lines[1].rect.top, 12.0,
        "block: カーソル省略ゆえ改行進行値 12 が残る（per-axis 合成）"
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
        "block: カーソル上書きが後勝ち（改行進行 12 を上書き）"
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

/// `Omitted`（軸省略）・実導出成功（Px/Em/Lh）は縮退でなく無音（warn しない・R5.5/5.2）。
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

// ── 実効位置（`@` の基点）と保留の軸ごと合成（タスク 4.2・design 5 段手順の 1 段目/5 段目） ──

/// **正典値（4.2 の軸ごと合成による）**: 文字を挟まずに連続する `\_l` の保留は
/// **軸ごとに合成**され、後の指定が動かさなかった軸は先の値を保つ（検証表 H2）。
///
/// `[\_l[10,], \_l[,20], あ]`（origin (0,0)）: 先の指定が X=10 を保留し、後の指定は
/// Y=20 だけを動かす（X は省略＝「移動しない」＝正典の正常形）。合成後の保留は
/// (X=10, Y=20) で、次のグリフは (10, 20) へ載る。
///
/// 書き換え前の現行値は X=0（＝行頭 `inline_start`）だった——後の `\_l` が保留を
/// **丸ごと上書き**していたため、先の `\_l` が動かした X が失われていた。正典
/// 「省略＝移動しない」は「先に保留された値を捨てる」ことまでは意味しない（R1.2/1.6/3.5）。
///
/// 併せて、両軸とも移動が成立しない `\_l[,]` が**既存の保留を消さない**ことを固定する
/// （R5.4/6.2・縮退表「両軸省略／両軸縮退＝完全無効果」は「保留を変えない」を含む）。
#[test]
fn consecutive_cursor_moves_compose_pending_per_axis() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // H2: `\\_l[10,]\\_l[,20]あ` → (10, 20)。
    let items = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Absolute {
                value: 20.0,
                unit: CursorUnit::Px,
            },
        },
        TextItem::Glyph { ch: 'あ' },
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
    assert_eq!(lines.len(), 1, "先頭フラッシュは空行を作らない");
    assert_eq!(
        inline_positions(&lines[0]),
        vec![10.0],
        "正典値（4.2 の軸ごと合成による）: 後の `\\_l` は X を動かさない\
         （省略）ので、先の `\\_l` の X=10 が保たれる。書き換え前の現行値は 0（丸ごと上書き）"
    );
    assert_eq!(lines[0].rect.top, 20.0, "後の `\\_l` の Y=20 が保留へ入る");

    // 両軸とも成立しない `\\_l[,]` は既存の保留を消さない（R5.4/6.2）。
    let kept = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Absolute {
                value: 20.0,
                unit: CursorUnit::Px,
            },
        },
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Omitted,
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &kept,
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
        "移動が成立した先の `\\_l` が行の分割点になる"
    );
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "両軸省略の `\\_l` は保留を変えない（X=10 が残る）"
    );
    assert_eq!(
        lines[1].rect.top, 20.0,
        "両軸省略の `\\_l` は保留を変えない（Y=20 が残る）"
    );
}

/// **正典値（4.2 の実効位置による）**: `@` 相対の基点は、走査位置に**保留中の改行と
/// 保留中のカーソルをフラッシュと同じ順で仮適用した**「次の文字が置かれる位置」である
/// （検証表 H3・R3.1/3.5）。仮適用は読み取りだけで、走査ローカル状態を書き換えない。
///
/// - `[あ, \_l[@0,@0], あ]`: 保留なし＝実効位置は走査位置そのもの (10, 0)。`@0` は
///   そこへ据え置く＝2 個目が 1 個目に続けて置かれる。
/// - `[あ, \n(1.0), \_l[@0,@0], あ]`: 保留改行があるので実効位置は改行を仮適用した
///   (0, 12)＝**次行の先頭**。`@0` はそこへ据え置くので、改行が取り消されない。
///   書き換え前の現行値は (10, 0) で、`@0` が走査位置を基点にしたせいで**保留改行を
///   打ち消していた**（改行が無かったことになる）。
/// - `[\_l[10,], \_l[@5,], あ]`: 保留カーソルも仮適用の対象。実効位置の X は先の
///   保留 10 なので `@5` は 15 になる（走査位置 0 を基点にすると 5 になる）。
/// - `[\n(1.0), \_l[10,], \_l[@0,], あ]`: 保留改行と保留カーソルが**同時に**居る
///   交差ケース。仮適用の順序はフラッシュ本体（ゲート②の保留フラッシュ）の (2)→(3) と同順である——
///   (2) 保留改行が行内軸を先頭（0）へ戻し、(3) 保留カーソルの X=10 がそれに**後勝ち**する。
///   ゆえに実効位置は (10, 12) で、`@0` はそこへ据え置き、最終着地も (10, 12) になる。
///   仮適用の 2 ブロック（`layout.rs` の `CursorMove` 腕・実効位置の算出）を入れ替えると (3)→(2) の順になり、改行の
///   行内リセットが後勝ちして `inline` が 0 へ落ちる＝この 1 本だけが赤になる。
#[test]
fn relative_cursor_basis_is_the_effective_position() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let relative_zero = || TextItem::CursorMove {
        x: CursorCoord::Relative {
            value: 0.0,
            unit: CursorUnit::Px,
        },
        y: CursorCoord::Relative {
            value: 0.0,
            unit: CursorUnit::Px,
        },
    };

    // 保留なし: 実効位置＝走査位置 (10, 0)。
    let plain = [
        TextItem::Glyph { ch: 'あ' },
        relative_zero(),
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &plain,
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "両軸とも移動が成立するので行の分割点になる");
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "保留なし＝実効位置は走査位置 (10, 0)。`@0` は続けて配置する"
    );
    assert_eq!(lines[1].rect.top, 0.0);

    // 保留改行あり: 実効位置は改行を仮適用した (0, 12)＝次行の先頭。
    let after_break = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
        relative_zero(),
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &after_break,
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
        vec![0.0],
        "正典値（4.2 の実効位置による）: 実効位置の行内軸は改行の行内リセット後の 0。\
         書き換え前の現行値は 10（走査位置を基点にして改行を打ち消していた）"
    );
    assert_eq!(
        lines[1].rect.top, 12.0,
        "正典値（4.2 の実効位置による）: 実効位置の行送り軸は改行送り後の 12。\
         書き換え前の現行値は 0（相対 0 指定が改行を取り消していた）"
    );

    // 保留カーソルあり: 実効位置の X は先の保留 10 なので `@5` は 15。
    let after_pending_cursor = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::CursorMove {
            x: CursorCoord::Relative {
                value: 5.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &after_pending_cursor,
        1,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(
        inline_positions(&lines[0]),
        vec![15.0],
        "正典値（4.2 の実効位置による）: 保留カーソル(X=10)を仮適用した実効位置から\
         `@5` → 15。走査位置(0)を基点にすると 5 になる"
    );

    // 保留改行と保留カーソルが**同時に**保留された状態（実経路で到達可能——`\n` →
    // `\_l[絶対]` → `\_l[@…]`。`\_l` 腕は `pending` を消費しないため）。
    // 仮適用の**順序**そのものを固定する 1 本＝主張は「保留改行が行内軸を先頭へ戻す規則より、
    // 保留カーソルの上書きが後勝ちである」（フラッシュ本体＝`layout.rs` ゲート②の (2)→(3) と同順）。
    let break_then_cursor = [
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 10.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::CursorMove {
            x: CursorCoord::Relative {
                value: 0.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &break_then_cursor,
        1,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 1, "先頭フラッシュは空行を作らない");
    assert_eq!(
        inline_positions(&lines[0]),
        vec![10.0],
        "正典値（4.2 の実効位置による）: 仮適用は (2) 保留改行 → (3) 保留カーソル の順\
         ＝フラッシュ本体（`layout.rs` ゲート②）と同順なので、改行の行内リセット(0)に\
         保留カーソルの X=10 が後勝ちする。実効位置 (10, 12) から `@0` は据え置き＝10。\
         仮適用の 2 ブロックを入れ替えると (3)→(2) になり 0 へ落ちる"
    );
    assert_eq!(
        lines[0].rect.top, 12.0,
        "行送り軸は保留改行の送り 12（保留カーソルは Y を動かしていない）"
    );
}
