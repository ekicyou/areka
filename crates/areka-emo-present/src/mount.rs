//! `VisualMount`（mount.rs）: 窓 Entity への最小 visual 構成の装着・text 層スロット予約・非表示切替。
//!
//! 提示段の**配置層**である。surface entity を wintf の `Visual::on_add` に乗る形で spawn し、以後は
//! `Arrangement`（**論理寸＝原寸・`scale`＝拡大率 k**）を書く。計画の節は `DrawNodes` が原寸の
//! まとめの節の下へ写し、面の生成・寸合わせ・変換・描画は wintf（`deferred_surface_creation_system`／
//! `render_surface`）、当たり判定マスクは hit-test が担い、本型は「visual 構成・z 順・配置・
//! 非表示切替」を UI スレッド上で受け持つ。**COM 呼び出しは 0**（純 ECS・失敗経路なし）。
//!
//! # 構成（窓あたり）
//!
//! 1. **surface entity**: `Visual` ＋ [`logical_arrangement`] ＋ [`OwnSurface`] ＋
//!    [`HitTest::alpha_mask`] ＋ [`AlphaMaskResource`]。根の命令の列を持たない描画面の持ち主で、
//!    窓の client の原点に置く。`GlobalArrangement.bounds`（＝原寸 × k・物理寸）が AlphaMask 座標の
//!    基準になり、wintf の `alpha_mask_hit` が境界に対する比例写像で ÷k を 1 回だけ掛けて原寸マスクを読む。
//!    根の命令の列を持たないのは、根の命令の列と描画の木の両持ちが変化のたびに全部を描かせるため。
//! 2. **原寸のまとめの節**（surface entity の唯一の子）: `VisualGroup { SOURCE_OVER }` ＋ 原寸の
//!    `Arrangement`。描画面の描き直しが k 倍で再生するので、中は原寸で描かれ、閉じるとき k 倍で
//!    重なる（原寸で合成してから拡大＝k が整数でないときも部品の境に継ぎ目を出さない）。まとめの中は
//!    自動で上げられないので、画面の絵は surface entity の入れ物 1 枚にそろう。計画の節は全部この下。
//! 3. **text-layer slot**: surface entity の**兄弟・上位 z**（描画で上）に置く空 entity
//!    （`Name("emo-text-layer-slot")` ＋ `Visual` ＋ `HitTest`・内容（brush）なし）。独立レイヤ描画／
//!    α 後の合成パス内レイヤ化の双方を「この entity の差し替え」で吸収する seam
//!    （emo-text-layer が消費）。`HitTest` を明示付与するのは非表示時にポインタ透過させるため
//!    （未付与は既定 `Bounds` 扱いで、`Bounds` の合成 α は `is_visible` を見ない）。
//!
//! # 非表示の契約（両 entity）
//!
//! 「バルーンを非表示にする」は**枠の面と文字層の双方が見えず・触れない**ことを意味する。
//! [`VisualMount::set_visible`] は両 entity の
//! `Visual` と `HitTest` を同時に切り替え、[`VisualMount::attach`] の `initially_visible=false` は
//! 両 entity を最初から不可視で spawn する（可視状態を一度も経由しない＝同）。
//!
//! # wintf `Visual::on_add` に乗る
//!
//! wintf の `Visual` の `on_add` フック（`graphics/visual.rs` `on_visual_add`）は owner Window 配下でのみ
//! `VisualGraphics`／`SurfaceGraphics`／`SurfaceGraphicsDirty` を、常に `BrushInherit` を連鎖挿入する。
//! 本型は surface entity へ **`VisualGraphics` を入れない**——既定値が連鎖挿入され、
//! `visual_resource_management_system` が SpriteVisual を、`deferred_surface_creation_system` が
//! `GlobalArrangement` から面・brush・`SetSize` を、`render_surface` が
//! `SetTransform(GlobalArrangement のスケール)` → 描画の木（まとめの節とその下の節）の再生を担う。
//!
//! # z 順（描画順）の典拠
//!
//! 兄弟の z 順は `Children` の順序が権威（`visual_hierarchy_sync_system`）。同システムは `Children` を
//! 前方反復し毎回 `InsertAtBottom` を呼ぶため、**先頭の子ほど最上（描画で前面）** になる。よって
//! text-layer slot を surface entity **より先**に子として追加し、slot を上位 z（surface の上に描画）に置く。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;

use areka_emo_compose::ScaleRatio;
use wintf::ecs::{
    AlphaMaskResource, Arrangement, HitTest, LayoutScale, MixMode, Offset, OwnSurface, Size,
    Visual, VisualGroup,
};

