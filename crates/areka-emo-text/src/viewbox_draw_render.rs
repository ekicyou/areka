//! viewbox_draw の子: 装飾つきの描画（`render_styled`・行レイアウトと書式の確保・強調の矩形と行の描画の補助）。
//! 足す予定の spec: text-typesetting（縦中横の塊）・text-reveal-fade・choice-marker-styling・anchor-tag-canon（強調の矩形・行の描画）。

use std::rc::Rc;

use tracing::warn;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, ID2D1Image, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_TEXT_RANGE, IDWriteTextFormat, IDWriteTextLayout,
};
use windows::core::IUnknown;
use windows_numerics::{Matrix3x2, Vector2};
use wintf::com::d2d::D2D1DeviceContextExt;

use super::decoration::{LineStyles, apply_color_ranges, apply_font_ranges, line_styles};
#[cfg(test)]
use super::none_err;
use super::plan::degrade_if_needed;
use super::{FormatKey, ViewboxExecutor, color_f, device_err};
use crate::TextLayerError;
use crate::canvas::GlyphRunContent;
use crate::canvas::{ContentCanvas, ResidentContent};
use crate::draw::{ResolvedFont, create_d2d_target_bitmap, create_text_format};
use crate::layout::{PositionedGlyph, VisibleWindow};
use crate::look::StyleTable;
use crate::region::ScaleContract;
use crate::surface::TextSurface;
use crate::viewbox::{FramePlan, LineOverhang};
use crate::writing::WritingMode;

