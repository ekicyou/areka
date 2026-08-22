//! # viewbox_draw — ViewboxExecutor（COM 層・plan 実行パイプライン）
//!
//! [`ScrollPlanner`](crate::viewbox::ScrollPlanner)（純粋・計画）が返す
//! [`FramePlan`](crate::viewbox::FramePlan) を 1 フレームの COM 実行へ落とす
//! [`ViewboxExecutor`]。旧 [`DrawExecutor`](crate::draw::DrawExecutor)（全域再描画）を
//! 「保持ピクセルの面内 blit ＋ ダーティ矩形限定の D2D 描画」へ差し替える実行部である
//! （viewbox 方式＝ダーティ矩形スクロール・`WM_PAINT` 規律の写し）。
//!
//! **層規律**: COM 層——UI スレッド専有。`windows`（DirectWrite/D2D）を触ってよい層。
//! 失敗は log-first（`tracing::error!`＋`Err`・当該フレーム skip＝**plan 未 commit**）で扱い
//! panic しない（記憶 areka-log-first-no-silent-failure）。
//!
//! ## 1 フレームの実行（design System Flows）
//!
//! `render` は「計画取得（[`ScrollPlanner::plan`]）→ 保持ピクセルの面内 blit
//! （[`TextSurface::copy_front_to_back_shifted`]）→ ダーティ矩形ごとの限定描画 → 面の役割交換
//! （[`TextSurface::flip`]）→ 計画の確定（[`ScrollPlanner::commit`]）」の順で行い、変化有無
//! （present 要否）を返す。[`FramePlan::NoChange`] は blit も描画も present も行わない
//! （`Ok(false)`・全カウンタ増分 0）。
//!
//! ## ダーティ描画の正準列（byte 等価の要・design①〜⑥）
//!
//! ダーティ矩形ごとに ①`SetTransform(identity)` → ②`PushAxisAlignedClip`（物理整数矩形・
//! `ALIASED`）→ ③`Clear(None)`（透明・クリップ内のみ）→ ④`SetTransform(scale(k))`（この一点
//! のみ）→ ⑤描画対象住人を `DrawTextLayout`（origin は [`DrawExecutor`] と**同一式**）→
//! ⑥`PopAxisAlignedClip`。恒等変換下で物理整数矩形へ描画範囲を限定してから透明化・合成スケール
//! 適用・描画・範囲解除の順を守る（ダーティ限定は Direct2D の矩形範囲限定機構を直接用い、wintf の
//! クリップ機構（`ClipShape`/`clip_sync_system`）には依存しない・R9.4）。
//!
//! ## 保持機構は「描画面ピクセル＋blit」のみ（R3.4）
//!
//! 確定済み content 用に別途のビットマップキャッシュや描画コマンド列キャッシュ（グリフ bitmap／
//! `ID2D1CommandList`）を設けない。行 TextLayout は [`LineLayoutStore`]（[`DrawExecutor`] と共有・
//! byte 等価の構造前提 RN5）を経由し、確定行（内容不変）は再生成しない。
//!
//! ## 共有経路（RN5——byte 等価の構造前提）
//!
//! format（[`create_text_format`]＋再利用規律）・行 TextLayout（[`LineLayoutStore`]）・
//! D2D ターゲット bitmap（[`create_d2d_target_bitmap`]・同一 props）・専用 D2D DC
//! （`D2D1_DEVICE_CONTEXT_OPTIONS_NONE`）を [`DrawExecutor`] と同一生成経路で用いる。origin 式・
//! スケール一点適用・描画状態は既定のまま両者同一——比較専用オラクル（[`DrawExecutor`]）との
//! byte 等価はこの構造共有に載る。

use tracing::warn;
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE,
    ID2D1DeviceContext, ID2D1Image, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_TEXT_RANGE, IDWriteFactory2, IDWriteTextFormat, IDWriteTextLayout,
};
use windows::core::IUnknown;
use windows_numerics::{Matrix3x2, Vector2};
use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::ecs::GraphicsCore;

use crate::TextLayerError;
use crate::canvas::{ContentCanvas, ResidentContent};
use crate::draw::{LineLayoutStore, ResolvedFont, create_d2d_target_bitmap, create_text_format};
use crate::layout::{PositionedGlyph, VisibleWindow};
use crate::region::ScaleContract;
use crate::surface::TextSurface;
use crate::viewbox::{FramePlan, LineOverhang, PhysicalRect, ScrollPlanner, ScrollState};
use crate::writing::WritingMode;

