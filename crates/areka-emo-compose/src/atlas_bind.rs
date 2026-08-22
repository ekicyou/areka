//! `AtlasBinder`: 正規化定義の `ElementPath` を `AtlasTable` の `ElementId` へ一度きり解決する。
//!
//! 構築時（load-time・ゴーストごと1回）に `ElementPath`→`ElementId` を resolve し、結果を
//! `AtlasBinding` コンポーネントとして World へ挿入する。以後の毎フレーム参照は O(1) の `entry`
//! 引きに帰着する。解決不能なパスはパニックせず `warn` 以上で観測可能に扱う。

use areka_emo_atlas::{AtlasTable, ElementId, SetId};
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use crate::normalized::SurfaceMaster;
use crate::world::AtlasBinding;

/// 全 surface entity の `SurfaceMaster.elements` を `AtlasTable::resolve` で一度きり
/// `ElementId` へ束縛し、平行な `AtlasBinding` を entity へ挿入する（要件 4.3）。
///
/// resolve は本関数（構築時）でのみ呼ぶ（design Postconditions「束縛後の compose 経路に
/// resolve 呼び出しが存在しない」）。`ElementPath.as_str()` は無改変で `AtlasTable` の
/// `rel_path` キーと同一規約ゆえ、bake 済みの既知パスは `Some(ElementId)`・未知パスは
/// `None`＋`warn` になる（design Risks）。パニックしない。
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
                let rel_path = element.path.as_str();
                match atlas.resolve(set, rel_path) {
                    Some(id) => Some(id),
                    None => {
                        tracing::warn!(
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
    use crate::world::{AtlasBinding, EmoWorld};
    use areka_emo_atlas::{AlphaParams, UseSelfAlpha};
    use areka_emo_atlas::{MemoryDecoder, PackConfig, SurfaceSet, bake};
    use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
    use std::path::Path;

    // ---- 合成モデルビルダ（emo2 相当構造・実 fixture パースは統合 task 8 送り）----

    /// element 1 本（layer/path・x,y は 0 固定）。
    fn elem(layer: u32, path: &str) -> Element {
        Element {
            layer,
            path: ElementPath::new(path.to_string()),
            x: 0,
            y: 0,
        }
    }

    /// element のみを持つ最小 surface（collision/animation 空・単一ターゲット）。
    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    /// 与えた surface 群を登場順 definitions（plain のみ）で包む `Shell`。
    fn shell_of(surfaces: Vec<Surface>) -> Shell {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        }
    }

    /// tightly-packed 2×2 の premultiplied BGRA・不透明（α=255）画像スペックを返す。
    fn opaque_2x2() -> (u32, u32, u32, Vec<u8>, bool) {
        let (w, h) = (2u32, 2u32);
        let stride = w * 4;
        let bgra = vec![
            10, 20, 30, 255, //
            40, 50, 60, 255, //
            70, 80, 90, 255, //
            100, 110, 120, 255,
        ];
        (w, h, stride, bgra, true)
    }

    /// `MemoryDecoder` へ base_dir.join(rel_path) の実パスで不透明画像を登録する。
    fn register(dec: &mut MemoryDecoder, base: &Path, rel: &str) {
        let (w, h, stride, bgra, has_alpha) = opaque_2x2();
        dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
    }

    /// 指定 rel_path 群を含む単一 SurfaceSet を bake し `AtlasTable` を得る（COM/WIC 非依存）。
    ///
    /// bake は emo-atlas と同一の headless 経路（`MemoryDecoder`＋`bake`）を辿る。set は常に
    /// `SetId(0)`（単一セット）。
    fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
        let elements: Vec<Element> = rels.iter().map(|r| elem(0, r)).collect();
        let surfaces = vec![surface(0, elements)];
        let mut dec = MemoryDecoder::new();
        for r in rels {
            register(&mut dec, base, r);
        }
        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let result = bake(&[set], &dec, PackConfig::default());
        assert!(result.errors.is_empty(), "bake セットアップは失敗しない");
        result.table
    }

    /// テスト①（受入基準）: 既知パスは `Some(ElementId)`・未知パスは `None`（warn 済み）に束縛される。
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
            vec![elem(0, "known.png"), elem(1, "bogus.png")],
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
            surface(0, vec![elem(0, "a.png"), elem(1, "b.png")]),
            surface(1, vec![elem(0, "c.png")]),
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
                assert_eq!(
                    binding.0[i],
                    atlas.resolve(SetId(0), e.path.as_str()),
                    "index {i}（{}）が resolve と一致",
                    e.path.as_str(),
                );
            }
        }
    }

    /// テスト③: 全パス未知でもパニックせず全 `None`（非パニック・warn のみ）。
    #[test]
    fn all_unknown_paths_yield_all_none_without_panic() {
        let base = Path::new("shell/master");
        // atlas は別パスのみ焼く（surface の element は一切 bake されない）。
        let atlas = bake_atlas(base, &["elsewhere.png"]);

        let shell = shell_of(vec![surface(
            1,
            vec![elem(0, "x.png"), elem(1, "y.png"), elem(2, "z.png")],
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
        let shell = shell_of(vec![surface(0, vec![elem(0, "known.png")])]);
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