impl ViewboxExecutor {
    /// 装飾入りの 1 フレームの実行（[`Self::render`] の本体・要件 3.5／3.6／14.2）。
    ///
    /// `styles` は当該 actor の装飾の表で、既定の見た目は `font.looks.default`。番号 0 しか
    /// 使われていない行は区間が 1 つになるので、範囲指定・色の解除・ブラシ生成のいずれも呼ばず、
    /// 行の箱寸も従来どおり `font.height` をそのまま渡す——生成物も呼出列も装飾導入前と同一
    /// （要件 14.2）。装飾のある行だけが `line_layout_decorated`（生成時に 1 度だけフォント系の
    /// 範囲指定）と `apply_color_ranges`（毎フレーム色を焼き直す）を通る。
    ///
    /// 範囲指定と描画基盤の呼び出しの失敗は `error!` を残して `Err` を返し、当該フレームを
    /// 見送る（plan 未 commit・front 不変＝次フレーム再試行安全・要件 13.3）。
    #[allow(clippy::too_many_arguments)]
    pub fn render_styled(
        &mut self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        font: &ResolvedFont,
        mode: WritingMode,
        contract: &ScaleContract,
        surface: &mut TextSurface,
        styles: &StyleTable,
    ) -> Result<bool, TextLayerError> {
        let size = surface.size();
        let (format, rebuilt) = self.ensure_format(font, mode)?;

        // ── plan 前に全住人の行レイアウトを確保し**実測インクはみ出し**を集める ──
        // 確定行はキャッシュヒット（安価）・変化行のみ生成（=行レイアウト生成はここで一括発生し、
        // 以降の Update 描画ループは再利用＝キャッシュヒット）。実測はみ出し（[`LineOverhang`]）は
        // ダーティ矩形が em ボックス下端はみ出しを取りこぼさないための入力（D2）——`GetOverhangMetrics`
        // で生成時に測定済みの値を index 整列で集める。
        let creations_before = self.line_store.creations();
        let mut overhangs: Vec<LineOverhang> = Vec::with_capacity(canvas.residents.len());
        for (index, resident) in canvas.residents.iter().enumerate() {
            let overhang = match &resident.content {
                ResidentContent::GlyphRun(run) if !run.glyphs.is_empty() => {
                    let text: String = run.glyphs.iter().map(|g| &*g.text).collect();
                    self.line_layout_for(index, &text, run, &format, font, mode, styles)?;
                    self.line_store.overhang(index).unwrap_or_default()
                }
                // Choice 住人は内包 run を GlyphRun と同一経路で計測する（R9.5）。
                ResidentContent::Choice(choice) if !choice.run.glyphs.is_empty() => {
                    let text: String = choice.run.glyphs.iter().map(|g| &*g.text).collect();
                    self.line_layout_for(index, &text, &choice.run, &format, font, mode, styles)?;
                    let measured = self.line_store.overhang(index).unwrap_or_default();
                    // ハイライト帯（band_offset ＋ band_extent）は em ボックス丈より外側へ出る
                    // （descent 込み＋行ボックス中央への寄せ）。ダーティ矩形が em ボックス＋
                    // 実測インクはみ出しのままだと、帯の外側部分がクリップで塗り残り／消し残りに
                    // なる（hover 解除フレームに塗りが残る）。ゆえにブロック軸の遠端はみ出しを
                    // **帯の超過分**まで広げる（横書き＝下・縦書き＝右——`highlight_rect` が帯を
                    // 伸ばす向きと同一）。
                    expand_overhang_for_band(
                        measured,
                        choice.band_extent,
                        choice.band_offset,
                        font.height,
                        mode,
                    )
                }
                // 空行/シーム住人は実インクを持たない＝はみ出し 0（em ボックス丈）。
                _ => LineOverhang::default(),
            };
            overhangs.push(overhang);
        }
        self.stats.line_layout_creations += self.line_store.creations() - creations_before;

        let plan = self
            .planner
            .plan_with_overhangs(canvas, window, mode, contract, size, &overhangs);
        // 縮退判定（正しさ優先）: フォント/方向変更 or 想定外不整合なら全域ダーティ Update へ差し替え。
        // 全域ダーティは面全域＝resident_rect を通らないため overhangs 不要（em ボックス経路と同一）。
        let plan = degrade_if_needed(plan, rebuilt, canvas, window, mode, contract, size);

        match &plan {
            // 変化なし——blit・描画・present とも 0・commit 不要（no-op）。
            FramePlan::NoChange => Ok(false),

            // Clear cue 適用——back を全域透明 Clear（描画 0 件）して flip。
            FramePlan::FullClear => {
                let target = create_d2d_target_bitmap(&self.dc, surface.back_tex())?;
                unsafe { self.dc.SetTarget(&target) };
                unsafe { self.dc.BeginDraw() };
                self.dc.clear(None);
                let end = unsafe { self.dc.EndDraw(None, None) };
                unsafe { self.dc.SetTarget(None::<&ID2D1Image>) };
                end.map_err(device_err("EndDraw(FullClear)"))?;

                surface.flip();
                self.planner.commit(canvas, window, mode, contract, &plan);
                self.stats.full_clears += 1;
                Ok(true)
            }

            // blit ＋ ダーティ矩形限定描画。
            FramePlan::Update {
                blit,
                dirty,
                draw_lines,
            } => {
                // 保持ピクセルの面内 blit（front→back・blit≠0 のみ blits 加算）。
                surface.copy_front_to_back_shifted(*blit);
                if *blit != (0, 0) {
                    self.stats.blits += 1;
                }

                // ── Phase 1（可謬）: 描画資源を BeginDraw の前に確定する ──
                // 描画対象住人（draw_lines＝矩形ごとの交差行の和集合）の TextLayout・origin・
                // Choice ハイライト資源を index 付きで組み、Phase 2 が矩形ごとの行から引く。行
                // レイアウトは plan 前のはみ出し収集ループで確保済み（ここは全てキャッシュヒット）。
                // origin 式は DrawExecutor と同一（validrect-local 平行移動＋ブロック軸の可視窓
                // オフセット）。ブラシ生成・`SetDrawingEffect`（**DrawingEffect リセット正準列**——
                // キャッシュ層 TextLayout を汚さないため全文字範囲へ `None` を必ず適用してから hover
                // 範囲へ文字色効果を焼く）は可謬ゆえこの BeginDraw 前区間で `?` 伝播する（失敗フレームは
                // target 未設定のまま skip＝再試行安全・R4.5/R4.6）。
                let block_offset = window.block_offset;
                let mut draws: Vec<(usize, LineDraw)> = Vec::new();
                for &index in draw_lines {
                    let resident = &canvas.residents[index];
                    let (run, choice) = match &resident.content {
                        ResidentContent::GlyphRun(run) => (run, None),
                        // Choice 住人は内包 run を GlyphRun と同格に描き、hover 時のみハイライトを重ねる。
                        ResidentContent::Choice(choice) => (&choice.run, Some(choice)),
                        seam @ (ResidentContent::Image(_) | ResidentContent::Surface(_)) => {
                            // planner の draw_lines は GlyphRun/Choice のみゆえ通常不発の防御経路
                            // （warn は executor ごと初回のみ・DrawExecutor と同規律）。
                            if !self.seam_warned {
                                self.seam_warned = true;
                                warn!(
                                    resident = ?seam,
                                    "Image/Surface 住人は M1 型シームのため描画を skip する（実挙動なし）"
                                );
                            }
                            continue;
                        }
                    };
                    if run.glyphs.is_empty() {
                        continue;
                    }
                    let text: String = run.glyphs.iter().map(|g| &*g.text).collect();
                    let (layout, line) =
                        self.line_layout_for(index, &text, run, &format, font, mode, styles)?;
                    let (dx, dy) = resident.transform.offset();
                    let origin = match mode {
                        WritingMode::HorizontalTb => Vector2 {
                            X: dx,
                            Y: dy + block_offset,
                        },
                        WritingMode::VerticalRl | WritingMode::VerticalLr => Vector2 {
                            X: dx + block_offset,
                            Y: dy,
                        },
                    };

                    // ── 色の焼き込み順（毎フレーム・要件 3.5）: 全範囲の解除 → 装飾の色 → 重ね表示の色 ──
                    // 色は `SetDrawingEffect` でキャッシュ層 TextLayout に残るため、フォント系
                    // 6 項目（生成時 1 度）と違って毎フレーム焼き直す。Choice 行はここで全文字範囲を
                    // `None` へリセットする（hover 解除フレームは全範囲 None のみ＝素描画）。
                    if choice.is_some() {
                        let full_len = text.encode_utf16().count() as u32;
                        unsafe {
                            layout.SetDrawingEffect(
                                None::<&IUnknown>,
                                DWRITE_TEXT_RANGE {
                                    startPosition: 0,
                                    length: full_len,
                                },
                            )
                        }
                        .map_err(device_err("SetDrawingEffect(reset None)"))?;
                    }
                    // 装飾の色（既定と異なる色の区間だけ）。Choice 行は直前で解除済みゆえ
                    // `reset = false`——解除を二重に行わない。既定だけの行は呼ばない（要件 14.2）。
                    if !line.default_only {
                        apply_color_ranges(
                            &layout,
                            &line.runs,
                            styles,
                            &font.looks.default,
                            &mut self.brushes,
                            &self.dc,
                            choice.is_none(),
                        )?;
                    }

                    let choice_draw = if let Some(choice) = choice {
                        // highlight=Some（hover 行）: hover セグメント（ordinal==hovered）へ矩形塗り
                        // ブラシ＋文字色効果を組む。NoMarker/非 hover は highlight=None＝素描画。
                        let hover = if let Some(paint) = choice.highlight {
                            let fill = self
                                .dc
                                .create_solid_color_brush(&color_f(paint.fill), None)
                                .map_err(device_err("CreateSolidColorBrush(fill)"))?;
                            let text_brush = self
                                .dc
                                .create_solid_color_brush(&color_f(paint.text), None)
                                .map_err(device_err("CreateSolidColorBrush(text)"))?;
                            let mut rects: Vec<D2D_RECT_F> = Vec::new();
                            for seg in &choice.segments {
                                if Some(seg.ordinal) != choice.hovered {
                                    continue;
                                }
                                // hover セグメント矩形（inline_range × ハイライト帯 band_extent・
                                // 住人 transform ＋block_offset ＋band_offset 反映＝ヒット矩形／
                                // 指紋と同座標系・R3.3/R13.2）。帯は em ボックス丈（font.height）
                                // ではなく choice.band_extent（descent 込みの実行ボックス丈）を
                                // choice.band_offset だけ行ボックスの中央へ寄せた位置に置く
                                // ——`derive_hit_rows` へ渡る 2 値と同一。
                                rects.push(highlight_rect(
                                    seg.inline_range,
                                    (dx, dy),
                                    block_offset,
                                    choice.band_extent,
                                    choice.band_offset,
                                    mode,
                                ));
                                // hover セグメントの文字範囲へ文字色効果を適用（run のグリフ位置から
                                // UTF-16 文字範囲を導く）。
                                if let Some(range) =
                                    segment_text_range(&run.glyphs, seg.inline_range)
                                {
                                    unsafe { layout.SetDrawingEffect(&text_brush, range) }
                                        .map_err(device_err("SetDrawingEffect(text)"))?;
                                }
                            }
                            Some(ChoiceHover { fill, rects })
                        } else {
                            None
                        };
                        Some(ChoiceDraw { hover })
                    } else {
                        None
                    };

                    draws.push((
                        index,
                        LineDraw {
                            origin,
                            layout,
                            choice: choice_draw,
                        },
                    ));
                }

                let (r, g, b) = font.color;
                let brush = self
                    .dc
                    .create_solid_color_brush(&color_f((r, g, b)), None)
                    .map_err(device_err("CreateSolidColorBrush"))?;
                let target = create_d2d_target_bitmap(&self.dc, surface.back_tex())?;

                // ── Phase 2（描画・ダーティ矩形限定）: この区間の D2D 呼び出しは不可謬（戻り値
                // なし）で、失敗は EndDraw に集約される。SetTarget は成否によらず必ず解除する。 ──
                let k = contract.scale;
                unsafe { self.dc.SetTarget(&target) };
                unsafe { self.dc.BeginDraw() };
                for dirty_rect in dirty {
                    let rect = &dirty_rect.rect;
                    // ① 恒等変換（物理整数矩形へ範囲限定するため）。
                    self.dc.set_transform(&Matrix3x2 {
                        M11: 1.0,
                        M12: 0.0,
                        M21: 0.0,
                        M22: 1.0,
                        M31: 0.0,
                        M32: 0.0,
                    });
                    // ② ダーティ矩形へ描画範囲を限定（D2D 矩形範囲限定機構を直接用いる・R9.4）。
                    let clip = D2D_RECT_F {
                        left: rect.x as f32,
                        top: rect.y as f32,
                        right: (rect.x + rect.w) as f32,
                        bottom: (rect.y + rect.h) as f32,
                    };
                    unsafe {
                        self.dc
                            .PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_ALIASED);
                    }
                    // ③ 範囲内だけを透明化（premultiplied 全 0）。
                    self.dc.clear(None);
                    // ④ 合成スケール一点適用（k はここだけ・レイアウト座標は image px）。
                    self.dc.set_transform(&Matrix3x2 {
                        M11: k,
                        M12: 0.0,
                        M21: 0.0,
                        M22: k,
                        M31: 0.0,
                        M32: 0.0,
                    });
                    // ⑤ **この矩形と交差する行だけ**を描く（クリップにより描画結果は当該矩形内へ
                    // 限定される）。交差しない行のインクはこの矩形の内側に 1 画素も無いので、
                    // 描かなくても画素は変わらない（R11.1/11.2）。Phase 1 で資源を組まなかった行
                    // （空グリフ列・シーム住人＝そこで skip 済み）は引けないので飛ばす。
                    // Choice hover 行は (a) セグメント矩形を塗り色で `FillRectangle` してから
                    // (b) `DrawTextLayout`（効果範囲の文字だけが Phase 1 で焼いた切替色で描かれる）。
                    for line in &dirty_rect.lines {
                        let Some((_, d)) = draws.iter().find(|(i, _)| i == line) else {
                            continue;
                        };
                        if let Some(ChoiceDraw {
                            hover: Some(hover), ..
                        }) = &d.choice
                        {
                            for rect in &hover.rects {
                                self.dc.fill_rectangle(rect, &hover.fill);
                            }
                        }
                        self.dc.draw_text_layout(
                            d.origin,
                            &d.layout,
                            &brush,
                            self.text_draw_options(),
                        );
                        self.stats.draw_text_layout_calls += 1;
                    }
                    // ⑥ 範囲限定を解除。
                    unsafe {
                        self.dc.PopAxisAlignedClip();
                    }
                }
                let end = unsafe { self.dc.EndDraw(None, None) };
                unsafe { self.dc.SetTarget(None::<&ID2D1Image>) };
                end.map_err(device_err("EndDraw"))?;

                // テスト専用 fault-injection（G5）: EndDraw 後・flip/commit の**前**に失敗を注入し、
                // 「失敗フレームは front 不変（flip せず）・planner 未 commit（次フレーム同一再計画）」の
                // 再試行安全を檻化する。実 COM 失敗を決定論的に再現できないための最小フック。
                #[cfg(test)]
                if self.fail_next_render {
                    self.fail_next_render = false;
                    return Err(none_err("injected render failure (test-only・G5)"));
                }

                // 面の役割交換 → 計画の確定（成功時のみ・失敗フレームは未 commit で再試行安全）。
                // 行レイアウト生成の集計は plan 前のはみ出し収集ループで済み（ここでは再計上しない）。
                surface.flip();
                self.planner.commit(canvas, window, mode, contract, &plan);
                Ok(true)
            }
        }
    }

    /// 1 行の行 TextLayout を装飾込みで確保する（はみ出し収集ループと Phase 1 が**同じ引数**で
    /// 呼ぶための 1 点・引数が食い違うと保持庫の鍵が変わって同じ行を 1 フレームに 2 度生成する）。
    ///
    /// 既定だけの行は従来の [`LineLayoutStore::line_layout`] をそのまま通る（空の番号列・
    /// 行の箱寸は `font.height`・焼き処理なし＝装飾導入前と 1 ビットも変わらない・要件 14.2）。
    /// 装飾のある行だけが [`LineLayoutStore::line_layout_decorated`] を通り、生成時に 1 度だけ
    /// フォント系の範囲指定（[`apply_font_ranges`]）を焼く。
    #[allow(clippy::too_many_arguments)]
    fn line_layout_for(
        &mut self,
        index: usize,
        text: &str,
        run: &GlyphRunContent,
        format: &IDWriteTextFormat,
        font: &ResolvedFont,
        mode: WritingMode,
        styles: &StyleTable,
    ) -> Result<(IDWriteTextLayout, LineStyles), TextLayerError> {
        let line = line_styles(run, font.height, mode);
        if line.default_only {
            let layout = self
                .line_store
                .line_layout(index, text, format, font.height, mode)?;
            return Ok((layout, line));
        }
        // 台帳は `&mut self.line_store` と同時に借りられないので Rc を複製して渡す。
        let fonts = Rc::clone(&self.fonts);
        let default = &font.looks.default;
        let layout = self.line_store.line_layout_decorated(
            index,
            text,
            format,
            line.extent,
            mode,
            &line.ids,
            |l| apply_font_ranges(l, &line.runs, styles, default, &fonts),
        )?;
        Ok((layout, line))
    }

    /// 描画/計測共用 format の確保（[`create_text_format`] 経路・`FormatKey` 不変なら再利用）。
    ///
    /// 戻り値の `bool` は**format と [`LineLayoutStore`] を組み直したか**（`true`＝組み直し）。
    /// フォント/方向の変更は行キャッシュの前提（同一 format で組んだ TextLayout）と committed
    /// ピクセル（旧 format で描いた面）を崩すため、`render` はこのフラグを見て当該フレームを
    /// 全域ダーティへ縮退する（`DrawExecutor::ensure_format` と同規律・実運用は actor ごと固定の
    /// ため通常発火しない・`debug!` 記録）。**初回生成**（`self.format` が `None`・組み直す前提が
    /// 無い＝committed ピクセルも無い）は `false`（初回フレームは prev_lines 空ゆえ元より全域ダーティ）。
    fn ensure_format(
        &mut self,
        font: &ResolvedFont,
        mode: WritingMode,
    ) -> Result<(IDWriteTextFormat, bool), TextLayerError> {
        let key: FormatKey = (font.name.clone(), font.height.to_bits(), mode);
        if let Some((cached_key, format)) = &self.format {
            if *cached_key == key {
                return Ok((format.clone(), false));
            }
            tracing::debug!(
                ?key,
                "フォント/方向が変わったため format と行レイアウトキャッシュを組み直す"
            );
            self.line_store.clear();
            let format = create_text_format(&self.dwrite, &self.fonts.pick(font), mode)?;
            self.format = Some((key, format.clone()));
            return Ok((format, true));
        }
        // 初回生成（committed ピクセルが無いため縮退トリガにしない）。
        let format = create_text_format(&self.dwrite, &self.fonts.pick(font), mode)?;
        self.format = Some((key, format.clone()));
        Ok((format, false))
    }
}

