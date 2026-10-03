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
//! のみ）→ ⑤その矩形と交差する行だけを `DrawTextLayout`（origin は [`DrawExecutor`] と**同一式**・design §13.3）→
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

use std::rc::Rc;

use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS, ID2D1DeviceContext,
};
use windows::Win32::Graphics::DirectWrite::{IDWriteFactory2, IDWriteTextFormat};
use wintf::com::d2d::D2D1DeviceExt;
use wintf::ecs::GraphicsCore;

use crate::TextLayerError;
use crate::canvas::ContentCanvas;
use crate::draw::{FontCatalog, LineLayoutStore, ResolvedFont, TEXT_DRAW_OPTIONS};
use crate::layout::VisibleWindow;
use crate::look::StyleTable;
use crate::region::ScaleContract;
use crate::surface::TextSurface;
use crate::viewbox::{ScrollPlanner, ScrollState};
use crate::writing::WritingMode;

#[path = "viewbox_draw_decoration.rs"]
mod decoration;
#[path = "viewbox_draw_plan.rs"]
mod plan;
#[path = "viewbox_draw_render.rs"]
mod render;

use decoration::BrushCache;
// 兄弟檻（`viewbox_draw_frame_render_tests.rs`）が `super::plan_inconsistency` で私有項目へ届くための
// 再束縛（design.md の File Structure Plan が要求するファサードの取り込み）。`full_domain_update` は
// 子モジュールの内部からしか呼ばれないため親へは束ねない（束ねると unused_imports が鳴る）。
#[cfg(test)]
use plan::plan_inconsistency;

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
    /// `DrawTextLayout` の累計実行回数（**矩形ごとの交差行数の和**に限られることの檻——
    /// 矩形の枚数 × 描画対象行数の積ではない・R11.1）。
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
    /// フォント候補列の解決台帳（計測器と共有できる・要件 9.8）。`ensure_format` はこの台帳が
    /// 解決した家族名で書式を組むので、バルーン定義のカンマ区切り候補列が描画の側でも効く。
    fonts: Rc<FontCatalog>,
    /// 装飾の色ごとの塗りブラシの記憶（同じ色を毎フレーム作り直さない）。
    brushes: BrushCache,
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
    /// テスト専用: 文字を描く描画オプション（既定は [`TEXT_DRAW_OPTIONS`]）。非テストビルドには
    /// この欄が無く、本番は定数そのものを渡す（[`Self::text_draw_options`]）。
    #[cfg(test)]
    text_draw_options: D2D1_DRAW_TEXT_OPTIONS,
}

impl ViewboxExecutor {
    /// `GraphicsCore` から plan 実行部を生成する（DWrite factory＋専用 D2D DC＋
    /// [`LineLayoutStore`]＋[`ScrollPlanner`]）。
    ///
    /// デバイス未初期化（`GraphicsCore` 無効化後）は log-first で `Device` エラー
    /// （`DrawExecutor::new` と同一経路）。
    ///
    /// **本番の呼び手は [`new_shared`](Self::new_shared) へ移った**（`actor_decoration.rs::build_actor_render`
    /// が台帳を 1 つだけ作って計測器と共有する・要件 9.8）。本関数の本番の呼び手は 0 で、
    /// 残しているのは自前の台帳で足りる決定論テスト（`viewbox_draw_*_tests.rs`）が呼ぶ
    /// 委譲の入口としてである。crate 外の呼び手も無い。
    pub fn new(core: &GraphicsCore) -> Result<ViewboxExecutor, TextLayerError> {
        let dwrite = core
            .dwrite_factory()
            .ok_or_else(|| none_err("GraphicsCore::dwrite_factory"))?
            .clone();
        let fonts = Rc::new(FontCatalog::new(&dwrite)?);
        ViewboxExecutor::new_shared(core, fonts)
    }

