//! バルーン背景色の受け口（`actor.rs` の子・要件 4.6）。
//!
//! 無効表示の見た目の色は「背景 + 文字色 × 2 を 3 で割る」混色で導く
//! （[`crate::color::mix_disabled`] が唯一の実装点）。その `背景` を結線層が
//! ランタイムへ預ける口が本モジュールである。
//!
//! 背景付きの解決（[`ResolvedBalloonText::resolve_with_background`]）も本モジュールが持つ
//! （`actor.rs` の [`resolve`](ResolvedBalloonText::resolve) が背景 白でここへ委譲する・
//! design.md「それ以上の追加は `actor_decoration.rs` へ」）。
//!
//! 2 層を純粋状態へ差し込む点は 1 つだけ——`actor.rs` の
//! [`TextLayerRuntime::register_actor`]（装着も再追従もそこへ合流する）。
//!
//! 範囲外ゆえ無視した `origin` 宣言の作者向け警告（[`warn_ignored_origin`]）も本モジュールが
//! 持つ——書く時点は登録口のまま、置き場所だけが子である（`actor.rs` の行数の余白のため）。

use std::rc::Rc;

use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::ActorKey;
use tracing::{debug, error, warn};
use wintf::ecs::GraphicsCore;

use super::{ActorRender, ResolvedBalloonText, TextLayerRuntime};
use crate::TextLayerError;
use crate::choice::ResolvedChoiceStyle;
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, DWriteMetrics, FontCatalog, ResolvedFont};
use crate::look::GlyphStyles;
use crate::region::{BALLOON_NAME_PLACEHOLDER, TextRegion};
use crate::state::{ActorTextState, TextLayerConfig};
use crate::surface::TextSurface;
use crate::viewbox_draw::ViewboxExecutor;
use crate::wrap::WrapMode;
use crate::writing::WritingMode;

/// 装着済みの供給面へ描画実行部と計測器を添えて 1 actor 分の描画資源を組む（task 7.3）。
///
/// フォント候補列の解決台帳（[`FontCatalog`]）は**ここで 1 つだけ**作り、計測器と描画器の
/// 双方へ共有で渡す（要件 9.8）。台帳が 2 つあると、同じ候補列が全滅したときの記録が計測側と
/// 描画側で二重に出る——警告の源を 1 つに保つための共有である。
///
/// `actor.rs` ではなく本ファイルに置くのは、`actor.rs` が 1 ファイル 1,000 行の見張りの
/// 間近にあるため（design.md「それ以上の追加は `actor_decoration.rs` へ」）。
pub(super) fn build_actor_render(
    core: &GraphicsCore,
    surface: TextSurface,
    font: &ResolvedFont,
    mode: WritingMode,
    config: &TextLayerConfig,
    actor: &ActorKey,
) -> Result<ActorRender, TextLayerError> {
    let Some(factory) = core.dwrite_factory() else {
        error!(actor = %actor, "present_frame: dwrite_factory 不在（metrics を構築できない）");
        return Err(TextLayerError::Device {
            hresult: 0,
            context: "GraphicsCore::dwrite_factory",
        });
    };
    let fonts = Rc::new(FontCatalog::new(factory)?);
    let executor = ViewboxExecutor::new_shared(core, Rc::clone(&fonts))?;
    let metrics = DWriteMetrics::new_shared(factory, font, mode, config, fonts)?;
    Ok(ActorRender {
        surface,
        executor,
        metrics,
    })
}

/// 配置層へ渡す「グリフ序数→見た目」の読み口を当該 actor の状態から組む（要件 3.3）。
///
/// `default` は**装着済みバルーン定義の既定**（`resolved.font.looks.default`）を使う——
/// 純粋状態が持つ 2 層と同一の値であり（装着の 1 点 [`TextLayerRuntime::register_actor`] が
/// 差し込む）、配置と描画が同じ既定を見ることを呼び出し側で揃える。
pub(super) fn glyph_styles_of<'a>(
    actor_state: &'a ActorTextState,
    resolved: &'a ResolvedBalloonText,
) -> GlyphStyles<'a> {
    GlyphStyles {
        table: actor_state.styles(),
        ids: actor_state.glyph_styles(),
        default: &resolved.font.looks.default,
        current: actor_state.current_look(),
    }
}

