//! `VisualMount`（mount.rs）: 窓 Entity への最小 visual 構成の装着・text 層スロット予約・非表示切替。
//!
//! 提示段の**表示層**である。[`crate::chain::SwapChainPresenter::new`] が返す
//! `ICompositionSurface` を `Compositor::CreateSurfaceBrushWithSurface` で `SurfaceBrush` へ束ね、
//! wintf の SpriteVisual へ `SetBrush`/`SetSize`（物理 px）して窓へ装着する。合成そのものは上流、
//! 供給面バイト転送は `SwapChainPresenter`、当たり判定マスクは hit-test が担い、本型は
//! 「visual 構成・z 順・bounds（物理 px）・非表示切替」を UI スレッド上で受け持つ。
//!
//! # 構成（窓あたり・最小・入れ子合成なし）
//!
//! 1. **surface entity**: SpriteVisual（emo 自前 brush 装着）＋ [`HitTest::alpha_mask`] ＋
//!    [`AlphaMaskResource`]。`Arrangement` は物理 px で直接設定（→ `GlobalArrangement.bounds` ＝
//!    AlphaMask 座標基準・任意 DPI でクリック座標一致 R2.5）。
//! 2. **text-layer slot**: surface entity の**兄弟・上位 z**（描画で上）に置く空 entity
//!    （`Name("emo-text-layer-slot")` ＋ `Visual` ＋ `HitTest`・内容（brush）なし）。M1 の独立レイヤ
//!    描画／M2 の合成パス内レイヤ化の双方を「この entity の差し替え」で吸収する seam
//!    （emo-text-layer が消費）。`HitTest` を明示付与するのは非表示時にポインタ透過させるため
//!    （未付与は既定 `Bounds` 扱いで、`Bounds` の合成 α は `is_visible` を見ない）。
//!
//! # 非表示の契約（両 entity）
//!
//! 「バルーンを非表示にする」は**枠の面と文字層の双方が見えず・触れない**ことを意味する
//! （`areka-P0-balloon-visibility` Requirement 1.7/1.8）。[`VisualMount::set_visible`] は両 entity の
//! `Visual` と `HitTest` を同時に切り替え、[`VisualMount::attach`] の `initially_visible=false` は
//! 両 entity を最初から不可視で spawn する（可視状態を一度も経由しない＝同 Requirement 1.2）。
//!
//! # wintf `Visual::on_add` との非衝突（design §VisualMount Implementation Notes）
//!
//! wintf の `Visual` の `on_add` フックは owner Window 配下でのみ `VisualGraphics`/`SurfaceGraphics`/
//! `SurfaceGraphicsDirty`/`BrushInherit` を連鎖挿入する。本型は surface entity へ**有効な**
//! `VisualGraphics::new(sprite)`（emo 自前 brush 装着済み）を同一 spawn バンドルへ含める。ゆえに
//! `on_add` は `VisualGraphics` を「既存」とみなし既定値で上書きしない。加えて emo の surface entity へは
//! `GraphicsCommandList` を挿入しないため、`deferred_surface_creation_system`（CommandList 駆動）は
//! 発火せず emo brush と競合する DrawingSurface を作らない。`resolve_inherited_brushes` は色 `Brushes`
//! コンポーネントのみを扱い SpriteVisual の brush には触れない。以上より既存経路と競合しない。
//!
//! # z 順（描画順）の典拠
//!
//! 兄弟の z 順は `Children` の順序が権威（`visual_hierarchy_sync_system`）。同システムは `Children` を
//! 前方反復し毎回 `InsertAtBottom` を呼ぶため、**先頭の子ほど最上（描画で前面）** になる。よって
//! text-layer slot を surface entity **より先**に子として追加し、slot を上位 z（surface の上に描画）に置く。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;

use windows::UI::Composition::{
    Compositor, ICompositionSurface, SpriteVisual, Visual as WucVisual,
};
use windows::core::Interface;
use windows_numerics::Vector2;

use wintf::ecs::{
    AlphaMaskResource, Arrangement, HitTest, LayoutScale, Offset, Size, Visual, VisualGraphics,
};

use crate::command::PresentError;

