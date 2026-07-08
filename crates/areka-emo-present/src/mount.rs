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
//!    （`Name("emo-text-layer-slot")` ＋ `Visual` のみ・内容なし）。M1 の独立レイヤ描画／M2 の
//!    合成パス内レイヤ化の双方を「この entity の差し替え」で吸収する seam（emo-text-layer が消費）。
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

use windows::UI::Composition::{Compositor, ICompositionSurface, SpriteVisual, Visual as WucVisual};
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
    /// 予約済み text 層スロット（surface の兄弟・上位 z・内容なし）。emo-text-layer が消費する。
    text_slot: Entity,
}

impl VisualMount {
    /// 窓 `window` の子として surface entity と text-layer slot を装着する。
    ///
    /// `surface` は [`crate::chain::SwapChainPresenter::new`] が返す `ICompositionSurface`、
    /// `size` は供給面の物理 px 外形。SpriteVisual へ emo 自前の `SurfaceBrush` を装着し
    /// （`SetBrush`/`SetSize`）、`Arrangement` を物理 px で直接設定する。
    ///
    /// z 順（描画）: text-layer slot を surface entity **より先**に子へ追加し、`Children` 先頭
    /// ＝最上（surface の上に描画）とする。
    pub(crate) fn attach(
        world: &mut World,
        window: Entity,
        surface: &ICompositionSurface,
        compositor: &Compositor,
        size: (u32, u32),
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
                Visual::default(),
                ChildOf(window),
            ))
            .id();

        // 3. surface entity（SpriteVisual ＋ 物理 px bounds ＋ αマスクヒットテスト）。
        let surface_entity = world
            .spawn((
                Name::new("emo-surface"),
                Visual::default(),
                vg,
                physical_arrangement(size),
                HitTest::alpha_mask(),
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

    /// 非表示／再表示の切替（R3.3）。
    ///
    /// `false`: `Visual::set_visible(false)` ＋ `HitTest::none()`（当たり判定停止）。swap chain・キャッシュは
    /// 呼び手が保持するため再表示は Present 不要で復帰する。`true`: 可視 ＋ `HitTest::alpha_mask()` へ戻す。
    /// 窓自体の show/hide は所有しない（placement/ghost 領分）。
    pub(crate) fn set_visible(&self, world: &mut World, visible: bool) {
        if let Some(mut v) = world.get_mut::<Visual>(self.surface_entity) {
            v.set_visible(visible);
        } else {
            tracing::warn!(
                entity = ?self.surface_entity,
                "set_visible: surface entity に Visual が無い（装着が未完了）"
            );
        }

        let mode = if visible {
            HitTest::alpha_mask()
        } else {
            HitTest::none()
        };
        if let Some(mut ht) = world.get_mut::<HitTest>(self.surface_entity) {
            *ht = mode;
        } else {
            tracing::warn!(
                entity = ?self.surface_entity,
                "set_visible: surface entity に HitTest が無い（装着が未完了）"
            );
        }
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
mod tests {
    use super::*;

    use bevy_ecs::hierarchy::Children;
    use wintf::com::wuc::create_dispatcher_queue_controller;
    use wintf::ecs::{GraphicsCore, HitTestMode};
    use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};

    use crate::chain::SwapChainPresenter;

    /// テスト用 WUC apartment / dispatcher（chain.rs / spike と同一方針）。
    ///
    /// cargo test の各テストは専用スレッドで COM 未初期化ゆえ ASTA を第一候補・NONE を保険にする。
    /// controller は Compositor より長寿命を要するため呼び出し側で保持する。
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

    /// 生存させておくべき WUC/GPU リソース群（drop 順の都合でまとめて保持する）。
    ///
    /// `ICompositionSurface` はスワップチェーンを内包する `SwapChainPresenter` に裏打ちされるため、
    /// 装着後も presenter を保持する（構造アサートには描画不要だが破棄を防ぐ）。
    #[allow(dead_code)]
    struct Guards {
        dq: windows::System::DispatcherQueueController,
        compositor: Compositor,
        core: GraphicsCore,
        presenter: SwapChainPresenter,
    }

    /// `w×h` の供給面を持つ窓へ `VisualMount::attach` した状態を組む共通フィクスチャ。
    ///
    /// 返り値: (world, window entity, mount, 生存ガード)。window は実 `Window` ではない素の entity
    /// （純 ECS 構造アサートのため。owner Window 不在でも surface/slot の構造は成立する）。
    fn attach_fixture(w: u32, h: u32) -> (World, Entity, VisualMount, Guards) {
        let (dq, compositor) = make_dispatcher_and_compositor();
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let (presenter, surface) = SwapChainPresenter::new(&core, &compositor, w, h)
            .expect("SwapChainPresenter::new 失敗");

        let mut world = World::new();
        let window = world.spawn_empty().id();
        let mount = VisualMount::attach(&mut world, window, &surface, &compositor, (w, h))
            .expect("VisualMount::attach 失敗");

        (
            world,
            window,
            mount,
            Guards {
                dq,
                compositor,
                core,
                presenter,
            },
        )
    }

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