/// 行 TextLayout format の前提（フォント名・高さビット・writing_mode）——変わると
/// キャッシュ済み行レイアウトの前提が崩れるため format と行キャッシュを組み直す
/// （draw.rs `FormatKey` と同一規律のインライン版）。float はビット表現で同値比較する。
type FormatKey = (String, u32, WritingMode);

/// 決定論観測用の描画統計（常時コンパイル・u64 加算のみ・R3.5/R10.3）。
///
/// blit／`DrawTextLayout`／FullClear／行 TextLayout 生成の累計を提供し、「可視窓が変化しない
/// 入力では blit・描画が発生しない」「可視窓のみ移動では保持ピクセルの複製と露出帯の描画だけが
/// 発生する」を決定論に観測する読み口（[`ViewboxExecutor::stats`]）。
#[derive(Clone, Copy, Debug, Default)]
pub struct DrawStats {
    /// 行 TextLayout の累計生成回数（[`LineLayoutStore`] 経由・確定行は再生成しないことの檻）。
    pub line_layout_creations: u64,
    /// `DrawTextLayout` の累計実行回数（ダーティ交差行ぶんに限られることの檻）。
    pub draw_text_layout_calls: u64,
    /// 面内 blit（`copy_front_to_back_shifted` の blit≠0）の累計回数。
    pub blits: u64,
    /// FullClear（back を全域透明 Clear）の累計回数。
    pub full_clears: u64,
}

/// [`FramePlan`] の COM 実行——blit 指示・ダーティ矩形限定の D2D 描画・FullClear・統計
/// （task 6・R1.1/R1.4/R2.2/R3.1–3.4/R9.4）。
///
/// `render` が 1 フレーム（plan→blit→ダーティ描画→flip→commit）を実行し present 要否を返す。
/// plan/commit の二相により、デバイス失敗フレームは**未 commit**のまま次フレームで再計画＝再試行
/// 安全（現行の「当該フレーム skip・次フレーム再試行」規律を保つ）。UI スレッド専有（COM 層規律）。
pub struct ViewboxExecutor {
    /// スクロール計画者（純粋・計画/commit 二相・committed 位置と行指紋を内部保持）。
    planner: ScrollPlanner,
    /// 行 TextLayout の生成・キャッシュストア（[`DrawExecutor`](crate::draw::DrawExecutor) と
    /// 同一型・確定行は再生成しない＝byte 等価の構造前提 RN5・保持機構は別キャッシュを設けない）。
    line_store: LineLayoutStore,
    /// 行 TextLayout 生成用 factory（format 組み直しに使い続けるため本体にも保持）。
    dwrite: IDWriteFactory2,
    /// 専用 D2D DC（`DrawExecutor` と同一生成経路・`D2D1_DEVICE_CONTEXT_OPTIONS_NONE`・
    /// wintf の共有 DC の描画状態を汚さない・ターゲットは render 中のみ設定）。
    dc: ID2D1DeviceContext,
    /// 描画/計測共用 format（[`create_text_format`] 経路・`FormatKey` 不変なら再利用）。
    format: Option<(FormatKey, IDWriteTextFormat)>,
    /// 決定論観測統計（常時コンパイル・[`Self::stats`] で読む）。
    stats: DrawStats,
    /// Image/Surface 住人シームの warn 抑制フラグ（executor ごと初回のみ・planner の
    /// `draw_lines` は GlyphRun のみを返すため通常不発の防御的経路）。
    seam_warned: bool,
    /// テスト専用 fault-injection: true の間、次の Update フレームの EndDraw 後に
    /// デバイス失敗を注入する（実 COM 失敗を決定論的に再現できないため・G5）。flip/commit の
    /// **前**に `Err` を返し、失敗フレームの再試行安全（front 不変・planner 未 commit）を檻化する。
    #[cfg(test)]
    fail_next_render: bool,
}

impl ViewboxExecutor {
    /// `GraphicsCore` から plan 実行部を生成する（DWrite factory＋専用 D2D DC＋
    /// [`LineLayoutStore`]＋[`ScrollPlanner`]）。
    ///
    /// デバイス未初期化（`GraphicsCore` 無効化後）は log-first で `Device` エラー
    /// （`DrawExecutor::new` と同一経路）。
    pub fn new(core: &GraphicsCore) -> Result<ViewboxExecutor, TextLayerError> {
        let dwrite = core
            .dwrite_factory()
            .ok_or_else(|| none_err("GraphicsCore::dwrite_factory"))?
            .clone();
        let d2d = core
            .d2d_device()
            .ok_or_else(|| none_err("GraphicsCore::d2d_device"))?;
        let dc = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .map_err(device_err("CreateDeviceContext(ViewboxExecutor)"))?;
        let line_store = LineLayoutStore::new(&dwrite);
        Ok(ViewboxExecutor {
            planner: ScrollPlanner::new(),
            line_store,
            dwrite,
            dc,
            format: None,
            stats: DrawStats::default(),
            seam_warned: false,
            #[cfg(test)]
            fail_next_render: false,
        })
    }

