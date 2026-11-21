//! # choice — 選択肢固有の純粋導出の集約モジュール（純粋層）
//!
//! `Choice` cue が state 層へ載せた選択肢スパン（[`ChoiceSpan`]・items のグリフ序数範囲）を、
//! layout 層が配置した行列（[`PositionedLine`]）へ写し、行×選択肢の注釈
//! （[`LineChoiceSegment`]）を導出する純関数群を単独所有する。描画（装飾）とヒット照会は
//! この単一の注釈導出を源にすることで座標整合を構造保証する。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻・lib.rs 構造檻へ登録）。
//! 全関数は同一入力→同一出力の純関数で失敗経路を持たない（縮退は値で表現）。
//!
//! ## 序数空間の統一（部分リビール整合）
//!
//! [`ChoiceSpan::glyph_range`] は **items 全体のグリフ序数空間**（`Glyph` のみを数える序数）で
//! 表される。一方 [`annotate_lines`] が受け取る `lines` は **可視 prefix 適用済み**の配置済み
//! グリフ（layout 出力）——追記順の先頭から `visible_count` 個だけが `PositionedLine::glyphs`
//! に現れる。両者はともに「グリフのみ・追記順」で数えるため、配置済みグリフを行を跨いで
//! 0 起点の連番で数えた値が、そのまま items 全体のグリフ序数に一致する。したがって
//! スパン範囲と各行のカバー範囲を**純整数の累積和**で交差判定すれば、可視 prefix より後ろの
//! グリフは配置済み総数（＝`visible_count`）を超えないため交差が自然に打ち切られる
//! （`min(glyph_range.end, visible_count)` の実現・浮動小数比較を交差判定に用いない）。
//!
//! ## 折返し跨ぎの行別分割
//!
//! 1 スパンのグリフ範囲が折返し境界を跨ぐ場合、跨いだ行ごとに 1 つの [`LineChoiceSegment`] を
//! 出す（`\q` は emo2 正典上は自動折返ししないが、構造的に正しく扱う）。

use areka_parsers::balloon::{BalloonCursor, CursorColor};

