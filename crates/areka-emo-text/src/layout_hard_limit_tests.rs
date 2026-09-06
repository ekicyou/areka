//! # layout_hard_limit_tests — 折返し基準と描画範囲の二段構え（配置側・純粋層・兄弟テスト）
//!
//! 出典 spec: `areka-P0-emo-text-line-height-canon`（要件 **6.2**／**6.3**／**6.4**／**6.6**／
//! **3.8**／**8.4(b)(c)**・design.md §4.3「折返し基準と描画範囲の二段構え」）。
//!
//! ## 兄弟ファイルとの分担
//!
//! - `region_inline_limit_tests.rs` — **領域を解決する側**（遠辺の軸解決・粗いバルーンの警告）。
//! - 本ファイル — **配置する側**。解決済みの 2 つの値を受け取った [`LayoutEngine::layout`] が、
//!   どのグリフの遠端も描画範囲の外へ出さないことを固定する。
//! - `layout_wrap_tests.rs`／`layout_segmented_tests.rs` — 折返し基準だけで決まる既存の折返し。
//!   これらは折返し基準が描画範囲の内にある領域だけを扱うので、本仕様の前後で 1 ビットも
//!   変わらない（その不変そのものは本ファイルの「基準が内にある入力」のテストが示す）。
//!
//! ## 何を固定するか
//!
//! 行内軸には別々の意味を持つ 2 つの値が立っている。
//!
//! - **折返し基準**（`wordwrappoint`・[`TextRegion::wrap_threshold`]・以下 soft）＝
//!   「ここを超えたら折り返す」。
//! - **描画範囲の行内軸の遠辺**（`validrect` の当該辺・[`TextRegion::inline_limit`]・以下 hard）＝
//!   「ここを超えてはならない」絶対上限。超えそうなら折返し基準に関わらず無条件に折り返す。
//!
//! 固定するのは次の 4 点である。
//!
//! 1. soft が hard の**外**にある領域へ長い文字列を置くと、soft に達する前に hard で折り返され、
//!    どのグリフの遠端も hard を超えない（要件 6.2／6.3・8.4(b)）。
//! 2. 折返し方式（1 文字ずつ／分節）のどちらでも、また塊の途中であっても、配置の直前に必ず
//!    hard の判定を通る。塊の途中で発火したときは理由が読める記録（`debug!`）が残る（要件 6.6）。
//! 3. 行頭の 1 グリフだけは soft も hard も超えて配置される（無限折返しを構造で排除する）。
//! 4. soft が hard の**内**にある入力では、hard の判定が無いときと出力が完全に一致する
//!    （要件 6.4・8.4(c)）。
//!
//! ## 書字方向（要件 3.8）
//!
//! 3 方向すべてを同じ檻に通す。配置側に書字方向の分岐は無く、遠辺の軸解決（横書き＝
//! `validrect.right`・縦書き 2 方向＝`validrect.bottom`）は領域側が済ませている。よって
//! 期待値は 3 方向で**同一の行内軸の値**になり、方向を変えても行の割れ方が変わらないこと
//! そのものが「軸の読み替えだけで同じ判定が効いている」ことの証拠になる。
//!
//! ## 「基準が内なら不変」を何と比べるか
//!
//! 比較の相手は、同じ折返し基準を持ち**遠辺だけを遥か遠く（10000）へ置いた領域**での出力
//! である。遠辺がグリフの届かない位置にあれば hard の判定は決して発火しないので、その出力は
//! 本仕様を入れる前の折返し（折返し基準だけを見る規則）と定義上一致する。期待値を手で書き
//! 写す代わりにこの対照を使うのは、行の割れ方を人間が転記する過程で緩めないためである。
//!
//! ## 決定論
//!
//! 実 DPI モニタ・実 GPU・実フォント・実窓を一切要さない。[`FixedMetrics`]（font 10 →
//! 全角 'あ' の送り 10・行送り 12）と純粋層の解決だけで完結する。`windows` 系 crate を
//! import しない（純粋層の規律）。

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use log_capture_kit::{CapturedEvent, capture};