    /// テスト専用: 次の Update フレームの EndDraw 後にデバイス失敗を 1 回注入する（G5・
    /// 失敗フレームの再試行安全を檻化するため）。次フレームで消費され自動解除される。
    #[cfg(test)]
    fn inject_render_failure(&mut self) {
        self.fail_next_render = true;
    }

    /// 決定論観測口（テスト・example 双方が読む・R3.5/R10.3）。
    pub fn stats(&self) -> DrawStats {
        self.stats
    }

    /// スクロール量子化状態の読み口（内部 [`ScrollPlanner`] へ委譲・additive）。
    ///
    /// 面に反映済みの whole-pixel スクロール（[`ScrollState::committed`]）を提示フレーム同期で
    /// 読むための口。結線層（task 8）の照会スナップショット写像が
    /// `scroll_state().committed` を [`to_window_physical`](crate::choice::to_window_physical) の
    /// committed 引数へ渡す（design.md「RuntimeContract」Implementation Notes・R9.3 契約点）。
    /// UI スレッド専有（COM 層規律）。
    pub fn scroll_state(&self) -> ScrollState {
        self.planner.scroll_state()
    }

    /// Clear cue の適用点（planner 初期化＋行 TextLayout キャッシュ全破棄——破棄はこの口だけ・
    /// R4.3）。
    ///
    /// [`ScrollPlanner::request_clear`]（`clear_requested`＋committed/pos/prev_lines 初期化）と
    /// [`LineLayoutStore::clear`] を呼ぶ。次フレームの [`Self::render`] は `plan` が
    /// [`FramePlan::FullClear`] を返し、back を全域透明 Clear（`full_clears` +1）→ flip → commit
    /// する（その後 prev_lines 空ゆえ次 content フレームは全域ダーティで再描画＝透明フラッシュは
    /// FullClear の 1 フレームのみ）。actor 結線（task 8）がこの口を Clear cue へ写像する。
    pub fn request_clear(&mut self) {
        self.planner.request_clear();
        self.line_store.clear();
    }

