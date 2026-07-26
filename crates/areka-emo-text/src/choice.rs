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
///   ゆえにピッチで頭打ちにする。実測 Yu Gothic UI 28px では `min(37.24, 35) = 35`——
///   インク下端（em + 4.27 ＝ 32.27px）を覆いつつ次行の帯へ食い込まない。
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
                inline_range: (s.inline_range.0 - inline_origin, s.inline_range.1 - inline_origin),
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
mod tests {
    use super::*;
    use crate::layout::{LineRect, PositionedGlyph, PositionedLine};
    use crate::state::ChoiceSpan;

    /// 行内軸位置 `positions` の各グリフ（送り幅 `adv`）を持つ横書き行を作る
    /// （行矩形は本注釈で未使用ゆえ最小値を入れる）。
    fn line(positions: &[(f32, f32)]) -> PositionedLine {
        let glyphs = positions
            .iter()
            .map(|&(inline_pos, advance)| PositionedGlyph {
                ch: 'あ',
                inline_pos,
                advance,
            })
            .collect();
        PositionedLine {
            rect: LineRect {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            },
            glyphs,
        }
    }

    /// glyph_range だけを指定した ChoiceSpan（id/label/references は注釈非関与ゆえ最小値）。
    fn span(ordinal: usize, range: core::ops::Range<usize>) -> ChoiceSpan {
        ChoiceSpan {
            ordinal,
            id: String::new(),
            label: String::new(),
            references: vec![],
            glyph_range: range,
        }
    }

