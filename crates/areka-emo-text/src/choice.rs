//! # choice — 選択肢固有の純粋導出の集約モジュール（純粋層）
//!
//! `Choice` cue が state 層へ載せた選択肢スパン（[`ChoiceSpan`]・items のグリフ序数範囲）を、
//! layout 層が配置した行列（[`PositionedLine`]）へ写し、行×選択肢の注釈
//! （[`LineChoiceSegment`]）を導出する純関数群を単独所有する。描画（装飾）とヒット照会は
//! この単一の注釈導出を源にすることで座標整合を構造保証する（design.md「ChoicePure」・R3.3）。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻・lib.rs 構造檻へ登録）。
//! 全関数は同一入力→同一出力の純関数で失敗経路を持たない（縮退は値で表現）。
//!
//! ## 序数空間の統一（部分リビール整合・design.md「Invariants」）
//!
//! [`ChoiceSpan::glyph_range`] は **items 全体のグリフ序数空間**（`Glyph` のみを数える序数）で
//! 表される。一方 [`annotate_lines`] が受け取る `lines` は **可視 prefix 適用済み**の配置済み
//! グリフ（layout 出力）——追記順の先頭から `visible_count` 個だけが `PositionedLine::glyphs`
//! に現れる。両者はともに「グリフのみ・追記順」で数えるため、配置済みグリフを行を跨いで
//! 0 起点の連番で数えた値が、そのまま items 全体のグリフ序数に一致する。したがって
//! スパン範囲と各行のカバー範囲を**純整数の累積和**で交差判定すれば、可視 prefix より後ろの
//! グリフは配置済み総数（＝`visible_count`）を超えないため交差が自然に打ち切られる
//! （`min(glyph_range.end, visible_count)` の実現・浮動小数比較を交差判定に用いない・R3.4）。
//!
//! ## 折返し跨ぎの行別分割（design.md「Risks」）
//!
//! 1 スパンのグリフ範囲が折返し境界を跨ぐ場合、跨いだ行ごとに 1 つの [`LineChoiceSegment`] を
//! 出す（`\q` は emo2 正典上は自動折返ししないが、構造的に正しく扱う）。

use areka_parsers::balloon::{BalloonCursor, CursorColor};
use tracing::warn;

use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, HighlightPaint, ResidentContent,
};
use crate::layout::{LineRect, PositionedLine};
use crate::region::{ScaleContract, TextRegion};
use crate::state::ChoiceSpan;
use crate::writing::WritingMode;

/// 行×選択肢セグメント注釈（配置済み行へのスパン写像・折返し跨ぎは行ごと分割）。
///
/// `inline_range` は行内軸の image px 絶対範囲（先頭グリフの配置位置〜末尾グリフの配置位置＋
/// 送り幅）。行内軸は 3 方向とも正方向（横書き＝x・縦書き＝y）で、[`PositionedGlyph::inline_pos`]
/// が既にその軸の絶対値ゆえ本注釈は writing mode に依存しない（窓物理写像＝mode 依存の写像は
/// 後段 `to_window_physical` の責務）。
///
/// [`PositionedGlyph::inline_pos`]: crate::layout::PositionedGlyph::inline_pos
#[derive(Clone, Debug, PartialEq)]
pub struct LineChoiceSegment {
    /// セグメントが属する行の index（`lines` に対する添字）。
    pub line_index: usize,
    /// スパンの配送順序数（[`ChoiceSpan::ordinal`]・hover／照会の主キー）。
    pub ordinal: usize,
    /// 行内軸 image px 絶対範囲（先頭グリフ位置〜末尾グリフ位置＋送り幅）。
    pub inline_range: (f32, f32),
}

