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
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE,
    ID2D1DeviceContext, ID2D1Image,
};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory2, IDWriteTextFormat, IDWriteTextLayout,
};
use windows_numerics::{Matrix3x2, Vector2};
use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::ecs::GraphicsCore;

use crate::TextLayerError;
use crate::canvas::{ContentCanvas, ResidentContent};
use crate::draw::{LineLayoutStore, ResolvedFont, create_d2d_target_bitmap, create_text_format};
use crate::layout::VisibleWindow;
use crate::region::ScaleContract;
use crate::surface::TextSurface;
use crate::viewbox::{FramePlan, ScrollPlanner};
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
        })
    }

    /// 決定論観測口（テスト・example 双方が読む・R3.5/R10.3）。
    pub fn stats(&self) -> DrawStats {
        self.stats
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
        let format = self.ensure_format(font, mode)?;
        let plan = self.planner.plan(canvas, window, mode, contract, surface.size());

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
                // 描画対象住人（draw_lines）の TextLayout と origin を組む。origin 式は
                // DrawExecutor と同一（validrect-local 平行移動＋ブロック軸の可視窓オフセット）。
                let creations_before = self.line_store.creations();
                let mut draws: Vec<(Vector2, IDWriteTextLayout)> = Vec::new();
                for &index in draw_lines {
                    let resident = &canvas.residents[index];
                    let run = match &resident.content {
                        ResidentContent::GlyphRun(run) => run,
                        seam => {
                            // planner の draw_lines は GlyphRun のみゆえ通常不発の防御経路
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
                            Y: dy + window.block_offset,
                        },
                        WritingMode::VerticalRl | WritingMode::VerticalLr => Vector2 {
                            X: dx + window.block_offset,
                            Y: dy,
                        },
                    };
                    draws.push((origin, layout));
                }

                let (r, g, b) = font.color;
                let brush = self
                    .dc
                    .create_solid_color_brush(
                        &D2D1_COLOR_F {
                            r: r as f32 / 255.0,
                            g: g as f32 / 255.0,
                            b: b as f32 / 255.0,
                            a: 1.0,
                        },
                        None,
                    )
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
                        self.dc.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_ALIASED);
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
                    for (origin, layout) in &draws {
                        self.dc
                            .draw_text_layout(*origin, layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
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

                // 面の役割交換 → 計画の確定（成功時のみ・失敗フレームは未 commit で再試行安全）。
                surface.flip();
                self.planner.commit(canvas, window, mode, contract, &plan);
                self.stats.line_layout_creations += self.line_store.creations() - creations_before;
                Ok(true)
            }
        }
    }

    /// 描画/計測共用 format の確保（[`create_text_format`] 経路・`FormatKey` 不変なら再利用）。
    ///
    /// フォント/方向の変更は行キャッシュの前提（同一 format で組んだ TextLayout）を崩すため
    /// format と [`LineLayoutStore`] を組み直す（`DrawExecutor::ensure_format` と同規律・実運用は
    /// actor ごと固定のため通常発火しない・`debug!` 記録）。
    fn ensure_format(
        &mut self,
        font: &ResolvedFont,
        mode: WritingMode,
    ) -> Result<IDWriteTextFormat, TextLayerError> {
        let key: FormatKey = (font.name.clone(), font.height.to_bits(), mode);
        if let Some((cached_key, format)) = &self.format {
            if *cached_key == key {
                return Ok(format.clone());
            }
            tracing::debug!(
                ?key,
                "フォント/方向が変わったため format と行レイアウトキャッシュを組み直す"
            );
            self.line_store.clear();
        }
        let format = create_text_format(&self.dwrite, font, mode)?;
        self.format = Some((key, format.clone()));
        Ok(format)
    }
}

/// `Option` が `None`（デバイス未初期化など本来到達しない欠落）を [`TextLayerError::Device`] に
/// する（draw.rs/surface.rs と同型の log-first ヘルパ）。
fn none_err(context: &'static str) -> TextLayerError {
    tracing::error!(context, "必須リソースが欠落（デバイス未初期化 または 前提不成立）");
    TextLayerError::Device { hresult: 0, context }
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

    use super::{DrawStats, ViewboxExecutor};
    use crate::actor::TextSlotBinding;
    use crate::canvas::ContentCanvas;
    use crate::draw::ResolvedFont;
    use crate::layout::{FixedMetrics, LayoutEngine, VisibleWindow};
    use crate::region::{ScaleContract, TextRegion};
    use crate::state::TextItem;
    use crate::surface::TextSurface;
    use crate::viewbox::{FramePlan, ScrollPlanner};
    use crate::writing::WritingMode;

    /// テスト用 WUC apartment/dispatcher（surface.rs/draw.rs テストと同一方針:
    /// COM 未初期化のテストスレッドでは ASTA 第一候補・NONE 保険）。
    fn make_dispatcher_and_compositor()
    -> (windows::System::DispatcherQueueController, Compositor) {
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
            let binding = TextSlotBinding::new(slot, window, k, physical);
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
        let lines = LayoutEngine::layout(items, visible, region, mode, font_height, &FixedMetrics);
        let canvas = ContentCanvas::from_layout(&lines, region, mode);
        let window = LayoutEngine::visible_window(&lines, region, mode);
        (canvas, window)
    }

    /// 非透明ピクセル数（BGRA 密配列の α≠0）。
    fn opaque_count(bytes: &[u8]) -> usize {
        bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
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
        assert!(changed, "content ありの初回フレームは変化あり（present 要）");

        let bytes = surface.read_back().expect("read_back 失敗");
        assert!(opaque_count(&bytes) > 0, "初回描画で content の非透明ピクセルが現れる");

        let stats: DrawStats = exec.stats();
        assert!(stats.draw_text_layout_calls >= 1, "初回は少なくとも 1 行描画する");
        assert_eq!(stats.blits, 0, "初回 blit=(0,0) は blits を増やさない（全面 CopyResource）");
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
        assert!(!changed, "可視窓不変・content 不変のフレームは変化なし（present 不要）");

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
}