/// 1 描画対象行の COM 描画資源（origin＋行 TextLayout＋Choice ハイライト資源）。
struct LineDraw {
    /// 描画原点（DrawExecutor と同一式・validrect-local 平行移動＋可視窓オフセット）。
    origin: Vector2,
    /// 行 TextLayout（[`LineLayoutStore`] 由来・効果は Phase 1 で焼込済み）。
    layout: IDWriteTextLayout,
    /// Choice 住人のときのみ Some（GlyphRun は None＝素描画）。
    choice: Option<ChoiceDraw>,
}

/// Choice 行のハイライト描画資源（hover 時のみ塗り資源を持つ・非 hover は全範囲 None リセットのみ）。
struct ChoiceDraw {
    /// hover 時の矩形塗り資源（`highlight=None`／NoMarker は None＝素描画）。
    hover: Option<ChoiceHover>,
}

/// hover セグメントの矩形塗り資源（文字色効果は Phase 1 で layout へ焼込済みゆえ持たない）。
struct ChoiceHover {
    /// 矩形塗りブラシ（`HighlightPaint::fill`）。
    fill: ID2D1SolidColorBrush,
    /// hover セグメント矩形（image px・住人 transform＋block_offset 反映）。
    rects: Vec<D2D_RECT_F>,
}