/// 注釈導出（純粋）: layout 出力の行グリフ列を序数走査してスパンを行セグメントへ写す。
///
/// 各スパンの [`ChoiceSpan::glyph_range`]（items 全体のグリフ序数空間）を、配置済みグリフを
/// 行を跨いで 0 起点で数えた累積和と純整数で交差させ、交差した行ごとに
/// [`LineChoiceSegment`] を 1 つ出す（折返し跨ぎは行別分割・design.md「Risks」）。
///
/// - 空範囲スパン（`start >= end`・空 `text` 選択肢）はセグメントを生まない（非退行）。
/// - 可視 prefix（部分リビール）の打切りは構造的に成立する: `lines` の配置済みグリフ総数は
///   `visible_count` ゆえ、スパン末尾がそれを超えても行カバー範囲との交差が
///   `min(glyph_range.end, visible_count)` で自然に切れる（浮動小数比較を交差判定に用いない）。
/// - 出力順はスパン順（＝ordinal 昇順）× 行昇順。
///
/// 同一入力→同一出力（純粋・決定論）。失敗経路なし（全入力で値を返す純関数）。
pub fn annotate_lines(lines: &[PositionedLine], spans: &[ChoiceSpan]) -> Vec<LineChoiceSegment> {
    let mut segments = Vec::new();
    for span in spans {
        let span_start = span.glyph_range.start;
        let span_end = span.glyph_range.end;
        // 空範囲スパン（空 text 選択肢）はセグメントを生まない（非退行・design.md Invariants）。
        if span_start >= span_end {
            continue;
        }
        // 配置済みグリフを行を跨いで 0 起点で数えた累積和（= items 全体のグリフ序数と一致）。
        let mut line_start = 0usize;
        for (line_index, line) in lines.iter().enumerate() {
            let line_end = line_start + line.glyphs.len();
            // 純整数の交差 [span_start, span_end) ∩ [line_start, line_end)。
            let lo = span_start.max(line_start);
            let hi = span_end.min(line_end);
            if lo < hi {
                // 行内ローカル添字へ変換（lo/hi は行を跨いだ通し序数）。
                let first = &line.glyphs[lo - line_start];
                let last = &line.glyphs[hi - 1 - line_start];
                segments.push(LineChoiceSegment {
                    line_index,
                    ordinal: span.ordinal,
                    inline_range: (first.inline_pos, last.inline_pos + last.advance),
                });
            }
            line_start = line_end;
        }
    }
    segments
}

/// ハイライト帯／ヒット帯のブロック軸寸を決める（純粋・**描画とヒットの唯一の源**・R3.3）。
///
/// ## なぜ行矩形（em ボックス）では足りないか（実機不具合の真因）
///
/// layout の行矩形は em ボックス丈（`font_height`）だが、DirectWrite は行を
/// **`ascent + descent`**（[`GlyphMetrics::line_box_height`]）で組む。和文フォントでは
/// これが em を大きく超える（実測: Yu Gothic UI ＝ 1.3301em ゆえ font.height 28 で 37.24px・
/// インクは em ボックス下端より 4.27px はみ出す）。帯を `font_height` で切ると
/// **descent のインクが帯の外に出る**——バルーン背景が白・hover 文字が白の実 fixture では
/// 帯の外の文字が背景に溶けて「文字の下が切れて見える」（実機サインオフで観測された不具合）。
/// ＭＳ ゴシックは比がちょうど 1.0 のため既定フォントの檻では観測されない（既定フォント盲点）。
///
/// ## 帯の決め方（下限＝em ボックス・上限＝行送りピッチ）
///
/// ```text
/// band = clamp(line_box_height, font_height, max(font_height, line_pitch))
/// ```
///
/// - **下限 `font_height`**: 行矩形より痩せさせない（従来挙動の非退行）。
/// - **上限 `line_pitch`**: 帯が行送りピッチを超えると**隣接行の帯と重なる**——ハイライトが
///   隣の行を侵し、かつヒット矩形が重なって同一点が 2 選択肢に当たる（照会の一意性が壊れる）。
///   ゆえにピッチで頭打ちにする。実測 Yu Gothic UI 28px（行送り 30 ＝ 28 + 行間 2）では
///   `clamp(37.24, 28, max(28, 30)) = 30`——次行の帯へ食い込まない。
///   行ボックス丈 37.24 は覆いきれず、実フォントの読み戻しでは文字のインクが帯の下端から
///   **1 画素**はみ出すが、これは開発者の裁定（2026-09-06）で許容している。帯を広げると
///   隣接行の帯と重なって「どの選択肢を指しているか」の一意性が壊れるため広げない
///   （はみ出しが 2 画素以上になったら帯を広げず、数値を添えて改めて裁定を仰ぐ）。
///   `line_pitch < font_height` の病的設定では下限が勝つ（帯 ＝ `font_height`）。
/// - 行送り比 `\n[ratio]` で ratio < 1 を指定した行間縮小時は帯が隣接行へ届き得る（M1 既知の
///   縮退——正典 fixture は ratio ≥ 1）。
///
/// 同一入力→同一出力（純粋・決定論）。失敗経路なし。
pub fn highlight_band_extent(font_height: f32, line_box_height: f32, line_pitch: f32) -> f32 {
    let upper = line_pitch.max(font_height);
    line_box_height.clamp(font_height, upper)
}