use super::test_support::{IMAGE, glyphs, inline_positions};
use super::{FixedMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::region::TextRegion;
use crate::segment::{Segment, SegmentPlan};
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;

/// 共通の文字高さ（[`FixedMetrics`] で全角 'あ' の送りが 10・行送りが 12 になる）。
const FONT: f32 = 10.0;
/// 全角 1 グリフの送り幅（`FixedMetrics::advance('あ', 10)`）。
const ADVANCE: f32 = 10.0;
/// 描画範囲の行内軸の遠辺（hard）＝ちょうど 10 グリフぶん。
const HARD: f32 = 100.0;
/// 折返し基準（soft）が遠辺の**外**にある粗いバルーンの値＝14 グリフぶん。
const SOFT_OUT: f32 = 140.0;
/// 折返し基準（soft）が遠辺の**内**にある通常のバルーンの値＝10 グリフぶん。
const SOFT_IN: f32 = 100.0;
/// 折返し基準が内にあるときの遠辺（グリフが届く範囲にある通常の値）。
const HARD_IN: f32 = 200.0;
/// 対照用の遠辺——グリフが決して届かない位置。hard の判定が発火しないので、ここでの出力は
/// 本仕様を入れる前の折返し（折返し基準だけを見る規則）と定義上一致する。
const HARD_FAR: f32 = 10000.0;

// 檻の前提を翻訳時に固定する（値を書き換えたときに、テストが緑のまま意味を失わないように）。
// 折返し基準が遠辺の内にあっては二段構えの有無を見分けられず、対照の遠辺が通常の遠辺より
// 遠くなければ「hard が発火しない対照」にならない。
const _: () = assert!(HARD < SOFT_OUT);
const _: () = assert!(SOFT_IN <= HARD_IN && HARD_IN < HARD_FAR);

/// 3 方向すべてを同じ檻に通す（要件 3.8）。
const MODES: [WritingMode; 3] = [
    WritingMode::HorizontalTb,
    WritingMode::VerticalRl,
    WritingMode::VerticalLr,
];

/// 幾何成分だけを指定する `BalloonModel`（描画開始点は 3 方向とも `(0, 0)` を字義で宣言する）。
///
/// 兄弟の `layout_test_support.rs` の `model`／`model_rect` は折返し基準と描画範囲の
/// **どちらか一方**しか渡せない。本ファイルは 2 つの値を食い違わせることが主題なので、
/// 両方を同時に指定できる形をここに置く。
fn model_of(
    wordwrap: (Option<i32>, Option<i32>),
    validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(wordwrap.0, wordwrap.1),
        ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// 領域の行内軸の開始位置（横書き＝x・縦書き 2 方向＝y——軸読み替え正準表）。
fn inline_start_of(region: &TextRegion, mode: WritingMode) -> f32 {
    match mode {
        WritingMode::HorizontalTb => region.start().0,
        WritingMode::VerticalRl | WritingMode::VerticalLr => region.start().1,
    }
}

/// 書字方向によらず「折返し基準 soft・遠辺 hard・行内開始 0」になる領域を作る。
///
/// 行内軸は横書きが x（遠辺は `validrect.right`）・縦書き 2 方向が y（遠辺は
/// `validrect.bottom`）なので、指定するキーだけが方向で入れ替わる。行内軸と直交する側は
/// どの方向でも十分に広く取り、折返しに関与させない。最後に「檻が意図した領域になって
/// いること」を確かめてから返す（前提が崩れたテストが緑のまま意味を失うのを防ぐ）。
fn region_of(mode: WritingMode, soft: f32, hard: f32) -> TextRegion {
    let (s, h) = (soft as i32, hard as i32);
    let model = match mode {
        WritingMode::HorizontalTb => {
            model_of((Some(s), None), (Some(0), Some(200), Some(0), Some(h)))
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            model_of((None, Some(s)), (Some(0), Some(h), Some(0), Some(300)))
        }
    };
    let region = TextRegion::resolve(&model, IMAGE, mode);
    assert_eq!(
        (region.wrap_threshold(), region.inline_limit()),
        (soft, hard),
        "{mode:?}: 折返し基準と遠辺が意図した値に解決されていない"
    );
    assert_eq!(
        inline_start_of(&region, mode),
        0.0,
        "{mode:?}: 行内開始が 0 でないと 3 方向の期待値を共有できない"
    );
    region
}

/// `(start, len)` 列から手組みの [`SegmentPlan`] を作る（budoux 非依存で塊境界を注入する）。
fn plan(segs: &[(usize, usize)]) -> SegmentPlan {
    SegmentPlan::from_segments(
        segs.iter()
            .map(|&(start, len)| Segment { start, len })
            .collect(),
    )
}

/// テスト用の最短経路（可視は全グリフ）。
fn layout_of(
    items: &[TextItem],
    region: &TextRegion,
    mode: WritingMode,
    wrap: WrapPlan<'_>,
) -> Vec<PositionedLine> {
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    LayoutEngine::layout(items, visible, region, mode, FONT, &FixedMetrics, wrap)
}

/// 行ごとのグリフ数（行の割れ方をひと目で比べるための形）。
fn glyph_counts(lines: &[PositionedLine]) -> Vec<usize> {
    lines.iter().map(|l| l.glyphs.len()).collect()
}

/// 全グリフの行内軸の遠端（配置位置＋送り幅）。
fn far_ends(lines: &[PositionedLine]) -> Vec<f32> {
    lines
        .iter()
        .flat_map(|l| l.glyphs.iter().map(|g| g.inline_pos + g.advance))
        .collect()
}

/// どのグリフの遠端も遠辺を超えないことを確かめる（本ファイルの中心命題）。
fn assert_within(lines: &[PositionedLine], hard: f32, context: &str) {
    for (index, far) in far_ends(lines).iter().enumerate() {
        assert!(
            *far <= hard,
            "{context}: {index} 番目のグリフの遠端 {far} が描画範囲の遠辺 {hard} を超えている"
        );
    }
}

/// 20 グリフを 4 グリフずつの塊に切った計画（塊の境界が遠辺をまたぐ形になる）。
fn plan_of_four() -> SegmentPlan {
    plan(&[(0, 4), (4, 4), (8, 4), (12, 4), (16, 4)])
}

// ── 要件 6.2／6.3／8.4(b): 折返し基準が描画範囲の外にある領域 ──

/// 折返し基準が描画範囲の外（140 > 100）にある領域へ 20 グリフを置くと、基準に達する前に
/// 描画範囲の遠辺で折り返され、どのグリフの遠端も遠辺を超えない。
///
/// 行の割れ方を手で導く（[`FixedMetrics`]・font 10 → 送り 10・行内開始 0）:
///
/// - 1 文字ずつ: 10 グリフ目の遠端が 100 でちょうど遠辺に載る（超えていないので置く）。
///   11 グリフ目は 110 で超えるから折り返す → `[10, 10]`。折返し基準だけを見る規則なら
///   14 グリフ目まで載る（`[14, 6]`）ので、期待値そのものが二段構えの有無を見分ける。
/// - 分節: 4 グリフの塊 2 つ（遠端 80）までが 1 行に入り、3 つ目の塊は残り行幅 20 に入らず
///   行頭幅 100 には入るので塊ごと次行へ移る → `[8, 8, 4]`。折返し基準だけなら 3 つ目の塊も
///   1 行目に入って `[12, 8]` になる。
///
/// 3 方向で同じ値を期待するのは、配置側に書字方向の分岐が無いこと（要件 3.8）の裏返しである。
#[test]
fn long_run_wraps_at_the_drawing_range_before_reaching_the_wrap_threshold() {
    let items = glyphs(20);
    let segments = plan_of_four();
    let cases: [(WrapPlan<'_>, Vec<usize>); 2] = [
        (WrapPlan::CharByChar, vec![10, 10]),
        (WrapPlan::Segmented(&segments), vec![8, 8, 4]),
    ];

    for mode in MODES {
        let region = region_of(mode, SOFT_OUT, HARD);
        for (wrap, expected) in &cases {
            let lines = layout_of(&items, &region, mode, *wrap);
            let context = format!("{mode:?}/{wrap:?}");
            assert_within(&lines, HARD, &context);
            assert_eq!(glyph_counts(&lines), *expected, "{context}: 行の割れ方");
            let widest = far_ends(&lines).into_iter().fold(0.0f32, f32::max);
            assert!(
                widest < SOFT_OUT,
                "{context}: どの行も折返し基準 {SOFT_OUT} に達する前に切れているはずが、\
                 最も遠いグリフの遠端が {widest} まで伸びている"
            );
        }
    }
}

// ── 要件 6.6: 塊の途中でも配置の直前に必ず遠辺の判定を通し、理由が読める記録を残す ──

/// 塊の途中で `\_l` が行内位置を進めた結果、先決済みの塊の残りが遠辺を超えるときは、塊の
/// 途中であっても折り返す。塊が途中で割れるのは先決の不変条件（塊は分割されない）に対する
/// 例外なので、`debug!` で理由（行内位置・送り幅・遠辺）を残す。
///
/// 檻の組み方: 遠辺 95・折返し基準 140（基準は外）・4 グリフ 1 塊。2 グリフを置いた時点で
/// 行内位置は 20。`\_l[80,]` が行を閉じて行内位置を 80 へ動かす。3 グリフ目は行頭なので
/// 無条件に置かれ（遠端 90）、4 グリフ目の遠端は 100 で遠辺 95 を超えるから、塊の途中でも
/// 折り返して次行の先頭へ置く。二段構えが無ければ 4 グリフ目は 90 に置かれ、遠端 100 が
/// 描画範囲の外へ出る。
#[test]
fn hard_limit_fires_inside_a_segment_and_leaves_a_readable_record() {
    const NARROW_HARD: f32 = 95.0;
    let region = region_of(WritingMode::HorizontalTb, SOFT_OUT, NARROW_HARD);
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::Glyph { ch: 'あ' },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 80.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::Glyph { ch: 'あ' },
        TextItem::Glyph { ch: 'あ' },
    ];
    let segments = plan(&[(0, 4)]);

    let (lines, events) = capture(|| {
        tracing::error!("捕捉窓が生きていることの対照イベント");
        layout_of(
            &items,
            &region,
            WritingMode::HorizontalTb,
            WrapPlan::Segmented(&segments),
        )
    });

    assert_within(&lines, NARROW_HARD, "塊の途中の遠辺判定");
    assert_eq!(
        glyph_counts(&lines),
        vec![2, 1, 1],
        "塊は途中で割れて次行へ続く（`\\_l` の直後の 1 グリフは行頭ゆえ無条件に置かれる）"
    );
    assert_eq!(inline_positions(&lines[1]), vec![80.0]);
    assert_eq!(
        inline_positions(&lines[2]),
        vec![0.0],
        "遠辺で折り返した塊の残りは次行の先頭から続く"
    );

    let records: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.level == tracing::Level::DEBUG && e.field("hard").is_some())
        .collect();
    assert_eq!(
        records.len(),
        1,
        "塊の途中で遠辺が発火した理由の記録がちょうど 1 件でない: {events:?}"
    );
    assert_eq!(number_field(records[0], "inline_pos"), 90.0);
    assert_eq!(number_field(records[0], "advance"), ADVANCE);
    assert_eq!(number_field(records[0], "hard"), NARROW_HARD);
    assert_eq!(
        events
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR)
            .count(),
        1,
        "対照イベントが数えられていない——記録の件数の主張が恒真になっている"
    );
}