    /// 単一行・単一スパン → 行内範囲が「先頭グリフ位置〜末尾グリフ位置＋送り幅」の 1 セグメント。
    #[test]
    fn single_line_single_span_yields_one_segment_with_inline_range() {
        let lines = [line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)])];
        let spans = [span(0, 0..3)];
        let segs = annotate_lines(&lines, &spans);
        assert_eq!(
            segs,
            vec![LineChoiceSegment {
                line_index: 0,
                ordinal: 0,
                inline_range: (0.0, 30.0), // 先頭 0.0 〜 末尾 20.0 + advance 10.0
            }]
        );
    }

    /// 半角/全角混在の送り幅でも inline_range は先頭位置〜末尾位置＋末尾送り幅で決まる。
    #[test]
    fn single_span_inline_range_uses_first_pos_and_last_pos_plus_advance() {
        // 'a'(6) 'あ'(12) 'b'(6): 位置 0,6,18。範囲 0..2 は先頭 0 〜 6+12=18。
        let lines = [line(&[(0.0, 6.0), (6.0, 12.0), (18.0, 6.0)])];
        let spans = [span(0, 0..2)];
        let segs = annotate_lines(&lines, &spans);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].inline_range, (0.0, 18.0));
    }

    /// 同一行に複数スパン（正典 `\q\q` 並置）→ スパン順に 2 セグメント（それぞれの行内範囲）。
    #[test]
    fn two_spans_on_one_line_yield_two_segments() {
        let lines = [line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0), (30.0, 10.0)])];
        let spans = [span(0, 0..2), span(1, 2..4)];
        let segs = annotate_lines(&lines, &spans);
        assert_eq!(
            segs,
            vec![
                LineChoiceSegment {
                    line_index: 0,
                    ordinal: 0,
                    inline_range: (0.0, 20.0),
                },
                LineChoiceSegment {
                    line_index: 0,
                    ordinal: 1,
                    inline_range: (20.0, 40.0),
                },
            ]
        );
    }

    /// Observable: 2 行に折り返された 3 スパン入力 → 行別分割のセグメント。
    /// span1 が折返し境界（line0 末尾グリフ 2 と line1 先頭グリフ 3）を跨ぎ、行ごとに分割される。
    #[test]
    fn three_spans_folded_across_two_lines_split_per_line() {
        // line0: 序数 0,1,2（位置 0,10,20）・line1: 序数 3,4,5（位置 0,10,20）。
        let lines = [
            line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)]),
            line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)]),
        ];
        let spans = [
            span(0, 0..2), // line0 内（序数 0,1）
            span(1, 2..4), // line0 末尾(2) ＋ line1 先頭(3) を跨ぐ
            span(2, 4..6), // line1 内（序数 4,5）
        ];
        let segs = annotate_lines(&lines, &spans);
        assert_eq!(
            segs,
            vec![
                // span0: line0 のみ。
                LineChoiceSegment {
                    line_index: 0,
                    ordinal: 0,
                    inline_range: (0.0, 20.0),
                },
                // span1: 折返し跨ぎ → line0 と line1 の 2 セグメントへ分割。
                LineChoiceSegment {
                    line_index: 0,
                    ordinal: 1,
                    inline_range: (20.0, 30.0), // line0 末尾グリフ（序数 2）: 位置 20 〜 20+10
                },
                LineChoiceSegment {
                    line_index: 1,
                    ordinal: 1,
                    inline_range: (0.0, 10.0), // line1 先頭グリフ（序数 3）: 位置 0 〜 0+10
                },
                // span2: line1 のみ。
                LineChoiceSegment {
                    line_index: 1,
                    ordinal: 2,
                    inline_range: (10.0, 30.0), // line1 グリフ（序数 4,5）: 位置 10 〜 20+10
                },
            ]
        );
    }

    /// 空範囲スパン（空 text 選択肢）はセグメントを生まない（非退行・design.md Invariants）。
    #[test]
    fn empty_range_span_produces_no_segment() {
        let lines = [line(&[(0.0, 10.0), (10.0, 10.0)])];
        // start == end（空範囲）。glyph_range が行内でも行末でも生まない。
        let spans = [span(0, 1..1), span(1, 2..2)];
        let segs = annotate_lines(&lines, &spans);
        assert!(segs.is_empty(), "空範囲スパンはセグメントを生まない: {segs:?}");
    }

    /// 部分リビール: 配置済みグリフ（可視 prefix）がスパン末尾より短いとき、
    /// 交差は配置済みグリフ数で打ち切られる（min(glyph_range.end, visible_count)）。
    #[test]
    fn partial_reveal_truncates_span_to_placed_glyphs() {
        // スパン範囲 0..5 だが配置済み（可視）は 3 グリフのみ（1 行）。
        let lines = [line(&[(0.0, 10.0), (10.0, 10.0), (20.0, 10.0)])];
        let spans = [span(0, 0..5)];
        let segs = annotate_lines(&lines, &spans);
        assert_eq!(
            segs,
            vec![LineChoiceSegment {
                line_index: 0,
                ordinal: 0,
                inline_range: (0.0, 30.0), // 配置済み末尾グリフ（序数 2）まで: 位置 20 + 10
            }],
            "可視 prefix より後ろは打ち切られる"
        );
    }

    /// 部分リビールが折返し跨ぎと合わさる場合: 可視が line1 の途中で切れると、
    /// line1 セグメントは配置済みグリフまで（超過分は line1 に存在しない＝交差せず）。
    #[test]
    fn partial_reveal_across_wrap_truncates_second_line() {
        // line0: 序数 0,1（配置 2）・line1: 序数 2,3（配置 2）。総配置 4。
        let lines = [
            line(&[(0.0, 10.0), (10.0, 10.0)]),
            line(&[(0.0, 10.0), (10.0, 10.0)]),
        ];
        // スパン範囲 1..6（line0 の 1、line1 の 2,3、そして未配置の 4,5 を含む指定）。
        let spans = [span(0, 1..6)];
        let segs = annotate_lines(&lines, &spans);
        assert_eq!(
            segs,
            vec![
                LineChoiceSegment {
                    line_index: 0,
                    ordinal: 0,
                    inline_range: (10.0, 20.0), // line0 序数 1: 位置 10 + 10
                },
                LineChoiceSegment {
                    line_index: 1,
                    ordinal: 0,
                    inline_range: (0.0, 20.0), // line1 序数 2,3: 位置 0 〜 10+10（未配置 4,5 は打切り）
                },
            ]
        );
    }

    /// 空の lines・空の spans は空セグメント（純粋・恒等）。
    #[test]
    fn empty_inputs_yield_empty_segments() {
        assert!(annotate_lines(&[], &[span(0, 0..2)]).is_empty());
        assert!(annotate_lines(&[line(&[(0.0, 10.0)])], &[]).is_empty());
        assert!(annotate_lines(&[], &[]).is_empty());
    }

    // ── タスク 5.2: ヒット行導出（derive_hit_rows）と窓物理写像（to_window_physical） ──
    //
    // 座標系: layout 出力（inline_range／行矩形）は絶対 image px（描画開始点＝validrect 原点起点）。
    // CanvasHitRow.rect は canvas-local（validrect-local）——from_layout と同一の原点差引きゆえ
    // ハイライト矩形と数値一致（正典確定「ハイライト矩形＝ヒット矩形と同一」・R3.3）。
    // 窓物理化は to_window_physical: 行内=(origin+inline)×k・ブロック=(origin+block)×k+committed。

    use crate::region::{ScaleContract, TextRegion};
    use crate::writing::WritingMode;
    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

    /// validrect 原点を持つ `TextRegion`（非負 validrect 素通し・derive/写像は left()/top() のみ使用）。
    fn region(left: i32, top: i32, right: i32, bottom: i32) -> TextRegion {
        let model = BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(None, None),
            WordWrapPoint::new(None, None),
            ValidRect::new(Some(top), Some(bottom), Some(left), Some(right)),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
            None,
        );
        TextRegion::resolve(&model, (400, 224), WritingMode::HorizontalTb)
    }

    /// 行矩形だけ指定した `PositionedLine`（derive_hit_rows は行矩形の block 帯のみ使用）。
    fn prow(left: f32, top: f32, right: f32, bottom: f32) -> PositionedLine {
        PositionedLine {
            rect: LineRect {
                left,
                top,
                right,
                bottom,
            },
            glyphs: vec![],
        }
    }

    /// `(line_index, ordinal, inline_range)` の [`LineChoiceSegment`]。
    fn seg(line_index: usize, ordinal: usize, range: (f32, f32)) -> LineChoiceSegment {
        LineChoiceSegment {
            line_index,
            ordinal,
            inline_range: range,
        }
    }

    /// 既存ケースのハイライト帯＝行矩形の em ボックス丈（font 10）——`band_extent == font_height`
    /// のとき従来（行矩形 block 帯）と完全一致することを既存期待値がそのまま檻にする（非退行）。
    const HIT_BAND: f32 = 10.0;

    // ── derive_hit_rows: canvas-local ヒット矩形（文字幅・block 帯・原点差引き） ──

    /// 横書き・原点 (0,0): ヒット矩形の行内範囲＝セグメントの inline_range（**文字幅**）・
    /// ブロック帯＝行矩形の top..bottom。行全幅（0..50）ではなくセグメント範囲（10..30）で切る。
    #[test]
    fn derive_horizontal_uses_char_width_inline_and_line_block_band() {
        let region = region(0, 0, 400, 224);
        // 行全幅 0..50・block 帯 0..10（font 10）。
        let lines = [prow(0.0, 0.0, 50.0, 10.0)];
        let segs = [seg(0, 7, (10.0, 30.0))]; // 選択肢グリフ範囲（文字幅）＝10..30。
        let rows = derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND);
        assert_eq!(
            rows,
            vec![CanvasHitRow {
                ordinal: 7,
                rect: LineRect {
                    left: 10.0, // 行全幅 0..50 ではなくセグメント 10..30（文字幅）。
                    top: 0.0,
                    right: 30.0,
                    bottom: 10.0,
                },
            }]
        );
    }

    /// 横書き・非零 validrect 原点 (36,46): from_layout と同一の原点差引きで canvas-local 化する
    /// （絶対 image px の行矩形/セグメントから validrect 原点を引く＝ハイライト矩形と同座標系）。
    #[test]
    fn derive_horizontal_subtracts_validrect_origin_to_canvas_local() {
        let region = region(36, 46, 356, 168);
        // 絶対 image px: 行矩形 left36/top46/right86/bottom56・セグメント 46..66。
        let lines = [prow(36.0, 46.0, 86.0, 56.0)];
        let segs = [seg(0, 0, (46.0, 66.0))];
        let rows = derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND);
        assert_eq!(
            rows[0].rect,
            LineRect {
                left: 10.0,   // 46 − 36
                top: 0.0,     // 46 − 46
                right: 30.0,  // 66 − 36
                bottom: 10.0, // 56 − 46
            },
            "絶対 image px から validrect 原点を差し引いた canvas-local"
        );
    }

    /// 縦書き（vertical_rl / vertical_lr）: 行内軸＝y（inline_range）・ブロック軸＝x（行矩形の
    /// left..right 帯）。軸割当が横書きと入れ替わる（正準表）。
    #[test]
    fn derive_vertical_assigns_inline_to_y_and_block_to_x() {
        let region = region(0, 0, 400, 224);
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            // 縦書き列矩形: x 帯 377..387（block・font 10）・y 全長 0..20。
            let lines = [prow(377.0, 0.0, 387.0, 20.0)];
            let segs = [seg(0, 1, (5.0, 15.0))]; // 行内軸（y）の文字幅範囲。
            let rows = derive_hit_rows(&lines, &segs, mode, &region, HIT_BAND);
            assert_eq!(
                rows[0].rect,
                LineRect {
                    left: 377.0, // block 帯＝行矩形 left（x）
                    top: 5.0,    // inline＝セグメント i0（y）
                    right: 387.0,
                    bottom: 15.0,
                },
                "{mode:?}: 行内=y・ブロック=x の軸割当"
            );
        }
    }

    /// 複数セグメント（同一行 2 個＋別行 1 個）→ 入力順どおり 3 行、それぞれ ordinal と矩形を保持。
    #[test]
    fn derive_multiple_segments_yield_rows_in_input_order() {
        let region = region(0, 0, 400, 224);
        let lines = [prow(0.0, 0.0, 40.0, 10.0), prow(0.0, 13.0, 30.0, 23.0)];
        let segs = [
            seg(0, 0, (0.0, 20.0)),
            seg(0, 1, (20.0, 40.0)),
            seg(1, 2, (0.0, 30.0)),
        ];
        let rows = derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].ordinal, 0);
        assert_eq!(rows[0].rect, LineRect { left: 0.0, top: 0.0, right: 20.0, bottom: 10.0 });
        assert_eq!(rows[1].ordinal, 1);
        assert_eq!(rows[1].rect, LineRect { left: 20.0, top: 0.0, right: 40.0, bottom: 10.0 });
        assert_eq!(rows[2].ordinal, 2);
        assert_eq!(rows[2].rect, LineRect { left: 0.0, top: 13.0, right: 30.0, bottom: 23.0 });
    }

    /// 空セグメント列 → 空のヒット行（非退行）。
    #[test]
    fn derive_empty_segments_yield_no_rows() {
        let region = region(0, 0, 400, 224);
        let lines = [prow(0.0, 0.0, 30.0, 10.0)];
        assert!(derive_hit_rows(&lines, &[], WritingMode::HorizontalTb, &region, HIT_BAND).is_empty());
    }

    /// 空範囲セグメント（`i0 >= i1`）は行を生まない（防御・annotate 既除外）。
    #[test]
    fn derive_empty_range_segment_produces_no_row() {
        let region = region(0, 0, 400, 224);
        let lines = [prow(0.0, 0.0, 30.0, 10.0)];
        let segs = [seg(0, 0, (10.0, 10.0)), seg(0, 1, (20.0, 15.0))];
        assert!(
            derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND).is_empty(),
            "空/逆順範囲はヒット行を生まない"
        );
    }

    /// `line_index` 範囲外セグメントは防御的にスキップ（annotate は有効添字のみ出す）。
    #[test]
    fn derive_out_of_range_line_index_is_skipped() {
        let region = region(0, 0, 400, 224);
        let lines = [prow(0.0, 0.0, 30.0, 10.0)];
        let segs = [seg(5, 0, (0.0, 10.0))];
        assert!(derive_hit_rows(&lines, &segs, WritingMode::HorizontalTb, &region, HIT_BAND).is_empty());
    }

    // ── to_window_physical: §座標写像式（行内=(origin+inline)×k・ブロック=(origin+block)×k+committed） ──

    /// canvas-local (10,0,30,10)・原点 (36,46)。横書きは committed をブロック軸（y=top/bottom）へ。
    #[test]
    fn to_window_physical_horizontal_applies_formula() {
        let region = region(36, 46, 356, 168);
        let row = CanvasHitRow {
            ordinal: 0,
            rect: LineRect { left: 10.0, top: 0.0, right: 30.0, bottom: 10.0 },
        };
        // k=2・committed=50: 行内 x=(36+inline)×2・ブロック y=(46+block)×2+50。
        let contract = ScaleContract::new(2.0, None);
        assert_eq!(
            to_window_physical(&row, &region, WritingMode::HorizontalTb, 50, &contract),
            HitRectPx {
                left: 92.0,   // (36+10)×2
                top: 142.0,   // (46+0)×2 + 50
                right: 132.0, // (36+30)×2
                bottom: 162.0 // (46+10)×2 + 50
            }
        );
    }

    /// 縦書き（rl/lr）は committed をブロック軸（x=left/right）へ・行内軸（y）は ×k のみ。
    #[test]
    fn to_window_physical_vertical_puts_committed_on_x_block_axis() {
        let region = region(36, 46, 356, 168);
        let row = CanvasHitRow {
            ordinal: 0,
            rect: LineRect { left: 10.0, top: 0.0, right: 30.0, bottom: 10.0 },
        };
        let contract = ScaleContract::new(2.0, None);
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            assert_eq!(
                to_window_physical(&row, &region, mode, 50, &contract),
                HitRectPx {
                    left: 142.0,  // (36+10)×2 + 50（x=ブロック）
                    top: 92.0,    // (46+0)×2（y=行内・committed 非加算）
                    right: 182.0, // (36+30)×2 + 50
                    bottom: 112.0 // (46+10)×2
                },
                "{mode:?}: committed は x（ブロック軸）のみ"
            );
        }
    }

    /// k∈{1.0,2.0}×committed∈{0,50}×3 方向の全網羅パラメタライズ（独立算出の期待値と一致）。
    #[test]
    fn to_window_physical_parameterized_over_k_committed_and_modes() {
        let region = region(36, 46, 356, 168);
        let (ox, oy) = (36.0f32, 46.0f32);
        let row = CanvasHitRow {
            ordinal: 0,
            rect: LineRect { left: 10.0, top: 2.0, right: 30.0, bottom: 12.0 },
        };
        let r = row.rect;
        for k in [1.0f32, 2.0] {
            let contract = ScaleContract::new(k, None);
            for c in [0i32, 50] {
                let cf = c as f32;
                for mode in [
                    WritingMode::HorizontalTb,
                    WritingMode::VerticalRl,
                    WritingMode::VerticalLr,
                ] {
                    let got = to_window_physical(&row, &region, mode, c, &contract);
                    let expected = match mode {
                        // horizontal: x=行内（×k）・y=ブロック（×k+committed）。
                        WritingMode::HorizontalTb => HitRectPx {
                            left: (ox + r.left) * k,
                            top: (oy + r.top) * k + cf,
                            right: (ox + r.right) * k,
                            bottom: (oy + r.bottom) * k + cf,
                        },
                        // vertical: x=ブロック（×k+committed）・y=行内（×k）。
                        WritingMode::VerticalRl | WritingMode::VerticalLr => HitRectPx {
                            left: (ox + r.left) * k + cf,
                            top: (oy + r.top) * k,
                            right: (ox + r.right) * k + cf,
                            bottom: (oy + r.bottom) * k,
                        },
                    };
                    assert_eq!(got, expected, "k={k} committed={c} mode={mode:?}");
                }
            }
        }
    }

    /// 現行契約 k=1.0・committed=0（非スクロール）: 窓物理 px は絶対 image px と一致
    /// （canvas-local + 原点 = 絶対）——2 空間モデルの恒等（R2.2 の DPI=96 経路）。
    #[test]
    fn to_window_physical_unit_scale_no_scroll_equals_absolute_image_px() {
        let region = region(36, 46, 356, 168);
        let row = CanvasHitRow {
            ordinal: 0,
            rect: LineRect { left: 10.0, top: 0.0, right: 30.0, bottom: 10.0 },
        };
        let contract = ScaleContract::new(1.0, None);
        assert_eq!(
            to_window_physical(&row, &region, WritingMode::HorizontalTb, 0, &contract),
            HitRectPx { left: 46.0, top: 46.0, right: 66.0, bottom: 56.0 } // = 絶対 image px
        );
    }

    // ── R3.3: 描画とヒットの単一導出（derive_hit_rows と from_layout の canvas-local が一致） ──

    /// Observable: 固定入力に対し、derive_hit_rows の canvas-local 矩形は、ハイライト描画が
    /// 同一 `LineChoiceSegment`＋行矩形から `from_layout` と同一の原点差引きで組む矩形と
    /// **完全一致**する（表示とヒットが同一導出パス＝座標整合の構造保証）。
    #[test]
    fn hit_row_rect_matches_canvas_local_highlight_derivation() {
        let region = region(36, 46, 356, 168);
        let line = prow(36.0, 46.0, 86.0, 56.0); // 絶対 image px の行矩形。
        let segment = seg(0, 3, (46.0, 66.0)); // 絶対 image px のセグメント inline 範囲。
        let rows = derive_hit_rows(
            std::slice::from_ref(&line),
            std::slice::from_ref(&segment),
            WritingMode::HorizontalTb,
            &region,
            HIT_BAND,
        );
        // ハイライト描画が使う canvas-local 矩形を from_layout と同一手順で独立算出:
        // 行矩形 block 近端（top）＋帯（HIT_BAND）＋セグメント inline 範囲（i0/i1）を validrect 原点差引き。
        let (ox, oy) = (region.left(), region.top());
        let (i0, i1) = segment.inline_range;
        let expected_highlight = LineRect {
            left: i0 - ox,
            top: line.rect.top - oy,
            right: i1 - ox,
            bottom: line.rect.top + HIT_BAND - oy,
        };
        assert_eq!(
            rows[0].rect, expected_highlight,
            "ヒット矩形とハイライト矩形は同一 canvas-local 座標（単一導出）"
        );
    }

    // ── descent 込みハイライト帯（highlight_band_extent／derive_hit_rows の帯適用） ──
    //
    // 実機不具合（実 fixture Yu Gothic UI 28px で hover 文字の下が帯からはみ出す）の真因は
    // 「帯＝em ボックス丈（font_height）」だった。帯は実 font metrics の行ボックス丈
    // （ascent+descent）を源にし、行送りピッチで頭打ちにする。

    /// 行ボックス丈が em ボックスとピッチの間にあるとき、帯は**行ボックス丈そのもの**
    /// （descent を覆う。ＭＳ ゴシック系＝比 1.0 は em と一致し従来どおり）。
    #[test]
    fn band_extent_takes_line_box_height_between_em_and_pitch() {
        // font 28・行ボックス 32.0・ピッチ 35 → 32.0（行ボックス丈が採られる）。
        assert_eq!(highlight_band_extent(28.0, 32.0, 35.0), 32.0);
        // 比 1.0 のフォント（ＭＳ ゴシック実測）は em ボックス丈と一致＝従来挙動（非退行）。
        assert_eq!(highlight_band_extent(28.0, 28.0, 35.0), 28.0);
    }

    /// 行ボックス丈が行送りピッチを超えるときはピッチで頭打ち（隣接行の帯／ヒット矩形と
    /// 重ならせない）。実測 Yu Gothic UI 28px（行ボックス 37.242・ピッチ 35）＝35。
    #[test]
    fn band_extent_is_capped_by_line_pitch() {
        assert_eq!(highlight_band_extent(28.0, 37.242, 35.0), 35.0);
        // 隣接行の帯は接するだけで重ならない（行送り 35 ＋ 帯 35）。
        assert!(highlight_band_extent(28.0, 37.242, 35.0) <= 35.0);
    }

    /// 下限は em ボックス丈——行ボックス丈がそれより小さくても行矩形より痩せない（非退行）。
    /// ピッチが em を下回る病的設定でも下限が勝つ。
    #[test]
    fn band_extent_never_below_font_height() {
        assert_eq!(highlight_band_extent(28.0, 20.0, 35.0), 28.0);
        assert_eq!(highlight_band_extent(28.0, 37.0, 20.0), 28.0);
    }

    /// Observable（本不具合の純粋層側の檻）: 帯が em ボックス丈を超えるとき、ヒット矩形の
    /// ブロック軸下端は**行矩形の bottom（em ボックス下端）より下**へ伸びる——描画側
    /// `highlight_rect`（住人原点＋band_extent）と同一の式ゆえ数値一致する（R3.3）。
    #[test]
    fn hit_row_block_band_extends_beyond_em_box_when_band_is_larger() {
        let region = region(36, 46, 356, 168);
        // 実 fixture 相当: 行矩形 block 帯 46..74（font 28）・帯 35（Yu Gothic UI 実測の頭打ち値）。
        let line = prow(36.0, 46.0, 200.0, 74.0);
        let segment = seg(0, 0, (36.0, 100.0));
        let band = highlight_band_extent(28.0, 37.242, 35.0);
        let rows = derive_hit_rows(
            std::slice::from_ref(&line),
            std::slice::from_ref(&segment),
            WritingMode::HorizontalTb,
            &region,
            band,
        );
        assert_eq!(rows[0].rect.top, 0.0, "帯の起点は行矩形 block 近端（46−46）");
        assert_eq!(
            rows[0].rect.bottom, 35.0,
            "帯の終端は近端＋band_extent（em ボックス下端 28 ではない＝descent を覆う）"
        );
        assert!(
            rows[0].rect.bottom > line.rect.bottom - region.top(),
            "em ボックス丈（28）より下へ伸びる"
        );
    }

    /// 縦書きも同一規則: 帯は行矩形の block 近端（left）から band_extent 分（右方向）。
    #[test]
    fn hit_row_vertical_block_band_uses_band_extent_from_left_edge() {
        let region = region(0, 0, 400, 224);
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            let lines = [prow(377.0, 0.0, 387.0, 20.0)]; // block 帯 377..387（font 10）。
            let segs = [seg(0, 1, (5.0, 15.0))];
            let rows = derive_hit_rows(&lines, &segs, mode, &region, 13.0);
            assert_eq!(
                rows[0].rect,
                LineRect { left: 377.0, top: 5.0, right: 390.0, bottom: 15.0 },
                "{mode:?}: block 帯は left から band_extent（13）分"
            );
        }
    }
}