use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, HighlightPaint, ResidentContent,
};
use crate::layout::{GlyphMetrics, LineRect, PositionedLine};
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
/// [`LineChoiceSegment`] を 1 つ出す（折返し跨ぎは行別分割）。
///
/// - 空範囲スパン（`start >= end`・空 `text` 選択肢）はセグメントを生まない（非退行）。
/// - 可視 prefix（部分リビール）の打切りは構造的に成立する: `lines` の配置済みグリフ総数は
///   `visible_count` ゆえ、スパン末尾がそれを超えても行カバー範囲との交差が
///   `min(glyph_range.end, visible_count)` で自然に切れる（浮動小数比較を交差判定に用いない）。
/// - 出力順はスパン順（＝ordinal 昇順）× 行昇順。
pub fn annotate_lines(lines: &[PositionedLine], spans: &[ChoiceSpan]) -> Vec<LineChoiceSegment> {
    let mut segments = Vec::new();
    for span in spans {
        let span_start = span.glyph_range.start;
        let span_end = span.glyph_range.end;
        if span_start >= span_end {
            continue;
        }
        let mut line_start = 0usize;
        for (line_index, line) in lines.iter().enumerate() {
            if line_start >= span_end {
                break;
            }
            let line_end = line_start + line.glyphs.len();
            let lo = span_start.max(line_start);
            let hi = span_end.min(line_end);
            if lo < hi {
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

/// ハイライト帯／ヒット帯のブロック軸寸を決める（純粋・**描画とヒットの唯一の源**）。
///
/// ## なぜ行矩形（em ボックス）では足りないか
///
/// layout の行矩形は em ボックス丈（`font_height`）だが、DirectWrite は行を
/// **`ascent + descent`**（[`GlyphMetrics::line_box_height`]）で組む。和文フォントでは
/// これが em を大きく超える（実測: Yu Gothic UI ＝ 1.3301em ゆえ font.height 28 で 37.24px・
/// インクは em ボックス下端より 4.27px はみ出す）。帯を `font_height` で切ると
/// **descent のインクが帯の外に出る**——バルーン背景が白・hover 文字が白のとき
/// 帯の外の文字が背景に溶けて「文字の下が切れて見える」。
/// ＭＳ ゴシックは比がちょうど 1.0 のため既定フォントでは観測されない。
///
/// ## 帯の決め方（下限＝em ボックス・上限＝行送りピッチ）
///
/// ```text
/// band = clamp(line_box_height, font_height, max(font_height, line_pitch))
/// ```
///
/// - **下限 `font_height`**: 行矩形より痩せさせない。
/// - **上限 `line_pitch`**: 帯が行送りピッチを超えると**隣接行の帯と重なる**——ハイライトが
///   隣の行を侵し、かつヒット矩形が重なって同一点が 2 選択肢に当たる（照会の一意性が壊れる）。
///   ゆえにピッチで頭打ちにする。実測 Yu Gothic UI 28px（行送り 30 ＝ 28 + 行間 2）では
///   `clamp(37.24, 28, max(28, 30)) = 30`——次行の帯へ食い込まない。
///   `line_pitch < font_height` の病的設定では下限が勝つ（帯 ＝ `font_height`）。
/// - 行送り比 `\n[ratio]` で ratio < 1 を指定した行間縮小時は帯が隣接行へ届き得る（既知の
///   縮退——正典 fixture は ratio ≥ 1）。
///
/// ## 位置
///
/// 帯の丈は広げない。行ボックス丈を覆いきれない分は、帯の位置を字の箱の中央へ寄せて吸収する
/// （量は [`band_offset_on_char_box`]・開発者裁定 2026-09-06）。
///
/// 式は `max` と `min` で書く（有限の値では上の `clamp` と同じ答え・NaN を渡しても落ちない）。
pub fn highlight_band_extent(font_height: f32, line_box_height: f32, line_pitch: f32) -> f32 {
    let upper = line_pitch.max(font_height);
    line_box_height.max(font_height).min(upper)
}

/// ハイライト帯／ヒット帯を字の箱の中央へ寄せる量を決める（純粋・**描画とヒットの唯一の源**）。
/// 本関数は字の箱の上端 t ＝ 0 の場合で、本番の横書きの帯は t を足した同じ式から出る。
///
/// ## なぜ寄せるのか（開発者裁定 2026-09-06）
///
/// [`highlight_band_extent`] の帯は行送りピッチで頭打ちになるので、行ボックス丈より短くなり得る
/// （実測 Yu Gothic UI `font.height,28`: 行ボックス 37.24・帯 30）。DirectWrite は行ボックスの
/// 中にインクを**中央寄り**に置くのに、帯を行矩形のブロック軸**近端**（横書き＝上端）へ揃えると、
/// 帯は上に余りながら下でインクを切る（実測 帯 y0..29／インク y7..31 ＝
/// **上に 7 画素の余白を残したまま下で 2 画素を切る**）。帯を広げるのではなく、
/// **帯を字の箱の中央へ寄せる**のが正しい。
///
/// ```text
/// band_offset = round(max(0, line_box_height − band_extent) / 2)
/// ```
///
/// | フォント | 行ボックス | 帯 | offset | 帯の位置 | インク | 上／下 |
/// |---|---|---|---|---|---|---|
/// | Yu Gothic UI 28（正典） | 37.24 | 30 | **4** | y4..33 | y7..31 | 3／0 |
/// | Yu Gothic UI 20（fixture） | 26.6 | 22 | **2** | y2..23 | y6..22 | 4／0 |
/// | 既定 ＭＳ ゴシック（比 1.0） | ＝ em | ＝ em | **0** | 従来どおり | — | 非退行 |
///
/// 行ボックス丈が em ボックス丈に等しいフォント（既定 ＭＳ ゴシック）では帯 ＝ 行ボックスゆえ
/// offset は 0 になり、寄せる前と 1 画素も変わらない。
///
/// ## 隣接行と重ならない
///
/// 28 では `offset + 帯 = 34` が行送り 30 を超えるが、次の行の帯も `30 + 4 = 34` から始まるので
/// **接するだけで重ならない**。ヒット判定は半開区間（`crates/areka-input/src/balloon.rs`
/// の `y >= top && y < bottom`）ゆえ、共有する辺の上の 1 点はたかだか 1 行にしか当たらない。
///
/// ## 字の箱の中央
///
/// 書体が行間を持つと、DirectWrite の行の箱は字の箱（ascent＋descent）より高く、字の箱は
/// 行矩形の上端から `t`（[`GlyphMetrics::horizontal_char_box_top`]）だけ下に在る。帯の寄せは
/// 本番では `round(max(0, t) + max(0, 行ボックス − 帯) / 2)`（丸め 1 回）で、
/// 本関数はそのうち `t = 0` のときの値を返す。`0.0 + x` は `x` とビットまで同じなので、上の表の値は変わらない。
#[cfg(test)]
pub fn highlight_band_offset(line_box_height: f32, band_extent: f32) -> f32 {
    band_offset_on_char_box(0.0, line_box_height, band_extent)
}

/// 帯の寄せの式の唯一の定義点（横書き・字の箱の上端 `char_box_top` から字の箱の中央へ置く量）。
///
/// 負の上端は 0 に切る＝寄せは 0 以上（`expand_overhang_for_band` の横書きは遠端だけを広げる）。
fn band_offset_on_char_box(char_box_top: f32, line_box_height: f32, band_extent: f32) -> f32 {
    (char_box_top.max(0.0) + (line_box_height - band_extent).max(0.0) / 2.0).round()
}

/// 1 行分のハイライト帯／ヒット帯（丈と寄せ量・**描画とヒットの単一の源**）。
///
/// 帯のブロック軸の範囲は、どの向きでも `[辺 + offset, 辺 + offset + extent]`
/// （辺＝横書きは行矩形の上辺・縦書きは左辺）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineBand {
    /// 帯のブロック軸の丈。横書き＝[`highlight_band_extent`] の出力・縦書き＝列の em。
    pub extent: f32,
    /// 行矩形の左辺（横書きは上辺）から帯の起点までの**符号つき**の量。横書き＝帯を字の箱の
    /// 中央へ置く量 `round(max(0, t) + max(0, 行ボックス − 帯) / 2)`（`t` は
    /// [`GlyphMetrics::horizontal_char_box_top`]・`t = 0` で `highlight_band_offset` の出力・
    /// 0 以上）・縦書き＝
    /// `round(metrics.vertical_em_box_left(em, mode))`（em の箱を書体の縦書きの基線に中央で
    /// 重ねた所・右から左は負になり得る＝帯は行矩形の左へ出る）。
    pub offset: f32,
}

/// 行ごとの帯を 1 度だけ決める（純粋・[`decorate_canvas`] と [`derive_hit_rows`] へ同じ列を配る）。
///
/// ## なぜ行ごとなのか
///
/// 帯の丈をバルーン定義の既定の大きさから**アクターに 1 つ**決めると、`\f[height,40]` を
/// 含む選択肢が表示 42 画素でも帯は 14 画素になり、**文字の上下がクリックできない**。行の丈は
/// 既に「その行に置かれた文字のうち最も大きい em」（行矩形のブロック軸寸）になっているので、
/// そこから行ごとに引き直す。帯の寸法は装飾込みの行から導く（行内軸も同じ）。
///
/// 塗る帯とクリックを受ける帯は同じ列から出る——片方だけを広げると「見えている帯の外が
/// クリックできる」という別の破れ方になる。
pub fn line_bands<M: GlyphMetrics + ?Sized>(
    lines: &[PositionedLine],
    mode: WritingMode,
    metrics: &M,
) -> Vec<LineBand> {
    lines
        .iter()
        .map(|line| {
            let em = match mode {
                WritingMode::HorizontalTb => line.rect.bottom - line.rect.top,
                WritingMode::VerticalRl | WritingMode::VerticalLr => {
                    line.rect.right - line.rect.left
                }
            };
            let line_box_height = metrics.line_box_height(em);
            match mode {
                // 縦書き: 厚み＝列の em、寄せ＝書体の縦書きの基線に中央で重ねた em の箱の左（符号つき）。
                WritingMode::VerticalLr | WritingMode::VerticalRl => LineBand {
                    extent: em,
                    offset: metrics.vertical_em_box_left(em, mode).round(),
                },
                WritingMode::HorizontalTb => {
                    let extent = highlight_band_extent(em, line_box_height, metrics.line_pitch(em));
                    LineBand {
                        extent,
                        offset: band_offset_on_char_box(
                            metrics.horizontal_char_box_top(em),
                            line_box_height,
                            extent,
                        ),
                    }
                }
            }
        })
        .collect()
}

/// ヒット行（純粋・canvas-local image px）: 1 選択肢セグメントの矩形＋配送順序数。
///
/// `rect` は **canvas-local（validrect-local）image px**——[`ContentCanvas::from_layout`] が
/// 住人へ与える座標系と同一（validrect 原点を差し引いた空間）。ゆえにハイライト描画矩形と
/// **数値まで一致**する（正典確定「ハイライト矩形＝ヒット矩形と同一」）。窓物理 px への
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
/// 窓物理 px。actor.rs の照会契約 API はこの型を再輸出する。
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
/// 範囲の文字幅。行全幅ではない）**、**ブロック軸帯＝行矩形の左辺（横書きは上辺）＋符号つきの
/// `band_offset` から `band_extent` 分**（[`LineBand`] の 2 値。横書きは
/// [`highlight_band_extent`] の descent 込みの帯を [`LineBand::offset`] が字の箱の中央へ寄せた位置、
/// 縦書きは列の em の帯を em の箱へ重ねた位置で、右から左の寄せは負）を軸読み替え正準表で x/y へ
/// 割り当て、絶対 image px のヒット矩形を組む。これを
/// [`ContentCanvas::from_layout`] と**同一の原点差引き**（`- region.left` / `- region.top`）で
/// canvas-local（validrect-local）へ写す——ゆえにハイライト描画（`decorate_canvas`）が同じ
/// 2 値から組む矩形と数値一致する（表示とヒットの単一導出）。
///
/// - 空範囲セグメント（`i0 >= i1`）は行を生まない（annotate は既に除外済みだが防御）。
/// - `line_index` 範囲外は防御的にスキップ（annotate は有効添字のみ出す）。
/// - 出力順は入力 `segments` 順（＝annotate 出力＝ordinal 昇順×行昇順）。
///
/// `region` を受けるのは、layout 出力（`inline_range`／行矩形）が**絶対 image px**で、
/// canvas-local 化に validrect 原点が要るため（[`to_window_physical`] が同じ `region` 原点を戻す）。
///
/// [`ContentCanvas::from_layout`]: crate::canvas::ContentCanvas::from_layout
pub fn derive_hit_rows(
    lines: &[PositionedLine],
    segments: &[LineChoiceSegment],
    mode: WritingMode,
    region: &TextRegion,
    bands: &[LineBand],
) -> Vec<CanvasHitRow> {
    let (ox, oy) = (region.left(), region.top());
    let mut rows = Vec::new();
    for seg in segments {
        let (i0, i1) = seg.inline_range;
        if i0 >= i1 {
            continue;
        }
        let Some(line) = lines.get(seg.line_index) else {
            continue;
        };
        let Some(&LineBand {
            extent: band_extent,
            offset: band_offset,
        }) = bands.get(seg.line_index)
        else {
            continue;
        };
        // 横書き＝行内 x／ブロック y・縦書き＝行内 y／ブロック x。block 近端（横書き＝rect.top・
        // 縦書き＝rect.left）は描画（住人 transform の offset＝行矩形の近端）と同一の起点ゆえ、
        // band_extent と band_offset が同値なら帯は描画と数値一致する。
        let abs = match mode {
            WritingMode::HorizontalTb => LineRect {
                left: i0,
                top: line.rect.top + band_offset,
                right: i1,
                bottom: line.rect.top + band_offset + band_extent,
            },
            WritingMode::VerticalRl | WritingMode::VerticalLr => LineRect {
                left: line.rect.left + band_offset,
                top: i0,
                right: line.rect.left + band_offset + band_extent,
                bottom: i1,
            },
        };
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

/// canvas-local ヒット矩形 → バルーン窓 client 物理 px 矩形（純粋）。
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
/// - `origin`＝場所の左上の窓の中の位置（image px）。箱はシェルの窓の中の箱の置き場所（X,Y）、
///   普通のバルーンは (0,0)。領域の原点と同じく ×k の前に足す（箱の面の装着位置
///   `(箱の X + 領域の左, 箱の Y + 領域の上) × k` と同源）。
pub fn to_window_physical(
    row: &CanvasHitRow,
    region: &TextRegion,
    mode: WritingMode,
    committed: i32,
    contract: &ScaleContract,
    origin: (f32, f32),
) -> HitRectPx {
    let k = contract.scale;
    let committed = committed as f32;
    let (ox, oy) = (origin.0 + region.left(), origin.1 + region.top());
    let r = &row.rect;
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

/// 表示されている字 1 字ずつの矩形（送りの幅 × 行の高さ。字の形の画素ではない）を、文字の面の
/// 左上を原点とする物理 px で返す（箱の文字の面の当たり判定の材料）。
///
/// `lines` は配置の結果（見えている字だけが載る＝表示の進み具合を反映済み）。式は
/// [`to_window_physical`] から面の装着位置（`(場所の左上 + 領域の原点) × k`）を引いたもの:
/// 行内軸＝`(字の位置 − 領域の原点) × k`、ブロック軸＝`(行の範囲 − 領域の原点) × k + committed`。
///
/// 行の範囲（ブロック軸）は**行矩形と選択肢の帯の和**: 帯の起点は行矩形の近い辺（横書き＝上・縦書き＝
/// 左。[`derive_hit_rows`] と同じ起点）＋符号つきの帯の offset。近い辺は
/// `min(行矩形の近い辺, 近い辺 + offset)`（縦書きの右から左は offset が負で左へ広がる）、遠い辺は
/// `max(行矩形の遠い辺, 近い辺 + 帯の offset + extent)`。
/// `bands` は [`decorate_canvas`]・[`derive_hit_rows`] へ渡すのと同じ [`line_bands`] の列で、帯が
/// em を越えるフォントでも選択肢の当たり行・強調の帯の全部が字の矩形に入る（単一導出）。
/// 帯の無い行は行矩形だけ。送りの幅が 0 の字は矩形を持たない。
pub fn glyph_cells(
    lines: &[PositionedLine],
    bands: &[LineBand],
    mode: WritingMode,
    region: &TextRegion,
    committed: i32,
    contract: &ScaleContract,
) -> Vec<HitRectPx> {
    let k = contract.scale;
    let c = committed as f32;
    let (ox, oy) = (region.left(), region.top());
    lines
        .iter()
        .enumerate()
        .flat_map(|(i, line)| {
            let r = line.rect;
            let (near, far) = match mode {
                WritingMode::HorizontalTb => (r.top, r.bottom),
                WritingMode::VerticalRl | WritingMode::VerticalLr => (r.left, r.right),
            };
            let (b0, b1) = bands.get(i).map_or((near, far), |b| {
                (
                    near.min(near + b.offset),
                    far.max(near + b.offset + b.extent),
                )
            });
            line.glyphs.iter().map(move |g| (b0, b1, g))
        })
        .filter(|(_, _, g)| g.advance > 0.0)
        .map(|(b0, b1, g)| {
            let (i0, i1) = (g.inline_pos, g.inline_pos + g.advance);
            match mode {
                WritingMode::HorizontalTb => HitRectPx {
                    left: (i0 - ox) * k,
                    top: (b0 - oy) * k + c,
                    right: (i1 - ox) * k,
                    bottom: (b1 - oy) * k + c,
                },
                WritingMode::VerticalRl | WritingMode::VerticalLr => HitRectPx {
                    left: (b0 - ox) * k + c,
                    top: (i0 - oy) * k,
                    right: (b1 - ox) * k + c,
                    bottom: (i1 - oy) * k,
                },
            }
        })
        .collect()
}

/// canvas 装飾（純粋）: 選択肢セグメントを含む GlyphRun 住人を Choice 住人へ置換する。
///
/// [`annotate_lines`] が出した行×選択肢セグメント（[`LineChoiceSegment`]）を源に、当該行の
/// [`ResidentContent::GlyphRun`] 住人を [`ResidentContent::Choice`]（[`ChoiceLineContent`]）へ
/// 置換し、hover 印（[`ChoiceLineContent::hovered`]）と解決済みハイライト塗り
/// （[`HighlightPaint`]＝canvas.rs 純データ型）を焼き込む。`style`→`paint` の正規化
/// （Invert の `255−c` 式含む）はこの一点で行い、下流（viewbox/COM）は choice.rs へ依存せず
/// 純データだけを読む。
///
/// **セグメント空 → canvas を無変更で返す**（恒等・非退行）。
///
/// `bands`（[`line_bands`] の出力・行ごとの [`LineBand`]）から当該行の 2 値を引いて Choice 住人へ
/// 焼き込み、COM 層のハイライト矩形とダーティ帯がこの 2 値を読む——[`derive_hit_rows`] へ**同じ列**
/// を渡すことで描画帯とヒット帯の数値一致を呼び手 1 箇所で担保する。
///
/// ## 座標系: 絶対 image px → resident-local（GlyphRunContent ローカル系）
///
/// [`LineChoiceSegment::inline_range`] は**絶対 image px**（描画開始点＝validrect 原点起点）だが、
/// [`ChoiceRowSegment::inline_range`] は**resident-local**（[`GlyphRunContent`] ローカル 0 起点）で
/// なければならない（[`ContentCanvas::from_layout`] がグリフを行内原点差引きで住人ローカルへ写すのと
/// 同一系）。よって行内軸の resident 原点＝`region.<inline_origin> + resident.transform.offset.<inline>`
/// （＝行矩形 `rect.<inline>`——`from_layout` がグリフから差し引くのと同じ原点）を絶対範囲から
/// 差し引く。これにより装飾側の座標が [`derive_hit_rows`] の canvas-local と数値整合する:
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
    bands: &[LineBand],
) -> ContentCanvas {
    if segments.is_empty() {
        return canvas;
    }
    let mut canvas = canvas;
    for (index, resident) in canvas.residents.iter_mut().enumerate() {
        let ResidentContent::GlyphRun(run) = &resident.content else {
            continue;
        };
        // 行内軸の resident 原点（絶対 image px）＝ validrect 原点 + 住人 inline offset ＝ 行矩形 inline。
        let (ox, oy) = resident.transform.offset();
        let inline_origin = match mode {
            WritingMode::HorizontalTb => region.left() + ox,
            WritingMode::VerticalRl | WritingMode::VerticalLr => region.top() + oy,
        };
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
        if row_segments.is_empty() {
            continue;
        }
        let Some(&LineBand {
            extent: band_extent,
            offset: band_offset,
        }) = bands.get(index)
        else {
            continue;
        };
        let hovered = match hover {
            Some(o) if row_segments.iter().any(|rs| rs.ordinal == o) => Some(o),
            _ => None,
        };
        let highlight = if hovered.is_some() {
            style
                .paint(default_font_color)
                .map(|(fill, text)| HighlightPaint { fill, text })
        } else {
            None
        };
        let run = run.clone();
        resident.content = ResidentContent::Choice(ChoiceLineContent {
            run,
            segments: row_segments,
            hovered,
            highlight,
            band_extent,
            band_offset,
        });
    }
    canvas
}

/// ハイライトスタイル差替シーム（cursor.\* 解決＋矩形反転＋将来非正典スタイルの開放口）。
///
/// balloon の `cursor.*` スタイルモデル（[`BalloonCursor`]）を hover ハイライトの描画正規形へ
/// 解決する純粋 enum。解決は
/// [`resolve`](ResolvedChoiceStyle::resolve) が一点で行い、描画実行側は [`paint`](ResolvedChoiceStyle::paint)
/// が返す `(塗り色, hover 文字色)` 正規形のみを読む（下流 viewbox/COM は本 enum に依存しない）。
///
/// `#[non_exhaustive]` により将来の非正典スタイル variant 追加を後方互換にする。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResolvedChoiceStyle {
    /// cursor.\* 指定形（fixture 実導出）: 矩形塗り色＋hover 文字色。
    ///
    /// `fill`＝`cursor.brush.color`（矩形内色）・`text`＝`cursor.font.color`（hover 文字色）。
    /// fixture 実導出形＝`SquareFill { fill: (105, 25, 25), text: (255, 255, 255) }`（square 塗り＋白文字）。
    SquareFill {
        /// 矩形内塗り色（`cursor.brush.color`）。
        fill: (u8, u8, u8),
        /// hover 文字色（`cursor.font.color`）。
        text: (u8, u8, u8),
    },
    /// cursor.\* 未指定バルーンの矩形反転（正典の導出で縮退ではない）。
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
    /// 判定:
    /// - `cursor` 不在、または cursor.\* 全キー未指定 → [`Invert`](ResolvedChoiceStyle::Invert)
    ///   （未指定バルーン判定・縮退ではない＝warn なし）。
    /// - `cursor.style,none` → [`NoMarker`](ResolvedChoiceStyle::NoMarker)（正典・マーカー無し）。
    /// - `cursor.style` underline 系（`underline`／`square+underline`）→
    ///   [`SquareFill`](ResolvedChoiceStyle::SquareFill) へ縮退（在る色を採る）。
    /// - `cursor.style` square または style 未指定（既定 square）で他キー在り →
    ///   [`SquareFill`](ResolvedChoiceStyle::SquareFill)（`fill=brush.color`・`text=font.color`）。
    /// - `cursor.style` が正典の 4 つのどれでもない値 → 既定 square と同じく
    ///   [`SquareFill`](ResolvedChoiceStyle::SquareFill)（ukadoc は知らない値の扱いを書かない）。
    /// - `cursor.blendmethod` が ROP 系（`none` 以外）→ `none` 扱い
    ///   （色ベース描画・variant は style 判定どおりで不変）。
    ///
    /// 本メソッドは記録を出さない。縮退した元の値は [`CursorFallback::of`] が取り出し、
    /// 警告は登録の側が置き場ごとに値が決まったときに 1 件出す
    /// （解き直し 1 回で本メソッドは 2 回呼ばれ、置き場の名前も知らないため）。
    ///
    /// `default_font_color` は分岐には用いない。将来の非正典 variant がバルーン既定色を焼き込めるよう
    /// 受けておく継ぎ目で、使う分岐が付いたらこの一文を消す。
    pub fn resolve(cursor: Option<&BalloonCursor>, default_font_color: (u8, u8, u8)) -> Self {
        let _ = default_font_color;
        let Some(cursor) = cursor else {
            return ResolvedChoiceStyle::Invert;
        };
        if cursor_all_unspecified(cursor) {
            return ResolvedChoiceStyle::Invert;
        }
        match cursor.style() {
            Some(s) if s.eq_ignore_ascii_case("none") => ResolvedChoiceStyle::NoMarker,
            // underline 系・square・style 未指定はすべて SquareFill（縮退した元の値は CursorFallback::of が取り出す）。
            _ => square_fill_from(cursor),
        }
    }

    /// 描画実行の一点写像（純粋）: `(塗り色, hover 文字色)` 正規形を返す。
    ///
    /// - [`SquareFill`](ResolvedChoiceStyle::SquareFill) → `Some((fill, text))`（`default_font_color` 非依存）。
    /// - [`Invert`](ResolvedChoiceStyle::Invert) → `Some((default_font_color, (255−r, 255−g, 255−b)))`
    ///   （塗り＝バルーン既定 font 色・文字＝各成分の補色・α不変）。
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
/// 全キー未指定＝「未指定バルーン」（`ResolvedChoiceStyle::resolve` が `Invert` へ写す）。
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

/// 選択肢の飾りの縮退した元の値（純粋）。
///
/// [`ResolvedChoiceStyle::resolve`] は縮退しても記録を出さない。登録の側が本値を置き場ごとに
/// 覚え、値が決まったとき（新しく在り、前と違うとき）に警告を 1 件出す。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CursorFallback {
    /// none 以外の cursor.blendmethod の値（ROP 系・none 扱いで描く）。
    pub(crate) blendmethod: Option<String>,
    /// 下線系の cursor.style の値（SquareFill で描く）。
    pub(crate) underline_style: Option<String>,
    /// 正典の 4 つ（square・underline・square+underline・none）のどれでもない cursor.style の値
    /// （ukadoc は知らない値の扱いを書かないので、既定の square＝SquareFill で描く）。
    pub(crate) unknown_style: Option<String>,
}

impl CursorFallback {
    /// `cursor.*` から縮退した元の値を取り出す（純粋）。下線系は [`style_has_underline`] の、
    /// 知らない style は [`is_canon_style`] の、ROP 系は [`rop_blendmethod`] の 1 か所だけで判定する
    /// （[`ResolvedChoiceStyle::resolve`] は `none` 以外の style を一律に SquareFill へ落とし、
    /// ROP 系も分岐に効かないため、どの判定も持たない）。
    pub(crate) fn of(cursor: &BalloonCursor) -> CursorFallback {
        CursorFallback {
            blendmethod: rop_blendmethod(cursor).map(str::to_string),
            underline_style: cursor
                .style()
                .filter(|s| style_has_underline(s))
                .map(str::to_string),
            unknown_style: cursor
                .style()
                .filter(|s| !is_canon_style(s))
                .map(str::to_string),
        }
    }
}

/// `cursor.blendmethod` が ROP 系（在って `none` でない・大小無視）ならその値を返す。
fn rop_blendmethod(cursor: &BalloonCursor) -> Option<&str> {
    cursor
        .blendmethod()
        .filter(|bm| !bm.eq_ignore_ascii_case("none"))
}

/// `cursor.style` が正典の値（`square`・`none`・underline 系）かを判定する（大小無視）。
fn is_canon_style(style: &str) -> bool {
    style.eq_ignore_ascii_case("square")
        || style.eq_ignore_ascii_case("none")
        || style_has_underline(style)
}

/// `cursor.style` が underline 系（`underline`／`square+underline`）かを判定する（大小無視の完全一致・
/// `underlined` のような正典に無い値は知らない値として [`is_canon_style`] の側で拾う）。
fn style_has_underline(style: &str) -> bool {
    style.eq_ignore_ascii_case("underline") || style.eq_ignore_ascii_case("square+underline")
}

/// `SquareFill { fill=brush.color, text=font.color }` を組む（欠落成分は `0` 既定・防御）。
///
/// 正典 fixture では brush/font とも全成分指定ゆえ既定は発火しない。style を持つが色を欠く
/// 防御経路では各成分 `0` を採る（ukadoc が `cursor.brush.color.*`・`cursor.font.color.*` の既定を
/// 0 と書く正典どおり）。
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
#[path = "choice_test_support.rs"]
pub(crate) mod test_support;

#[cfg(test)]
#[path = "choice_style_resolve_tests.rs"]
mod style_resolve_tests;

/// 全行に同じ帯を配る試験の補助（`tests` と `decorate_tests` の共有・行ごとの帯の導出そのものは
/// `choice_tests.rs` の `line_bands` の試験が見張る）。
#[cfg(test)]
fn bands_of(extent: f32, offset: f32) -> Vec<LineBand> {
    vec![LineBand { extent, offset }; 16]
}

#[cfg(test)]
#[path = "choice_decorate_tests.rs"]
mod decorate_tests;

#[cfg(test)]
#[path = "choice_glyph_cells_tests.rs"]
mod glyph_cells_tests;

#[cfg(test)]
#[path = "choice_anchor_tests.rs"]
mod anchor_tests;