/// `windows_core::Error` を [`PresentError::Device`]（ログ＋`HRESULT`＋文脈）へ写像するクロージャ。
///
/// 失敗経路はログ規律（error! + `Err` 戻り値・パニック禁止）に従い、発生箇所の静的文脈を添えて
/// 構造化エラーへ畳む。`.map_err(device_err("<where>"))?` の形で COM 呼び出しを包む。
fn device_err(context: &'static str) -> impl FnOnce(windows::core::Error) -> PresentError {
    move |e| {
        let hresult = e.code().0;
        tracing::error!(hresult, context, "WUC 呼び出しが失敗");
        PresentError::Device { hresult, context }
    }
}

/// 物理 px の外形から surface entity 用 [`Arrangement`] を作る（原点 0,0・等倍・寸＝surface 原寸）。
///
/// `BoxStyle`/taffy を経由せず物理 px を直接与える（DPI 表示契約: 論理/物理混在事故の構造的排除）。
fn physical_arrangement(size: (u32, u32)) -> Arrangement {
    Arrangement {
        offset: Offset { x: 0.0, y: 0.0 },
        scale: LayoutScale::default(),
        size: Size {
            width: size.0 as f32,
            height: size.1 as f32,
        },
    }
}

/// 窓 Entity へ装着した最小 visual 構成（surface entity ＋ text-layer slot）のハンドル。
///
/// `pub(crate)`（公開 API ではない）。後続の `EmoPresenter`（task 4.1）が target ごとに保持し、
/// apply/resize/hide で本型のメソッドを呼ぶ。装着そのもの（COM 生成）は [`Self::attach`] が担う。
pub(crate) struct VisualMount {
    /// SpriteVisual ＋ `HitTest` ＋ `AlphaMaskResource` を持つ表示 entity（窓の子）。
    surface_entity: Entity,
    /// 予約済み text 層スロット（surface の兄弟・上位 z・brush なし）。emo-text-layer が消費する。
    /// 可視性とポインタ判定は surface entity と一括で [`VisualMount::set_visible`] が切り替える。
    text_slot: Entity,
}

impl VisualMount {
    /// 指定可視性の `Visual`（spawn 用）。可視で作ってから消す経路を持たせないための構築子。
    fn visual_for(visible: bool) -> Visual {
        Visual {
            is_visible: visible,
            ..Visual::default()
        }
    }

    /// surface entity の `HitTest`: 可視＝αマスク判定・不可視＝判定停止（ポインタ透過）。
    fn surface_hit_test(visible: bool) -> HitTest {
        if visible {
            HitTest::alpha_mask()
        } else {
            HitTest::none()
        }
    }

    /// text-layer slot の `HitTest`: 可視＝矩形判定・不可視＝判定停止（ポインタ透過）。
    ///
    /// 可視時に `bounds()` を**明示**付与するのは、component 未付与が wintf 側で既定
    /// `HitTestMode::Bounds` と扱われる（`hit_test/mod.rs`）ため従来挙動と同値だからである。
    /// 明示付与にしておくことで不可視時に `none()` へ差し替えられる——`Bounds` 判定の合成 α は
    /// `Visual::clamped_opacity()`（`visual.rs`）で決まり `is_visible` を見ないため、
    /// `Visual` を不可視にするだけでは文字層のポインタ透過が成立しない。
    fn slot_hit_test(visible: bool) -> HitTest {
        if visible {
            HitTest::bounds()
        } else {
            HitTest::none()
        }
    }

    /// 1 つの entity へ可視性（`Visual`）とポインタ判定（`HitTest`）を同時に適用する。
    ///
    /// 未装着（component 不在）は装着契約の破綻ゆえ warn で記録する（沈黙する失敗経路を作らない）。
    fn apply_visibility(
        world: &mut World,
        entity: Entity,
        visible: bool,
        mode: HitTest,
        role: &'static str,
    ) {
        if let Some(mut v) = world.get_mut::<Visual>(entity) {
            v.set_visible(visible);
        } else {
            tracing::warn!(?entity, role, "set_visible: Visual が無い（装着が未完了）");
        }

        if let Some(mut ht) = world.get_mut::<HitTest>(entity) {
            *ht = mode;
        } else {
            tracing::warn!(
                ?entity,
                role,
                "set_visible: HitTest が無い（装着が未完了・ポインタ透過が成立しない）"
            );
        }
    }