/// 範囲外ゆえ無視した `origin` 宣言を、装着 1 回につき成分 1 つ 1 件の `warn!` で知らせる
/// （要件 3.1〜3.5）。
///
/// 書く**時点**は登録口（[`TextLayerRuntime::register_actor`]）のままである——「読み込み
/// （装着）1 回につき 1 件」という意味を持つ層がそこだからで、先例の折返しの警告と同じ決着
/// である。関数の**置き場所**だけを子モジュールにするのは、`actor.rs` が 1 ファイル 1,000 行の
/// 見張りの間近にあるため（`build_actor_render` を引き受けたのと同じ理由・同じ形）。
///
/// # 件数（要件 3.2）
///
/// `previous` は当該 actor の**前回の**解決済み領域（未登録＝装着なら `None`）。領域の値が
/// 新しく決まったとき——初回、または前回と異なる領域——だけ書く。値の同じ再追従は 0 件で
/// あり、先例の折返しの警告と同一の件数規律である。
///
/// 範囲の内外の判定そのものは**ここでは引き直さない**。純粋層が
/// [`TextRegion::ignored_origin`] に載せて運んだ値を読むだけである（同じ判定式が 2 か所に
/// あると、片方だけが直って静かに食い違う）。
pub(super) fn warn_ignored_origin(resolved: &ResolvedBalloonText, previous: Option<TextRegion>) {
    let region = resolved.region;
    if previous == Some(region) {
        return;
    }
    let (ignored_x, ignored_y) = region.ignored_origin();
    let (corner_x, corner_y) = region.start();
    let (left, right) = (region.left(), region.right());
    let (top, bottom) = (region.top(), region.bottom());
    // 欄の名前・無視した解決値・範囲の両端・実際に用いた角を成分ごとに 1 組ずつ並べる。
    for (key, ignored, range_min, range_max, corner) in [
        ("origin.x", ignored_x, left, right, corner_x),
        ("origin.y", ignored_y, top, bottom, corner_y),
    ] {
        let Some(ignored) = ignored else { continue };
        warn!(
            balloon = BALLOON_NAME_PLACEHOLDER,
            key,
            resolved = ignored,
            range_min,
            range_max,
            corner,
            "origin に指定した文字の書き始めの位置が、文字を描いてよい範囲（validrect）の外にある——指定は使わず、範囲の書き始めの角から書いた"
        );
    }
}

impl ResolvedBalloonText {
    /// [`resolve`](Self::resolve) にバルーンの**背景色**（面 0 の原点画素・sRGB 非
    /// premultiplied）を与えた形（要件 4.6）。
    ///
    /// 背景色は無効表示の見た目の色を導くためだけに使う（混色の式は
    /// [`crate::color::mix_disabled`] が唯一の実装点）。背景を知らない呼び手は
    /// [`resolve`](Self::resolve) を使い、[`DEFAULT_BALLOON_BACKGROUND`]（白）が採られる。
    ///
    /// 装着（[`TextLayerRuntime::register_actor_binding`]）と再追従
    /// （[`TextLayerRuntime::refresh_actor_binding`]）はどちらもこの口を
    /// `background_of(actor)` 付きで通る——churn ガードの比較が同じ導出で揃うため、
    /// 背景が変わらない限り再追従は作り直しを起こさない。
    pub fn resolve_with_background(
        model: &BalloonModel,
        image_size: (u32, u32),
        background: (u8, u8, u8),
    ) -> ResolvedBalloonText {
        let mode = WritingMode::resolve(model);
        let font = ResolvedFont::resolve_with_background(model, background);
        // hover ハイライトスタイルはバルーン cursor.* モデル＋解決済み既定文字色から一度だけ解決する
        // （下流 present_actor の装飾は本値を読むだけ・choice.rs へは依存しない・design.md Integration）。
        let choice_style = ResolvedChoiceStyle::resolve(Some(model.cursor()), font.color);
        ResolvedBalloonText {
            mode,
            region: TextRegion::resolve(model, image_size, mode),
            font,
            wrap: WrapMode::resolve(model),
            choice_style,
        }
    }
}

impl TextLayerRuntime {
    /// バルーン背景の原点画素（sRGB・非 premultiplied）を actor ごとに覚える（要件 4.6）。
    ///
    /// 装着（[`register_actor_view`](Self::register_actor_view)）の**前**に呼ぶ——装着の
    /// 時点で覚えている値が 2 層の無効表示の色に焼かれるため、後から入れても既に装着済みの
    /// スコープには効かない（次の再追従で効く）。
    pub fn set_balloon_background(&mut self, actor: ActorKey, background: (u8, u8, u8)) {
        debug!(
            actor = %actor,
            background = ?background,
            "バルーン背景色を受け取った（無効表示の色の混色に使う）"
        );
        self.balloon_background.insert(actor, background);
    }

    /// 覚えている背景色（未設定は [`DEFAULT_BALLOON_BACKGROUND`]＝白）。
    ///
    /// 装着と再追従が**同じ値**を読むことで、churn ガードの比較（解決済み
    /// [`ResolvedBalloonText`](super::ResolvedBalloonText) の同値判定）が背景の側で
    /// 揺れなくなる——背景が変わらない限り再追従は作り直しを起こさない。
    pub(super) fn background_of(&self, actor: &ActorKey) -> (u8, u8, u8) {
        self.balloon_background
            .get(actor)
            .copied()
            .unwrap_or(DEFAULT_BALLOON_BACKGROUND)
    }
}