// ── タスク 5.3: ハイライトスタイル解決（ResolvedChoiceStyle::resolve / paint） ──
//
// design.md「純粋層 / ChoicePure」Service Interface（ResolvedChoiceStyle enum + resolve/paint）・
// 縮退表（cursor.style underline 系→SquareFill／cursor.blendmethod ROP 系→none 扱い／
// cursor.* 全キー未指定→Invert）・正典確定（cursor.* マップ「既定 square」・fixture 実導出形＝
// square 塗り(105,25,25)＋白文字／矩形反転縮退「塗り=既定 font.color・文字=各成分 255−c」）。
#[cfg(test)]
mod style_resolve_tests {
    use super::*;
    use areka_parsers::balloon::{BalloonCursor, CursorColor};

    /// fixture 実導出の cursor.*（square・brush=(105,25,25)・font=(255,255,255)・pen/blend 未指定）。
    fn fixture_cursor() -> BalloonCursor {
        BalloonCursor::new(
            Some("square".to_string()),
            CursorColor::new(Some(105), Some(25), Some(25)), // brush.color＝矩形内色
            CursorColor::new(None, None, None),              // pen.color（M1 非参照）
            CursorColor::new(Some(255), Some(255), Some(255)), // font.color＝hover 白文字
            None,                                            // blendmethod（既定 none）
        )
    }

