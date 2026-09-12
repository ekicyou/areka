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

use areka_parsers::balloon::BalloonModel;
use areka_sakura::contract::ActorKey;
use tracing::debug;

use super::{ResolvedBalloonText, TextLayerRuntime};
use crate::choice::ResolvedChoiceStyle;
use crate::draw::{DEFAULT_BALLOON_BACKGROUND, ResolvedFont};
use crate::region::TextRegion;
use crate::wrap::WrapMode;
use crate::writing::WritingMode;

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
