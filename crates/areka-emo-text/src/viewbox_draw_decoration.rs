//! # viewbox_draw_decoration — 行の装飾区間と DirectWrite の範囲指定（COM 層・viewbox_draw.rs の子）
//!
//! 1 行のグリフ列を「連続する同じ装飾番号」＝区間（[`StyleRun`]）へ切り、既定と異なる区間にだけ
//! 既定と異なる項目を焼く（要件 3.5／3.6／5.1／5.2／5.6／5.7／5.8）。executor 本体
//! （`viewbox_draw.rs`）から切り出したのは、装飾の追加でファサードを膨らませないため
//! （design.md「新しい部品の理由」）。
//!
//! ## 焼くもの・焼かないもの
//!
//! 焼くのはフォント家族名・大きさ・太さ・斜体・下線・打ち消し線の 6 項目だけで、いずれも
//! **既定と値が違うときだけ**発行する（既定と同じ項目への範囲指定は意味が無く COM 呼出が
//! 増えるだけ——これは節約であって要件 14.2 の担保ではない。「既定だけの行は範囲指定 0 回」を
//! 実際に担っているのは、呼び手 `line_layout_for` の `line.default_only` 短絡と、下の
//! `apply_font_ranges` が外側で行う `if run.style == StyleId::DEFAULT { continue; }` の 2 点で、
//! この 6 項目のガードを全撤去しても crate は緑のまま——実測 2026-09-12）。白抜き（`outline`）と
//! 上下付き（`script`）は語彙のみで読まない。下線・打ち消し線の**位置**は DirectWrite の
//! 既定に委ね、areka 側で線を描き分けない（要件 5.7）——だから `SetUnderline`／
//! `SetStrikethrough` の真偽だけを渡す。太字・斜体に対応する書体が無いときの合成も
//! DirectWrite に委ねる（要件 5.6・失敗扱いにしない）。
//!
//! ## 色は毎フレーム焼き直す
//!
//! フォント系 6 項目は行 TextLayout の生成時に 1 度だけ焼く（[`crate::draw::LineLayoutStore`]
//! が装飾番号列込みで再利用を判定する）。一方**色**は `SetDrawingEffect` でブラシを差すので、
//! 前フレームの重ね表示（hover）の色がキャッシュ層の TextLayout に残り得る。ゆえに色だけは
//! 毎フレーム「全範囲の解除 → 装飾の色 → 重ね表示の色」の順で焼き直す
//! （[`apply_color_ranges`] が前 2 つ・呼び手が最後の 1 つ）。選択肢の行は呼び手が既に全範囲を
//! 解除しているので、解除を二重に行わないよう `reset = false` で呼ぶ。
//!
//! **層規律**: COM 層——UI スレッド専有。失敗は log-first（`error!`＋`Err`・当該フレーム見送り）
//! で扱い panic しない（要件 13.3）。

use std::collections::HashMap;

use windows::Win32::Graphics::Direct2D::{ID2D1DeviceContext, ID2D1SolidColorBrush};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_TEXT_RANGE, IDWriteTextLayout,
};
use windows::core::{HSTRING, IUnknown};
use wintf::com::d2d::D2D1DeviceContextExt;

use super::{color_f, device_err};
use crate::TextLayerError;
use crate::canvas::GlyphRunContent;
use crate::draw::FontCatalog;
use crate::layout::PositionedGlyph;
use crate::look::{StyleId, StyleTable, TextLook};
use crate::writing::WritingMode;

/// 装飾の区間——UTF-16 の文字範囲と、その範囲に効く装飾番号。
#[derive(Clone, Copy, Debug)]
pub(crate) struct StyleRun {
    /// DirectWrite の範囲指定へそのまま渡す UTF-16 文字範囲。
    pub range: DWRITE_TEXT_RANGE,
    /// この範囲に効く装飾番号（[`StyleId::DEFAULT`]＝そのスコープの既定の見た目）。
    pub style: StyleId,
}

/// 行のグリフ列を「連続する同じ装飾番号」の区間へ切る（要件 3.6）。
///
/// 文字範囲の数え方は選択肢の範囲算出（`super::segment_text_range`）と同じ——行 TextLayout の
/// text は各グリフの `ch` の連結なので、UTF-16 長の累積がそのまま文字位置になる（`𠮷` のような
/// サロゲートペアは 2 単位）。グリフの無い行は区間 0 個。
pub(crate) fn style_runs(glyphs: &[PositionedGlyph]) -> Vec<StyleRun> {
    let mut runs: Vec<StyleRun> = Vec::new();
    let mut acc: u32 = 0;
    for g in glyphs {
        let units = g.ch.len_utf16() as u32;
        match runs.last_mut() {
            Some(last) if last.style == g.style => last.range.length += units,
            _ => runs.push(StyleRun {
                range: DWRITE_TEXT_RANGE {
                    startPosition: acc,
                    length: units,
                },
                style: g.style,
            }),
        }
        acc += units;
    }
    runs
}