    /// cursor 全体を組む簡易ビルダ。
    fn cursor(
        style: Option<&str>,
        brush: (Option<u8>, Option<u8>, Option<u8>),
        font: (Option<u8>, Option<u8>, Option<u8>),
        blend: Option<&str>,
    ) -> BalloonCursor {
        BalloonCursor::new(
            style.map(str::to_string),
            CursorColor::new(brush.0, brush.1, brush.2),
            CursorColor::new(None, None, None),
            CursorColor::new(font.0, font.1, font.2),
            blend.map(str::to_string),
        )
    }

    // ── resolve: 未指定 → Invert（M1 実導出・縮退ではない） ──

    /// cursor 不在（`None`）→ Invert（未指定バルーン判定・4.3/6.1）。
    #[test]
    fn resolve_none_cursor_is_invert() {
        assert_eq!(
            ResolvedChoiceStyle::resolve(None, (0, 0, 0)),
            ResolvedChoiceStyle::Invert
        );
    }

    /// cursor.* 全キー未指定（`BalloonCursor::default()`）→ Invert（4.3/6.1）。
    #[test]
    fn resolve_all_unspecified_cursor_is_invert() {
        let c = BalloonCursor::default();
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (12, 34, 56)),
            ResolvedChoiceStyle::Invert
        );
    }

    // ── resolve: style=none → NoMarker ──

    /// `cursor.style,none`（正典・マーカー無し）→ NoMarker。
    #[test]
    fn resolve_style_none_is_no_marker() {
        let c = cursor(Some("none"), (None, None, None), (None, None, None), None);
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::NoMarker
        );
    }

    // ── resolve: fixture square → SquareFill{(105,25,25),(255,255,255)} ──

    /// fixture 実導出形（square＋brush(105,25,25)＋font(255,255,255)）→ SquareFill。
    #[test]
    fn resolve_fixture_square_is_square_fill() {
        let c = fixture_cursor();
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::SquareFill {
                fill: (105, 25, 25),
                text: (255, 255, 255),
            }
        );
    }

    /// style 未指定でも色/キーが在れば「既定 square」→ SquareFill（正典確定 cursor.* マップ）。
    #[test]
    fn resolve_specified_colors_without_style_defaults_to_square_fill() {
        let c = cursor(
            None,
            (Some(105), Some(25), Some(25)),
            (Some(255), Some(255), Some(255)),
            None,
        );
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::SquareFill {
                fill: (105, 25, 25),
                text: (255, 255, 255),
            }
        );
    }

    // ── resolve: underline 系 → warn-once + SquareFill 縮退（縮退表・6.5） ──

    /// `cursor.style,underline` → SquareFill へ縮退（在る色を採る・warn は解決時 1 回）。
    #[test]
    fn resolve_underline_degrades_to_square_fill() {
        let c = cursor(
            Some("underline"),
            (Some(105), Some(25), Some(25)),
            (Some(255), Some(255), Some(255)),
            None,
        );
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::SquareFill {
                fill: (105, 25, 25),
                text: (255, 255, 255),
            }
        );
    }

    /// `cursor.style,square+underline`（underline 系）→ SquareFill へ縮退。
    #[test]
    fn resolve_square_plus_underline_degrades_to_square_fill() {
        let c = cursor(
            Some("square+underline"),
            (Some(105), Some(25), Some(25)),
            (Some(255), Some(255), Some(255)),
            None,
        );
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::SquareFill {
                fill: (105, 25, 25),
                text: (255, 255, 255),
            }
        );
    }

    // ── resolve: blendmethod ROP 系 → warn-once + none 扱い（variant は変えない・6.5） ──

    /// `cursor.blendmethod,notmaskpen`（ROP 系）→ none 扱い（色ベース）へ縮退し variant は
    /// style（square）どおり SquareFill のまま（blend は無視・warn は解決時 1 回）。
    #[test]
    fn resolve_rop_blendmethod_is_treated_as_none_and_keeps_square_fill() {
        let c = cursor(
            Some("square"),
            (Some(105), Some(25), Some(25)),
            (Some(255), Some(255), Some(255)),
            Some("notmaskpen"),
        );
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::SquareFill {
                fill: (105, 25, 25),
                text: (255, 255, 255),
            }
        );
    }

    /// `cursor.blendmethod,none`（大小無視）は縮退警告を出さず variant はスタイルどおり。
    #[test]
    fn resolve_blendmethod_none_keeps_square_fill_without_degrade() {
        let c = cursor(
            Some("square"),
            (Some(105), Some(25), Some(25)),
            (Some(255), Some(255), Some(255)),
            Some("none"),
        );
        assert_eq!(
            ResolvedChoiceStyle::resolve(Some(&c), (0, 0, 0)),
            ResolvedChoiceStyle::SquareFill {
                fill: (105, 25, 25),
                text: (255, 255, 255),
            }
        );
    }

    // ── paint: 描画実行用 (fill, text) 正規形 ──

    /// SquareFill.paint → Some((fill, text))（dfc 非依存）。
    #[test]
    fn paint_square_fill_returns_fill_and_text() {
        let s = ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        };
        assert_eq!(s.paint((77, 88, 99)), Some(((105, 25, 25), (255, 255, 255))));
    }

    /// Invert.paint（dfc=(0,0,0)）→ 塗り=(0,0,0)・文字=各成分 255−c=(255,255,255)（古典反転同観）。
    #[test]
    fn paint_invert_black_default_is_black_fill_white_text() {
        assert_eq!(
            ResolvedChoiceStyle::Invert.paint((0, 0, 0)),
            Some(((0, 0, 0), (255, 255, 255)))
        );
    }

    /// Invert.paint（dfc=(10,20,30)）→ 塗り=既定 font 色・文字=(245,235,225)（各成分 255−c）。
    #[test]
    fn paint_invert_uses_default_font_fill_and_per_component_complement() {
        assert_eq!(
            ResolvedChoiceStyle::Invert.paint((10, 20, 30)),
            Some(((10, 20, 30), (245, 235, 225)))
        );
    }

    /// NoMarker.paint → None（マーカー無し＝素描画・dfc 非依存）。
    #[test]
    fn paint_no_marker_is_none() {
        assert_eq!(ResolvedChoiceStyle::NoMarker.paint((10, 20, 30)), None);
    }
}