/// ヒット行（純粋・canvas-local image px）: 1 選択肢セグメントの矩形＋配送順序数。
///
/// `rect` は **canvas-local（validrect-local）image px**——[`ContentCanvas::from_layout`] が
/// 住人へ与える座標系と同一（validrect 原点を差し引いた空間）。ゆえにハイライト描画矩形と
/// **数値まで一致**する（正典確定「ハイライト矩形＝ヒット矩形と同一」・R3.3）。窓物理 px への
/// 写像は [`to_window_physical`]（`region` 原点 × k を戻し committed を反映）の責務。
///
/// [`ContentCanvas::from_layout`]: crate::canvas::ContentCanvas::from_layout
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasHitRow {
    /// スパンの配送順序数（[`ChoiceSpan::ordinal`]・照会の主キー）。
    pub ordinal: usize,
    /// ヒット矩形（canvas-local＝validrect-local image px・ハイライト矩形と同座標系）。
    pub rect: LineRect,
}

/// バルーン窓 client 座標系の物理 px 矩形（f32・Send 純データ・choice.rs 所有）。
///
/// [`to_window_physical`] の出力＝スクロール `committed`（面反映済み whole-pixel）を反映済みの
/// 窓物理 px。actor.rs の照会契約 API はこの型を再輸出する（design Data Model）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRectPx {
    /// 左辺（窓物理 px）。
    pub left: f32,
    /// 上辺（窓物理 px）。
    pub top: f32,
    /// 右辺（窓物理 px）。
    pub right: f32,
    /// 下辺（窓物理 px）。
    pub bottom: f32,
}