    /// フォント候補列の解決台帳を**共有して**plan 実行部を生成する（要件 9.8）。
    ///
    /// 計測器（[`DWriteMetrics::new_shared`](crate::draw::DWriteMetrics::new_shared)）と同じ台帳を
    /// 渡すと、同じ候補列の全滅の記録が計測側と描画側で二重に出ない（警告の源が 1 つ）。
    pub fn new_shared(
        core: &GraphicsCore,
        fonts: Rc<FontCatalog>,
    ) -> Result<ViewboxExecutor, TextLayerError> {
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
            fonts,
            brushes: BrushCache::default(),
            stats: DrawStats::default(),
            seam_warned: false,
            #[cfg(test)]
            fail_next_render: false,
            #[cfg(test)]
            text_draw_options: TEXT_DRAW_OPTIONS,
        })
    }

    /// 文字を描く描画オプション。本番ビルドは常に定数 [`TEXT_DRAW_OPTIONS`] へ戻り、
    /// テストビルドだけ欄の値（既定は同じ定数）を返す。
    #[cfg(not(test))]
    fn text_draw_options(&self) -> D2D1_DRAW_TEXT_OPTIONS {
        TEXT_DRAW_OPTIONS
    }

    #[cfg(test)]
    fn text_draw_options(&self) -> D2D1_DRAW_TEXT_OPTIONS {
        self.text_draw_options
    }

    /// テスト専用: 描画オプションだけを差し替える（色つきと単色の描き比べ・要件 6.4）。
    ///
    /// **最初の描画の前に 1 度だけ呼ぶ**。描画オプションは行の指紋に入らないので、同じ executor で
    /// 描き直しても何も描かれず前の面が残る——対照は executor と面を別々に作って取る。
    /// 行のキャッシュ・planner・面には触れない。
    #[cfg(test)]
    pub(crate) fn set_text_draw_options_for_test(&mut self, options: D2D1_DRAW_TEXT_OPTIONS) {
        self.text_draw_options = options;
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
    ///   限定描画（各矩形は**その矩形と交差する行だけ**を描く・R11.1）→ flip → commit → `Ok(true)`。
    ///
    /// **エラー縮退規律**（Error Handling）: フォント/方向変更（`ensure_format` が format/行キャッシュを
    /// 組み直したフレーム）または `plan` の想定外不整合（[`plan::plan_inconsistency`]）を検知した場合、当該
    /// フレームを**全域ダーティ Update**（`blit=(0,0)`・dirty=面全域・draw_lines=**可視窓の**
    /// GlyphRun 住人）へ差し替えて描画する（正しさ優先・1 フレームで全域再描画のオラクル
    /// `DrawExecutor::render` と同じ結果になる・透明フラッシュを起こす
    /// FullClear ではない）。フォント/方向変更は `debug!`・想定外不整合は `warn!` を残す（記憶
    /// areka-log-first-no-silent-failure）。縮退後の commit が prev_lines を張り直すため次フレームは
    /// 正常導出へ復帰する。
    ///
    /// 失敗は log-first（`error!`＋`Err`・当該フレーム skip＝**plan 未 commit**・front 不変ゆえ
    /// 表示は前フレームを保持・次フレーム再計画）。
    ///
    /// **本番の呼び手は [`render_styled`](Self::render_styled) へ移った**（`actor.rs` の
    /// `present_actor` が唯一の本番呼出点）。本関数の本番の呼び手は 0 で、残しているのは
    /// 装飾の表を持たない決定論テスト（`viewbox_draw_choice_hover_tests.rs`／
    /// `viewbox_draw_frame_render_tests.rs`／`viewbox_draw_live_diff_tests.rs`／
    /// `viewbox_draw_oracle_regression_tests.rs`／`viewbox_draw_png_dump_tests.rs`／
    /// `viewbox_draw_scroll_retain_tests.rs`）が呼ぶ委譲の入口としてである——空の
    /// [`StyleTable`] を添えて `render_styled` へ流すだけなので、装飾なしの呼出列が
    /// 装飾導入前と同一であることの照合面にもなっている（要件 14.2）。crate 外の呼び手は無い
    /// （`draw_oracle_tests.rs` の `.render(...)` は同名メソッドを持つオラクル
    /// `draw.rs::DrawExecutor`（`#[cfg(test)]`）のもので、本関数ではない）。
    pub fn render(
        &mut self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        font: &ResolvedFont,
        mode: WritingMode,
        contract: &ScaleContract,
        surface: &mut TextSurface,
    ) -> Result<bool, TextLayerError> {
        self.render_styled(
            canvas,
            window,
            font,
            mode,
            contract,
            surface,
            &StyleTable::default(),
        )
    }
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

#[cfg(test)]
#[path = "viewbox_draw_choice_hover_tests.rs"]
mod choice_hover_tests;
#[cfg(test)]
#[path = "viewbox_draw_color_emoji_tests.rs"]
mod color_emoji_tests;
#[cfg(test)]
#[path = "viewbox_draw_decoration_tests.rs"]
mod decoration_tests;
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
#[path = "viewbox_draw_scroll_retain_tests.rs"]
mod scroll_retain_tests;
#[cfg(test)]
#[path = "viewbox_draw_test_support.rs"]
mod test_support;