// ── タスク 5.4: canvas 装飾（decorate_canvas）——GlyphRun 住人 → Choice 住人置換 ──
//
// design.md「純粋層 / ChoicePure」・要件 1.1（Choice cue→行 resident 描画）/4.2（cursor.* 指定→
// SquareFill）/4.3（未指定→Invert 反転）/4.5（描画実行の一点写像＝焼込済み純データ）。
// 座標系: LineChoiceSegment.inline_range は絶対 image px・ChoiceRowSegment.inline_range は
// resident-local（from_layout がグリフから行内原点を差し引くのと同一原点で変換）。
#[cfg(test)]
mod decorate_tests {
    use super::*;
    use crate::canvas::{GlyphRunContent, RegionTransform, Resident, TextEffects};
    use crate::layout::PositionedGlyph;
    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

    /// validrect 原点を持つ `TextRegion`（left()/top() のみ inline 原点差引きに使用）。
    fn region(left: i32, top: i32, right: i32, bottom: i32) -> TextRegion {
        let model = BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(None, None),
            WordWrapPoint::new(None, None),
            ValidRect::new(Some(top), Some(bottom), Some(left), Some(right)),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
            None,
        );
        TextRegion::resolve(&model, (400, 224), WritingMode::HorizontalTb)
    }

    /// 変換 offset `offset` の GlyphRun 住人（グリフ内容は装飾非関与ゆえ最小の 1 グリフ）。
    fn glyph_resident(offset: (f32, f32)) -> Resident {
        Resident {
            content: ResidentContent::GlyphRun(GlyphRunContent {
                glyphs: vec![PositionedGlyph {
                    ch: 'あ',
                    inline_pos: 0.0,
                    advance: 10.0,
                }],
                size: (10.0, 10.0),
            }),
            transform: RegionTransform::translation(offset.0, offset.1),
            effects: TextEffects::default(),
        }
    }

    /// 住人列から canvas を組む（size は validrect 寸相当の固定値）。
    fn canvas(residents: Vec<Resident>) -> ContentCanvas {
        ContentCanvas {
            residents,
            size: (400.0, 224.0),
        }
    }

    /// `(line_index, ordinal, 絶対 inline_range)` の [`LineChoiceSegment`]。
    fn seg(line_index: usize, ordinal: usize, range: (f32, f32)) -> LineChoiceSegment {
        LineChoiceSegment {
            line_index,
            ordinal,
            inline_range: range,
        }
    }

    /// 装飾テストのハイライト帯（em ボックス丈 10.0 と**異なる**値＝焼込みが観測可能）。
    const TEST_BAND: f32 = 13.0;

    /// 住人が Choice ならその中身を取り出す（さもなくば panic）。
    fn choice(resident: &Resident) -> &ChoiceLineContent {
        match &resident.content {
            ResidentContent::Choice(c) => c,
            other => panic!("Choice 住人を期待したが {other:?}"),
        }
    }

    /// fixture 実導出の SquareFill スタイル（square 塗り(105,25,25)＋白文字）。
    fn square_fill() -> ResolvedChoiceStyle {
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        }
    }

    // ── 恒等（非退行）: セグメント空 → canvas 無変更 ──

    /// セグメント空 → 入力 canvas をまったく同一で返す（恒等・要件 1.4）。
    #[test]
    fn empty_segments_returns_canvas_unchanged() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0)), glyph_resident((0.0, 13.0))]);
        let out = decorate_canvas(
            input.clone(),
            &[],
            Some(0),
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        assert_eq!(out, input, "セグメント空は恒等（無変更）");
    }

    // ── hover: 一致行のみ highlight・他行 None ──

    /// hover=Some(0) → ordinal 0 を持つ行のみ hovered/highlight が付き、他行は None。
    #[test]
    fn hover_sets_highlight_on_matching_line_only() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0)), glyph_resident((0.0, 13.0))]);
        let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
        let out = decorate_canvas(
            input,
            &segments,
            Some(0),
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        let l0 = choice(&out.residents[0]);
        assert_eq!(l0.hovered, Some(0));
        assert_eq!(
            l0.highlight,
            Some(HighlightPaint {
                fill: (105, 25, 25),
                text: (255, 255, 255),
            })
        );
        let l1 = choice(&out.residents[1]);
        assert_eq!(l1.hovered, None, "hover 対象外の行は hovered None");
        assert_eq!(l1.highlight, None, "hover 対象外の行は highlight None");
    }

    // ── hover None: セグメントは記録するが highlight は焼かない ──

    /// hover=None でもセグメント持ち行は Choice 住人化（セグメント記録・hovered/highlight None）。
    #[test]
    fn hover_none_still_records_segments_without_highlight() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0)), glyph_resident((0.0, 13.0))]);
        let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
        let out = decorate_canvas(
            input,
            &segments,
            None,
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        for (i, ordinal) in [(0usize, 0usize), (1, 1)] {
            let c = choice(&out.residents[i]);
            assert_eq!(c.hovered, None);
            assert_eq!(c.highlight, None);
            assert_eq!(c.segments.len(), 1);
            assert_eq!(c.segments[0].ordinal, ordinal);
        }
    }

    // ── 座標系: 絶対 inline_range → resident-local（行内原点差引き） ──

    /// 横書き・行内 offset 100（rect.left ≠ region.left()）: 絶対 100..120 → local 0..20
    /// （resident 原点 = region.left() + offset.0 = 100 を差し引く）。
    #[test]
    fn segment_inline_range_is_resident_local_subtracting_line_origin() {
        let region = region(0, 0, 400, 224);
        // 住人の行内 offset は 100（rect.left = region.left()(0) + 100 = 100）。
        let input = canvas(vec![glyph_resident((100.0, 0.0))]);
        let segments = [seg(0, 0, (100.0, 120.0))]; // 絶対 image px。
        let out = decorate_canvas(
            input,
            &segments,
            None,
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        let c = choice(&out.residents[0]);
        assert_eq!(
            c.segments[0].inline_range,
            (0.0, 20.0),
            "絶対 100..120 から行内原点 100 を差し引いた resident-local 0..20"
        );
    }

    /// Observable（R3.3 の帯単一化）: 装飾は受け取った `band_extent` を Choice 住人へそのまま
    /// 焼き込む（em ボックス丈 10.0 ではなく 13.0）——COM 層のハイライト矩形／ダーティ帯は
    /// この値だけを読み、`derive_hit_rows` へ渡す値と同一にすることで帯が数値一致する。
    #[test]
    fn decorate_bakes_band_extent_into_choice_residents() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0)), glyph_resident((0.0, 13.0))]);
        let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
        let out = decorate_canvas(
            input,
            &segments,
            Some(0),
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        for i in [0usize, 1] {
            assert_eq!(
                choice(&out.residents[i]).band_extent,
                TEST_BAND,
                "住人 {i}: hover 有無に依らず帯を焼き込む（hover 解除フレームのダーティ帯にも要る）"
            );
        }
    }

    /// 縦書き（vertical_rl/lr）: 行内軸＝y。行内 offset.1=50 → 絶対 50..70 は top 原点差引きで local 0..20。
    #[test]
    fn segment_inline_range_vertical_subtracts_top_origin() {
        let region = region(0, 0, 400, 224);
        for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            // 縦書きは inline 軸 = y。住人 offset.1 = 50（rect.top = region.top()(0) + 50）。
            let input = canvas(vec![glyph_resident((377.0, 50.0))]);
            let segments = [seg(0, 0, (50.0, 70.0))]; // 絶対 y 範囲。
            let out = decorate_canvas(
                input,
                &segments,
                None,
                square_fill(),
                (0, 0, 0),
                &region,
                mode,
                TEST_BAND,
            );
            let c = choice(&out.residents[0]);
            assert_eq!(
                c.segments[0].inline_range,
                (0.0, 20.0),
                "{mode:?}: 縦書きは top 原点（region.top()+offset.1）を差し引く"
            );
        }
    }

    // ── スタイル分岐: NoMarker は hover でも塗らない・Invert は dfc から解決 ──

    /// NoMarker: hover 一致でも paint→None ゆえ highlight None（hovered は Some のまま）。
    #[test]
    fn no_marker_style_yields_no_highlight_even_when_hovered() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0))]);
        let segments = [seg(0, 0, (0.0, 20.0))];
        let out = decorate_canvas(
            input,
            &segments,
            Some(0),
            ResolvedChoiceStyle::NoMarker,
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        let c = choice(&out.residents[0]);
        assert_eq!(c.hovered, Some(0), "hover 印は付く");
        assert_eq!(c.highlight, None, "NoMarker は hover でも塗らない");
    }

    /// Invert: dfc=(10,20,30)・hover 一致 → 塗り=dfc・文字=各成分 255−c=(245,235,225)。
    #[test]
    fn invert_style_resolves_highlight_from_default_font_color() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0))]);
        let segments = [seg(0, 0, (0.0, 20.0))];
        let out = decorate_canvas(
            input,
            &segments,
            Some(0),
            ResolvedChoiceStyle::Invert,
            (10, 20, 30),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        let c = choice(&out.residents[0]);
        assert_eq!(
            c.highlight,
            Some(HighlightPaint {
                fill: (10, 20, 30),
                text: (245, 235, 225),
            })
        );
    }

    // ── 集約: 同一行の複数 ordinal は 1 住人・折返し跨ぎは 2 住人 ──

    /// `\q\q` 並置（同一行 2 ordinal）→ 1 つの Choice 住人へ両セグメント集約。hover=Some(1)。
    #[test]
    fn two_choices_on_one_line_group_into_one_resident() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0))]);
        let segments = [seg(0, 0, (0.0, 20.0)), seg(0, 1, (20.0, 40.0))];
        let out = decorate_canvas(
            input,
            &segments,
            Some(1),
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        assert_eq!(out.residents.len(), 1, "住人数は不変（1 行 1 住人）");
        let c = choice(&out.residents[0]);
        assert_eq!(c.segments.len(), 2, "同一行の 2 ordinal を 1 住人へ集約");
        assert_eq!(c.segments[0].ordinal, 0);
        assert_eq!(c.segments[1].ordinal, 1);
        assert_eq!(c.hovered, Some(1), "hover 対象 ordinal 1 が住人内にある");
        assert!(c.highlight.is_some());
    }

    /// 折返し跨ぎ（同一 ordinal 0 が 2 行）→ 2 つの Choice 住人・hover=Some(0) で両方 highlight。
    #[test]
    fn wrapped_choice_highlights_both_lines() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0)), glyph_resident((0.0, 13.0))]);
        // 同一 ordinal 0 が line0 末尾と line1 先頭に跨る（annotate_lines の行別分割）。
        let segments = [seg(0, 0, (20.0, 30.0)), seg(1, 0, (0.0, 10.0))];
        let out = decorate_canvas(
            input,
            &segments,
            Some(0),
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        for i in [0usize, 1] {
            let c = choice(&out.residents[i]);
            assert_eq!(c.hovered, Some(0), "行 {i}: 跨いだ ordinal 0 が hover");
            assert!(c.highlight.is_some(), "行 {i}: 両行とも highlight");
        }
    }

    // ── 素通し: セグメントを持たない行は GlyphRun のまま ──

    /// セグメントが line 1 のみ → line 0/2 は GlyphRun 素通し・line 1 のみ Choice 化。
    #[test]
    fn lines_without_segments_stay_glyph_run() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![
            glyph_resident((0.0, 0.0)),
            glyph_resident((0.0, 13.0)),
            glyph_resident((0.0, 26.0)),
        ]);
        let segments = [seg(1, 0, (0.0, 20.0))];
        let out = decorate_canvas(
            input,
            &segments,
            None,
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        assert!(
            matches!(out.residents[0].content, ResidentContent::GlyphRun(_)),
            "セグメント無し行 0 は GlyphRun 素通し"
        );
        assert!(
            matches!(out.residents[1].content, ResidentContent::Choice(_)),
            "セグメント有り行 1 は Choice 化"
        );
        assert!(
            matches!(out.residents[2].content, ResidentContent::GlyphRun(_)),
            "セグメント無し行 2 は GlyphRun 素通し"
        );
    }

    // ── stale ordinal: hover が存在しない ordinal → どの行も highlight 無し（不変条件 3・4.5/6.5） ──

    /// hover=Some(99)（どのセグメントも持たない ordinal）→ 全住人 hovered/highlight None・
    /// ただしセグメント持ち行は Choice 住人化しセグメントは記録する（stale-safe 不変条件・
    /// Data Models §不変条件 3・要件 4.5/6.5）。
    #[test]
    fn hover_stale_ordinal_yields_no_highlight() {
        let region = region(0, 0, 400, 224);
        let input = canvas(vec![glyph_resident((0.0, 0.0)), glyph_resident((0.0, 13.0))]);
        // セグメントは ordinal 0/1 のみ。hover はそのどちらでもない 99（stale）。
        let segments = [seg(0, 0, (0.0, 20.0)), seg(1, 1, (0.0, 20.0))];
        let out = decorate_canvas(
            input,
            &segments,
            Some(99),
            square_fill(),
            (0, 0, 0),
            &region,
            WritingMode::HorizontalTb,
            TEST_BAND,
        );
        for (i, ordinal) in [(0usize, 0usize), (1, 1)] {
            let c = choice(&out.residents[i]);
            assert_eq!(c.hovered, None, "行 {i}: stale ordinal は hovered None");
            assert_eq!(c.highlight, None, "行 {i}: stale ordinal はどの行も塗らない");
            // ただしセグメントは記録される（Choice 住人化は hover に依存しない）。
            assert_eq!(c.segments.len(), 1, "行 {i}: セグメントは記録される");
            assert_eq!(c.segments[0].ordinal, ordinal);
        }
    }
}