/// hover セグメント矩形を描画空間（image px・住人 transform＋block_offset 反映）で組む。
///
/// 行内軸＝セグメントの `inline_range`（文字幅）・ブロック軸帯＝`band_offset` だけ内側へ寄せた
/// 起点から `band_extent`（[`ChoiceLineContent::band_extent`]＝実 font metrics の `ascent + descent`
/// 由来。em ボックス丈 `font_height` ではない——em で切ると和文フォントの descent インクが帯から
/// はみ出す。[`ChoiceLineContent::band_offset`]＝帯を行ボックスの中央へ置く寄せ——近端へ揃えると
/// 帯が上に余りながら下でインクを切る・R13.1）。住人平行移動 `(dx, dy)` と可視窓オフセット
/// `block_offset`（横＝Y・縦＝X）を反映し、glyph 描画原点と同一系へ揃える（ヒット矩形／指紋と
/// 同座標系・R3.3）。座標は合成スケール `k` 適用前の image px
/// （呼び手が `SetTransform(scale(k))` 下で `FillRectangle` する）。
fn highlight_rect(
    inline_range: (f32, f32),
    offset: (f32, f32),
    block_offset: f32,
    band_extent: f32,
    band_offset: f32,
    mode: WritingMode,
) -> D2D_RECT_F {
    let (i0, i1) = inline_range;
    let (dx, dy) = offset;
    // ブロック軸の帯の起点は「住人原点 ＋ 可視窓オフセット ＋ 帯の寄せ」。寄せ（band_offset）は
    // 帯を行ボックスの中央へ置くための量で、ヒット導出（`derive_hit_rows`）へ渡す値と同一である。
    match mode {
        WritingMode::HorizontalTb => D2D_RECT_F {
            left: dx + i0,
            top: dy + block_offset + band_offset,
            right: dx + i1,
            bottom: dy + block_offset + band_offset + band_extent,
        },
        WritingMode::VerticalRl | WritingMode::VerticalLr => D2D_RECT_F {
            left: dx + block_offset + band_offset,
            top: dy + i0,
            right: dx + block_offset + band_offset + band_extent,
            bottom: dy + i1,
        },
    }
}