use crate::draw_nodes::place;

/// 原寸 `native` と拡大率 `k` から surface entity 用 [`Arrangement`] を作る
/// （**原点 0・寸＝原寸（論理 px）・スケール＝k の係数**）。
///
/// `size` は表示中エントリの原寸、`scale` は実適用 k。wintf が `GlobalArrangement.bounds`＝`原寸 × k`
/// （物理・f32）を導き、`render_surface` が `SetTransform` で同じ係数を描画に掛ける。`k.as_f32()` は
/// 変換行列の係数であって寸法演算ではない（`ScaleRatio::as_f32` の裁定済み消費者）。物理寸の照会値は
/// `ScaleRatio::scaled_extent`（丸め権威）が返す。
///
/// # 不変条件: `offset` は 0 でなければならない
///
/// wintf は `visual_property_sync_system` が `offset × 自 entity の累積スケール（k 込み）` を WUC へ
/// 書き、`impl Mul<Arrangement> for GlobalArrangement` は `offset × 親スケール（1.0）` で bounds を
/// 出す。両者は **offset が 0 のときだけ一致する**——0 以外を置くと、描画位置と当たり判定の境界が
/// k 倍だけ食い違う。本関数は常に 0 を書き、他の書き手は無い。
pub(crate) fn logical_arrangement(native: (u32, u32), k: ScaleRatio) -> Arrangement {
    Arrangement {
        offset: Offset { x: 0.0, y: 0.0 },
        scale: LayoutScale {
            x: k.as_f32(),
            y: k.as_f32(),
        },
        size: Size {
            width: native.0 as f32,
            height: native.1 as f32,
        },
    }
}

/// 窓 Entity へ装着した visual 構成（surface entity ＋ 原寸のまとめの節 ＋ text-layer slot）のハンドル。
///
/// `pub(crate)`（公開 API ではない）。`EmoPresenter` が target ごとに保持する。
pub(crate) struct VisualMount {
    /// `Visual` ＋ `Arrangement` ＋ `OwnSurface` ＋ `HitTest` ＋ `AlphaMaskResource` を持つ
    /// 表示 entity（窓の子）。
    surface_entity: Entity,
    /// 原寸のまとめの節（surface entity の唯一の子・計画の節の根）。
    group: Entity,
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
    /// 可視時に `bounds()` を**明示**付与するのは、不可視時に `none()` へ差し替えるため。component 未付与は
    /// wintf 側で既定 `HitTestMode::Bounds` と扱われ（`hit_test/mod.rs`）、`Bounds` 判定の合成 α は
    /// `Visual::clamped_opacity()`（`visual.rs`）で決まり `is_visible` を見ないため、`Visual` を不可視に
    /// するだけでは文字層のポインタ透過が成立しない。
    fn slot_hit_test(visible: bool) -> HitTest {
        if visible {
            HitTest::bounds()
        } else {
            HitTest::none()
        }
    }

    /// 1 つの entity へ可視性（`Visual`）とポインタ判定（`HitTest`）を同時に適用する。
    ///
    /// **現値と同値なら書かない**（`set_layout` と同じ守り）——書けば `Changed<Visual>`／
    /// `Changed<HitTest>` が立ち、wintf が毎コマ COM を呼ぶ。
    /// 未装着（component 不在）は装着契約の破綻ゆえ warn で記録する（沈黙する失敗経路を作らない）。
    fn apply_visibility(
        world: &mut World,
        entity: Entity,
        visible: bool,
        mode: HitTest,
        role: &'static str,
    ) {
        if let Some(mut v) = world.get_mut::<Visual>(entity) {
            if v.is_visible != visible {
                v.set_visible(visible);
            }
        } else {
            tracing::warn!(?entity, role, "set_visible: Visual が無い（装着が未完了）");
        }

        if let Some(mut ht) = world.get_mut::<HitTest>(entity) {
            ht.set_if_neq(mode);
        } else {
            tracing::warn!(
                ?entity,
                role,
                "set_visible: HitTest が無い（装着が未完了・ポインタ透過が成立しない）"
            );
        }
    }

