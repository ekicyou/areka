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
fn segment_text_range(glyphs: &[PositionedGlyph], inline_range: (f32, f32)) -> Option<DWRITE_TEXT_RANGE> {
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
mod tests {
    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_ecs::name::Name;
    use bevy_ecs::prelude::World;
    use windows::UI::Composition::Compositor;
    use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
    use wintf::com::wuc::create_dispatcher_queue_controller;
    use wintf::ecs::{GraphicsCore, Visual};

    use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

    use super::{DrawStats, ViewboxExecutor};
    use crate::actor::TextSlotBinding;
    use crate::canvas::{
        ChoiceLineContent, ChoiceRowSegment, ContentCanvas, HighlightPaint, Resident,
        ResidentContent,
    };
    use crate::draw::{DWriteMetrics, DrawExecutor, ResolvedFont};
    use crate::layout::{FixedMetrics, LayoutEngine, VisibleWindow, WrapPlan};
    use crate::region::{ScaleContract, TextRegion};
    use crate::state::{TextItem, TextLayerConfig, TextLayerState};
    use crate::surface::TextSurface;
    use crate::viewbox::{FramePlan, ScrollPlanner};
    use crate::writing::WritingMode;

    /// テスト用 WUC apartment/dispatcher（surface.rs/draw.rs テストと同一方針:
    /// COM 未初期化のテストスレッドでは ASTA 第一候補・NONE 保険）。
    fn make_dispatcher_and_compositor() -> (windows::System::DispatcherQueueController, Compositor)
    {
        let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
            .or_else(|e_asta| {
                create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta)
            })
            .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
        let compositor = Compositor::new().expect("Compositor::new 失敗");
        (dq, compositor)
    }

    /// ViewboxExecutor 検証リグ（dispatcher/compositor/core/World の寿命を束ねる・headless）。
    struct Rig {
        _dq: windows::System::DispatcherQueueController,
        compositor: Compositor,
        core: GraphicsCore,
        world: World,
    }

    impl Rig {
        fn new() -> Rig {
            let (_dq, compositor) = make_dispatcher_and_compositor();
            let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
            Rig {
                _dq,
                compositor,
                core,
                world: World::new(),
            }
        }

        /// 予約スロット（emo-present VisualMount 同型）を組み、image px 原寸と k から
        /// TextSurface を装着する（物理寸＝ceil(image × k)・offset (0,0)）。
        fn attach(&mut self, image_size: (u32, u32), k: f32) -> TextSurface {
            let window = self.world.spawn_empty().id();
            let slot = self
                .world
                .spawn((
                    Name::new("emo-text-layer-slot"),
                    Visual::default(),
                    ChildOf(window),
                ))
                .id();
            self.world.flush();
            let physical = (
                (image_size.0 as f32 * k).ceil() as u32,
                (image_size.1 as f32 * k).ceil() as u32,
            );
            let binding = TextSlotBinding::new(slot, window, k, physical, image_size);
            TextSurface::attach(
                &mut self.world,
                &binding,
                &self.compositor,
                &self.core,
                physical,
                (0.0, 0.0),
            )
            .expect("TextSurface::attach 失敗")
        }
    }

    /// テスト用 BalloonModel（幾何のみ・origin (0,0)・font 高さ指定可・validrect 全域）。
    fn geo_model(font_height: Option<u32>) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(Some(0), Some(0)),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            Font::new(None, font_height, FontColor::new(None, None, None)),
            None,
            None,
        )
    }

    /// live-diff 用 BalloonModel（**origin 未指定**＝mode ごとの書字開始角へ寄せる・font 高さ
    /// 指定可・validrect 全域）。origin (0,0) を明示すると vertical_rl では validrect 内の
    /// 左上に留まり列が面外（負の x）へ描かれてしまうため、origin は None にして
    /// クランプ正準（horizontal/vertical_lr＝左上・vertical_rl＝右上）へ委ねる。
    /// フォント名＋高さを明示した live-diff 用モデル（既定フォントは `name=None`＝ＭＳ ゴシック・
    /// G4——非 default フォント/大サイズで AA こぼれガード `DIRTY_GUARD_IMG_PX` の実効性を byte
    /// 等価で検証するため）。
    fn live_diff_model_font(name: Option<&str>, font_height: Option<u32>) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(None, None),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            Font::new(
                name.map(str::to_owned),
                font_height,
                FontColor::new(None, None, None),
            ),
            None,
            None,
        )
    }

    /// 文字列→グリフ item 列。
    fn glyph_items(s: &str) -> Vec<TextItem> {
        s.chars().map(|ch| TextItem::Glyph { ch }).collect()
    }

    /// items→(canvas, visible_window)（純粋レイアウト・FixedMetrics・visible は全量）。
    fn build(
        items: &[TextItem],
        region: &TextRegion,
        mode: WritingMode,
        font_height: f32,
    ) -> (ContentCanvas, VisibleWindow) {
        let visible = items
            .iter()
            .filter(|i| matches!(i, TextItem::Glyph { .. }))
            .count();
        let lines = LayoutEngine::layout(
            items,
            visible,
            &region,
            mode,
            font_height,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let canvas = ContentCanvas::from_layout(&lines, &region, mode);
        let window = LayoutEngine::visible_window(&lines, &region, mode);
        (canvas, window)
    }

    /// 非透明ピクセル数（BGRA 密配列の α≠0）。
    fn opaque_count(bytes: &[u8]) -> usize {
        bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
    }

    /// ブロック軸（行送り軸）方向のインク範囲 `(near, far)`（物理 px・両端含む・インクなしは `None`）。
    /// horizontal_tb＝y（上端〜下端）・vertical_rl/lr＝x（左端〜右端）——スクロールで content が
    /// 動く軸。k≠1.0 の許容差比較（G2）で oracle と viewbox のインク位置差を測る。
    fn block_axis_ink_span(bytes: &[u8], w: u32, h: u32, mode: WritingMode) -> Option<(u32, u32)> {
        let mut lo: Option<u32> = None;
        let mut hi: u32 = 0;
        for y in 0..h {
            for x in 0..w {
                if bytes[((y * w + x) * 4 + 3) as usize] != 0 {
                    let b = match mode {
                        WritingMode::HorizontalTb => y,
                        WritingMode::VerticalRl | WritingMode::VerticalLr => x,
                    };
                    lo = Some(lo.map_or(b, |v| v.min(b)));
                    hi = hi.max(b);
                }
            }
        }
        lo.map(|l| (l, hi))
    }

    /// 観測可能な完了状態 1: content ありの初回フレーム → render `Ok(true)`・read_back に
    /// content の非透明ピクセルが現れる（初回は全域ダーティ＝全行描画・blit=(0,0) ゆえ blits 増なし）。
    #[test]
    fn initial_frame_renders_content() {
        let mut rig = Rig::new();
        let image = (80u32, 40u32);
        let mut surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        let items = glyph_items("■■");
        let (canvas, window) = build(&items, &region, mode, 10.0);
        let changed = exec
            .render(&canvas, &window, &font, mode, &contract, &mut surface)
            .expect("render 失敗");
        assert!(
            changed,
            "content ありの初回フレームは変化あり（present 要）"
        );

        let bytes = surface.read_back().expect("read_back 失敗");
        assert!(
            opaque_count(&bytes) > 0,
            "初回描画で content の非透明ピクセルが現れる"
        );

        let stats: DrawStats = exec.stats();
        assert!(
            stats.draw_text_layout_calls >= 1,
            "初回は少なくとも 1 行描画する"
        );
        assert_eq!(
            stats.blits, 0,
            "初回 blit=(0,0) は blits を増やさない（全面 CopyResource）"
        );
    }

    /// canvas の GlyphRun 住人を、内包 run が等価な非 hover の Choice 住人へ写す
    /// （`segments` 空・`hovered=None`・`highlight=None`）。transform/effects は不変。
    /// 「Choice は GlyphRun と同格の素描画」（R1.4/R9.5）を検証するための等価変換。
    fn as_choice_canvas(canvas: &ContentCanvas) -> ContentCanvas {
        let residents = canvas
            .residents
            .iter()
            .map(|r| match &r.content {
                ResidentContent::GlyphRun(run) => Resident {
                    content: ResidentContent::Choice(ChoiceLineContent {
                        run: run.clone(),
                        segments: Vec::new(),
                        hovered: None,
                        highlight: None,
                        // 素描画等価の検証ゆえ帯は em ボックス丈（塗りを持たない）。
                        band_extent: run.size.1,
                    }),
                    transform: r.transform,
                    effects: r.effects,
                },
                _ => r.clone(),
            })
            .collect();
        ContentCanvas {
            residents,
            size: canvas.size,
        }
    }

    /// R1.4/R9.5（ピクセル同一）: 非 hover（`highlight=None`）の Choice 住人は、内包 run が
    /// 等価な GlyphRun 住人と**readback バイト完全一致**で描画される。GlyphRun canvas と
    /// Choice canvas をそれぞれ独立の executor/供給面へ初回フレーム描画し、read_back を byte 比較する。
    /// （Choice アームが run を GlyphRun と別経路で描く・寸を取り違える等の変異はこの檻で赤くなる。）
    #[test]
    fn choice_resident_renders_pixel_identical_to_glyph_run() {
        let mut rig = Rig::new();
        let image = (80u32, 40u32);
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);

        // 2 行（"あい" / "うえお"）＝複数住人。全角混在で実インクを確実に載せる。
        let mut items = glyph_items("あい");
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.extend(glyph_items("うえお"));
        let (glyph_canvas, window) = build(&items, &region, mode, 10.0);
        let choice_canvas = as_choice_canvas(&glyph_canvas);

        // 等価変換の健全性（偽 GO 防止）: Choice canvas は実際に Choice 住人を持ち、
        // 住人数・寸は GlyphRun canvas と一致する。
        assert!(
            choice_canvas
                .residents
                .iter()
                .any(|r| matches!(r.content, ResidentContent::Choice(_))),
            "変換後 canvas は Choice 住人を含む"
        );
        assert_eq!(
            choice_canvas.residents.len(),
            glyph_canvas.residents.len(),
            "住人数は等価"
        );

        // GlyphRun canvas を独立 executor/供給面へ初回描画。
        let mut surface_glyph = rig.attach(image, 1.0);
        let mut exec_glyph = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
        exec_glyph
            .render(&glyph_canvas, &window, &font, mode, &contract, &mut surface_glyph)
            .expect("GlyphRun render 失敗");
        let bytes_glyph = surface_glyph.read_back().expect("read_back 失敗");

        // Choice canvas を別の独立 executor/供給面へ初回描画。
        let mut surface_choice = rig.attach(image, 1.0);
        let mut exec_choice = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
        exec_choice
            .render(&choice_canvas, &window, &font, mode, &contract, &mut surface_choice)
            .expect("Choice render 失敗");
        let bytes_choice = surface_choice.read_back().expect("read_back 失敗");

        assert!(
            opaque_count(&bytes_glyph) > 0,
            "GlyphRun 描画で実インクが載る（空比較で偽 GO にしない）"
        );
        assert_eq!(
            bytes_choice, bytes_glyph,
            "非 hover の Choice 住人は等価な GlyphRun 住人とピクセル同一に描画される（R1.4/R9.5）"
        );
    }

    /// premultiplied BGRA 密配列で、列帯 `x0..x1`（全 y）に指定 BGRA と完全一致する画素数を数える。
    fn count_bgra_in_x_band(bytes: &[u8], w: u32, h: u32, x0: u32, x1: u32, target: [u8; 4]) -> usize {
        let mut n = 0usize;
        for y in 0..h {
            for x in x0..x1.min(w) {
                let o = ((y * w + x) * 4) as usize;
                if bytes[o..o + 4] == target {
                    n += 1;
                }
            }
        }
        n
    }

    /// 観測可能な完了状態（ハイライト描画・R4.2/4.3/4.5/4.6）: hover 中の Choice 行を描画すると
    /// hover セグメント矩形内は塗り色（fill）＋切替文字色（白）の画素になり、セグメント外は素描画
    /// （塗りなし）になる。さらに hover 解除フレーム（highlight=None・同一 executor＝キャッシュ
    /// TextLayout 再利用）は塗り画素ゼロ・文字色も既定へ戻る（DrawingEffect リセット正準列＝
    /// キャッシュ層 TextLayout を汚さない・4.5）。
    #[test]
    fn hover_choice_line_paints_segment_and_resets_on_hover_off() {
        let mut rig = Rig::new();
        let image = (40u32, 20u32);
        let mut surface = rig.attach(image, 1.0);
        let (w, h) = surface.size();
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        // 1 行「あい」（全角 2・font 10）＝グリフ位置 0/10・送り 10。GlyphRun canvas をベースに
        // 先頭グリフのみ選択肢セグメント（ordinal 0・inline_range (0,10)＝resident-local）にして
        // hover=Some(0)・fixture 実導出色（fill=(105,25,25)・text=(255,255,255)）を焼く。
        let (base_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
        let fill = (105u8, 25u8, 25u8);
        let text_color = (255u8, 255u8, 255u8);
        let make_choice = |highlight: Option<HighlightPaint>, hovered: Option<usize>| {
            let residents = base_canvas
                .residents
                .iter()
                .map(|r| match &r.content {
                    ResidentContent::GlyphRun(run) => Resident {
                        content: ResidentContent::Choice(ChoiceLineContent {
                            run: run.clone(),
                            segments: vec![ChoiceRowSegment {
                                ordinal: 0,
                                inline_range: (0.0, 10.0), // 先頭グリフ（resident-local）。
                            }],
                            hovered,
                            highlight,
                            // 帯は em ボックス丈（10）より**大きい** 13——実フォント
                            // （Yu Gothic UI 比 1.33）の descent 込み帯と同じ関係を檻に持ち込む。
                            // hover 解除フレームの「塗り画素ゼロ」判定が、ダーティ帯の帯超過分
                            // 拡張（expand_overhang_for_band）まで含めて赤くなる。
                            band_extent: 13.0,
                        }),
                        transform: r.transform,
                        effects: r.effects,
                    },
                    _ => r.clone(),
                })
                .collect();
            ContentCanvas {
                residents,
                size: base_canvas.size,
            }
        };

        // premultiplied BGRA（α=255 ゆえ非乗算）: 塗り＝(b,g,r,a)=(25,25,105,255)・白文字＝(255,255,255,255)。
        let fill_bgra = [25u8, 25, 105, 255];
        let white_bgra = [255u8, 255, 255, 255];

        // ── frame 1: hover 中（highlight=Some）。 ──
        let hover_canvas = make_choice(
            Some(HighlightPaint {
                fill,
                text: text_color,
            }),
            Some(0),
        );
        exec.render(&hover_canvas, &window, &font, mode, &contract, &mut surface)
            .expect("hover render 失敗");
        let hovered_px = surface.read_back().expect("read_back(hover) 失敗");

        // セグメント帯（x 0..10）: 塗り画素あり＋白文字画素あり（矩形塗り＋文字色切替の双方）。
        let seg_fill = count_bgra_in_x_band(&hovered_px, w, h, 0, 10, fill_bgra);
        let seg_white = count_bgra_in_x_band(&hovered_px, w, h, 0, 10, white_bgra);
        assert!(
            seg_fill > 0,
            "hover セグメント矩形内に塗り色（fill）画素が現れる（R4.2）"
        );
        assert!(
            seg_white > 0,
            "hover セグメント範囲の文字が切替色（白）で描かれる（R4.6）"
        );

        // セグメント外（x 10..20＝2 グリフ目）: 塗り画素ゼロ・白文字ゼロ（素描画＝既定色）。
        let out_fill = count_bgra_in_x_band(&hovered_px, w, h, 10, 20, fill_bgra);
        let out_white = count_bgra_in_x_band(&hovered_px, w, h, 10, 20, white_bgra);
        assert_eq!(
            out_fill, 0,
            "hover セグメント外は塗られない（文字幅＝クリック領域幅・行全幅でない・R4.2）"
        );
        assert_eq!(
            out_white, 0,
            "hover セグメント外の文字は切替色にならない（効果範囲限定・R4.6）"
        );

        // ── frame 2: hover 解除（highlight=None・同一 executor＝行 TextLayout はキャッシュ再利用）。 ──
        let plain_canvas = make_choice(None, None);
        exec.render(&plain_canvas, &window, &font, mode, &contract, &mut surface)
            .expect("hover 解除 render 失敗");
        let plain_px = surface.read_back().expect("read_back(hover 解除) 失敗");

        // 塗り画素は全域ゼロ（塗りが残らない）・白文字も全域ゼロ（効果が全範囲 None へリセット
        // され既定黒へ戻る＝キャッシュ層 TextLayout を汚していない・4.5）。
        let all_fill = count_bgra_in_x_band(&plain_px, w, h, 0, w, fill_bgra);
        let all_white = count_bgra_in_x_band(&plain_px, w, h, 0, w, white_bgra);
        assert_eq!(all_fill, 0, "hover 解除フレームは塗り画素ゼロ（素描画）");
        assert_eq!(
            all_white, 0,
            "hover 解除フレームは切替文字色が残らない（DrawingEffect リセット正準列・4.5）"
        );
        // 非退化: 素描画でも content の非透明インクは在る（vacuous な空面一致を排除）。
        assert!(
            opaque_count(&plain_px) > 0,
            "hover 解除でも素のグリフインクは描かれる"
        );
    }

    /// 観測可能な完了状態 2: 同一 content・同一 window の再フレーム → plan=NoChange →
    /// render `Ok(false)`・blit も描画も行レイアウト生成も発生しない（全カウンタ増分 0）。
    #[test]
    fn unchanged_frame_is_nochange_no_blit_no_draw() {
        let mut rig = Rig::new();
        let image = (80u32, 40u32);
        let mut surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        let items = glyph_items("■■");
        let (canvas, window) = build(&items, &region, mode, 10.0);
        exec.render(&canvas, &window, &font, mode, &contract, &mut surface)
            .expect("初回 render 失敗");
        let after_first: DrawStats = exec.stats();

        let changed = exec
            .render(&canvas, &window, &font, mode, &contract, &mut surface)
            .expect("再 render 失敗");
        assert!(
            !changed,
            "可視窓不変・content 不変のフレームは変化なし（present 不要）"
        );

        let after_second: DrawStats = exec.stats();
        assert_eq!(
            after_second.blits, after_first.blits,
            "NoChange は blit を発生させない"
        );
        assert_eq!(
            after_second.draw_text_layout_calls, after_first.draw_text_layout_calls,
            "NoChange は描画を発生させない"
        );
        assert_eq!(
            after_second.line_layout_creations, after_first.line_layout_creations,
            "NoChange は行レイアウト生成を発生させない"
        );
    }

    /// 観測可能な完了状態 3: 可視窓のみ移動（content 不変）→ 保持ピクセルの面内 blit（1 回）と
    /// 露出帯の描画だけが発生し、確定行の再描画は起きない（行レイアウト再生成 0）。read_back は
    /// スクロールで内容が移動する。期待計画は独立な mirror planner（純粋層）で算出する。
    #[test]
    fn visible_window_move_blits_and_redraws_only_exposure_band() {
        let mut rig = Rig::new();
        let image = (60u32, 60u32);
        let mut surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let size = surface.size();
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        // 6 行（pitch 13・canvas-local y=0,13,…,65）。各行を相異なるグリフにし、13px の縦
        // スクロールで供給面の内容が実際に移動する（同一グリフだと行が同型で readback 不変になる）
        // ことを観測可能にする。canvas は validrect-local に全行保持。
        let line_chars = ['あ', 'い', 'う', 'え', 'お', 'か'];
        let mut items = Vec::new();
        for (i, &ch) in line_chars.iter().enumerate() {
            if i > 0 {
                items.push(TextItem::LineBreak { ratio: 1.0 });
            }
            items.push(TextItem::Glyph { ch });
        }
        let (canvas, _auto_window) = build(&items, &region, mode, 10.0);
        let total_glyph_lines = canvas.residents.len();

        // frame A: block_offset 0（スクロール前）。
        let window_a = VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0,
        };
        exec.render(&canvas, &window_a, &font, mode, &contract, &mut surface)
            .expect("frame A render 失敗");
        let before = surface.read_back().expect("read_back(A) 失敗");
        let stats_a: DrawStats = exec.stats();

        // frame B: 同一 canvas・block_offset -13（1 行ぶん上へスクロール・content 不変）。
        let window_b = VisibleWindow {
            first_visible_line: 1,
            block_offset: -13.0,
        };

        // mirror planner（純粋層）で期待計画を独立に算出する。
        let mut mirror = ScrollPlanner::new();
        let plan_a = mirror.plan(&canvas, &window_a, mode, &contract, size);
        mirror.commit(&canvas, &window_a, mode, &contract, &plan_a);
        let plan_b = mirror.plan(&canvas, &window_b, mode, &contract, size);
        let (exp_blit, exp_draw_lines, exp_dirty_len) = match plan_b {
            FramePlan::Update {
                blit,
                ref draw_lines,
                ref dirty,
            } => (blit, draw_lines.len(), dirty.len()),
            other => panic!("frame B は Update を期待したが {other:?}"),
        };
        assert_ne!(exp_blit, (0, 0), "可視窓移動は blit を生む");
        assert_eq!(exp_dirty_len, 1, "content 不変ゆえ dirty は露出帯 1 枚のみ");
        assert!(exp_draw_lines >= 1, "露出帯には流入行が交差する");
        assert!(
            exp_draw_lines < total_glyph_lines,
            "確定行は再描画対象から外れる（全行描画でない）: {exp_draw_lines} < {total_glyph_lines}"
        );

        let changed = exec
            .render(&canvas, &window_b, &font, mode, &contract, &mut surface)
            .expect("frame B render 失敗");
        assert!(changed, "可視窓移動フレームは変化あり");
        let stats_b: DrawStats = exec.stats();

        assert_eq!(
            stats_b.blits,
            stats_a.blits + 1,
            "面内 blit がちょうど 1 回発生する"
        );
        let draw_delta = (stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls) as usize;
        assert_eq!(
            draw_delta,
            exp_dirty_len * exp_draw_lines,
            "描画は露出帯（1 枚）× 交差行に限られる"
        );
        assert!(
            draw_delta <= exp_draw_lines,
            "draw_text_layout 増分 {draw_delta} ≤ 露出帯交差行数 {exp_draw_lines}"
        );
        assert_eq!(
            stats_b.line_layout_creations, stats_a.line_layout_creations,
            "content 不変ゆえ確定行レイアウトは再生成されない（保持面＋blit が保持機構）"
        );

        let after = surface.read_back().expect("read_back(B) 失敗");
        assert_ne!(
            before, after,
            "スクロールで供給面の内容が移動する（先頭行が消え内容が上へ）"
        );
    }

    /// 観測可能な完了状態 4: typewriter 1 グリフ進行 → 描画は現在行（変化行）1 行に限られ、
    /// 確定行は再描画も行レイアウト再生成もされない（blit=0）。
    #[test]
    fn typewriter_progress_draws_only_current_line() {
        let mut rig = Rig::new();
        let image = (120u32, 60u32);
        let mut surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        // 行0 確定「■■」＋行1 リビール中「■」（2 行・60px に収まる＝block_offset 0）。
        let mut items = glyph_items("■■");
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.push(TextItem::Glyph { ch: '■' });
        let (canvas_a, window_a) = build(&items, &region, mode, 10.0);
        exec.render(&canvas_a, &window_a, &font, mode, &contract, &mut surface)
            .expect("frame A render 失敗");
        let stats_a: DrawStats = exec.stats();

        // 行1 が「■」→「■■」へ 1 グリフ進行（行0 は確定・不変）。
        items.push(TextItem::Glyph { ch: '■' });
        let (canvas_b, window_b) = build(&items, &region, mode, 10.0);
        let changed = exec
            .render(&canvas_b, &window_b, &font, mode, &contract, &mut surface)
            .expect("frame B render 失敗");
        assert!(changed, "typewriter 進行フレームは変化あり");
        let stats_b: DrawStats = exec.stats();

        let draw_delta = stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls;
        assert_eq!(
            draw_delta, 1,
            "描画は現在行（変化行）1 行のみ・確定行 0 は再描画されない"
        );
        let create_delta = stats_b.line_layout_creations - stats_a.line_layout_creations;
        assert_eq!(
            create_delta, 1,
            "行レイアウト再生成は現在行のみ（確定行 0 はキャッシュ）"
        );
        assert_eq!(
            stats_b.blits, stats_a.blits,
            "typewriter 進行は blit を発生させない（blit=0）"
        );
    }

    /// 複数行（各行相異なるグリフ）の item 列を組む（縮退檻で「全住人再描画」を数える台）。
    fn multiline_items(chars: &[char]) -> Vec<TextItem> {
        let mut items = Vec::new();
        for (i, &ch) in chars.iter().enumerate() {
            if i > 0 {
                items.push(TextItem::LineBreak { ratio: 1.0 });
            }
            items.push(TextItem::Glyph { ch });
        }
        items
    }

    /// 観測可能な完了状態（R4.3）: content 描画済み → `request_clear()` → 次フレームが
    /// FullClear（`full_clears` +1・read_back 全域透明）→ さらに次フレームで content が
    /// 全域ダーティ再描画で復帰する（透明フラッシュは 1 フレームのみ）。
    #[test]
    fn request_clear_triggers_full_clear_then_content_returns() {
        let mut rig = Rig::new();
        let image = (80u32, 40u32);
        let mut surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        let items = glyph_items("■■");
        let (canvas, window) = build(&items, &region, mode, 10.0);

        // frame 1: content 描画（初回＝全域ダーティ）。
        exec.render(&canvas, &window, &font, mode, &contract, &mut surface)
            .expect("content render 失敗");
        let painted = surface.read_back().expect("read_back(content) 失敗");
        assert!(
            opaque_count(&painted) > 0,
            "content の非透明ピクセルが現れる"
        );
        let full_before = exec.stats().full_clears;

        // Clear cue 適用点（planner 初期化＋行キャッシュ破棄はこの口だけ）。
        exec.request_clear();

        // frame 2: FullClear（描画 0 件・back 全域透明 Clear → flip）。
        let changed = exec
            .render(&canvas, &window, &font, mode, &contract, &mut surface)
            .expect("FullClear render 失敗");
        assert!(changed, "FullClear は present 要（変化あり）");
        assert_eq!(
            exec.stats().full_clears,
            full_before + 1,
            "FullClear は full_clears をちょうど 1 増やす"
        );
        let cleared = surface.read_back().expect("read_back(cleared) 失敗");
        assert_eq!(opaque_count(&cleared), 0, "Clear 後は全域透明（全 α=0）");

        // frame 3: 同一 content の再フレーム → prev_lines 空ゆえ全域ダーティで content 復帰
        //（透明フラッシュは frame 2 の 1 回のみ・FullClear は増えない）。
        let changed3 = exec
            .render(&canvas, &window, &font, mode, &contract, &mut surface)
            .expect("Clear 後 content render 失敗");
        assert!(changed3, "Clear 後の再描画は全域ダーティで変化あり");
        assert_eq!(
            exec.stats().full_clears,
            full_before + 1,
            "content 復帰は FullClear を増やさない（全域ダーティ Update）"
        );
        let restored = surface.read_back().expect("read_back(restored) 失敗");
        assert!(
            opaque_count(&restored) > 0,
            "content が全域ダーティで復帰する"
        );
    }

    /// 観測可能な完了状態（縮退の主檻・R3.5/Error Handling）: フォント（高さ）変更を注入すると
    /// format と行キャッシュが組み直され、当該フレームが全域ダーティへ縮退して全住人が再描画される
    ///（`draw_text_layout_calls` 増分＝全 GlyphRun 住人数・`line_layout_creations` 増分＝全住人数
    /// ＝キャッシュ破棄で全再生成・`full_clears` 不変＝透明フラッシュを起こす FullClear と区別・
    /// read_back は font B の content で非透明）。
    #[test]
    fn font_change_degrades_to_full_domain_redraw() {
        let mut rig = Rig::new();
        let image = (120u32, 120u32);
        let mut surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let contract = ScaleContract::new(1.0, None);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        // 3 行（相異なるグリフ・全行が 120px に収まる＝block_offset 0）。
        let chars = ['あ', 'い', 'う'];
        let items = multiline_items(&chars);

        // font A（高さ 10）で描画（初回＝全域ダーティ）。
        let font_a = ResolvedFont::resolve(&geo_model(Some(10)));
        let region_a = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let (canvas_a, window_a) = build(&items, &region_a, mode, 10.0);
        let total = canvas_a.residents.len();
        assert_eq!(total, 3, "3 行の GlyphRun 住人（全住人再描画を数える台）");
        exec.render(&canvas_a, &window_a, &font_a, mode, &contract, &mut surface)
            .expect("font A render 失敗");
        let stats_a = exec.stats();
        let full_before = stats_a.full_clears;

        // font B（高さ 14）へ変更（FormatKey の高さビットが変わり format/行キャッシュ組み直し）。
        let font_b = ResolvedFont::resolve(&geo_model(Some(14)));
        let region_b = TextRegion::resolve(&geo_model(Some(14)), image, mode);
        let (canvas_b, window_b) = build(&items, &region_b, mode, 14.0);
        let total_b = canvas_b.residents.len();
        assert_eq!(total_b, 3, "font B でも 3 行（全住人が縮退で再描画される）");

        let changed = exec
            .render(&canvas_b, &window_b, &font_b, mode, &contract, &mut surface)
            .expect("font B render 失敗");
        assert!(changed, "font 変更フレームは変化あり（全域ダーティ縮退）");
        let stats_b = exec.stats();

        let draw_delta = stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls;
        assert_eq!(
            draw_delta, total_b as u64,
            "縮退フレームは全 GlyphRun 住人を再描画する（全域ダーティ 1 枚 × 全住人）"
        );
        let create_delta = stats_b.line_layout_creations - stats_a.line_layout_creations;
        assert_eq!(
            create_delta, total_b as u64,
            "行キャッシュ破棄ゆえ全住人ぶん行レイアウトを再生成する"
        );
        assert_eq!(
            stats_b.full_clears, full_before,
            "縮退は全域ダーティ Update（透明フラッシュを起こす FullClear ではない）"
        );
        let bytes = surface.read_back().expect("read_back(font B) 失敗");
        assert!(
            opaque_count(&bytes) > 0,
            "font B の content が全域ダーティで正しく描かれる（透明のままでない）"
        );
    }

    /// 想定外不整合の検査関数（render の縮退経路が呼ぶ述語）が矛盾入力で理由を返し、整合入力で
    /// `None` を返す（範囲外 draw_lines・面寸超過 dirty の両トリガ・R Error Handling）。
    /// render からの結線は `font_change_degrades_to_full_domain_redraw`（縮退の実発火）が担保する。
    #[test]
    fn plan_inconsistency_detects_out_of_range_and_oversize() {
        use super::plan_inconsistency;
        use crate::viewbox::PhysicalRect;

        let size = (100u32, 50u32);
        let full = PhysicalRect {
            x: 0,
            y: 0,
            w: 100,
            h: 50,
        };

        // 整合: 範囲内 index・面寸ちょうどの dirty → None。
        assert!(
            plan_inconsistency(&[full], &[0, 1], 2, size).is_none(),
            "範囲内・面寸内は整合（縮退しない）"
        );

        // 範囲外 draw_lines（住人数 2 に対し index 5）→ Some。
        assert!(
            plan_inconsistency(&[], &[5], 2, size).is_some(),
            "draw_lines の範囲外 index を検知する"
        );

        // dirty が面幅を超える（x+w=101 > 100）→ Some。
        assert!(
            plan_inconsistency(
                &[PhysicalRect {
                    x: 0,
                    y: 0,
                    w: 101,
                    h: 50
                }],
                &[],
                2,
                size
            )
            .is_some(),
            "面幅を超える dirty を検知する"
        );

        // dirty が面高を超える（y+h=60 > 50）→ Some。
        assert!(
            plan_inconsistency(
                &[PhysicalRect {
                    x: 0,
                    y: 40,
                    w: 100,
                    h: 20
                }],
                &[],
                2,
                size
            )
            .is_some(),
            "面高を超える dirty を検知する"
        );
    }

    /// G5: 描画中デバイス失敗の再試行安全を実描画で檻化する。EndDraw 後・flip/commit 前に失敗を
    /// 注入（test-only fault-injection）すると、その Update フレームは `Err` を返し、**front は不変**
    /// （flip されない＝前フレームの確定面を保持）・**planner は未 commit**（次フレームで同一計画を
    /// 再試行）になる。失敗解除後の再提示で正しい content が反映されることで「未 commit＝再試行安全」を
    /// 証明する（もし失敗フレームで誤って commit していたら、再試行が NoChange になり content が
    /// 反映されず本檻が落ちる）。純粋 ScrollPlanner の再試行檻（task 3.3）の COM 側 runtime 版。
    #[test]
    fn device_failure_mid_render_is_retry_safe_front_unchanged_no_commit() {
        let mut rig = Rig::new();
        let image = (80u32, 40u32);
        let mut surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let font = ResolvedFont::resolve(&geo_model(Some(10)));
        let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
        let contract = ScaleContract::new(1.0, None);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

        // フレーム 1: content "■" を描き確定（front＝"■"）。
        let (canvas1, window1) = build(&glyph_items("■"), &region, mode, 10.0);
        assert!(
            exec.render(&canvas1, &window1, &font, mode, &contract, &mut surface)
                .expect("フレーム1 render 失敗"),
            "初回 content フレームは変化あり"
        );
        let r1 = surface.read_back().expect("read_back(frame1)");
        let ink1 = opaque_count(&r1);
        assert!(ink1 > 0, "フレーム1で content が描かれる");

        // フレーム 2: content を "■■" へ伸長（Update）。だが EndDraw 後に失敗を注入する。
        let (canvas2, window2) = build(&glyph_items("■■"), &region, mode, 10.0);
        exec.inject_render_failure();
        let err = exec
            .render(&canvas2, &window2, &font, mode, &contract, &mut surface)
            .err()
            .expect("失敗注入フレームは Err を返す（panic しない・log-first）");
        assert!(
            matches!(err, crate::TextLayerError::Device { .. }),
            "注入失敗は Device エラー: {err:?}"
        );
        // front 不変: flip していないため read_back は前フレーム "■" のまま（伸長が反映されない）。
        let r2 = surface.read_back().expect("read_back(frame2 失敗後)");
        assert_eq!(
            r2, r1,
            "失敗フレームは front を変えない（flip せず＝前フレームの確定面を保持）"
        );

        // フレーム 3: 失敗は解除済み（自動消費）。同一 content "■■" を再提示すると、planner が
        // 未 commit（prev_lines は依然 "■"）ゆえ Update を再計画し、今度は成功して front が更新される。
        let changed = exec
            .render(&canvas2, &window2, &font, mode, &contract, &mut surface)
            .expect("フレーム3（再試行）render 失敗");
        assert!(
            changed,
            "再試行フレームは変化あり（未 commit ゆえ再計画される）"
        );
        let r3 = surface.read_back().expect("read_back(frame3 再試行)");
        assert_ne!(
            r3, r1,
            "再試行で content 伸長が反映される（front が更新＝失敗フレームで commit していない証拠）"
        );
        assert!(
            opaque_count(&r3) > ink1,
            "再試行後は伸長ぶんインクが増える: {} > {ink1}",
            opaque_count(&r3)
        );
    }

    /// D1 診断（実機で行間の文字欠けを観測）: example の共有 fixture（font 28px・pitch 35px・
    /// validrect 320×122・**実 DWriteMetrics**・行1「おっはよー！」行2「めっちゃええ朝やん！」）を
    /// oracle（全域再描画）と viewbox（ダーティスクロール）の両方で typewriter 進行させ read_back を
    /// byte 比較する。diverge すれば **viewbox 固有の描画欠陥**（行間で確定行 ink をクリアして
    /// 再描画しない等）、byte 一致すれば **layout 由来**（両方式に同じ＝emo-text-layer 責務）と切り分ける。
    /// live-diff（FixedMetrics＋あいうえお）が見逃した「実 fixture の実文字」条件を実測する。
    #[test]
    fn diag_line_boundary_dropout_vs_oracle() {
        let mut rig = Rig::new();
        let image = (320u32, 122u32);
        let mut oracle_surface = rig.attach(image, 1.0);
        let mut viewbox_surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let model = geo_model(Some(28));
        let font = ResolvedFont::resolve(&model);
        let region = TextRegion::resolve(&model, image, mode);
        let contract = ScaleContract::new(1.0, None);
        let config = TextLayerConfig::default();
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let metrics = DWriteMetrics::new(&factory, &font, mode, &config).expect("DWriteMetrics");
        let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
        let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");
        let actor = ActorKey::from("0");
        let mut state = TextLayerState::default();

        // reveal は配送 duration 由来（interval = duration / N）。全 cue at=0.0 ゆえ、
        // Text へ N×REVEAL_INTERVAL を焼き込むと reveal が連続 typewriter 進行する
        // （旧 char_wait=0.05 と機能等価・注入時刻はこの間隔で刻む）。
        const REVEAL_INTERVAL: f64 = 0.05;
        let mk = |cmd: CueCommand| TalkCue {
            at: 0.0,
            actor: actor.clone(),
            duration: match &cmd {
                CueCommand::Text(t) => t.chars().count() as f64 * REVEAL_INTERVAL,
                _ => 0.0,
            },
            command: cmd,
        };
        // example の共有 fixture フル シナリオ（行1/2/3＋あふれ誘発の短行 9 本でスクロール発火）。
        state.apply_cue(&mk(CueCommand::Text("おっはよー！".into())));
        state.apply_cue(&mk(CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&mk(CueCommand::Text("めっちゃええ朝やん！".into())));
        state.apply_cue(&mk(CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&mk(CueCommand::Text("今日もいくでー！".into())));
        for _ in 0..9 {
            state.apply_cue(&mk(CueCommand::NewLine { ratio: 1.0 }));
            state.apply_cue(&mk(CueCommand::Text("ほな".into())));
        }

        let total_glyphs = 6 + 10 + 8 + 9 * 2;
        for step in 0..=(total_glyphs + 2) {
            let t = step as f64 * REVEAL_INTERVAL + 0.001;
            let visible = state.visible_glyphs(&actor, t);
            let items: Vec<TextItem> = state
                .actor_state(&actor)
                .map(|s| s.items().to_vec())
                .unwrap_or_default();
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &region,
                mode,
                font.height,
                &metrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, &region, mode);
            let canvas = ContentCanvas::from_layout(&lines, &region, mode);
            oracle
                .render(
                    &canvas,
                    &window,
                    &font,
                    mode,
                    &contract,
                    &mut oracle_surface,
                )
                .expect("oracle render");
            viewbox
                .render(
                    &canvas,
                    &window,
                    &font,
                    mode,
                    &contract,
                    &mut viewbox_surface,
                )
                .expect("viewbox render");
            let ob = oracle_surface.read_back().expect("oracle read_back");
            let vb = viewbox_surface.read_back().expect("viewbox read_back");
            if ob != vb {
                let (w, h) = oracle_surface.size();
                let mut diff_rows: Vec<u32> = Vec::new();
                for y in 0..h {
                    let row = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                    if ob[row.clone()] != vb[row] {
                        diff_rows.push(y);
                    }
                }
                panic!(
                    "viewbox が oracle と diverge（viewbox 固有欠陥）: step={step} t={t:.3} visible={visible} \
                     相違行 y={diff_rows:?}（行1セル 0..28・行間 28..35・行2セル 35..63）"
                );
            }
        }
        // 前方進行では全フレーム byte 一致。次に**後方時刻ジャンプ**（example の C8 検分が
        // present_at(earlier t_mid) で行うパターン）を検証する: 大 t（スクロール済み）から
        // 小 t（未スクロール）へ戻したとき、viewbox が un-scroll を正しく扱い oracle と一致するか。
        // viewbox は前方スクロール前提ゆえ、後方でスクロールアウト行の再露出を取りこぼすと diverge。
        let big_t = (total_glyphs as f64 + 5.0) * REVEAL_INTERVAL;
        for &back_t in &[big_t, 0.05, 0.3, 0.1, 0.5] {
            let visible = state.visible_glyphs(&actor, back_t);
            let items: Vec<TextItem> = state
                .actor_state(&actor)
                .map(|s| s.items().to_vec())
                .unwrap_or_default();
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &region,
                mode,
                font.height,
                &metrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, &region, mode);
            let canvas = ContentCanvas::from_layout(&lines, &region, mode);
            oracle
                .render(
                    &canvas,
                    &window,
                    &font,
                    mode,
                    &contract,
                    &mut oracle_surface,
                )
                .expect("oracle render(後方)");
            viewbox
                .render(
                    &canvas,
                    &window,
                    &font,
                    mode,
                    &contract,
                    &mut viewbox_surface,
                )
                .expect("viewbox render(後方)");
            let ob = oracle_surface.read_back().expect("oracle read_back(後方)");
            let vb = viewbox_surface
                .read_back()
                .expect("viewbox read_back(後方)");
            if ob != vb {
                let (w, h) = oracle_surface.size();
                let mut diff_rows: Vec<u32> = Vec::new();
                for y in 0..h {
                    let row = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                    if ob[row.clone()] != vb[row] {
                        diff_rows.push(y);
                    }
                }
                panic!(
                    "後方時刻ジャンプで viewbox が oracle と diverge（un-scroll 欠陥・C8 probing が誘発）: \
                     back_t={back_t:.3} visible={visible} 相違行数={} y={:?}...",
                    diff_rows.len(),
                    &diff_rows.iter().take(12).collect::<Vec<_>>()
                );
            }
        }
    }

    /// DD-9: 改行を一切含まない行内縮小（visible=6「おっはよー！」→ visible=2「おっ」）で
    /// oracle（全域再描画）と viewbox（ダーティスクロール）の read_back が byte 一致する。
    /// `diag_line_boundary_dropout_vs_oracle` の後方ジャンプ検分が捕えた退避インク未クリア欠陥が
    /// **改行非依存**（本 spec の遅延化と直交・後方時刻ジャンプ/un-reveal で単一行が縮む任意
    /// アクセスで再現）であることを恒久固定する。guard 拡張（viewbox.rs DD-9）で緑になる。
    #[test]
    fn within_line_shrink_no_newline_stays_byte_equal_to_oracle() {
        let mut rig = Rig::new();
        let image = (320u32, 122u32);
        let mut oracle_surface = rig.attach(image, 1.0);
        let mut viewbox_surface = rig.attach(image, 1.0);
        let mode = WritingMode::HorizontalTb;
        let model = geo_model(Some(28));
        let font = ResolvedFont::resolve(&model);
        let region = TextRegion::resolve(&model, image, mode);
        let contract = ScaleContract::new(1.0, None);
        let config = TextLayerConfig::default();
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let metrics = DWriteMetrics::new(&factory, &font, mode, &config).expect("DWriteMetrics");
        let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
        let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");
        // 単一行・改行なし。visible を 6→2 と縮める 2 フレーム（後方時刻ジャンプの un-reveal 相当）。
        let items: Vec<TextItem> = "おっはよー！"
            .chars()
            .map(|ch| TextItem::Glyph { ch })
            .collect();
        for &visible in &[6usize, 2usize] {
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &region,
                mode,
                font.height,
                &metrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, &region, mode);
            let canvas = ContentCanvas::from_layout(&lines, &region, mode);
            oracle
                .render(
                    &canvas,
                    &window,
                    &font,
                    mode,
                    &contract,
                    &mut oracle_surface,
                )
                .expect("oracle render");
            viewbox
                .render(
                    &canvas,
                    &window,
                    &font,
                    mode,
                    &contract,
                    &mut viewbox_surface,
                )
                .expect("viewbox render");
            let ob = oracle_surface.read_back().expect("oracle read_back");
            let vb = viewbox_surface.read_back().expect("viewbox read_back");
            assert_eq!(
                ob, vb,
                "行内縮小 visible={visible} で viewbox が oracle と byte 一致（DD-9・退避インク一掃）"
            );
        }
    }

    // ════ live-diff pixel 等価主檻（task 10・R4.5/R6.1/R6.2/R6.3/R6.5/R8.1・design Testing
    //      Strategy「Integration Tests #1」） ════
    //
    // 同一プロセス・同一ターゲット型（headless World＋Compositor＋GraphicsCore）で、同一 cue 列・
    // 同一注入時刻列を比較専用オラクル（DrawExecutor 全域再描画・#[cfg(test)]）と新実行部
    // （ViewboxExecutor ダーティ矩形スクロール）の双方へ流し、read_back を k=1.0 で byte 比較する。
    // オラクルは front へ全域再描画し、viewbox は back へダーティ描画→flip する——どちらも read_back
    // は front を読むため直接比較できる（surface.rs の front 一本化契約）。Clear は actor.rs::apply_cue
    // と同じ写像で両方式の Clear 適用点（oracle=行キャッシュ破棄／viewbox=planner 初期化＋FullClear
    // 予約）を経由させる。

    /// live-diff 検証リグ: 2 つの独立 TextSurface（同一物理寸・k=1.0・同一 core/compositor）へ
    /// オラクルと viewbox を装着し、1 本の TextLayerState を両方式へ同一入力で流す。
    struct LiveDiffRig {
        /// オラクル（全域再描画）の供給面。
        oracle_surface: TextSurface,
        /// viewbox（ダーティスクロール）の供給面。
        viewbox_surface: TextSurface,
        /// 比較専用オラクル（全域再描画・front へ焼く）。
        oracle: DrawExecutor,
        /// 新実行部（保持ピクセル面内 blit ＋ ダーティ描画・back へ焼き flip）。
        viewbox: ViewboxExecutor,
        /// 解決済みテキスト領域（両方式共通・image px）。
        region: TextRegion,
        /// 解決済みフォント（既定 ＭＳ ゴシック 10px）。
        font: ResolvedFont,
        /// DPI/スケール契約（k=1.0＝byte 一致の受け入れ基準）。
        contract: ScaleContract,
        /// 書字方向（横書き／vertical_rl）。
        mode: WritingMode,
        /// フォント高さ（純粋レイアウトへ渡す・FixedMetrics と対）。
        font_height: f32,
        /// 対象 actor（単一 actor で十分——byte 等価は actor 非依存）。
        actor: ActorKey,
        /// cue 駆動の純粋状態機械（両方式へ同一入力を供給する単一の正本）。
        state: TextLayerState,
        /// World／Compositor／Core／DispatcherQueue の寿命を束ねる（供給面より後に drop）。
        #[allow(dead_code)]
        rig: Rig,
    }

    impl LiveDiffRig {
        /// image px 原寸と mode から 2 面（オラクル／viewbox）を装着し、両実行部を生成する
        /// （k=1.0＝byte 一致の受け入れ基準・既定）。
        fn new(mode: WritingMode, image: (u32, u32)) -> LiveDiffRig {
            Self::new_scaled(mode, image, 1.0)
        }

        /// 合成スケール k を明示して装着する（k≠1.0＝byte 完全一致でなく ≤0.5px 許容の受け入れ
        /// 基準・G2）。フォントは既定 ＭＳ ゴシック 10px。
        fn new_scaled(mode: WritingMode, image: (u32, u32), k: f32) -> LiveDiffRig {
            Self::new_full(mode, image, k, None, 10)
        }

        /// フォント名・高さ・k を明示して装着する（G4——非 default フォント/大サイズで AA こぼれ
        /// ガードの実効性を byte 等価検証）。両面とも物理寸＝ceil(image × k)・region は image px。
        /// レイアウトは FixedMetrics（font 非依存の決定論位置）ゆえ font 名は AA ラスタライズ
        /// （描画）にのみ効く——両方式が同一 font ゆえガードが十分なら byte 等価が保たれる。
        fn new_full(
            mode: WritingMode,
            image: (u32, u32),
            k: f32,
            font_name: Option<&str>,
            font_height: u32,
        ) -> LiveDiffRig {
            let mut rig = Rig::new();
            let oracle_surface = rig.attach(image, k);
            let viewbox_surface = rig.attach(image, k);
            let model = live_diff_model_font(font_name, Some(font_height));
            let font = ResolvedFont::resolve(&model);
            let region = TextRegion::resolve(&model, image, mode);
            let contract = ScaleContract::new(k, None);
            let oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor::new 失敗");
            let viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
            LiveDiffRig {
                oracle_surface,
                viewbox_surface,
                oracle,
                viewbox,
                region,
                font,
                contract,
                mode,
                font_height: font_height as f32,
                actor: ActorKey::from("0"),
                state: TextLayerState::default(),
                rig,
            }
        }

        /// cue を純粋状態へ適用し、Clear は両方式の Clear 適用点も呼ぶ（actor.rs::apply_cue と同一写像）。
        fn apply(&mut self, cue: &TalkCue) {
            if matches!(cue.command, CueCommand::Clear) {
                self.oracle.clear_cache();
                self.viewbox.request_clear();
            }
            self.state.apply_cue(cue);
        }

        /// Text cue（追記）を適用する。
        fn apply_text(&mut self, at: f64, s: &str) {
            let cue = TalkCue {
                at,
                actor: self.actor.clone(),
                command: CueCommand::Text(s.to_owned()),
                duration: 0.0,
            };
            self.apply(&cue);
        }

        /// NewLine cue（改行マーカー・ratio 1.0）を適用する。
        fn apply_newline(&mut self, at: f64) {
            let cue = TalkCue {
                at,
                actor: self.actor.clone(),
                command: CueCommand::NewLine { ratio: 1.0 },
                duration: 0.0,
            };
            self.apply(&cue);
        }

        /// Clear cue（全消去）を適用する。
        fn apply_clear(&mut self, at: f64) {
            let cue = TalkCue {
                at,
                actor: self.actor.clone(),
                command: CueCommand::Clear,
                duration: 0.0,
            };
            self.apply(&cue);
        }

        /// 注入時刻 `t` で state から可視 prefix→純粋レイアウト→canvas/window を導き、オラクルと
        /// viewbox を**同一入力**で描いて read_back を byte 比較する。`expect_opaque` で content
        /// フレーム（非透明 > 0）／Clear フレーム（全域透明）を区別し、空面同士の vacuous な一致を排除する。
        fn checkpoint(&mut self, label: &str, t: f64, expect_opaque: bool) -> VisibleWindow {
            // state → 可視 prefix → 純粋レイアウト（FixedMetrics で決定論・両方式は同一 canvas を受ける）。
            let items: Vec<TextItem> = self
                .state
                .actor_state(&self.actor)
                .map(|s| s.items().to_vec())
                .unwrap_or_default();
            let visible = self.state.visible_glyphs(&self.actor, t);
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &self.region,
                self.mode,
                self.font_height,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, &self.region, self.mode);
            let canvas = ContentCanvas::from_layout(&lines, &self.region, self.mode);

            // オラクル（全域再描画・front）と viewbox（ダーティ描画・back→flip）を同一入力で描く。
            self.oracle
                .render(
                    &canvas,
                    &window,
                    &self.font,
                    self.mode,
                    &self.contract,
                    &mut self.oracle_surface,
                )
                .unwrap_or_else(|e| panic!("{label}: オラクル render 失敗: {e:?}"));
            self.viewbox
                .render(
                    &canvas,
                    &window,
                    &self.font,
                    self.mode,
                    &self.contract,
                    &mut self.viewbox_surface,
                )
                .unwrap_or_else(|e| panic!("{label}: viewbox render 失敗: {e:?}"));

            let ob = self
                .oracle_surface
                .read_back()
                .unwrap_or_else(|e| panic!("{label}: オラクル read_back 失敗: {e:?}"));
            let vb = self
                .viewbox_surface
                .read_back()
                .unwrap_or_else(|e| panic!("{label}: viewbox read_back 失敗: {e:?}"));

            // 非退化担保: content フレームはオラクル面に非透明ピクセルがある（空面比較=vacuous を排除）。
            if expect_opaque {
                assert!(
                    opaque_count(&ob) > 0,
                    "{label}: content フレームはオラクル面に非透明ピクセルを持つ（vacuous な空面一致を排除）"
                );
            } else {
                assert_eq!(
                    opaque_count(&ob),
                    0,
                    "{label}: Clear 直後はオラクル面が全域透明（全 α=0）"
                );
            }

            // 主檻: k=1.0 で byte 完全一致（受け入れ基準——甘くしない）。
            assert_eq!(
                ob, vb,
                "{label}: k=1.0 で オラクル（全域再描画）と viewbox（ダーティスクロール）の read_back が byte 完全一致する"
            );
            window
        }

        /// k≠1.0 用の許容差チェックポイント（G2・R6.4）: byte 完全一致でなく、オラクル（真位置で
        /// 全域再描画）と viewbox（確定 content は whole-pixel blit で量子化位置・ダーティは真位置で
        /// 再描画）の read_back の**ブロック軸インク範囲**の差が `tol` 物理 px 以内であることを檻化する
        /// （小数アキュムレータで |committed − pos| ≤ 0.5 ⇒ 確定 content 位置差 ≤ ceil(0.5×k)）。
        /// k≠1.0 の実 GPU 描画経路（小数アキュムレータ→whole-pixel blit→実描画→readback）を実際に
        /// 走らせて検証する（従来 k=1.0 でしか走っていなかった経路）。返り値は VisibleWindow。
        fn checkpoint_block_tol(&mut self, label: &str, t: f64, tol: u32) -> VisibleWindow {
            let items: Vec<TextItem> = self
                .state
                .actor_state(&self.actor)
                .map(|s| s.items().to_vec())
                .unwrap_or_default();
            let visible = self.state.visible_glyphs(&self.actor, t);
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &self.region,
                self.mode,
                self.font_height,
                &FixedMetrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, &self.region, self.mode);
            let canvas = ContentCanvas::from_layout(&lines, &self.region, self.mode);

            self.oracle
                .render(
                    &canvas,
                    &window,
                    &self.font,
                    self.mode,
                    &self.contract,
                    &mut self.oracle_surface,
                )
                .unwrap_or_else(|e| panic!("{label}: オラクル render 失敗: {e:?}"));
            self.viewbox
                .render(
                    &canvas,
                    &window,
                    &self.font,
                    self.mode,
                    &self.contract,
                    &mut self.viewbox_surface,
                )
                .unwrap_or_else(|e| panic!("{label}: viewbox render 失敗: {e:?}"));

            let ob = self
                .oracle_surface
                .read_back()
                .unwrap_or_else(|e| panic!("{label}: オラクル read_back 失敗: {e:?}"));
            let vb = self
                .viewbox_surface
                .read_back()
                .unwrap_or_else(|e| panic!("{label}: viewbox read_back 失敗: {e:?}"));

            let (w, h) = self.oracle_surface.size();
            // 非退化担保（空面同士の vacuous な一致を排除）。
            assert!(
                opaque_count(&ob) > 0,
                "{label}: オラクル面に非透明ピクセルがある"
            );
            assert!(
                opaque_count(&vb) > 0,
                "{label}: viewbox 面に非透明ピクセルがある"
            );

            let os = block_axis_ink_span(&ob, w, h, self.mode).expect("オラクル面のインク範囲");
            let vs = block_axis_ink_span(&vb, w, h, self.mode).expect("viewbox 面のインク範囲");
            let d0 = os.0.abs_diff(vs.0);
            let d1 = os.1.abs_diff(vs.1);
            assert!(
                d0 <= tol && d1 <= tol,
                "{label}: k≠1.0 でブロック軸インク範囲の差が ≤{tol}px（確定 content の量子化位置差 ≤0.5px）\
                 ——oracle {os:?} vs viewbox {vs:?}（差 near={d0} far={d1}）"
            );
            window
        }
    }

    /// live-diff シナリオ（mode パラメタライズ）: あふれ前→スクロール発火直後→連続スクロール→
    /// Clear 直後→Clear 後再追記 の 5 チェックポイントで、オラクルと viewbox の read_back が
    /// 常に byte 完全一致することを檻化する（NewLine 区切りの単一グリフ行であふれを決定論制御）。
    fn run_live_diff_scenario(mode: WritingMode, image: (u32, u32)) {
        let mut ld = LiveDiffRig::new(mode, image);
        run_live_diff_scenario_on(&mut ld);
    }

    /// 構築済みリグに対し 5 チェックポイント（あふれ前→スクロール発火→連続→Clear→再追記）を
    /// byte 完全一致で檻化する（font/サイズを変えたリグで再利用——G4 の AA ガード実効性検証）。
    ///
    /// 【newline-defer】本シナリオは全 cue を `at=0.0` で発行し、各 checkpoint 時点までに追記済み
    /// content が完全リビールされる（＝末尾/部分の保留改行が残らない）。ゆえ各改行は次のグリフを
    /// 伴って実体化し、あふれ発火のタイミングは遅延化の影響を受けない（幽霊空行由来の発火が無く
    /// checkpoint 前提は不変）。発火時刻の後退は部分リビール（at 分散）を伴う診断ダンプ側の論点。
    fn run_live_diff_scenario_on(ld: &mut LiveDiffRig) {
        // ① あふれ前（3 行・可視窓は不動）。
        ld.apply_text(0.0, "あ");
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "い");
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "う");
        let w = ld.checkpoint("あふれ前", 10.0, true);
        assert_eq!(
            (w.first_visible_line, w.block_offset),
            (0, 0.0),
            "あふれ前は可視窓が動かない（先頭可視行 0・オフセット 0）: {w:?}"
        );

        // ② スクロール発火直後（4 行目であふれ・可視窓が初めて移動）。
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "え");
        let w = ld.checkpoint("スクロール発火直後", 20.0, true);
        assert!(
            w.first_visible_line >= 1,
            "あふれ発火で可視窓が移動する（先頭可視行 ≥ 1）: {w:?}"
        );
        assert_ne!(
            w.block_offset, 0.0,
            "発火後はブロックオフセットが非零: {w:?}"
        );

        // ③ 連続スクロール（さらに 2 行追記・複数回スクロール）。
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "お");
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "か");
        let w = ld.checkpoint("連続スクロール", 30.0, true);
        assert!(
            w.first_visible_line >= 2,
            "連続スクロールで可視窓が複数行進む（先頭可視行 ≥ 2）: {w:?}"
        );

        // ④ Clear 直後（両方式の Clear 適用点を経由・全域透明）。
        ld.apply_clear(0.0);
        let w = ld.checkpoint("Clear 直後", 40.0, false);
        assert_eq!(
            w.first_visible_line, 0,
            "Clear 後は既定窓（先頭可視行 0）: {w:?}"
        );

        // ⑤ Clear 後再追記（新規 content が全域ダーティで復帰）。
        ld.apply_text(0.0, "ら");
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "り");
        let w = ld.checkpoint("Clear 後再追記", 50.0, true);
        assert_eq!(
            w.first_visible_line, 0,
            "短い再追記はあふれない（先頭可視行 0）: {w:?}"
        );
    }

    /// 観測可能な完了状態（前半・横書き）: 横書きの全シナリオでオラクルと viewbox の read_back が
    /// k=1.0 で byte 完全一致する（R6.1/R6.2/R6.5/R4.5/R8.1）。
    #[test]
    fn live_diff_horizontal_matches_oracle_byte_for_byte() {
        run_live_diff_scenario(WritingMode::HorizontalTb, (80, 40));
    }

    /// 観測可能な完了状態（前半・縦書き vertical_rl）: 縦書きの全シナリオでも byte 完全一致する
    /// （R6.3——横/縦 両 mode パラメタライズ）。
    #[test]
    fn live_diff_vertical_rl_matches_oracle_byte_for_byte() {
        run_live_diff_scenario(WritingMode::VerticalRl, (40, 80));
    }

    /// 観測可能な完了状態（前半・縦書き vertical_lr）: 縦書き左送り（列が左→右へ流れ、あふれ時は
    /// content が**左へ** blit・露出帯＝**右端**）でも全シナリオで byte 完全一致する（R5.2/R6.3——
    /// スクロール軸は横（x）・vertical_rl と同一の x 軸だが符号が逆＝blit 左方向。純粋層の軸写像
    /// unit 檻（task 3.2）だけでなく、実 GPU 描画 readback で oracle↔viewbox の byte 等価を証明する）。
    #[test]
    fn live_diff_vertical_lr_matches_oracle_byte_for_byte() {
        run_live_diff_scenario(WritingMode::VerticalLr, (40, 80));
    }

    /// k≠1.0（非96DPI）のスクロール実描画を許容差で檻化する（G2・R6.4）。あふれ→スクロールの
    /// 各段で、オラクル（真位置・全域再描画）と viewbox（確定 content は whole-pixel blit で量子化・
    /// ダーティは真位置）の read_back のブロック軸インク範囲が `tol` 物理 px 以内に収まることを実 GPU
    /// 描画で確認する（小数アキュムレータ→whole-pixel blit→実描画→readback の経路を実走）。
    /// byte 完全一致は k=1.0 に scope（design）ゆえ k≠1.0 は ≤0.5px 相当（ceil(0.5×k)＋AA 余白）の許容差。
    fn run_live_diff_nonunit_scale(mode: WritingMode, image: (u32, u32), k: f32, tol: u32) {
        let mut ld = LiveDiffRig::new_scaled(mode, image, k);

        // ① あふれ前（3 行）。
        ld.apply_text(0.0, "あ");
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "い");
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "う");
        ld.checkpoint_block_tol("k≠1 あふれ前", 10.0, tol);

        // ② スクロール発火直後。
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "え");
        let w = ld.checkpoint_block_tol("k≠1 スクロール発火", 20.0, tol);
        assert!(
            w.first_visible_line >= 1,
            "k≠1 あふれ発火で可視窓が移動: {w:?}"
        );

        // ③ 連続スクロール。
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "お");
        ld.apply_newline(0.0);
        ld.apply_text(0.0, "か");
        let w = ld.checkpoint_block_tol("k≠1 連続スクロール", 30.0, tol);
        assert!(
            w.first_visible_line >= 2,
            "k≠1 連続スクロールで可視窓が複数行進む: {w:?}"
        );
    }

    /// 横書き k=1.25: あふれ→スクロール実描画で viewbox が oracle とブロック軸 ≤2px（≈ceil(0.5×1.25)
    /// ＋AA 余白）に収まる（R6.4——小数アキュムレータの実 GPU 経路検証）。
    #[test]
    fn live_diff_nonunit_scale_horizontal_within_tolerance() {
        run_live_diff_nonunit_scale(WritingMode::HorizontalTb, (80, 40), 1.25, 2);
    }

    /// 縦書き vertical_rl k=1.25: 同上（スクロール軸＝横 x）。
    #[test]
    fn live_diff_nonunit_scale_vertical_rl_within_tolerance() {
        run_live_diff_nonunit_scale(WritingMode::VerticalRl, (40, 80), 1.25, 2);
    }

    /// 縦書き vertical_lr k=1.25: 同上（スクロール軸＝横 x・左送り）。
    #[test]
    fn live_diff_nonunit_scale_vertical_lr_within_tolerance() {
        run_live_diff_nonunit_scale(WritingMode::VerticalLr, (40, 80), 1.25, 2);
    }

    /// G4: 大サイズフォント（20px・既定 ＭＳ ゴシック）でも全シナリオで oracle↔viewbox byte
    /// 完全一致する——AA こぼれが `DIRTY_GUARD_IMG_PX`(=1 image px) を超えず、ダーティ矩形が
    /// AA を取りこぼさないことを実描画 byte 等価で確認（spike/live-diff は従来 10px のみ）。
    /// image 寸は `2P+F ≤ block ≤ 3P+F`（P=ceil(20×1.25)=25・F=20）を満たし ①3行収容／②あふれ。
    #[test]
    fn live_diff_larger_font_matches_oracle_byte_for_byte() {
        run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
            WritingMode::HorizontalTb,
            (160, 80),
            1.0,
            None,
            20,
        ));
        run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
            WritingMode::VerticalRl,
            (80, 160),
            1.0,
            None,
            20,
        ));
    }

    /// G4: プロポーショナルフォント（ＭＳ Ｐゴシック・可変幅）でも byte 完全一致する——font 種別
    /// を変えても両方式が同一 format ゆえ AA ラスタライズが一致し、ガードが十分なら byte 等価が
    /// 保たれることを確認（P=ceil(12×1.25)=15・F=12）。
    #[test]
    fn live_diff_proportional_font_matches_oracle_byte_for_byte() {
        run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
            WritingMode::HorizontalTb,
            (80, 50),
            1.0,
            Some("ＭＳ Ｐゴシック"),
            12,
        ));
        run_live_diff_scenario_on(&mut LiveDiffRig::new_full(
            WritingMode::VerticalRl,
            (50, 80),
            1.0,
            Some("ＭＳ Ｐゴシック"),
            12,
        ));
    }

    /// 観測可能な完了状態（後半・負のコントロール）: 意図的に不一致を起こす細工（viewbox 側だけ
    /// 1 グリフ多い content を描く）を入れると read_back の byte 比較が差を検出する——比較器は
    /// ゼロでない差を捕捉できる（＝live-diff はトートロジーでない）。本物のバグ注入ではなく、
    /// 比較器の識別能力を示す最小の細工。
    #[test]
    fn live_diff_detects_injected_divergence() {
        let mut ld = LiveDiffRig::new(WritingMode::HorizontalTb, (60, 40));

        // オラクルは 1 グリフ・viewbox は 2 グリフ（1 グリフぶん確実に異なる pixel）を同一窓で描く。
        let items_oracle = glyph_items("■");
        let (canvas_oracle, window) = build(&items_oracle, &ld.region, ld.mode, ld.font_height);
        let items_viewbox = glyph_items("■■");
        let (canvas_viewbox, _) = build(&items_viewbox, &ld.region, ld.mode, ld.font_height);

        ld.oracle
            .render(
                &canvas_oracle,
                &window,
                &ld.font,
                ld.mode,
                &ld.contract,
                &mut ld.oracle_surface,
            )
            .expect("オラクル render 失敗");
        ld.viewbox
            .render(
                &canvas_viewbox,
                &window,
                &ld.font,
                ld.mode,
                &ld.contract,
                &mut ld.viewbox_surface,
            )
            .expect("viewbox render 失敗");

        let ob = ld
            .oracle_surface
            .read_back()
            .expect("オラクル read_back 失敗");
        let vb = ld
            .viewbox_surface
            .read_back()
            .expect("viewbox read_back 失敗");

        // 双方とも非退化（空面同士の vacuous な不一致でない）。
        assert!(
            opaque_count(&ob) > 0,
            "オラクル面は非透明（content が描かれている）"
        );
        assert!(
            opaque_count(&vb) > 0,
            "viewbox 面は非透明（content が描かれている）"
        );
        // 比較器は差を検出する（＝live-diff の有効性・トートロジーでない）。
        assert_ne!(
            ob, vb,
            "異なる content（1 グリフ差）は byte 比較で差として検出される——live-diff は有効"
        );
    }

    /// 回帰檻（実機の行下端欠け・D2）: **実 fixture の実フォント Yu Gothic UI** を使い、typewriter
    /// 前進＋あふれスクロールの全フレームで oracle（全域再描画）と viewbox（ダーティスクロール）の
    /// read_back が byte 完全一致することを檻化する。
    ///
    /// このバグ（各行 em ボックス下端の 2〜3px はみ出しインクをダーティが切り落とす）は既定 ＭＳ
    /// ゴシック（はみ出さない字）では出ず、`diag_line_boundary_dropout_vs_oracle`（ＭＳゴシック）が
    /// 見逃していた。Yu Gothic UI は descent 側へインクがこぼれるため、[`crate::viewbox::BLOCK_INK_BLEED_FRACTION`]
    /// による行スロット拡張が無いと diverge する（＝この檻が修正前に落ちる）。fixture が読めない環境
    /// （font 不在等）でも oracle↔viewbox は font に依らず一致するため頑健。実 example の cue `at`
    /// スケジュール（LINE1 0.0／LINE2 0.5／LINE3 1.2／あふれ 2.0）を忠実再現する。
    #[test]
    fn yugothic_real_fixture_matches_oracle_byte_for_byte() {
        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku");
        let read_dec = |name: &str| -> String {
            let bytes = std::fs::read(fixture_dir.join(name))
                .unwrap_or_else(|e| panic!("fixture {name} 読取失敗: {e}"));
            areka_parsers::charset::decode(&bytes, areka_parsers::charset::DefaultEncoding::Utf8)
        };
        let model = areka_parsers::balloon::parse_str(
            &read_dec("descript.txt"),
            Some(&read_dec("balloons0s.txt")),
        );
        let balloon_image = (400u32, 224u32);
        let resolved = crate::actor::ResolvedBalloonText::resolve(&model, balloon_image);
        let mode = resolved.mode;
        let font = &resolved.font;
        let region = &resolved.region;
        assert_eq!(
            mode,
            WritingMode::HorizontalTb,
            "fixture は横書き（D2 は横書き不具合）"
        );

        let image = (
            (region.right() - region.left()).ceil() as u32,
            (region.bottom() - region.top()).ceil() as u32,
        );
        let mut rig = Rig::new();
        let mut oracle_surface = rig.attach(image, 1.0);
        let mut viewbox_surface = rig.attach(image, 1.0);
        let contract = ScaleContract::new(1.0, None);
        let config = TextLayerConfig::default();
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let metrics = DWriteMetrics::new(&factory, font, mode, &config).expect("DWriteMetrics");
        let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
        let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");
        let actor = ActorKey::from("0");
        let mut state = TextLayerState::default();
        // reveal は配送 duration 由来（interval = duration / N）。Text へ N×0.05 を焼き込むと
        // 各 chunk 内が旧 char_wait=0.05 と同一ペースで per-glyph 進行する（chunk 境界は at で gate）。
        let cue_at = |at: f64, cmd: CueCommand| TalkCue {
            at,
            actor: actor.clone(),
            duration: match &cmd {
                CueCommand::Text(t) => t.chars().count() as f64 * 0.05,
                _ => 0.0,
            },
            command: cmd,
        };
        state.apply_cue(&cue_at(0.0, CueCommand::Text("おっはよー！".into())));
        state.apply_cue(&cue_at(0.5, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue_at(
            0.5,
            CueCommand::Text("めっちゃええ朝やん！".into()),
        ));
        state.apply_cue(&cue_at(1.2, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue_at(1.2, CueCommand::Text("今日もいくでー！".into())));
        for _ in 0..9 {
            state.apply_cue(&cue_at(2.0, CueCommand::NewLine { ratio: 1.0 }));
            state.apply_cue(&cue_at(2.0, CueCommand::Text("ほな".into())));
        }

        // 密な時間格子で前進提示（viewbox は状態依存ゆえ昇順に全フレーム）。各フレームで byte 比較。
        let mut checked_opaque = false;
        let mut t = 0.0f64;
        while t <= 3.05 {
            let visible = state.visible_glyphs(&actor, t);
            let items: Vec<TextItem> = state
                .actor_state(&actor)
                .map(|s| s.items().to_vec())
                .unwrap_or_default();
            let lines = LayoutEngine::layout(
                &items,
                visible,
                region,
                mode,
                font.height,
                &metrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, region, mode);
            let canvas = ContentCanvas::from_layout(&lines, region, mode);
            oracle
                .render(&canvas, &window, font, mode, &contract, &mut oracle_surface)
                .expect("oracle render");
            viewbox
                .render(
                    &canvas,
                    &window,
                    font,
                    mode,
                    &contract,
                    &mut viewbox_surface,
                )
                .expect("viewbox render");
            let ob = oracle_surface.read_back().expect("oracle read_back");
            let vb = viewbox_surface.read_back().expect("viewbox read_back");
            if ob != vb {
                let (w, h) = oracle_surface.size();
                let diff_rows: Vec<u32> = (0..h)
                    .filter(|&y| {
                        let r = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                        ob[r.clone()] != vb[r]
                    })
                    .collect();
                panic!(
                    "Yu Gothic UI 実 fixture で viewbox が oracle と diverge（行下端はみ出しインクの切り落とし・D2）: \
                     t={t:.2} visible={visible} 相違行 y={diff_rows:?}"
                );
            }
            if visible > 0 {
                checked_opaque |= opaque_count(&vb) > 0;
            }
            t += 0.02;
        }
        assert!(
            checked_opaque,
            "非退化: content フレームで非透明ピクセルを観測している（vacuous でない）"
        );
    }

    // ════ 目視診断（PNG ダンプ・#[ignore]・開発者の「出力画像を見せて」要求への回答） ════
    //
    // 実 fixture（font 28px・validrect 320×122・実 DWriteMetrics・行1「おっはよー！」/行2
    // 「めっちゃええ朝やん！」/行3「今日もいくでー！」＋あふれ短行）を typewriter 前進で
    // viewbox/oracle 双方へ描き、指定フレームで read_back を白背景へ合成して PNG 保存する。
    // byte 等価檻（`diag_line_boundary_dropout_vs_oracle`）は「viewbox==oracle」を保証するが、
    // 「その全域再描画自体が正しく見えるか（下端欠け等）」は人（AI vision）が画像を見るしかない。
    // 出力先は env `AREKA_DIAG_OUT`（無指定なら CARGO_TARGET_TMPDIR 相当のカレント）。
    // 依存を足さないため PNG は自前エンコード（無圧縮 deflate・RGBA8）。

    /// CRC-32（PNG チャンク用・多項式 0xEDB88320）。
    fn diag_crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    /// Adler-32（zlib ストリーム末尾）。
    fn diag_adler32(data: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in data {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }

    /// PNG チャンク（長さ＋種別＋データ＋CRC）を書き足す。
    fn diag_png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut kd = Vec::with_capacity(4 + data.len());
        kd.extend_from_slice(kind);
        kd.extend_from_slice(data);
        out.extend_from_slice(&kd);
        out.extend_from_slice(&diag_crc32(&kd).to_be_bytes());
    }

    /// RGBA8 を無圧縮 deflate（stored ブロック）で PNG エンコードする（依存追加なし）。
    fn diag_encode_png_rgba(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
        // フィルタ 0 のスキャンライン列。
        let mut raw = Vec::with_capacity((h * (1 + w * 4)) as usize);
        for y in 0..h {
            raw.push(0u8);
            let row = ((y * w * 4) as usize)..(((y + 1) * w * 4) as usize);
            raw.extend_from_slice(&rgba[row]);
        }
        // zlib: ヘッダ 0x78 0x01 ＋ stored ブロック列 ＋ adler32。
        let mut zlib = vec![0x78u8, 0x01];
        let mut i = 0usize;
        while i < raw.len() {
            let chunk = (raw.len() - i).min(65535);
            let bfinal = if i + chunk >= raw.len() { 1u8 } else { 0u8 };
            zlib.push(bfinal); // BFINAL ＋ BTYPE=00（stored）
            zlib.extend_from_slice(&(chunk as u16).to_le_bytes());
            zlib.extend_from_slice(&(!(chunk as u16)).to_le_bytes());
            zlib.extend_from_slice(&raw[i..i + chunk]);
            i += chunk;
        }
        zlib.extend_from_slice(&diag_adler32(&raw).to_be_bytes());

        let mut out = Vec::new();
        out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8bit・color type 6（RGBA）
        diag_png_chunk(&mut out, b"IHDR", &ihdr);
        diag_png_chunk(&mut out, b"IDAT", &zlib);
        diag_png_chunk(&mut out, b"IEND", &[]);
        out
    }

    /// premultiplied BGRA read_back を白背景へ合成し、pitch グリッド線（薄青・破線的に）と
    /// 面境界（マゼンタ）を重ねた RGBA8 を返す（下端欠け・行境界の欠落を目視しやすくする）。
    fn diag_composite_rgba(bgra: &[u8], w: u32, h: u32, pitch: u32) -> Vec<u8> {
        let bg = [255u8, 255, 255];
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for (idx, px) in bgra.chunks_exact(4).enumerate() {
            let (b, g, r, a) = (px[0] as u32, px[1] as u32, px[2] as u32, px[3] as u32);
            let inv = 255 - a;
            let out_r = (r + bg[0] as u32 * inv / 255).min(255) as u8;
            let out_g = (g + bg[1] as u32 * inv / 255).min(255) as u8;
            let out_b = (b + bg[2] as u32 * inv / 255).min(255) as u8;
            let o = idx * 4;
            rgba[o] = out_r;
            rgba[o + 1] = out_g;
            rgba[o + 2] = out_b;
            rgba[o + 3] = 255;
        }
        // pitch グリッド線（各行の上端 y=0,pitch,2pitch,…）を薄青で、境界をマゼンタで。
        let put = |rgba: &mut [u8], x: u32, y: u32, c: [u8; 3]| {
            if x < w && y < h {
                let o = ((y * w + x) * 4) as usize;
                rgba[o] = c[0];
                rgba[o + 1] = c[1];
                rgba[o + 2] = c[2];
            }
        };
        if pitch > 0 {
            let mut gy = 0u32;
            while gy < h {
                for x in (0..w).step_by(3) {
                    put(&mut rgba, x, gy, [170, 200, 255]);
                }
                gy += pitch;
            }
        }
        for x in 0..w {
            put(&mut rgba, x, 0, [255, 0, 255]);
            put(&mut rgba, x, h - 1, [255, 0, 255]);
        }
        for y in 0..h {
            put(&mut rgba, 0, y, [255, 0, 255]);
            put(&mut rgba, w - 1, y, [255, 0, 255]);
        }
        rgba
    }

    /// 最下端の非透明インク行 y（画面下端欠けの定量指標・インクなしは None）。
    fn diag_bottom_ink_row(bgra: &[u8], w: u32, h: u32) -> Option<u32> {
        for y in (0..h).rev() {
            for x in 0..w {
                if bgra[((y * w + x) * 4 + 3) as usize] != 0 {
                    return Some(y);
                }
            }
        }
        None
    }

    /// 目視診断: 実 fixture を typewriter 前進で描き、指定 visible 数のフレームで viewbox/oracle の
    /// read_back を PNG 保存する（`cargo test -p areka-emo-text --lib -- --ignored --nocapture
    /// diag_dump_horizontal_pngs`・env `AREKA_DIAG_OUT` で出力先指定）。
    #[test]
    #[ignore = "PNG ダンプ（ファイル副作用・目視診断用・明示実行のみ）"]
    fn diag_dump_horizontal_pngs() {
        let out_dir = std::env::var("AREKA_DIAG_OUT").unwrap_or_else(|_| ".".to_string());
        let mut rig = Rig::new();
        // 実 fixture（emo2-kakukaku）の balloon model を実ファイルからロードする（Yu Gothic UI・
        // validrect [36,356]×[46,168]＝320×122・wordwrappoint.x=-49）——example の load_balloon_model
        // と同一経路。フォント/折返し/validrect を実機と完全一致させる（既定 ＭＳ ゴシックでない）。
        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku");
        let read_dec = |name: &str| -> String {
            let bytes = std::fs::read(fixture_dir.join(name)).expect("fixture 読取");
            areka_parsers::charset::decode(&bytes, areka_parsers::charset::DefaultEncoding::Utf8)
        };
        let model = areka_parsers::balloon::parse_str(
            &read_dec("descript.txt"),
            Some(&read_dec("balloons0s.txt")),
        );
        // balloon 画像原寸（surface0＝400×224）で region を解決する（validrect は image 相対）。
        let balloon_image = (400u32, 224u32);
        let resolved = crate::actor::ResolvedBalloonText::resolve(&model, balloon_image);
        let mode = resolved.mode;
        let font = &resolved.font;
        let region = &resolved.region;
        // 供給面＝validrect 物理寸（ceil(validrect × k)・k=1）。
        let image = (
            (region.right() - region.left()).ceil() as u32,
            (region.bottom() - region.top()).ceil() as u32,
        );
        let mut oracle_surface = rig.attach(image, 1.0);
        let mut viewbox_surface = rig.attach(image, 1.0);
        let contract = ScaleContract::new(1.0, None);
        let config = TextLayerConfig::default();
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let metrics = DWriteMetrics::new(&factory, font, mode, &config).expect("DWriteMetrics");
        let pitch = {
            use crate::layout::GlyphMetrics;
            metrics.line_pitch(font.height) as u32
        };
        eprintln!(
            "[diag] model: font={} height={} mode={mode:?} validrect=[{},{}]x[{},{}] surface={image:?} wrap={}",
            font.name,
            font.height,
            region.left(),
            region.right(),
            region.top(),
            region.bottom(),
            region.wrap_threshold()
        );
        let mut oracle = DrawExecutor::new(&rig.core).expect("DrawExecutor");
        let mut viewbox = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor");
        let actor = ActorKey::from("0");
        let mut state = TextLayerState::default();
        let mk = |cmd| TalkCue {
            at: 0.0,
            actor: actor.clone(),
            command: cmd,
            duration: 0.0,
        };
        // 実 example と同一の cue `at` スケジュール（LINE1 at 0.0・LINE2 at 0.5・LINE3 at 1.2・
        // あふれ短行 at 2.0）。reveal は配送 duration 由来（interval = duration / N）。Text へ
        // N×0.05 を焼き込むと各 chunk 内が旧 char_wait=0.05 と同一ペースで per-glyph 進行する
        // （chunk 境界は at で gate）。
        // 【newline-defer】かつて（即時意味論では）at を分散させると未リビール NewLine が即座に
        // 「幽霊空行」を出し人工的なスクロールを誘発した——遅延化（deferred newline）で保留改行は
        // 次の可視グリフが reveal されるまで行を開かないため、幽霊空行はもはや生じず、あふれ発火は
        // 実体化時刻（改行より後ろのグリフの reveal 時）へ後退する。at 分散は実機の reveal タイミングを
        // 模す診断ダンプの時間対応としてのみ残す（幽霊空行の再現目的ではない）。
        let cue_at = |at: f64, cmd: CueCommand| TalkCue {
            at,
            actor: actor.clone(),
            duration: match &cmd {
                CueCommand::Text(t) => t.chars().count() as f64 * 0.05,
                _ => 0.0,
            },
            command: cmd,
        };
        state.apply_cue(&cue_at(0.0, CueCommand::Text("おっはよー！".into())));
        state.apply_cue(&cue_at(0.5, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue_at(
            0.5,
            CueCommand::Text("めっちゃええ朝やん！".into()),
        ));
        state.apply_cue(&cue_at(1.2, CueCommand::NewLine { ratio: 1.0 }));
        state.apply_cue(&cue_at(1.2, CueCommand::Text("今日もいくでー！".into())));
        for _ in 0..9 {
            state.apply_cue(&cue_at(2.0, CueCommand::NewLine { ratio: 1.0 }));
            state.apply_cue(&cue_at(2.0, CueCommand::Text("ほな".into())));
        }
        let _ = mk;

        // 密な時間格子で前進提示（viewbox は状態依存ゆえ昇順に全フレーム回す）。dump は目標時刻通過時。
        let dt = 0.02f64;
        let mut dump_times: Vec<f64> = vec![0.35, 1.10, 1.90, 2.02, 2.10, 2.30, 2.60, 3.00];
        eprintln!(
            "[diag] out_dir={out_dir} pitch={pitch} region_h={} reveal_interval={}",
            region.bottom() - region.top(),
            0.05
        );
        let mut t = 0.0f64;
        while t <= 3.05 {
            let visible = state.visible_glyphs(&actor, t);
            let items: Vec<TextItem> = state
                .actor_state(&actor)
                .map(|s| s.items().to_vec())
                .unwrap_or_default();
            let lines = LayoutEngine::layout(
                &items,
                visible,
                &region,
                mode,
                font.height,
                &metrics,
                WrapPlan::CharByChar,
            );
            let window = LayoutEngine::visible_window(&lines, &region, mode);
            let canvas = ContentCanvas::from_layout(&lines, &region, mode);
            oracle
                .render(&canvas, &window, font, mode, &contract, &mut oracle_surface)
                .expect("oracle render");
            viewbox
                .render(
                    &canvas,
                    &window,
                    font,
                    mode,
                    &contract,
                    &mut viewbox_surface,
                )
                .expect("viewbox render");

            if dump_times.first().is_some_and(|&t_target| t >= t_target) {
                let target = dump_times.remove(0);
                let ob = oracle_surface.read_back().expect("oracle read_back");
                let vb = viewbox_surface.read_back().expect("viewbox read_back");
                let (w, h) = viewbox_surface.size();
                let diff_rows = (0..h)
                    .filter(|&y| {
                        let r = ((y * w) * 4) as usize..(((y + 1) * w) * 4) as usize;
                        ob[r.clone()] != vb[r]
                    })
                    .count();
                let vb_bottom = diag_bottom_ink_row(&vb, w, h);
                // 差分行の内訳: 各 y で「oracle インク有り・viewbox 空」= 欠け画素数。
                let mut miss_rows: Vec<(u32, u32)> = Vec::new();
                for y in 0..h {
                    let mut miss = 0u32;
                    for x in 0..w {
                        let o = ((y * w + x) * 4) as usize;
                        if ob[o + 3] != 0 && vb[o + 3] == 0 {
                            miss += 1;
                        }
                    }
                    if miss > 0 {
                        miss_rows.push((y, miss));
                    }
                }
                eprintln!(
                    "[diag] t={t:.2}(target {target:.2}) visible={visible} lines={} fvl={} \
                     block_off={:.1} diff_rows={diff_rows} viewbox_bottom_ink_y={vb_bottom:?} (面高={h})",
                    lines.len(),
                    window.first_visible_line,
                    window.block_offset
                );
                eprintln!("[diag]   欠け行(oracle有/viewbox空) y,画素数 = {miss_rows:?}");
                let ms = (target * 100.0).round() as u32;
                for (tag, bytes) in [("viewbox", &vb), ("oracle", &ob)] {
                    let rgba = diag_composite_rgba(bytes, w, h, pitch);
                    let png = diag_encode_png_rgba(&rgba, w, h);
                    let path = format!("{out_dir}/diag_h_{tag}_t{ms:03}.png");
                    std::fs::write(&path, &png).expect("PNG 書き込み");
                    eprintln!("[diag]   saved {path}");
                }
                // diff 画像: oracle を薄く敷き、viewbox≠oracle の画素を赤で強調（欠け位置の可視化）。
                {
                    let mut rgba = diag_composite_rgba(&ob, w, h, pitch);
                    // oracle をグレーアウト（インク位置の文脈だけ残す）。
                    for px in rgba.chunks_exact_mut(4) {
                        px[0] = 200 + (px[0] as u16 * 55 / 255) as u8;
                        px[1] = 200 + (px[1] as u16 * 55 / 255) as u8;
                        px[2] = 200 + (px[2] as u16 * 55 / 255) as u8;
                    }
                    for y in 0..h {
                        for x in 0..w {
                            let o = ((y * w + x) * 4) as usize;
                            let (oa, va) = (ob[o + 3], vb[o + 3]);
                            let diff = ob[o] != vb[o]
                                || ob[o + 1] != vb[o + 1]
                                || ob[o + 2] != vb[o + 2]
                                || oa != va;
                            if diff {
                                // oracle にインクがあり viewbox に無い＝欠け（赤）。逆＝過剰（青）。
                                if oa != 0 && va == 0 {
                                    rgba[o] = 255;
                                    rgba[o + 1] = 0;
                                    rgba[o + 2] = 0;
                                } else {
                                    rgba[o] = 0;
                                    rgba[o + 1] = 80;
                                    rgba[o + 2] = 255;
                                }
                            }
                        }
                    }
                    let png = diag_encode_png_rgba(&rgba, w, h);
                    let path = format!("{out_dir}/diag_h_DIFF_t{ms:03}.png");
                    std::fs::write(&path, &png).expect("PNG 書き込み");
                    eprintln!("[diag]   saved {path} (赤=viewbox欠け・青=過剰)");
                }
            }
            t += dt;
        }
    }

    /// 目視診断（budoux ワードラップ・R9.2/9.3）: 実 fixture（emo2-kakukaku・Yu Gothic UI 28px・
    /// 狭い validrect 320×122・`wordwrappoint.x=-49`）へ、(1) 通常文を **OFF（文字単位）** と
    /// **ON（分かち書き）** の両方、(2) budoux 境界を持たない長大塊を **ON** で描き、fully-revealed
    /// （全グリフ可視）で PNG を保存する。
    ///
    /// - **通常ケース（OFF/ON 並置）**: 狭いバルーンでは OFF が文節を行末で途中分割するが、ON は
    ///   塊を丸ごと次行へ送る（R9.1）。`diag_budoux_normal_off.png` と `diag_budoux_normal_on.png`
    ///   を並べて改善を目視できる。
    /// - **長大塊ケース（ON）**: 行頭からでも 1 行に収まらない塊（budoux 境界のない連続カナ）は
    ///   当該塊に限って文字単位縮退し、バルーン（validrect＝供給面寸）からはみ出さない（R9.2）。
    ///   `diag_budoux_longseg_on.png` を出す。最右インク列が供給面右端で張り付いていない（＝
    ///   はみ出しクリップでない）ことを `eprintln!` の数値でも補助確認する。
    ///
    /// byte 等価檻（`diag_line_boundary_dropout_vs_oracle` 等）は「viewbox==oracle」を保証するが、
    /// 「分かち書き折返しが読みやすく見えるか・長大塊がはみ出さないか」は人（AI vision）が画像を
    /// 見るしかない（記憶 emo-text-byte-equiv-default-font-blindspot）。全域再描画オラクル
    /// [`DrawExecutor`] を使い fully-revealed の完成画像を 1 フレームで得る。
    /// `cargo test -p areka-emo-text --lib -- --ignored --nocapture diag_dump_budoux_wordwrap_pngs`
    /// （出力先は env `AREKA_DIAG_OUT`・無指定はカレント）。
    #[test]
    #[ignore = "PNG ダンプ（ファイル副作用・目視診断用・明示実行のみ）"]
    fn diag_dump_budoux_wordwrap_pngs() {
        let out_dir = std::env::var("AREKA_DIAG_OUT").unwrap_or_else(|_| ".".to_string());
        let mut rig = Rig::new();
        // 実 fixture（emo2-kakukaku）を実ファイルからロード（Yu Gothic UI・validrect 320×122・
        // wordwrappoint.x=-49）——diag_dump_horizontal_pngs と同一経路。狭いバルーンが char-by-char
        // の途中分割を誘発する（＝ON の改善が見える台）。
        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku");
        let read_dec = |name: &str| -> String {
            let bytes = std::fs::read(fixture_dir.join(name)).expect("fixture 読取");
            areka_parsers::charset::decode(&bytes, areka_parsers::charset::DefaultEncoding::Utf8)
        };
        let model = areka_parsers::balloon::parse_str(
            &read_dec("descript.txt"),
            Some(&read_dec("balloons0s.txt")),
        );
        // balloon 画像原寸（surface0＝400×224）で region を解決する（validrect は image 相対）。
        let balloon_image = (400u32, 224u32);
        let resolved = crate::actor::ResolvedBalloonText::resolve(&model, balloon_image);
        let mode = resolved.mode;
        let font = &resolved.font;
        let region = &resolved.region;
        // 供給面＝validrect 物理寸（ceil(validrect × k)・k=1）＝バルーンの折返し閾/はみ出し境界。
        let image = (
            (region.right() - region.left()).ceil() as u32,
            (region.bottom() - region.top()).ceil() as u32,
        );
        let contract = ScaleContract::new(1.0, None);
        let config = TextLayerConfig::default();
        let factory = rig.core.dwrite_factory().expect("dwrite_factory").clone();
        let metrics = DWriteMetrics::new(&factory, font, mode, &config).expect("DWriteMetrics");
        let pitch = {
            use crate::layout::GlyphMetrics;
            metrics.line_pitch(font.height) as u32
        };
        eprintln!(
            "[diag-budoux] model: font={} height={} mode={mode:?} validrect=[{},{}]x[{},{}] surface={image:?} wrap_threshold={}",
            font.name,
            font.height,
            region.left(),
            region.right(),
            region.top(),
            region.bottom(),
            region.wrap_threshold()
        );

        // 1 ケース（sentence を fully-revealed で描画）を PNG 保存するヘルパ。ON（use_budoux=true）は
        // actor.rs と同型の遅延初期化で segment_plan を計算し WrapPlan::Segmented を供給する（OFF は
        // segment_plan を呼ばず CharByChar）。戻り値＝(行数, 最右インク列 x)。
        let dump_case =
            |rig: &mut Rig, tag: &str, sentence: &str, use_budoux: bool| -> (usize, Option<u32>) {
                let items = glyph_items(sentence);
                let visible = items
                    .iter()
                    .filter(|i| matches!(i, TextItem::Glyph { .. }))
                    .count();
                let plan; // ON アームでのみ束縛（借用 &plan が layout 呼出まで生存）。
                let wrap = if use_budoux {
                    plan = crate::segment::segment_plan(&items);
                    let segs: Vec<(usize, usize)> =
                        plan.segments().iter().map(|s| (s.start, s.len)).collect();
                    eprintln!("[diag-budoux]   segments(start,len)={segs:?}");
                    WrapPlan::Segmented(&plan)
                } else {
                    WrapPlan::CharByChar
                };
                let lines = LayoutEngine::layout(
                    &items,
                    visible,
                    region,
                    mode,
                    font.height,
                    &metrics,
                    wrap,
                );
                let window = LayoutEngine::visible_window(&lines, region, mode);
                let canvas = ContentCanvas::from_layout(&lines, region, mode);
                let mut surface = rig.attach(image, 1.0);
                let mut exec = DrawExecutor::new(&rig.core).expect("DrawExecutor");
                exec.render(&canvas, &window, font, mode, &contract, &mut surface)
                    .expect("render");
                let bytes = surface.read_back().expect("read_back");
                let (w, h) = surface.size();
                // 最右インク列（inline はみ出しの定量指標・供給面は validrect 寸ゆえ描画は自動クリップ＝
                // w-1 に張り付くならはみ出しをクリップで隠している疑い）。
                let rightmost = {
                    let mut r: Option<u32> = None;
                    for x in (0..w).rev() {
                        let hit = (0..h).any(|y| bytes[((y * w + x) * 4 + 3) as usize] != 0);
                        if hit {
                            r = Some(x);
                            break;
                        }
                    }
                    r
                };
                let rgba = diag_composite_rgba(&bytes, w, h, pitch);
                let png = diag_encode_png_rgba(&rgba, w, h);
                let path = format!("{out_dir}/diag_budoux_{tag}.png");
                std::fs::write(&path, &png).expect("PNG 書き込み");
                eprintln!(
                    "[diag-budoux]   saved {path}  budoux={use_budoux} sentence=「{sentence}」 \
                     visible={visible} lines={} rightmost_ink_x={rightmost:?} (surface_w={w} pitch={pitch})",
                    lines.len()
                );
                (lines.len(), rightmost)
            };

        // ── 通常ケース: 狭いバルーンで char-by-char が文節を途中分割する和文（OFF/ON 並置）。 ──
        let normal = "今日はとても良い天気ですね一緒に近くの公園へ遊びに行きましょう";
        eprintln!("[diag-budoux] === 通常ケース（OFF: 文字単位） ===");
        dump_case(&mut rig, "normal_off", normal, false);
        eprintln!("[diag-budoux] === 通常ケース（ON: 分かち書き） ===");
        dump_case(&mut rig, "normal_on", normal, true);

        // ── 長大塊ケース: budoux 境界を持たない連続カナ（行頭からでも 1 行に収まらない）を ON で。 ──
        // 前後に通常の和文を置き「長大塊のみ縮退し前後は分かち書き継続」を目視できる形にする（R3.3）。
        let longseg = "むかしむかしバアアアアアアアアアアアアアアアアアアアアアア山という所に住んでいました";
        eprintln!("[diag-budoux] === 長大塊ケース（ON: 縮退＋はみ出しなし） ===");
        dump_case(&mut rig, "longseg_on", longseg, true);
    }
}