/// Choice 住人のダーティ帯を **ハイライト帯の超過分**まで広げた [`LineOverhang`] を返す（純粋）。
///
/// ダーティ矩形は em ボックス（`font_height`）＋実測インクはみ出しで組まれる（D2）。ハイライト帯
/// （`band_extent`＝`ascent + descent` 由来）は行矩形の block 近端から `band_offset` だけ寄せた
/// 位置に置かれるので、遠端側の超過分は `band_offset + band_extent − font_height` になる
/// （寄せを数えないと、寄せた分だけ帯の下（縦書きは右）がダーティ矩形の外へ落ち、hover 解除の
/// フレームで塗りが消し残る・R13.5）。この超過分をブロック軸**遠端**（横書き＝`bottom`・
/// 縦書き＝`right`——[`highlight_rect`] が帯を伸ばす向き）の下限として与える。実測インクはみ出しの
/// 方が大きい場合は実測値を保つ（`max`）。これにより hover フレームの塗りが欠けず、hover 解除
/// フレームで塗りが消し残らない（同一帯が両フレームでダーティになる）。
fn expand_overhang_for_band(
    measured: LineOverhang,
    band_extent: f32,
    band_offset: f32,
    font_height: f32,
    mode: WritingMode,
) -> LineOverhang {
    let excess = (band_offset + band_extent - font_height).max(0.0);
    match mode {
        WritingMode::HorizontalTb => LineOverhang {
            bottom: measured.bottom.max(excess),
            ..measured
        },
        WritingMode::VerticalRl | WritingMode::VerticalLr => LineOverhang {
            right: measured.right.max(excess),
            ..measured
        },
    }
}

