//! # actor — UI ドレインとフレーム提示ステップ（結線層）
//!
//! `spawn_emo_text`（`spawn_ui` 結線・UI ドレイン起動）・`TextLayerRuntime`
//! （UI スレッド所有の集約ルート）・`TextSlotBinding`・`present_frame`
//! （毎フレームの注入時刻駆動：リビール進行→レイアウト→描画→装着）を担う。
//!
//! **層規律**: 結線層。終了経路はちょうど 2 つ——`TextMsg::Close` 受領＝`Ok(Break)`、
//! 全 `UiSender` drop＝drain 正常終了（いずれも error ログなし）。個別メッセージの処理失敗は
//! `Err` 戻し→基盤が `error!`＋継続（log-first・ループを殺さない）。

use bevy_ecs::entity::Entity;

use crate::region::ScaleContract;

/// actor の装着先（結線側が emo-present `TextSlotView` から構築して routing へ登録する）。
///
/// [`crate::surface::TextSurface::attach`] の入力。emo-present は actor を知らない
/// （層純度維持・R9.5）——`ActorKey → TargetId` の対応は結線側（example/emo2-boot）が所有し、
/// `text_slot_view(target)` で得た view の値から本型を [`Self::new`] で組む。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSlotBinding {
    /// 予約スロット（`emo-text-layer-slot` Visual entity・emo-present `VisualMount` が予約）。
    pub slot: Entity,
    /// 装着先の窓 entity。
    pub window: Entity,
    /// 合成スケール k（`TextSlotView.scale` 由来・現行契約 1.0 恒常）。
    /// 不正値は構築時に [`ScaleContract`] の縮退規約（warn!＋1.0）で正規化済み。
    pub scale: f32,
    /// バルーン surface の物理原寸（TextSurface/swapchain の物理化に使用）。
    pub surface_size: (u32, u32),
    /// 画像座標空間の原寸（負値=反対辺解決・`TextRegion::resolve` の入力）。
    ///
    /// **構築時に一点導出**: `image_size = round(surface_size / k)`（k=1.0 恒常の現行契約では
    /// `surface_size` と同値）。`TextRegion::resolve` へ物理 px を渡すのはレビューエラー
    /// （2 空間モデルの綻び目をここで構造閉塞——design.md「DPI/スケール契約」）。
    pub image_size: (u32, u32),
}

impl TextSlotBinding {
    /// `TextSlotView` の読み値（slot/window/scale/surface_size）から binding を構築し、
    /// `image_size = round(surface_size / k)` をここで**一点導出**する。
    ///
    /// k の正規化（0 以下・非有限→warn!＋1.0 縮退）は [`ScaleContract::new`] に委譲し、
    /// 保持する `scale` と `image_size` の導出が常に同一の k で自己整合するようにする
    /// （k の多重適用・混在の構造排除——design.md 不変条件 (3)）。
    pub fn new(slot: Entity, window: Entity, scale: f32, surface_size: (u32, u32)) -> Self {
        let contract = ScaleContract::new(scale, None);
        TextSlotBinding {
            slot,
            window,
            scale: contract.scale,
            surface_size,
            image_size: contract.image_size(surface_size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TextSlotBinding;
    use bevy_ecs::prelude::World;

    /// k=1.0（現行契約の物理 1:1）: `image_size` は `surface_size` と同値、
    /// slot/window/scale/surface_size は透過保持される。
    #[test]
    fn binding_identity_scale_keeps_surface_size() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 1.0, (434, 687));
        assert_eq!(binding.slot, slot);
        assert_eq!(binding.window, window);
        assert_eq!(binding.scale, 1.0);
        assert_eq!(binding.surface_size, (434, 687));
        assert_eq!(
            binding.image_size,
            (434, 687),
            "k=1.0 恒常の現行契約では image_size == surface_size"
        );
    }

    /// 一点導出 `image_size = round(surface_size / k)` の端数檻（Implementation Notes 2.4:
    /// floor/ceil 変異を殺す値で檻化する）。
    ///
    /// k=1.25・surface=(127, 94): 127/1.25=101.6→**102**（floor 変異=101 を殺す）、
    /// 94/1.25=75.2→**75**（ceil 変異=76 を殺す）。
    #[test]
    fn binding_derives_image_size_by_round_fractional() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 1.25, (127, 94));
        assert_eq!(
            binding.image_size,
            (102, 75),
            "round(127/1.25)=102（floor では 101）・round(94/1.25)=75（ceil では 76）"
        );
        assert_eq!(binding.surface_size, (127, 94), "物理原寸はそのまま保持");
    }

    /// k=2.0 の .5 境界: 101/2=50.5→51・51/2=25.5→26（round half away from zero）。
    #[test]
    fn binding_derives_image_size_half_boundary() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 2.0, (101, 51));
        assert_eq!(binding.image_size, (51, 26));
    }

    /// 不正な k（0 以下・非有限）は ScaleContract の縮退規約（warn!＋1.0）へ乗り、
    /// binding の scale / image_size とも物理 1:1 で自己整合する（k の多重適用・混在の構造排除）。
    #[test]
    fn binding_degrades_invalid_scale_to_identity() {
        let mut world = World::new();
        let slot = world.spawn_empty().id();
        let window = world.spawn_empty().id();

        let binding = TextSlotBinding::new(slot, window, 0.0, (320, 240));
        assert_eq!(binding.scale, 1.0, "不正 k は 1.0 へ縮退（log-first・panic なし）");
        assert_eq!(binding.image_size, (320, 240));
    }
}