/// ヒット行導出（純粋・canvas-local image px）: セグメント×行矩形からヒット矩形を組む。
///
/// 各 [`LineChoiceSegment`] について、**行内軸範囲＝セグメントの `inline_range`（＝選択肢グリフ
/// 範囲の文字幅。行全幅ではない・正典確定「クリック領域幅＝文字幅」）**、**ブロック軸帯＝行矩形の
/// block 近端から `band_extent` 分**（[`highlight_band_extent`] が決める descent 込みの帯——
/// 行矩形の em ボックス丈ではない。ハイライト描画（`highlight_rect`）と同一の値を受け取ることで
/// 帯が数値一致する・R3.3）を軸読み替え正準表で x/y へ割り当て、絶対 image px のヒット矩形を組む。これを
/// [`ContentCanvas::from_layout`] と**同一の原点差引き**（`- region.left()` / `- region.top()`）で
/// canvas-local（validrect-local）へ写す——ゆえにハイライト描画（`decorate_canvas`）が同じ
/// `LineChoiceSegment`＋行矩形から組む矩形と数値一致する（表示とヒットの単一導出・R3.3・
/// 正典確定「ハイライト矩形＝ヒット矩形と同一」）。
///
/// - 空範囲セグメント（`i0 >= i1`）は行を生まない（annotate は既に除外済みだが防御）。
/// - `line_index` 範囲外は防御的にスキップ（annotate は有効添字のみ出す）。
/// - 出力順は入力 `segments` 順（＝annotate 出力＝ordinal 昇順×行昇順）。
///
/// **設計注記（`region` 引数）**: design Service Interface の型枠は `region` を欠くが、
/// §座標写像式（正本）は入力を「canvas-local（validrect-local）」と明記し
/// [`to_window_physical`] が `region` 原点を戻す。layout 出力（`inline_range`／行矩形）は
/// **絶対 image px**（描画開始点＝validrect 原点起点）ゆえ、canvas-local 化には validrect 原点が
/// 必須。よって本関数は `region` を受け取り原点差引きを行う（追加・新規関数・呼び手は task 8）。
///
/// 同一入力→同一出力（純粋・決定論）。失敗経路なし。
///
/// [`ContentCanvas::from_layout`]: crate::canvas::ContentCanvas::from_layout
pub fn derive_hit_rows(
    lines: &[PositionedLine],
    segments: &[LineChoiceSegment],
    mode: WritingMode,
    region: &TextRegion,
    band_extent: f32,
) -> Vec<CanvasHitRow> {
    let (ox, oy) = (region.left(), region.top());
    let mut rows = Vec::new();
    for seg in segments {
        let (i0, i1) = seg.inline_range;
        // 空範囲/逆順セグメントは行を生まない（防御・annotate は既に除外済み）。
        if i0 >= i1 {
            continue;
        }
        // line_index 範囲外は防御的にスキップ（annotate は有効添字のみ出す）。
        let Some(line) = lines.get(seg.line_index) else {
            continue;
        };
        // 行内軸＝セグメントの inline_range（文字幅）・ブロック軸帯＝行矩形の block 近端から
        // band_extent 分を軸読み替え正準表で x/y へ割り当て絶対 image px のヒット矩形を組む
        // （横書き＝行内 x／ブロック y・縦書き＝行内 y／ブロック x）。block 近端（横書き＝rect.top・
        // 縦書き＝rect.left）は描画（住人 transform の offset＝行矩形の近端）と同一の起点ゆえ、
        // band_extent が同値なら帯は描画と数値一致する（R3.3・`band_extent == font_height` なら
        // 従来の行矩形 block 帯と完全一致＝非退行）。
        let abs = match mode {
            WritingMode::HorizontalTb => LineRect {
                left: i0,
                top: line.rect.top,
                right: i1,
                bottom: line.rect.top + band_extent,
            },
            WritingMode::VerticalRl | WritingMode::VerticalLr => LineRect {
                left: line.rect.left,
                top: i0,
                right: line.rect.left + band_extent,
                bottom: i1,
            },
        };
        // canvas-local 化: from_layout と同一の validrect 原点差引き（x へ left・y へ top）。
        rows.push(CanvasHitRow {
            ordinal: seg.ordinal,
            rect: LineRect {
                left: abs.left - ox,
                top: abs.top - oy,
                right: abs.right - ox,
                bottom: abs.bottom - oy,
            },
        });
    }
    rows
}

/// canvas-local ヒット矩形 → バルーン窓 client 物理 px 矩形（純粋・§座標写像式の正本実装）。
///
/// ```text
/// 行内軸:   phys = (region_inline_origin + inline) × k
/// ブロック軸: phys = (region_block_origin + block) × k + committed
/// ```
///
/// - `k`＝[`ScaleContract::scale`]（×k 一点適用・**DPI 追従により k≠1.0 が実供給される**）。
/// - `committed`＝面反映済み whole-pixel スクロール（物理 px・符号済み・`ScrollPlanner::scroll_state`）。
///   ブロック軸のみ加算する（行内軸は不動）。
/// - `region_*_origin`＝validrect 原点（[`TextRegion::left`]/[`TextRegion::top`]）——TextSurface の
///   窓内装着 offset（`validrect 原点 × k`）と同源ゆえ結果はバルーン窓 client 物理 px に一致する。
/// - 軸割当は writing_mode 正準表: **horizontal_tb**＝行内 x／ブロック y・
///   **vertical_rl/lr**＝行内 y／ブロック x（committed はブロック軸へ）。
///
/// 同一入力→同一出力（純粋・決定論）。失敗経路なし。
pub fn to_window_physical(
    row: &CanvasHitRow,
    region: &TextRegion,
    mode: WritingMode,
    committed: i32,
    contract: &ScaleContract,
) -> HitRectPx {
    let k = contract.scale;
    let committed = committed as f32;
    let (ox, oy) = (region.left(), region.top());
    let r = &row.rect;
    // 行内軸: phys = (region_inline_origin + inline) × k
    // ブロック軸: phys = (region_block_origin + block) × k + committed
    // 軸割当（正準表）: horizontal_tb＝行内 x／ブロック y・vertical_rl/lr＝行内 y／ブロック x。
    // committed（面反映済み物理スクロール）はブロック軸のみ加算する。
    match mode {
        WritingMode::HorizontalTb => HitRectPx {
            left: (ox + r.left) * k,
            top: (oy + r.top) * k + committed,
            right: (ox + r.right) * k,
            bottom: (oy + r.bottom) * k + committed,
        },
        WritingMode::VerticalRl | WritingMode::VerticalLr => HitRectPx {
            left: (ox + r.left) * k + committed,
            top: (oy + r.top) * k,
            right: (ox + r.right) * k + committed,
            bottom: (oy + r.bottom) * k,
        },
    }
}