/// 行の箱のブロック軸寸（横書き＝高さ・縦書き＝幅）＝行内最大 em。
///
/// **装飾のある行にだけ使う**——`run.size` は行矩形の引き算（f32）なので、既定だけの行では
/// `font.height` と厳密に一致しない可能性がある。要件 14.1 の「1 画素も変えない」を丸めに
/// 依存させないため、既定だけの行は従来どおり `font.height` をそのまま渡す。
pub(crate) fn block_extent(size: (f32, f32), mode: WritingMode) -> f32 {
    match mode {
        WritingMode::HorizontalTb => size.1,
        WritingMode::VerticalRl | WritingMode::VerticalLr => size.0,
    }
}

/// 1 行ぶんの装飾の切り出し結果（区間列・番号列・行の箱のブロック軸寸）。
///
/// はみ出し収集ループと Phase 1 の**両方**が同じ値で行 TextLayout を引くための束——
/// 引数が食い違うと保持庫の鍵が変わり、同じ行を 1 フレームに 2 度生成してしまう。
pub(crate) struct LineStyles {
    /// 装飾の区間列。
    pub runs: Vec<StyleRun>,
    /// グリフ序数と同じ順の装飾番号列（保持庫の再利用の鍵・要件 11.4）。既定だけの行は空。
    pub ids: Vec<StyleId>,
    /// 行の箱のブロック軸寸（既定だけの行は `font_height`・装飾のある行は [`block_extent`]）。
    pub extent: f32,
    /// 既定の見た目しか使っていない行か（真なら範囲指定・色の解除・ブラシ生成のいずれも行わない）。
    pub default_only: bool,
}

/// 行の装飾を切り出す（既定だけの行は従来の引数——空の番号列と `font_height`——へ落とす）。
pub(crate) fn line_styles(
    run: &GlyphRunContent,
    font_height: f32,
    mode: WritingMode,
) -> LineStyles {
    let runs = style_runs(&run.glyphs);
    let default_only = runs.iter().all(|r| r.style == StyleId::DEFAULT);
    LineStyles {
        ids: if default_only {
            Vec::new()
        } else {
            run.glyphs.iter().map(|g| g.style).collect()
        },
        extent: if default_only {
            font_height
        } else {
            block_extent(run.size, mode)
        },
        default_only,
        runs,
    }
}

/// 既定と異なる区間にだけ、既定と異なるフォント系項目を焼く（要件 5.1／5.2／5.6／5.7／5.8／9.8）。
///
/// 行 TextLayout の**生成時に 1 度だけ**呼ばれる（[`crate::draw::LineLayoutStore::line_layout_decorated`]
/// の `decorate` 口）。家族名は [`FontCatalog::family_for`] が解決した名前だけを渡す——
/// 実在しない候補列は解決に失敗して `None` になり、そのときは既定の家族名のまま描く（要件 9.8）。
/// 失敗は log-first（`error!`＋`Err`）で当該フレームを見送る（要件 13.3）。
pub(crate) fn apply_font_ranges(
    layout: &IDWriteTextLayout,
    runs: &[StyleRun],
    styles: &StyleTable,
    default: &TextLook,
    fonts: &FontCatalog,
) -> Result<(), TextLayerError> {
    for run in runs {
        if run.style == StyleId::DEFAULT {
            continue;
        }
        let look = styles.resolve(run.style, default);

        if look.name != default.name
            && let Some(family) = fonts.family_for(&look.name)
        {
            note_range_call();
            unsafe { layout.SetFontFamilyName(&HSTRING::from(family.as_str()), run.range) }
                .map_err(device_err("SetFontFamilyName(装飾の区間)"))?;
        }
        if look.height != default.height {
            note_range_call();
            unsafe { layout.SetFontSize(look.height, run.range) }
                .map_err(device_err("SetFontSize(装飾の区間)"))?;
        }
        if look.bold != default.bold {
            note_range_call();
            let weight = if look.bold {
                DWRITE_FONT_WEIGHT_BOLD
            } else {
                DWRITE_FONT_WEIGHT_NORMAL
            };
            unsafe { layout.SetFontWeight(weight, run.range) }
                .map_err(device_err("SetFontWeight(装飾の区間)"))?;
        }
        if look.italic != default.italic {
            note_range_call();
            let style = if look.italic {
                DWRITE_FONT_STYLE_ITALIC
            } else {
                DWRITE_FONT_STYLE_NORMAL
            };
            unsafe { layout.SetFontStyle(style, run.range) }
                .map_err(device_err("SetFontStyle(装飾の区間)"))?;
        }
        if look.underline != default.underline {
            note_range_call();
            unsafe { layout.SetUnderline(look.underline, run.range) }
                .map_err(device_err("SetUnderline(装飾の区間)"))?;
        }
        if look.strike != default.strike {
            note_range_call();
            unsafe { layout.SetStrikethrough(look.strike, run.range) }
                .map_err(device_err("SetStrikethrough(装飾の区間)"))?;
        }
    }
    Ok(())
}