/// hover セグメントの resident-local `inline_range` を run のグリフ列へ照合し、`SetDrawingEffect`
/// 用の UTF-16 文字範囲（[`DWRITE_TEXT_RANGE`]）を導く。
///
/// 行 TextLayout の text は `run.glyphs` の文字列（クラスタ）の連結ゆえ、グリフ index と text 位置は
/// 各グリフの文字列の UTF-16 長の累積で 1:1 対応する（範囲の境界はクラスタの境界に揃う）。グリフ中心（`inline_pos + advance/2`）が `inline_range` に入る
/// **連続**グリフ subrange を採り、その手前までの累積を `startPosition`・subrange の累積長を
/// `length` とする（境界がグリフ境界に一致するため中心判定が浮動小数誤差に頑健）。交差グリフ
/// なしは `None`（効果を適用しない）。
fn segment_text_range(
    glyphs: &[PositionedGlyph],
    inline_range: (f32, f32),
) -> Option<DWRITE_TEXT_RANGE> {
    let (i0, i1) = inline_range;
    let mut acc: u32 = 0;
    let mut start: Option<u32> = None;
    let mut length: u32 = 0;
    for g in glyphs {
        let units = g.text.encode_utf16().count() as u32;
        let center = g.inline_pos + g.advance * 0.5;
        if center > i0 && center < i1 {
            if start.is_none() {
                start = Some(acc);
            }
            length += units;
        }
        acc += units;
    }
    start.map(|start_position| DWRITE_TEXT_RANGE {
        startPosition: start_position,
        length,
    })
}