/// canvas 装飾（純粋）: 選択肢セグメントを含む GlyphRun 住人を Choice 住人へ置換する。
///
/// [`annotate_lines`] が出した行×選択肢セグメント（[`LineChoiceSegment`]）を源に、当該行の
/// [`ResidentContent::GlyphRun`] 住人を [`ResidentContent::Choice`]（[`ChoiceLineContent`]）へ
/// 置換し、hover 印（[`ChoiceLineContent::hovered`]）と解決済みハイライト塗り
/// （[`HighlightPaint`]＝canvas.rs 純データ型）を焼き込む。`style`→`paint` の正規化
/// （Invert の `255−c` 式含む）はこの一点で行い、下流（viewbox/COM）は choice.rs へ依存せず
/// 純データだけを読む（design.md「純粋層 / ChoicePure」・要件 1.1/4.2/4.3/4.5）。
///
/// **セグメント空 → canvas を無変更で返す**（恒等・非退行・要件 1.4/design.md Invariants）。
///
/// `band_extent`（[`highlight_band_extent`] の出力）は Choice 住人へそのまま焼き込み、COM 層の
/// ハイライト矩形とダーティ帯がこの単一値を読む——[`derive_hit_rows`] へ渡す値と同一にすることで
/// 描画帯とヒット帯の数値一致（R3.3）を呼び手 1 箇所で担保する。
///
/// ## 座標系: 絶対 image px → resident-local（GlyphRunContent ローカル系）
///
/// [`LineChoiceSegment::inline_range`] は**絶対 image px**（描画開始点＝validrect 原点起点）だが、
/// [`ChoiceRowSegment::inline_range`] は**resident-local**（[`GlyphRunContent`] ローカル 0 起点）で
/// なければならない（[`ContentCanvas::from_layout`] がグリフを行内原点差引きで住人ローカルへ写すのと
/// 同一系）。よって行内軸の resident 原点＝`region.<inline_origin>() + resident.transform.offset.<inline>`
/// （＝行矩形 `rect.<inline>`——`from_layout` がグリフから差し引くのと同じ原点）を絶対範囲から
/// 差し引く。これにより装飾側の座標が [`derive_hit_rows`] の canvas-local と数値整合する（R3.3）:
/// 住人変換で戻すと `(絶対 − rect.inline) + (rect.inline − region_origin) = 絶対 − region_origin`＝
/// `derive_hit_rows` の出力に一致する。
///
/// ## 住人写像（line_index ↔ resident は 1:1）
///
/// [`ContentCanvas::from_layout`] は layout の各行に住人を 1 つ順番どおり生成するため、
/// [`LineChoiceSegment::line_index`] は `canvas.residents` を直接添字する。
/// - セグメントを持つ GlyphRun 住人 → Choice 住人へ置換（当該行の全セグメントを集約）。
///   `\q\q` 並置（同一行複数 ordinal）は 1 つの Choice 住人へ集約・折返し跨ぎ（同一 ordinal が
///   2 行）は 2 つの Choice 住人（両方がその ordinal hover 時に highlight）。
/// - hover: `hover == Some(o)` かつ**この行に ordinal `o` のセグメントがある**とき
///   [`hovered`](ChoiceLineContent::hovered)＝`Some(o)`・さもなくば `None`。hover 行のみ
///   `style.paint(default_font_color)` を焼く（[`NoMarker`](ResolvedChoiceStyle::NoMarker) は
///   `None`＝hover でも塗らない）。非 hover 行のセグメント持ち住人も Choice 住人になる
///   （セグメント記録・`hovered=None`・`highlight=None`）。
/// - Image/Surface 住人・セグメントを持たない GlyphRun 住人は素通し（無変更）。
///
/// 同一入力→同一出力（純粋・決定論）。失敗経路なし。
///
/// [`GlyphRunContent`]: crate::canvas::GlyphRunContent
/// [`ContentCanvas::from_layout`]: crate::canvas::ContentCanvas::from_layout
pub fn decorate_canvas(
    canvas: ContentCanvas,
    segments: &[LineChoiceSegment],
    hover: Option<usize>,
    style: ResolvedChoiceStyle,
    default_font_color: (u8, u8, u8),
    region: &TextRegion,
    mode: WritingMode,
    band_extent: f32,
) -> ContentCanvas {
    // セグメント空は恒等（非退行・要件 1.4）——入力 canvas をそのまま返す。
    if segments.is_empty() {
        return canvas;
    }
    let mut canvas = canvas;
    for (index, resident) in canvas.residents.iter_mut().enumerate() {
        // GlyphRun 以外（Image/Surface/既 Choice）は素通し。run はローカル系のまま複製する。
        let run = match &resident.content {
            ResidentContent::GlyphRun(run) => run.clone(),
            _ => continue,
        };
        // 行内軸の resident 原点（絶対 image px）＝ validrect 原点 + 住人 inline offset ＝ 行矩形 inline。
        // from_layout がグリフから差し引く原点と同一——絶対 inline_range をこれで resident-local 化する。
        let (ox, oy) = resident.transform.offset();
        let inline_origin = match mode {
            WritingMode::HorizontalTb => region.left() + ox,
            WritingMode::VerticalRl | WritingMode::VerticalLr => region.top() + oy,
        };
        // この行に属するセグメント（line_index 一致）を resident-local 範囲へ写して集約する。
        let row_segments: Vec<ChoiceRowSegment> = segments
            .iter()
            .filter(|s| s.line_index == index)
            .map(|s| ChoiceRowSegment {
                ordinal: s.ordinal,
                inline_range: (
                    s.inline_range.0 - inline_origin,
                    s.inline_range.1 - inline_origin,
                ),
            })
            .collect();
        // セグメントを持たない GlyphRun 住人は素通し（無変更）。
        if row_segments.is_empty() {
            continue;
        }
        // hover 印: hover==Some(o) かつ この行に ordinal o のセグメントがあるときのみ Some(o)。
        let hovered = match hover {
            Some(o) if row_segments.iter().any(|rs| rs.ordinal == o) => Some(o),
            _ => None,
        };
        // ハイライト塗り: hover 行のみ style→paint 正規形を焼く（NoMarker は None＝hover でも塗らない）。
        let highlight = if hovered.is_some() {
            style
                .paint(default_font_color)
                .map(|(fill, text)| HighlightPaint { fill, text })
        } else {
            None
        };
        resident.content = ResidentContent::Choice(ChoiceLineContent {
            run,
            segments: row_segments,
            hovered,
            highlight,
            band_extent,
        });
    }
    canvas
}

