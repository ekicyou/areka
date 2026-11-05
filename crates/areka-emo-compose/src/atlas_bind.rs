//! `AtlasBinder`: 正規化定義の `ElementPath` を `AtlasTable` の `ElementId` へ一度きり解決する。
//!
//! 構築時（load-time・ゴーストごと1回）に `ElementPath`→`ElementId` を resolve し、結果を
//! `AtlasBinding` コンポーネントとして World へ挿入する。以後の毎フレーム参照は O(1) の `entry`
//! 引きに帰着する。解決不能なパスはパニックせず `debug` で残す（脱落は bake の段の error が知らせる）。

use areka_emo_atlas::{AtlasTable, ElementId, SetId};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use crate::nesting::ElementKind;
use crate::normalized::SurfaceMaster;
use crate::world::AtlasBinding;

/// 全 surface entity の `SurfaceMaster.elements` を `AtlasTable::resolve` で一度きり
/// `ElementId` へ束縛し、平行な `AtlasBinding` を entity へ挿入する（要件 4.3）。
///
/// resolve は本関数（構築時）でのみ呼ぶ（design Postconditions「束縛後の compose 経路に
/// resolve 呼び出しが存在しない」）。`ElementPath.as_str()` は無改変で `AtlasTable` の
/// `rel_path` キーと同一規約ゆえ、bake 済みの既知パスは `Some(ElementId)`・未知パスは
/// `None`＋`debug` になる（design Risks）。パニックしない。画像でない element
/// （[`ElementKind`] が `Image` 以外）は引かずに `None` を置き、警告も出さない。
///
/// `AtlasBinding.0` は `SurfaceMaster.elements` と同じ添字で並ぶ（index 対応の契約）。
pub(crate) fn bind_atlas(world: &mut World, atlas: &AtlasTable, set: SetId) {
    // 借用衝突回避のため、先に (entity, 束縛結果) を不変クエリで確定してから挿入する。
    let mut bindings: Vec<(Entity, AtlasBinding)> = Vec::new();
    let mut query = world.query::<(Entity, &SurfaceMaster)>();
    for (entity, master) in query.iter(world) {
        let ids: Vec<Option<ElementId>> = master
            .elements
            .iter()
            .map(|element| {
                // 画像でない element（サーフェスの番号・範囲を超える数）はアトラスから引かない。
                // 画像ではないので未束縛の警告も出さない（記録は読み込みの入口の役目・要件 1.5/3.5）。
                if element.kind != ElementKind::Image {
                    return None;
                }
                let rel_path = element.path.as_str();
                match atlas.resolve(set, rel_path) {
                    Some(id) => Some(id),
                    None => {
                        // 未知のパスは bake で脱落した絵だけで、読み込みの入口が error で 1 度出している（1 誤り 1 行）。
                        tracing::debug!(
                            target: "areka_emo_compose",
                            set = set.0,
                            surface_id = master.id,
                            path = rel_path,
                            "atlas 未束縛 element: resolve 失敗（None として記録）"
                        );
                        None
                    }
                }
            })
            .collect();
        bindings.push((entity, AtlasBinding(ids)));
    }

    for (entity, binding) in bindings {
        world.entity_mut(entity).insert(binding);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_test_support::{bake_atlas, elem, shell_of, surface};
    use crate::world::{AtlasBinding, EmoWorld};
    use std::path::Path;

    /// テスト①（受入基準）: 既知パスは `Some(ElementId)`・未知パスは `None`（debug 済み）に束縛される。
    ///
    /// KNOWN パスと BOGUS パスの 2 element を持つ surface を fold し、KNOWN のみ bake した
    /// `AtlasTable` に対し `bind_atlas` を実行。`AtlasBinding` の既知 index が `Some`・未知 index が
    /// `None` であることを検証する（要件 4.3・design AtlasBinder）。
    #[test]
    fn known_resolves_some_unknown_none() {
        let base = Path::new("shell/master");
        // atlas には known.png のみを焼く（bogus.png は未束縛になる）。
        let atlas = bake_atlas(base, &["known.png"]);
        let expected_id = atlas
            .resolve(SetId(0), "known.png")
            .expect("known.png は bake 済みで resolve できる");

        // surface の element は [known.png(layer0), bogus.png(layer1)]（layer 昇順で保たれる）。
        let shell = shell_of(vec![surface(
            1000,
            vec![elem(0, "known.png", 0, 0), elem(1, "bogus.png", 0, 0)],
        )]);
        let mut world = EmoWorld::build(&shell);

        world.bind_atlas(&atlas, SetId(0));

        let master = world.surface(1000).expect("surface1000 常駐");
        let binding = surface_binding(&world, 1000);
        // 既知 index（known.png）→ Some(expected_id)。
        let known_idx = master
            .elements
            .iter()
            .position(|e| e.path.as_str() == "known.png")
            .expect("known.png element がある");
        assert_eq!(binding.0[known_idx], Some(expected_id));
        // 未知 index（bogus.png）→ None。
        let bogus_idx = master
            .elements
            .iter()
            .position(|e| e.path.as_str() == "bogus.png")
            .expect("bogus.png element がある");
        assert_eq!(binding.0[bogus_idx], None);
    }

    /// テスト②: 束縛は `SurfaceMaster.elements` と index 対応で平行（長さ一致・全 surface）。
    #[test]
    fn binding_parallel_to_elements_len_matches() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["a.png", "b.png", "c.png"]);

        // 2 surface（要素数の異なる）を fold し、各 surface で長さ一致を確かめる。
        let shell = shell_of(vec![
            surface(0, vec![elem(0, "a.png", 0, 0), elem(1, "b.png", 0, 0)]),
            surface(1, vec![elem(0, "c.png", 0, 0)]),
        ]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        for id in world.surface_ids().collect::<Vec<_>>() {
            let master = world.surface(id).expect("常駐 surface");
            let binding = surface_binding(&world, id);
            assert_eq!(
                binding.0.len(),
                master.elements.len(),
                "surface {id} の AtlasBinding は elements と平行（同長）",
            );
            // 全て bake 済みゆえ Some（index 対応の実挙動確認）。
            for (i, e) in master.elements.iter().enumerate() {
                let resolved = atlas.resolve(SetId(0), e.path.as_str());
                assert!(
                    resolved.is_some(),
                    "index {i}（{}）は焼いてある",
                    e.path.as_str()
                );
                assert_eq!(
                    binding.0[i],
                    resolved,
                    "index {i}（{}）が resolve と一致",
                    e.path.as_str(),
                );
            }
        }
    }

    /// テスト③: 全パス未知でもパニックせず全 `None`（非パニック・debug のみ）。
    #[test]
    fn all_unknown_paths_yield_all_none_without_panic() {
        let base = Path::new("shell/master");
        // atlas は別パスのみ焼く（surface の element は一切 bake されない）。
        let atlas = bake_atlas(base, &["elsewhere.png"]);

        let shell = shell_of(vec![surface(
            1,
            vec![
                elem(0, "x.png", 0, 0),
                elem(1, "y.png", 0, 0),
                elem(2, "z.png", 0, 0),
            ],
        )]);
        let mut world = EmoWorld::build(&shell);
        // パニックしないこと（呼び出しが返ること自体が保証）。
        world.bind_atlas(&atlas, SetId(0));

        let binding = surface_binding(&world, 1);
        assert_eq!(binding.0.len(), 3);
        assert!(binding.0.iter().all(|b| b.is_none()), "全 element が None");
    }

    /// テスト④: 空 World（surface ゼロ）でも `bind_atlas` はパニックせず何もしない。
    #[test]
    fn empty_world_bind_is_noop() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["only.png"]);
        let mut world = EmoWorld::build(&shell_of(Vec::new()));
        world.bind_atlas(&atlas, SetId(0));
        assert_eq!(world.surface_ids().count(), 0);
    }

    /// テスト⑤: 別 SetId で bind すると（set がキーの一部ゆえ）既知パスも `None` になる。
    ///
    /// bake は SetId(0) に焼く。SetId(1) で bind すると同名 rel_path でも resolve が None を返す
    /// （atlas 契約 `resolve` は set スコープ）。set 取り違えが未束縛として観測される実証。
    #[test]
    fn wrong_set_id_resolves_none() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["known.png"]);
        let shell = shell_of(vec![surface(0, vec![elem(0, "known.png", 0, 0)])]);
        let mut world = EmoWorld::build(&shell);

        world.bind_atlas(&atlas, SetId(1)); // 焼いたのは SetId(0)。

        let binding = surface_binding(&world, 0);
        assert_eq!(binding.0, vec![None], "別 set は resolve が None");
    }

    /// 指定 surface id の `AtlasBinding` component を World から引く（テスト補助）。
    ///
    /// `SurfaceIndex`（疎 id→Entity・O(1)）で entity を引き、その `AtlasBinding` を複製する。
    fn surface_binding(world: &EmoWorld, id: u32) -> AtlasBinding {
        let w = world.world();
        let entity = *w
            .resource::<crate::world::SurfaceIndex>()
            .0
            .get(&id)
            .unwrap_or_else(|| panic!("surface {id} entity がある"));
        w.get::<AtlasBinding>(entity)
            .unwrap_or_else(|| panic!("surface {id} に AtlasBinding が挿入されている"))
            .clone()
    }
}