    /// 窓 `window` の子として surface entity と text-layer slot を装着する。
    ///
    /// `surface` は [`crate::chain::SwapChainPresenter::new`] が返す `ICompositionSurface`、
    /// `size` は供給面の物理 px 外形。SpriteVisual へ emo 自前の `SurfaceBrush` を装着し
    /// （`SetBrush`/`SetSize`）、`Arrangement` を物理 px で直接設定する。
    ///
    /// z 順（描画）: text-layer slot を surface entity **より先**に子へ追加し、`Children` 先頭
    /// ＝最上（surface の上に描画）とする。
    ///
    /// `initially_visible` は装着直後の可視性。`false` の場合は surface entity・text-layer slot の
    /// **双方**を最初から不可視（`Visual{is_visible:false}` ＋ `HitTest::none()`）で spawn する。
    /// 「可視で spawn してから消す」経路を持たないことで、可視状態を一度も経由しないことを
    /// 構造的に保証する（Requirement 1.2——フレーム遅延や合成のタイミングに成否を委ねない）。
    pub(crate) fn attach(
        world: &mut World,
        window: Entity,
        surface: &ICompositionSurface,
        compositor: &Compositor,
        size: (u32, u32),
        initially_visible: bool,
    ) -> Result<Self, PresentError> {
        let (w, h) = size;

        // 1. SpriteVisual ＋ surface brush 装着（deferred_surface_creation_system と同一パターンを再利用）。
        let sprite = compositor
            .CreateSpriteVisual()
            .map_err(device_err("CreateSpriteVisual"))?;
        let brush = compositor
            .CreateSurfaceBrushWithSurface(surface)
            .map_err(device_err("CreateSurfaceBrushWithSurface"))?;
        // WUC 固有: SpriteVisual は自身の Size 内にのみ brush を描画する。供給面と同一の物理サイズを
        // 設定して空描画を防ぐ（DPI 表示契約: SetSize は物理 px）。
        sprite
            .SetSize(Vector2 {
                X: w as f32,
                Y: h as f32,
            })
            .map_err(device_err("SpriteVisual::SetSize"))?;
        sprite
            .SetBrush(&brush)
            .map_err(device_err("SpriteVisual::SetBrush"))?;
        let wuc_visual: WucVisual = sprite
            .cast()
            .map_err(device_err("SpriteVisual->Visual cast"))?;
        // 有効な VisualGraphics を同一バンドルへ含めることで Visual::on_add の既定値上書きを避ける
        // （visual_resource_management_system も is_valid ゆえ再作成しない＝emo brush が保持される）。
        let vg = VisualGraphics::new(wuc_visual);

        // 2. text-layer slot（兄弟・上位 z）を **先に** 追加 → Children 先頭 ＝ 最上（描画で前面）。
        let text_slot = world
            .spawn((
                Name::new("emo-text-layer-slot"),
                Self::visual_for(initially_visible),
                Self::slot_hit_test(initially_visible),
                ChildOf(window),
            ))
            .id();

        // 3. surface entity（SpriteVisual ＋ 物理 px bounds ＋ αマスクヒットテスト）。
        let surface_entity = world
            .spawn((
                Name::new("emo-surface"),
                Self::visual_for(initially_visible),
                vg,
                physical_arrangement(size),
                Self::surface_hit_test(initially_visible),
                AlphaMaskResource::new(),
                ChildOf(window),
            ))
            .id();

        // ChildOf のリレーション反映（親の Children 更新）と on_add 連鎖の遅延コマンドを確定させる。
        world.flush();

        Ok(Self {
            surface_entity,
            text_slot,
        })
    }