/// ハイライトスタイル差替シーム（cursor.\* 解決＋矩形反転縮退＋将来非正典スタイルの開放口）。
///
/// balloon の `cursor.*` スタイルモデル（[`BalloonCursor`]）を hover ハイライトの描画正規形へ
/// 解決する純粋 enum（design.md「純粋層 / ChoicePure」・要件 4.2/4.3/6.1/6.5）。解決は
/// [`resolve`](ResolvedChoiceStyle::resolve) が一点で行い、描画実行側は [`paint`](ResolvedChoiceStyle::paint)
/// が返す `(塗り色, hover 文字色)` 正規形のみを読む（下流 viewbox/COM は本 enum に依存しない）。
///
/// `#[non_exhaustive]` により将来の非正典スタイル variant 追加を後方互換にする（要件 6.2）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResolvedChoiceStyle {
    /// cursor.\* 指定形（fixture 実導出・要件 4.2）: 矩形塗り色＋hover 文字色。
    ///
    /// `fill`＝`cursor.brush.color`（矩形内色）・`text`＝`cursor.font.color`（hover 文字色）。
    /// fixture 実導出形＝`SquareFill { fill: (105, 25, 25), text: (255, 255, 255) }`（square 塗り＋白文字）。
    SquareFill {
        /// 矩形内塗り色（`cursor.brush.color`）。
        fill: (u8, u8, u8),
        /// hover 文字色（`cursor.font.color`）。
        text: (u8, u8, u8),
    },
    /// cursor.\* 未指定バルーンの矩形反転縮退（要件 4.3/6.1・M1 実導出＝縮退ではない）。
    ///
    /// [`paint`](ResolvedChoiceStyle::paint) が塗り＝バルーン既定 `font.color`・文字＝各成分 `255−c` を返す
    /// （既定黒文字なら黒矩形＋白文字＝古典反転と同観）。
    Invert,
    /// `cursor.style,none`（正典・マーカー無し）。[`paint`](ResolvedChoiceStyle::paint) は `None` を返す。
    NoMarker,
}