/// 数値欄を f32 として読む（`{:?}` 表現の細部に依存しないよう、解析してから比べる）。
fn number_field(event: &CapturedEvent, name: &str) -> f32 {
    let raw = event
        .field(name)
        .unwrap_or_else(|| panic!("欄 {name} が記録に載っていない"));
    raw.parse::<f32>()
        .unwrap_or_else(|_| panic!("欄 {name} の値 {raw} を数値として読めない"))
}

// ── 無限折返しの構造排除: 行頭の 1 グリフだけは両方の判定を超えても置く ──

/// 折返し基準（8）も遠辺（5）も 1 グリフの送り（10）より小さい領域では、行頭の 1 グリフは
/// 両方を超えて置かれる。これが無いと「置けないから折り返す」が無限に続く。
///
/// 3 グリフを置けば 1 行 1 グリフの 3 行になる。塊を 1 グリフずつに切った分節でも同じで、
/// 塊が行頭幅にも収まらない長大塊として 1 文字ずつの規則へ委ねられるため、記録も残らない
/// （塊の途中の折返しではないので理由を書く相手がいない）。
#[test]
fn the_first_glyph_of_a_line_is_placed_even_beyond_both_limits() {
    const TINY_SOFT: f32 = 8.0;
    const TINY_HARD: f32 = 5.0;
    let items = glyphs(3);
    let segments = plan(&[(0, 1), (1, 1), (2, 1)]);
    let cases = [WrapPlan::CharByChar, WrapPlan::Segmented(&segments)];

    for mode in MODES {
        let region = region_of(mode, TINY_SOFT, TINY_HARD);
        for wrap in cases {
            let (lines, events) = capture(|| {
                tracing::error!("捕捉窓が生きていることの対照イベント");
                layout_of(&items, &region, mode, wrap)
            });
            let context = format!("{mode:?}/{wrap:?}");
            assert_eq!(
                glyph_counts(&lines),
                vec![1, 1, 1],
                "{context}: 行頭 1 グリフだけが置かれ、以降は毎行折り返す"
            );
            for line in &lines {
                assert_eq!(inline_positions(line), vec![0.0], "{context}");
            }
            assert!(
                !events
                    .iter()
                    .any(|e| e.level == tracing::Level::DEBUG && e.field("hard").is_some()),
                "{context}: 行頭の 1 グリフは例外であって、塊の途中の折返しではない: {events:?}"
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.level == tracing::Level::ERROR)
                    .count(),
                1,
                "{context}: 対照イベントが数えられていない"
            );
        }
    }
}