    /// 供給面の外形変更時に呼ぶ。surface entity の `Arrangement`（→bounds＝αマスク座標基準）と
    /// SpriteVisual の `Size` を物理 px へ追随させる（原点 0,0・等倍）。
    ///
    /// bounds の伝播は既存 `propagate_global_arrangements` に委ねる（本型は `Arrangement` を直接更新）。
    /// SpriteVisual の SetSize は最善努力（失敗はログのみ・authoritative な bounds は `Arrangement`）。
    pub(crate) fn set_bounds(&self, world: &mut World, size: (u32, u32)) {
        if let Some(mut arr) = world.get_mut::<Arrangement>(self.surface_entity) {
            arr.offset = Offset { x: 0.0, y: 0.0 };
            arr.size = Size {
                width: size.0 as f32,
                height: size.1 as f32,
            };
        } else {
            tracing::warn!(
                entity = ?self.surface_entity,
                "set_bounds: surface entity に Arrangement が無い（装着が未完了）"
            );
        }

        if let Some(sprite) = self.sprite_visual(world) {
            if let Err(e) = sprite.SetSize(Vector2 {
                X: size.0 as f32,
                Y: size.1 as f32,
            }) {
                tracing::warn!(error = ?e, "set_bounds: SpriteVisual::SetSize 失敗（bounds は Arrangement で確定済み）");
            }
        }
    }

    /// 非表示／再表示の切替（R3.3・balloon-visibility Requirement 1.7/1.8）。
    ///
    /// **surface entity と text-layer slot の双方**へ同時に及ぶ。`false`: 両者の
    /// `Visual::set_visible(false)` ＋ 当たり判定停止（surface・slot とも `HitTest::none()`）。
    /// `true`: 両者を可視へ戻し、surface は `HitTest::alpha_mask()`、slot は `HitTest::bounds()`
    /// （＝component 未付与時の既定挙動と同値）へ復帰する。
    ///
    /// 枠の面だけを不可視にすると文字層が画面に残り、さらにスロットは既定 `Bounds` 判定のまま
    /// ポインタを受け続ける（`Bounds` の合成 α は `is_visible` を見ない）。両 entity を同時に
    /// 切り替えることで「非表示＝枠と文字の双方が見えず・触れず」の契約が成立する。
    ///
    /// swap chain・キャッシュは呼び手が保持するため再表示は Present 不要で復帰する。
    /// 窓自体の show/hide は所有しない（placement/ghost 領分）。
    pub(crate) fn set_visible(&self, world: &mut World, visible: bool) {
        Self::apply_visibility(
            world,
            self.surface_entity,
            visible,
            Self::surface_hit_test(visible),
            "surface",
        );
        Self::apply_visibility(
            world,
            self.text_slot,
            visible,
            Self::slot_hit_test(visible),
            "text-layer-slot",
        );
    }

    /// SpriteVisual＋HitTest＋AlphaMaskResource を持つ表示 entity。
    pub(crate) fn surface_entity(&self) -> Entity {
        self.surface_entity
    }

    /// 予約済み text 層スロット（emo-text-layer が消費）。
    pub(crate) fn text_slot(&self) -> Entity {
        self.text_slot
    }

    /// surface entity の `VisualGraphics` から SpriteVisual を取り出す（未装着なら `None`）。
    fn sprite_visual(&self, world: &World) -> Option<SpriteVisual> {
        world
            .get::<VisualGraphics>(self.surface_entity)
            .and_then(|vg| vg.visual().cloned())
            .and_then(|v| v.cast::<SpriteVisual>().ok())
    }
}

#[cfg(test)]
#[path = "mount_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "mount_visibility_tests.rs"]
mod visibility_tests;

#[cfg(test)]
mod tests {
    use super::*;

    use bevy_ecs::hierarchy::Children;
    use wintf::ecs::HitTestMode;

    use super::test_support::attach_fixture;