impl ResolvedChoiceStyle {
    /// balloon `cursor.*` モデル＋バルーン既定文字色から hover ハイライトスタイルを解決する（純粋）。
    ///
    /// 判定（design.md 縮退表＋正典確定）:
    /// - `cursor` 不在、または cursor.\* 全キー未指定 → [`Invert`](ResolvedChoiceStyle::Invert)
    ///   （未指定バルーン判定・M1 実導出・縮退ではない＝warn なし・要件 4.3/6.1）。
    /// - `cursor.style,none` → [`NoMarker`](ResolvedChoiceStyle::NoMarker)（正典・マーカー無し）。
    /// - `cursor.style` underline 系（`underline`／`square+underline`）→ warn-once（解決時 1 回）＋
    ///   [`SquareFill`](ResolvedChoiceStyle::SquareFill) へ縮退（在る色を採る・要件 6.5）。
    /// - `cursor.style` square または style 未指定（既定 square）で他キー在り →
    ///   [`SquareFill`](ResolvedChoiceStyle::SquareFill)（`fill=brush.color`・`text=font.color`・要件 4.2）。
    /// - `cursor.blendmethod` が ROP 系（`none` 以外）→ warn-once（解決時 1 回）＋`none` 扱い
    ///   （色ベース描画・variant は style 判定どおりで不変・要件 6.5）。
    ///
    /// `default_font_color` は [`Invert`](ResolvedChoiceStyle::Invert) の paint 材料であり本メソッドの
    /// 分岐には用いないが、将来の非正典 variant がバルーン既定色を焼き込む拡張余地として受ける
    /// （安定シーム）。同一入力→同一出力の純関数（失敗経路なし・縮退は値＋呼び手警告で表現）。
    pub fn resolve(cursor: Option<&BalloonCursor>, default_font_color: (u8, u8, u8)) -> Self {
        let _ = default_font_color;
        // cursor 不在 → 未指定バルーン＝Invert（4.3/6.1）。
        let Some(cursor) = cursor else {
            return ResolvedChoiceStyle::Invert;
        };
        // cursor.* 全キー未指定 → 未指定バルーン＝Invert（4.3/6.1・縮退ではない＝warn なし）。
        if cursor_all_unspecified(cursor) {
            return ResolvedChoiceStyle::Invert;
        }
        // blendmethod ROP 系（none 以外）→ warn-once＋none 扱い（色ベース描画・variant 不変・6.5）。
        if let Some(bm) = cursor.blendmethod() {
            if !bm.eq_ignore_ascii_case("none") {
                warn!(
                    blendmethod = bm,
                    "cursor.blendmethod ROP 系は M1 未対応: none 扱い（色ベース描画）へ縮退"
                );
            }
        }
        // style 解決（none→NoMarker・underline 系→warn＋SquareFill 縮退・それ以外＝既定 square→SquareFill）。
        match cursor.style() {
            Some(s) if s.eq_ignore_ascii_case("none") => ResolvedChoiceStyle::NoMarker,
            Some(s) if style_has_underline(s) => {
                warn!(
                    style = s,
                    "cursor.style underline 系は M1 未対応: SquareFill へ縮退"
                );
                square_fill_from(cursor)
            }
            // square 明示・style 未指定（既定 square）ともに SquareFill（正典確定 cursor.* マップ）。
            _ => square_fill_from(cursor),
        }
    }