    /// 1 フレームの実行。戻り値＝変化有無（`true` なら呼び手が present する・R1.1/R3.1）。
    ///
    /// - format 確保（`ensure_format`——フォント/方向不変なら再利用）。
    /// - `plan`（[`ScrollPlanner::plan`]・状態不変・純粋）。
    /// - [`FramePlan::NoChange`]: blit も描画も present も行わない（`Ok(false)`・stats 増分なし・
    ///   commit 不要）。
    /// - [`FramePlan::FullClear`]: back を全域透明 Clear（描画 0 件）→ flip → commit → `Ok(true)`。
    /// - [`FramePlan::Update`]: 保持ピクセルの面内 blit → ダーティ矩形ごとの正準列（①〜⑥）で
    ///   限定描画 → flip → commit → `Ok(true)`。
    ///
    /// **エラー縮退規律**（Error Handling）: フォント/方向変更（`ensure_format` が format/行キャッシュを
    /// 組み直したフレーム）または `plan` の想定外不整合（[`plan_inconsistency`]）を検知した場合、当該
    /// フレームを**全域ダーティ Update**（`blit=(0,0)`・dirty=面全域・draw_lines=全 GlyphRun 住人）へ
    /// 差し替えて描画する（正しさ優先・1 フレームでレガシー全域再描画と等価・透明フラッシュを起こす
    /// FullClear ではない）。フォント/方向変更は `debug!`・想定外不整合は `warn!` を残す（記憶
    /// areka-log-first-no-silent-failure）。縮退後の commit が prev_lines を張り直すため次フレームは
    /// 正常導出へ復帰する。
    ///
    /// 失敗は log-first（`error!`＋`Err`・当該フレーム skip＝**plan 未 commit**・front 不変ゆえ
    /// 表示は前フレームを保持・次フレーム再計画）。
    pub fn render(
        &mut self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        font: &ResolvedFont,
        mode: WritingMode,
        contract: &ScaleContract,
        surface: &mut TextSurface,
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
                    let text: String = run.glyphs.iter().map(|g| g.ch).collect();
                    self.line_store
                        .line_layout(index, &text, &format, font.height, mode)?;
                    self.line_store.overhang(index).unwrap_or_default()
                }
                // Choice 住人は内包 run を GlyphRun と同一経路で計測する（R9.5）。
                ResidentContent::Choice(choice) if !choice.run.glyphs.is_empty() => {
                    let text: String = choice.run.glyphs.iter().map(|g| g.ch).collect();
                    self.line_store
                        .line_layout(index, &text, &format, font.height, mode)?;
                    let measured = self.line_store.overhang(index).unwrap_or_default();
                    // ハイライト帯（band_extent）は em ボックス丈より外側へ出る（descent 込み）。
                    // ダーティ矩形が em ボックス＋実測インクはみ出しのままだと、帯の外側部分が
                    // クリップで塗り残り／消し残りになる（hover 解除フレームに塗りが残る）。
                    // ゆえにブロック軸の遠端はみ出しを **帯の超過分**まで広げる（横書き＝下・
                    // 縦書き＝右——`highlight_rect` が帯を伸ばす向きと同一）。
                    expand_overhang_for_band(measured, choice.band_extent, font.height, mode)
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
                // 描画対象住人（draw_lines）の TextLayout・origin・Choice ハイライト資源を組む。行
                // レイアウトは plan 前のはみ出し収集ループで確保済み（ここは全てキャッシュヒット）。
                // origin 式は DrawExecutor と同一（validrect-local 平行移動＋ブロック軸の可視窓
                // オフセット）。ブラシ生成・`SetDrawingEffect`（**DrawingEffect リセット正準列**——
                // キャッシュ層 TextLayout を汚さないため全文字範囲へ `None` を必ず適用してから hover
                // 範囲へ文字色効果を焼く）は可謬ゆえこの BeginDraw 前区間で `?` 伝播する（失敗フレームは
                // target 未設定のまま skip＝再試行安全・R4.5/R4.6）。
                let block_offset = window.block_offset;
                let mut draws: Vec<LineDraw> = Vec::new();
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
                    let text: String = run.glyphs.iter().map(|g| g.ch).collect();
                    let layout =
                        self.line_store
                            .line_layout(index, &text, &format, font.height, mode)?;
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

                    // Choice 行はキャッシュ層 TextLayout に前フレームの効果が残り得るため、描画毎に
                    // 全文字範囲を `None` へリセットする（hover 解除フレームは全範囲 None のみ＝素描画）。
                    let choice_draw = if let Some(choice) = choice {
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
                                // 住人 transform ＋block_offset 反映＝ヒット矩形／指紋と同座標系・R3.3）。
                                // 帯は em ボックス丈（font.height）ではなく choice.band_extent
                                // （descent 込みの実行ボックス丈）——`derive_hit_rows` へ渡る値と同一。
                                rects.push(highlight_rect(
                                    seg.inline_range,
                                    (dx, dy),
                                    block_offset,
                                    choice.band_extent,
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

                    draws.push(LineDraw {
                        origin,
                        layout,
                        choice: choice_draw,
                    });
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
                for rect in dirty {
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
                    // ⑤ 該当行を描画（クリップにより描画結果は dirty 矩形内へ限定される）。
                    // Choice hover 行は (a) セグメント矩形を塗り色で `FillRectangle` してから
                    // (b) `DrawTextLayout`（効果範囲の文字だけが Phase 1 で焼いた切替色で描かれる）。
                    for d in &draws {
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
                            D2D1_DRAW_TEXT_OPTIONS_NONE,
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
            let format = create_text_format(&self.dwrite, font, mode)?;
            self.format = Some((key, format.clone()));
            return Ok((format, true));
        }
        // 初回生成（committed ピクセルが無いため縮退トリガにしない）。
        let format = create_text_format(&self.dwrite, font, mode)?;
        self.format = Some((key, format.clone()));
        Ok((format, false))
    }
}

/// エラー縮退規律の適用（Error Handling）——`plan` を必要なら**全域ダーティ Update** へ縮退する。
///
/// 2 トリガ（正しさ優先・いずれも透明フラッシュを起こす FullClear ではなく全域ダーティ Update）:
/// - **フォント/方向変更**（`rebuilt`＝`ensure_format` が format/行キャッシュを組み直した）: committed
///   ピクセルは旧 format で描かれ前提が崩れるため `debug!`＋全域ダーティへ縮退。
/// - **想定外不整合**（[`plan_inconsistency`] が理由を返す）: `plan` の `Update` が canvas/面寸と矛盾
///   （draw_lines 範囲外・dirty 面寸超過）する場合 `warn!`＋全域ダーティへ縮退（ログ無し失敗経路を
///   作らない・記憶 areka-log-first-no-silent-failure）。
///
/// [`FramePlan::FullClear`] は縮退対象外（Clear は既に全域リセット）。[`FramePlan::NoChange`] は
/// `rebuilt` のときのみ縮退（format 組み直し後の committed ピクセル前提消失を全域再描画で回復）。
fn degrade_if_needed(
    plan: FramePlan,
    rebuilt: bool,
    canvas: &ContentCanvas,
    window: &VisibleWindow,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
) -> FramePlan {
    match plan {
        FramePlan::FullClear => FramePlan::FullClear,
        FramePlan::NoChange => {
            if rebuilt {
                tracing::debug!(
                    "フォント/方向変更を検知——committed ピクセル前提消失のため全域ダーティへ縮退"
                );
                full_domain_update(canvas, window, mode, contract, surface_size)
            } else {
                FramePlan::NoChange
            }
        }
        FramePlan::Update {
            blit,
            dirty,
            draw_lines,
        } => {
            if rebuilt {
                tracing::debug!(
                    "フォント/方向変更を検知——format/行キャッシュ組み直し・全域ダーティへ縮退（committed ピクセル前提消失）"
                );
                full_domain_update(canvas, window, mode, contract, surface_size)
            } else if let Some(reason) =
                plan_inconsistency(&dirty, &draw_lines, canvas.residents.len(), surface_size)
            {
                tracing::warn!(
                    reason,
                    "plan と canvas/面寸の想定外不整合——全域ダーティ再描画へ縮退（正しさ優先・最悪でもレガシー全域再描画と等価）"
                );
                full_domain_update(canvas, window, mode, contract, surface_size)
            } else {
                FramePlan::Update {
                    blit,
                    dirty,
                    draw_lines,
                }
            }
        }
    }
}

/// 全域ダーティ Update（`blit=(0,0)`・dirty=面全域 1 枚・draw_lines=全 GlyphRun 住人）を組む。
///
/// [`ScrollPlanner::derive_dirty`] を**空 prev**（`&[]`）で呼び、面全域 1 枚のダーティと全 GlyphRun
/// 住人の描画対象を得る（初回フレームと同一経路＝レガシー全域再描画と等価）。縮退の唯一の生成口。
fn full_domain_update(
    canvas: &ContentCanvas,
    window: &VisibleWindow,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
) -> FramePlan {
    let (dirty, draw_lines) =
        ScrollPlanner::derive_dirty(canvas, window, mode, contract, (0, 0), surface_size, &[]);
    FramePlan::Update {
        blit: (0, 0),
        dirty,
        draw_lines,
    }
}

/// `plan` の `Update` が canvas/面寸と矛盾していないか検査する（防御的・render の縮退経路が呼ぶ）。
///
/// 返り値 `Some(reason)` は縮退のログ理由・`None` は整合。通常経路（[`ScrollPlanner::derive_dirty`]）は
/// 面寸クランプ済み・住人範囲内の index のみを返すため不発だが、行指紋と内容キャンバスの想定外不整合
/// （範囲外 index・面寸超過矩形）を検知したら全域ダーティへ縮退させる（ログ無し失敗経路を作らない）。
fn plan_inconsistency(
    dirty: &[PhysicalRect],
    draw_lines: &[usize],
    residents_len: usize,
    surface_size: (u32, u32),
) -> Option<&'static str> {
    if draw_lines.iter().any(|&i| i >= residents_len) {
        return Some("draw_lines に canvas 住人範囲外の index が含まれる");
    }
    let (w, h) = (surface_size.0 as u64, surface_size.1 as u64);
    if dirty
        .iter()
        .any(|r| r.x as u64 + r.w as u64 > w || r.y as u64 + r.h as u64 > h)
    {
        return Some("dirty 矩形が面寸を超える");
    }
    None
}

/// `Option` が `None`（デバイス未初期化など本来到達しない欠落）を [`TextLayerError::Device`] に
/// する（draw.rs/surface.rs と同型の log-first ヘルパ）。
fn none_err(context: &'static str) -> TextLayerError {
    tracing::error!(
        context,
        "必須リソースが欠落（デバイス未初期化 または 前提不成立）"
    );
    TextLayerError::Device {
        hresult: 0,
        context,
    }
}

/// `windows_core::Error` を [`TextLayerError::Device`] へ写像する（draw.rs/surface.rs と同型の
/// log-first ヘルパ: `error!`＋`Err` 戻り値・panic 禁止）。
fn device_err(context: &'static str) -> impl FnOnce(windows::core::Error) -> TextLayerError {
    move |e| {
        let hresult = e.code().0;
        tracing::error!(hresult, context, "DirectWrite/D2D 呼び出しが失敗");
        TextLayerError::Device { hresult, context }
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

/// RGB を不透明（α=1.0）の [`D2D1_COLOR_F`] へ写す（0..255→0.0..1.0）。
fn color_f(rgb: (u8, u8, u8)) -> D2D1_COLOR_F {
    let (r, g, b) = rgb;
    D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

/// hover セグメント矩形を描画空間（image px・住人 transform＋block_offset 反映）で組む。
///
/// 行内軸＝セグメントの `inline_range`（文字幅）・ブロック軸帯＝`band_extent`
/// （[`ChoiceLineContent::band_extent`]＝実 font metrics の `ascent + descent` 由来。em ボックス丈
/// `font_height` ではない——em で切ると和文フォントの descent インクが帯からはみ出す）。住人平行移動
/// `(dx, dy)` と可視窓オフセット `block_offset`（横＝Y・縦＝X）を反映し、glyph 描画原点と同一系へ
/// 揃える（ヒット矩形／指紋と同座標系・R3.3）。座標は合成スケール `k` 適用前の image px
/// （呼び手が `SetTransform(scale(k))` 下で `FillRectangle` する）。
fn highlight_rect(
    inline_range: (f32, f32),
    offset: (f32, f32),
    block_offset: f32,
    band_extent: f32,
    mode: WritingMode,
) -> D2D_RECT_F {
    let (i0, i1) = inline_range;
    let (dx, dy) = offset;
    match mode {
        WritingMode::HorizontalTb => D2D_RECT_F {
            left: dx + i0,
            top: dy + block_offset,
            right: dx + i1,
            bottom: dy + block_offset + band_extent,
        },
        WritingMode::VerticalRl | WritingMode::VerticalLr => D2D_RECT_F {
            left: dx + block_offset,
            top: dy + i0,
            right: dx + block_offset + band_extent,
            bottom: dy + i1,
        },
    }
}

/// Choice 住人のダーティ帯を **ハイライト帯の超過分**まで広げた [`LineOverhang`] を返す（純粋）。
///
/// ダーティ矩形は em ボックス（`font_height`）＋実測インクはみ出しで組まれる（D2）。ハイライト帯
/// （`band_extent`＝`ascent + descent` 由来）はそれより外側へ出るため、超過分
/// `band_extent − font_height` をブロック軸**遠端**（横書き＝`bottom`・縦書き＝`right`——
/// [`highlight_rect`] が帯を伸ばす向き）の下限として与える。実測インクはみ出しの方が大きい場合は
/// 実測値を保つ（`max`）。これにより hover フレームの塗りが欠けず、hover 解除フレームで
/// 塗りが消し残らない（同一帯が両フレームでダーティになる）。
fn expand_overhang_for_band(
    measured: LineOverhang,
    band_extent: f32,
    font_height: f32,
    mode: WritingMode,
) -> LineOverhang {
    let excess = (band_extent - font_height).max(0.0);
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
/// 行 TextLayout の text は `run.glyphs` の `ch` 連結ゆえ、グリフ index と text 位置は各 `ch` の
/// UTF-16 長の累積で 1:1 対応する。グリフ中心（`inline_pos + advance/2`）が `inline_range` に入る
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
        let units = g.ch.len_utf16() as u32;
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

#[cfg(test)]
#[path = "viewbox_draw_choice_hover_tests.rs"]
mod choice_hover_tests;
#[cfg(test)]
#[path = "viewbox_draw_frame_render_tests.rs"]
mod frame_render_tests;
#[cfg(test)]
#[path = "viewbox_draw_live_diff_tests.rs"]
mod live_diff_tests;
#[cfg(test)]
#[path = "viewbox_draw_oracle_regression_tests.rs"]
mod oracle_regression_tests;
#[cfg(test)]
#[path = "viewbox_draw_png_dump_tests.rs"]
mod png_dump_tests;
#[cfg(test)]
#[path = "viewbox_draw_test_support.rs"]
mod test_support;