    /// R1.4: attach 後、text-layer slot が surface entity の**兄弟**かつ**上位 z**（`Children` 先頭）で
    /// 存在し、`Name("emo-text-layer-slot")` を持ち、内容（描画リソース）を持たない予約 seam であること。
    ///
    /// z 典拠: `visual_hierarchy_sync_system` は `Children` を前方反復し `InsertAtBottom` するため
    /// 先頭の子ほど最上（描画で前面）。ゆえに index(slot) < index(surface) が「slot が上位 z」を意味する。
    #[test]
    fn text_slot_is_higher_z_sibling_with_name() {
        let (world, window, mount, _g) = attach_fixture(3, 2);

        let children = world
            .get::<Children>(window)
            .expect("窓に Children（子 visual）が無い");
        let idx_slot = children
            .iter()
            .position(|e| e == mount.text_slot())
            .expect("text-layer slot が窓の子でない");
        let idx_surface = children
            .iter()
            .position(|e| e == mount.surface_entity())
            .expect("surface entity が窓の子でない");

        // 兄弟（同一親 window の子）であることは両 position の成立で担保。
        assert!(
            idx_slot < idx_surface,
            "text-layer slot は surface より上位 z（Children 先頭＝最上）でなければならない: \
             idx_slot={idx_slot} idx_surface={idx_surface}"
        );

        let name = world
            .get::<Name>(mount.text_slot())
            .expect("text-layer slot に Name が無い");
        assert_eq!(name.as_str(), "emo-text-layer-slot");

        // 予約スロットは Visual のみ・内容なし（emo 自前 brush の VisualGraphics を持たない）。
        assert!(
            world.get::<Visual>(mount.text_slot()).is_some(),
            "text-layer slot は Visual を持つ"
        );
        assert!(
            world.get::<VisualGraphics>(mount.text_slot()).is_none(),
            "予約スロットは内容（VisualGraphics/brush）を持たない seam であること"
        );
    }

    /// R3.3: 非表示切替で surface entity の `HitTest` が `None`（当たり判定停止）＋ `Visual` 不可視へ、
    /// 再表示で `AlphaMask`（α判定）＋可視へ戻ること。swap chain/キャッシュは呼び手保持ゆえ触れない。
    #[test]
    fn hide_toggle_switches_hittest_and_visibility() {
        let (mut world, _window, mount, _g) = attach_fixture(4, 4);

        // 初期状態: 可視 ＋ αマスクヒットテスト。
        assert!(
            world
                .get::<Visual>(mount.surface_entity())
                .unwrap()
                .is_visible,
            "装着直後は可視"
        );
        assert_eq!(
            world.get::<HitTest>(mount.surface_entity()).unwrap().mode,
            HitTestMode::AlphaMask,
            "装着直後は αマスクヒットテスト"
        );

        // 非表示へ。
        mount.set_visible(&mut world, false);
        assert!(
            !world
                .get::<Visual>(mount.surface_entity())
                .unwrap()
                .is_visible,
            "非表示後は Visual 不可視"
        );
        assert_eq!(
            world.get::<HitTest>(mount.surface_entity()).unwrap().mode,
            HitTestMode::None,
            "非表示後は当たり判定停止（HitTest::none）"
        );

        // 再表示へ。
        mount.set_visible(&mut world, true);
        assert!(
            world
                .get::<Visual>(mount.surface_entity())
                .unwrap()
                .is_visible,
            "再表示後は可視へ復帰"
        );
        assert_eq!(
            world.get::<HitTest>(mount.surface_entity()).unwrap().mode,
            HitTestMode::AlphaMask,
            "再表示後は αマスクヒットテストへ復帰"
        );
    }

    /// R1.6: attach 時・resize 時とも surface entity の `Arrangement`（→bounds＝αマスク座標基準）が
    /// 物理 px 外形（原点 0,0・寸＝given）へ直接設定されること（`BoxStyle`/taffy 非経由）。
    #[test]
    fn arrangement_bounds_are_physical_size_on_attach_and_resize() {
        let (mut world, _window, mount, _g) = attach_fixture(3, 2);

        let arr = world
            .get::<Arrangement>(mount.surface_entity())
            .expect("surface entity に Arrangement が無い");
        assert_eq!(
            (arr.size.width, arr.size.height),
            (3.0, 2.0),
            "装着時 Arrangement 寸は物理 px 外形"
        );
        assert_eq!(
            (arr.offset.x, arr.offset.y),
            (0.0, 0.0),
            "原点は窓クライアント 0,0"
        );

        // 外形変更（resize 経路）。
        mount.set_bounds(&mut world, (7, 5));
        let arr = world
            .get::<Arrangement>(mount.surface_entity())
            .expect("resize 後も Arrangement を持つ");
        assert_eq!(
            (arr.size.width, arr.size.height),
            (7.0, 5.0),
            "resize 後 Arrangement 寸が新外形を反映"
        );
        assert_eq!(
            (arr.offset.x, arr.offset.y),
            (0.0, 0.0),
            "resize 後も原点 0,0"
        );
    }
}