    /// 描画実行の一点写像（純粋）: `(塗り色, hover 文字色)` 正規形を返す。
    ///
    /// - [`SquareFill`](ResolvedChoiceStyle::SquareFill) → `Some((fill, text))`（`default_font_color` 非依存）。
    /// - [`Invert`](ResolvedChoiceStyle::Invert) → `Some((default_font_color, (255−r, 255−g, 255−b)))`
    ///   （塗り＝バルーン既定 font 色・文字＝各成分の補色・α不変・要件 4.3）。
    /// - [`NoMarker`](ResolvedChoiceStyle::NoMarker) → `None`（マーカー無し＝素描画）。
    pub fn paint(&self, default_font_color: (u8, u8, u8)) -> Option<((u8, u8, u8), (u8, u8, u8))> {
        match *self {
            ResolvedChoiceStyle::SquareFill { fill, text } => Some((fill, text)),
            ResolvedChoiceStyle::Invert => {
                let (r, g, b) = default_font_color;
                Some((default_font_color, (255 - r, 255 - g, 255 - b)))
            }
            ResolvedChoiceStyle::NoMarker => None,
        }
    }
}

/// cursor.\* 全キー未指定（style/blendmethod/brush・pen・font 各色成分がすべて `None`）を判定する。
///
/// 全キー未指定＝「未指定バルーン」（`ResolvedChoiceStyle::resolve` が `Invert` へ写す・要件 4.3/6.1）。
/// いずれか 1 キーでも指定されていれば「指定バルーン」として扱う（style 未指定なら既定 square）。
fn cursor_all_unspecified(cursor: &BalloonCursor) -> bool {
    cursor.style().is_none()
        && cursor.blendmethod().is_none()
        && color_unspecified(cursor.brush_color())
        && color_unspecified(cursor.pen_color())
        && color_unspecified(cursor.font_color())
}

/// `CursorColor` の r/g/b 全成分が `None`（未指定）かを判定する。
fn color_unspecified(c: CursorColor) -> bool {
    c.r().is_none() && c.g().is_none() && c.b().is_none()
}

/// `cursor.style` が underline 系（`underline`／`square+underline`）かを判定する（大小無視）。
fn style_has_underline(style: &str) -> bool {
    style.to_ascii_lowercase().contains("underline")
}

/// `SquareFill { fill=brush.color, text=font.color }` を組む（欠落成分は `0` 既定・防御）。
///
/// 正典 fixture では brush/font とも全成分指定ゆえ既定は発火しない。style を持つが色を欠く
/// 防御経路では各成分 `0` を採る（決定論・design は部分色の既定を規定しないため最小既定）。
fn square_fill_from(cursor: &BalloonCursor) -> ResolvedChoiceStyle {
    ResolvedChoiceStyle::SquareFill {
        fill: color_tuple(cursor.brush_color()),
        text: color_tuple(cursor.font_color()),
    }
}

/// `CursorColor` を `(u8, u8, u8)` へ写す（欠落成分は `0` 既定・防御）。
fn color_tuple(c: CursorColor) -> (u8, u8, u8) {
    (c.r().unwrap_or(0), c.g().unwrap_or(0), c.b().unwrap_or(0))
}

#[cfg(test)]
#[path = "choice_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "choice_style_resolve_tests.rs"]
mod style_resolve_tests;

#[cfg(test)]
#[path = "choice_decorate_tests.rs"]
mod decorate_tests;