    /// 窓 `window` の子として surface entity・原寸のまとめの節・text-layer slot を装着する
    /// （純 ECS・失敗経路なし）。
    ///
    /// `native` は表示中エントリの原寸、`k` は実適用の拡大率（[`logical_arrangement`] へ渡す）。
    /// `VisualGraphics` は入れない——`Visual::on_add` の連鎖挿入に委ねる。
    ///
    /// `initially_visible` が `false` なら surface entity・text-layer slot の**双方**を最初から不可視
    /// （`Visual{is_visible:false}` ＋ `HitTest::none()`）で spawn する。可視で spawn してから消す経路を
    /// 持たないので、可視状態を一度も経由しない。
    pub(crate) fn attach(
        world: &mut World,
        window: Entity,
        native: (u32, u32),
        k: ScaleRatio,
        initially_visible: bool,
    ) -> Self {
        // text-layer slot を先に追加する: Children 先頭 ＝ 最上（描画で前面）。
        let text_slot = world
            .spawn((
                Name::new("emo-text-layer-slot"),
                Self::visual_for(initially_visible),
                Self::slot_hit_test(initially_visible),
                ChildOf(window),
            ))
            .id();

        let surface_entity = world
            .spawn((
                Name::new("emo-surface"),
                Self::visual_for(initially_visible),
                logical_arrangement(native, k),
                OwnSurface,
                Self::surface_hit_test(initially_visible),
                AlphaMaskResource::new(),
                ChildOf(window),
            ))
            .id();
        let group = world
            .spawn((
                Name::new("emo-surface-group"),
                Visual::default(),
                VisualGroup {
                    mix: MixMode::default(),
                },
                place(0, 0, native.0, native.1),
                HitTest::none(),
                ChildOf(surface_entity),
            ))
            .id();

        world.flush();

        Self {
            surface_entity,
            group,
            text_slot,
        }
    }

    /// surface entity の配置を原寸 `native`・拡大率 `k` へ、まとめの節の寸を原寸へ合わせる
    /// （[`logical_arrangement`]）。
    ///
    /// **現値と同値なら書かない**——変化のない適用で `Changed<Arrangement>` → `GlobalArrangement` →
    /// 面の寸合わせ・全面再描画の連鎖を毎コマ起こさないため。
    pub(crate) fn set_layout(&self, world: &mut World, native: (u32, u32), k: ScaleRatio) {
        for (entity, next) in [
            (self.surface_entity, logical_arrangement(native, k)),
            (self.group, place(0, 0, native.0, native.1)),
        ] {
            match world.get_mut::<Arrangement>(entity) {
                Some(mut arr) => {
                    arr.set_if_neq(next);
                }
                None => tracing::warn!(
                    ?entity,
                    "set_layout: 装着の実体に Arrangement が無い（装着が未完了）"
                ),
            }
        }
    }

    /// 非表示／再表示の切替。
    ///
    /// **surface entity と text-layer slot の双方**へ同時に及ぶ。`false`: 両者の
    /// `Visual::set_visible(false)` ＋ `HitTest::none()`。`true`: 両者を可視へ戻し、surface は
    /// `HitTest::alpha_mask()`、slot は `HitTest::bounds()` へ復帰する。
    ///
    /// 枠の面だけを不可視にすると文字層が画面に残り、さらにスロットは既定 `Bounds` 判定のまま
    /// ポインタを受け続けるため、両 entity を同時に切り替える。
    ///
    /// 節・キャッシュは呼び手が保持するため再表示は再描画不要。窓自体の show/hide は所有しない
    /// （placement/ghost 領分）。
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

    /// 装着の子 2 つ（surface entity と text-layer slot）を窓から消す（シェル・バルーンの差し替えの片付け）。
    ///
    /// `self` を消費するので消した後のハンドルは残らない。`ChildOf` により窓の `Children` からも同時に
    /// 外れる。窓そのものは消さない（窓の寿命は placement/ghost 領分）。既に居ない entity は装着契約の
    /// 破綻ゆえ warn で記録する。
    pub(crate) fn despawn(self, world: &mut World) {
        for (entity, role) in [
            (self.text_slot, "text-layer-slot"),
            (self.surface_entity, "surface"),
        ] {
            if !world.despawn(entity) {
                tracing::warn!(?entity, role, "despawn: 装着の子が既に居ない");
            }
        }
    }

    /// `Arrangement`＋`OwnSurface`＋`HitTest`＋`AlphaMaskResource` を持つ表示 entity。
    pub(crate) fn surface_entity(&self) -> Entity {
        self.surface_entity
    }

    /// 原寸のまとめの節（計画の節の根）。
    pub(crate) fn group(&self) -> Entity {
        self.group
    }

    /// 予約済み text 層スロット（emo-text-layer が消費）。
    pub(crate) fn text_slot(&self) -> Entity {
        self.text_slot
    }
}

#[cfg(test)]
#[path = "mount_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "mount_visibility_tests.rs"]
mod visibility_tests;

#[cfg(test)]
#[path = "mount_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mount_extent_tests.rs"]
mod extent_tests;