/// 既定と異なる色の区間にブラシを差す（要件 3.5／8.x の描画側の着地点）。
///
/// 既定と異なる色の区間が 1 つも無ければ何も発行せず `Ok(false)`。1 つでもあれば
/// 「（`reset` が真なら）全範囲の解除 → 区間ごとのブラシ」の順で焼き、戻り値は**解除を行ったか**。
/// 選択肢の行は呼び手が既に全範囲を解除しているので `reset = false` で呼ぶ（二重化を避ける）。
/// 重ね表示（hover）の色は呼び手がこの後に焼くので、同じ範囲では後勝ちで hover が勝つ。
pub(crate) fn apply_color_ranges(
    layout: &IDWriteTextLayout,
    runs: &[StyleRun],
    styles: &StyleTable,
    default: &TextLook,
    brushes: &mut BrushCache,
    dc: &ID2D1DeviceContext,
    reset: bool,
) -> Result<bool, TextLayerError> {
    let colored: Vec<(DWRITE_TEXT_RANGE, (u8, u8, u8))> = runs
        .iter()
        .filter(|r| r.style != StyleId::DEFAULT)
        .filter_map(|r| {
            let look = styles.resolve(r.style, default);
            (look.color != default.color).then_some((r.range, look.color))
        })
        .collect();

    if colored.is_empty() {
        return Ok(false);
    }

    if reset {
        note_range_call();
        note_reset_call();
        let length = runs
            .last()
            .map_or(0, |r| r.range.startPosition + r.range.length);
        unsafe {
            layout.SetDrawingEffect(
                None::<&IUnknown>,
                DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length,
                },
            )
        }
        .map_err(device_err("SetDrawingEffect(reset None・装飾の色)"))?;
    }
    for (range, rgb) in colored {
        let brush = brushes.get_or_create(dc, rgb)?;
        note_range_call();
        unsafe { layout.SetDrawingEffect(&brush, range) }
            .map_err(device_err("SetDrawingEffect(装飾の色)"))?;
    }
    Ok(reset)
}

/// 色ごとの塗りブラシの記憶（同じ色を毎フレーム作り直さない）。
///
/// executor が持ち、`ViewboxExecutor` の寿命＝専用 D2D DC の寿命と一致する。
///
/// **色は台詞をまたいで累積する**：[`StyleTable`] は台詞ごとに既定へ戻るのに、この保持庫だけは
/// executor の寿命いっぱい残るという非対称がある。上限を設けないのは、鍵が RGB 3 バイトで
/// 実際に現れる色数はゴーストの辞書が使う色数に押さえられるため（ブラシ 1 本は数十バイト）で、
/// LRU を入れる方が実態より高いと判断したから。
#[derive(Default)]
pub(crate) struct BrushCache {
    brushes: HashMap<(u8, u8, u8), ID2D1SolidColorBrush>,
}

impl BrushCache {
    /// その色のブラシを返す（無ければ作って覚える）。生成失敗は log-first（`error!`＋`Err`）。
    fn get_or_create(
        &mut self,
        dc: &ID2D1DeviceContext,
        rgb: (u8, u8, u8),
    ) -> Result<ID2D1SolidColorBrush, TextLayerError> {
        if let Some(brush) = self.brushes.get(&rgb) {
            return Ok(brush.clone());
        }
        note_range_call();
        let brush = dc
            .create_solid_color_brush(&color_f(rgb), None)
            .map_err(device_err("CreateSolidColorBrush(装飾の色)"))?;
        self.brushes.insert(rgb, brush.clone());
        Ok(brush)
    }
}

#[cfg(test)]
thread_local! {
    /// 範囲指定・色の解除・ブラシ生成を**実際に発行した回数**（テスト観測用）。
    ///
    /// 保持庫の要素数で数えると同じ鍵で作り直しても増えず「N 回だけ」が恒真になるので、
    /// 発行のたびに 1 増やす素の勘定にしてある（記憶 6.4 の申し送り）。
    pub(super) static RANGE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// 全範囲の解除（`SetDrawingEffect(None, 全域)`）を発行した回数（解除の二重化の見張り）。
    pub(super) static RESET_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// 範囲指定・ブラシ生成を 1 件発行したことを記録する（テストビルドのみ実体を持つ）。
#[cfg(test)]
fn note_range_call() {
    RANGE_CALLS.with(|c| c.set(c.get() + 1));
}

#[cfg(not(test))]
fn note_range_call() {}

/// 全範囲の解除を 1 件発行したことを記録する（テストビルドのみ実体を持つ）。
#[cfg(test)]
fn note_reset_call() {
    RESET_CALLS.with(|c| c.set(c.get() + 1));
}

#[cfg(not(test))]
fn note_reset_call() {}