// ── 要件 6.4／8.4(c): 折返し基準が描画範囲の内にある入力では出力が変わらない ──

/// 折返し基準が遠辺の内（100 ≤ 200）にある通常のバルーンでは、遠辺の判定が一度も発火せず、
/// 出力は「遠辺がグリフの届かない位置にある領域」＝本仕様を入れる前の折返しと完全に一致する。
///
/// 行が 2 行以上に割れていることを併せて確かめるのは、比較が空振り（1 行しか無く折返しの
/// 判断自体が起きていない）になっていないことを示すためである。
#[test]
fn output_is_unchanged_when_the_wrap_threshold_is_inside_the_drawing_range() {
    let items = glyphs(20);
    let segments = plan_of_four();
    let cases = [WrapPlan::CharByChar, WrapPlan::Segmented(&segments)];

    for mode in MODES {
        let region = region_of(mode, SOFT_IN, HARD_IN);
        let reference = region_of(mode, SOFT_IN, HARD_FAR);
        assert_ne!(
            region.inline_limit(),
            reference.inline_limit(),
            "{mode:?}: 2 つの領域が同じでは対照にならない"
        );
        for wrap in cases {
            let lines = layout_of(&items, &region, mode, wrap);
            let expected = layout_of(&items, &reference, mode, wrap);
            let context = format!("{mode:?}/{wrap:?}");
            assert!(
                lines.len() >= 2,
                "{context}: 折返しが起きていない入力では不変の主張が空振りする"
            );
            assert_eq!(
                lines, expected,
                "{context}: 折返し基準が遠辺の内にある入力で出力が変わっている"
            );
            assert_within(&lines, HARD_IN, &context);
        }
    }
}
