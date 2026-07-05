//! `BlitOp` / `PlanBuilder`: 正規化定義からバックエンド非依存の合成命令列を導出する。
//!
//! element レイヤ順・有効 bind 集合の `animation-sort`→animation ID 順の2段合成規則を確定し、
//! 各命令へ `AtlasTable` 解決結果（`ElementId`・`Placement`）と変換行列を含める。入れ子 surface
//! 参照はオフセット累積で再帰的に inline 展開（flatten）し、visited 集合で循環を検出する。
//! キャンバス外形を算出し、同一入力に対して決定的な命令列を生成する。
//!
//! # 本 task（5.1）の範囲＝element レイヤ層の基礎のみ
//!
//! 本 module は plan の5サブ task（5.1〜5.5）の礎石であり、本 task は **静的 element 層の
//! 列挙のみ**を実装する。以下は明示的なシーム（後続 task が本 module を拡張して埋める）:
//!
//! - **bind（`BindSet`）層の列挙＋`animation-sort`→ID 順の2段規則** → task 5.2
//! - **入れ子 surface 参照の flatten＋循環検出（`visited`）** → task 5.3
//! - **placement None スキップ＋静的キャンバス外形算出（`Extent`）** → task 5.4
//! - **描画可能命令ゼロの分類（`SurfaceNotFound`/`EmptyComposition`）** → task 5.5
//!
//! 本 task はこれらを stub で偽装せず、`push_static_element_ops` が生む element 命令列は
//! すべて実挙動・決定的・テスト済みである。design 署名 `build_plan`（out_ops/visited/binds/
//! Extent）は後続 task が本 module を wrap する形で導入する。

use areka_emo_atlas::ElementId;
use bevy_ecs::entity::Entity;

use crate::method::ComposeMethod;
use crate::normalized::{SurfaceMaster, Transform};
use crate::world::{AtlasBinding, EmoWorld, SurfaceIndex};

/// バックエンド非依存の転写命令（これがバックエンド差替えシーム＝design 決定1）。
///
/// `element` はアトラス参照（束縛時に `Some(ElementId)` へ解決済みのもののみ命令化される）、
/// `transform` は flatten 済み最終配置（M1 は平行移動のみ）、`method` は M1 では常に
/// [`ComposeMethod::Overlay`]。`Placement` の実引きは blit 実行時（task 6）に `AtlasTable::entry`
/// で O(1) で行うため、本命令は `ElementId` のみを保持する（design Contracts）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlitOp {
    /// アトラス参照（束縛済み `ElementId`・placement の実引きは blit 時）。
    pub element: ElementId,
    /// flatten 済み最終配置（M1 は平行移動のみ）。
    pub transform: Transform,
    /// 合成メソッド（M1 は常に [`ComposeMethod::Overlay`]）。
    pub method: ComposeMethod,
}

/// 静的 element 層の転写命令を `out_ops` へ layer 昇順（同 layer は登場順）で積む（要件 4.1/4.5）。
///
/// 指定 surface の [`SurfaceMaster`] と平行な [`AtlasBinding`] を World から読み、element を
/// **layer 昇順・同 layer は登場順**（安定ソート）で列挙する。束縛が `Some(ElementId)` の
/// element のみ [`BlitOp`] を積み、`None`（未解決・bind 時に warn 済み）の element はスキップ
/// する（要件 4.3・6.3 の element 側前段）。element の [`Transform`]（転記 x,y の行列表現・
/// 要件 4.2）と [`ComposeMethod`] をそのまま命令へ伝播する。
///
/// `out_ops` は呼び手のスクラッチ Vec を再利用する意図（要件 10.3・完全な再利用形は
/// task 7 で実現）ゆえ、本関数は **追記のみ**（clear しない）を行う。element 数ぶんの命令を
/// 末尾へ push する。surface が存在しない場合・`AtlasBinding` 未挿入の場合は何も積まない
/// （後続 task が `SurfaceNotFound`/未束縛の分類を担う）。
///
/// # 決定性（要件 4.5/10.1）
///
/// 同一の World／surface に対して、本関数は毎回同一の命令列を append する。layer 昇順の
/// **安定ソート**により同 layer element の登場順が保たれ、束縛（index 平行）も決定的ゆえ、
/// 生成される `Vec<BlitOp>` はバイト等価になる。
///
/// # シーム（後続 task）
///
/// bind 層（5.2）・入れ子 flatten（5.3）・placement None スキップと外形（5.4）・zero-op 分類
/// （5.5）は本関数を **拡張**して埋める。本関数は静的 element 経路のみを担い、偽の結果を
/// 返さない。
///
/// 本 task（5.1）では非テストの lib 経路からの呼び出し口（`build_plan` ファサード）が
/// 未導入ゆえ `dead_code` になる。消費は task 5.2 の `build_plan` が本関数を wrap して行う
/// （それまで意図的な未使用シーム）。
#[allow(dead_code)]
pub(crate) fn push_static_element_ops(out_ops: &mut Vec<BlitOp>, world: &EmoWorld, surface_id: u32) {
    let Some((master, binding)) = surface_and_binding(world, surface_id) else {
        // surface 不在・または binding 未挿入。本 task では静的 element を積まない（後続 task が
        // SurfaceNotFound/未束縛の分類を担当する）。
        return;
    };

    // layer 昇順・同 layer は登場順（安定ソート）。fold 段で既に整列済み（不変条件）だが、
    // plan は自 module で決定性を保証するため添字と layer を安定ソートし直す。
    let mut order: Vec<usize> = (0..master.elements.len()).collect();
    order.sort_by_key(|&i| master.elements[i].layer);

    for i in order {
        let element = &master.elements[i];
        // 束縛が Some の element のみ命令化する。None（未解決・bind 時に warn 済み）はスキップ。
        let Some(element_id) = binding.0.get(i).copied().flatten() else {
            tracing::trace!(
                target: "areka_emo_compose",
                surface_id,
                layer = element.layer,
                path = element.path.as_str(),
                "未束縛 element を静的命令からスキップ（bind 時 warn 済み）"
            );
            continue;
        };
        out_ops.push(BlitOp {
            element: element_id,
            transform: element.transform,
            method: element.method.clone(),
        });
    }
}

/// 指定 surface id の [`SurfaceMaster`] と [`AtlasBinding`] を同時に読む（本 module 内補助）。
///
/// `SurfaceIndex`（疎 id→Entity・O(1)）で entity を引き、`SurfaceMaster` と `AtlasBinding` を
/// 併せて借用する。いずれか（surface 不在・binding 未挿入）が欠けると `None`。既存の公開
/// `world()` アクセサ経由で読むため、公開 API を広げない。
///
/// `push_static_element_ops` 経由でのみ使うため、非テスト lib 経路が未結線の本 task では
/// `dead_code` になる（消費は task 5.2）。
#[allow(dead_code)]
fn surface_and_binding(
    world: &EmoWorld,
    surface_id: u32,
) -> Option<(&SurfaceMaster, &AtlasBinding)> {
    let w = world.world();
    let entity: Entity = *w.resource::<SurfaceIndex>().0.get(&surface_id)?;
    let master = w.get::<SurfaceMaster>(entity)?;
    let binding = w.get::<AtlasBinding>(entity)?;
    Some((master, binding))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::EmoWorld;
    use areka_emo_atlas::{
        AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
    };
    use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
    use std::path::Path;

    // ---- 合成モデルビルダ（task 4 テストと同一パターン・実 fixture パースは統合 task 8 送り）----

    /// element 1 本（layer/path/x/y 指定）。
    fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
        Element {
            layer,
            path: ElementPath::new(path.to_string()),
            x,
            y,
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

    /// tightly-packed 2×2 の premultiplied BGRA・不透明（α=255）画像スペック。
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

    /// `MemoryDecoder` へ base.join(rel) の実パスで不透明画像を登録する。
    fn register(dec: &mut MemoryDecoder, base: &Path, rel: &str) {
        let (w, h, stride, bgra, has_alpha) = opaque_2x2();
        dec.insert(base.join(rel), w, h, stride, bgra, has_alpha);
    }

    /// 指定 rel_path 群を含む単一 SurfaceSet を bake し `AtlasTable` を得る（COM/WIC 非依存・SetId(0)）。
    fn bake_atlas(base: &Path, rels: &[&str]) -> AtlasTable {
        let elements: Vec<Element> = rels.iter().map(|r| elem(0, r, 0, 0)).collect();
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

    /// element を、その `path` の ElementId から逆引きするための対応表を作る（テスト補助）。
    ///
    /// BlitOp は ElementId のみを保持するため、命令列の順序を element の識別子（ここでは path）
    /// へ写像して検証する。bake 済み atlas は path→ElementId が一意ゆえ、ElementId→path も一意。
    fn id_to_path<'a>(atlas: &AtlasTable, rels: &[&'a str]) -> Vec<(ElementId, &'a str)> {
        rels.iter()
            .map(|r| {
                let id = atlas
                    .resolve(SetId(0), r)
                    .unwrap_or_else(|| panic!("{r} は bake 済みで resolve できる"));
                (id, *r)
            })
            .collect()
    }

    /// テスト①（受入基準・要件 4.1）: layer [2,0,1]（登場順）→ ops は layer 昇順 0,1,2。
    ///
    /// 各 op を ElementId 経由で element path へ逆写像し、命令列が layer 昇順に整列することを
    /// 検証する（登場順は 2→0→1 だが、layer 昇順で 0(b)→1(c)→2(a)）。
    #[test]
    fn ops_ordered_by_layer_ascending() {
        let base = Path::new("shell/master");
        let rels = ["a.png", "b.png", "c.png"];
        let atlas = bake_atlas(base, &rels);
        let map = id_to_path(&atlas, &rels);

        // 登場順 [layer2=a, layer0=b, layer1=c]。
        let shell = shell_of(vec![surface(
            1000,
            vec![
                elem(2, "a.png", 0, 0),
                elem(0, "b.png", 0, 0),
                elem(1, "c.png", 0, 0),
            ],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, 1000);

        let paths: Vec<&str> = ops
            .iter()
            .map(|op| {
                map.iter()
                    .find(|(id, _)| *id == op.element)
                    .map(|(_, p)| *p)
                    .expect("op の ElementId は既知")
            })
            .collect();
        // layer 昇順: b(0) → c(1) → a(2)。
        assert_eq!(paths, vec!["b.png", "c.png", "a.png"]);
    }

    /// テスト②（要件 4.1）: 同一 layer は登場（定義）順を保つ。
    #[test]
    fn same_layer_keeps_appearance_order() {
        let base = Path::new("shell/master");
        let rels = ["first.png", "second.png"];
        let atlas = bake_atlas(base, &rels);
        let map = id_to_path(&atlas, &rels);

        // 両者 layer=5。登場順は first → second。
        let shell = shell_of(vec![surface(
            1,
            vec![elem(5, "first.png", 0, 0), elem(5, "second.png", 0, 0)],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, 1);

        let paths: Vec<&str> = ops
            .iter()
            .map(|op| map.iter().find(|(id, _)| *id == op.element).unwrap().1)
            .collect();
        assert_eq!(paths, vec!["first.png", "second.png"]);
    }

    /// テスト③（要件 4.5/10.1）: 同一 World／surface から2回導出→命令列がバイト等価。
    #[test]
    fn derivation_is_deterministic() {
        let base = Path::new("shell/master");
        let rels = ["a.png", "b.png", "c.png"];
        let atlas = bake_atlas(base, &rels);

        let shell = shell_of(vec![surface(
            1000,
            vec![
                elem(2, "a.png", 1, 2),
                elem(0, "b.png", 3, 4),
                elem(1, "c.png", 5, 6),
            ],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops1 = Vec::new();
        push_static_element_ops(&mut ops1, &world, 1000);
        let mut ops2 = Vec::new();
        push_static_element_ops(&mut ops2, &world, 1000);

        assert_eq!(ops1, ops2, "同一入力→同一 ops（バイト等価）");
        assert_eq!(ops1.len(), 3);
    }

    /// テスト④（要件 4.3/6.3 前段）: 未束縛（None）element は命令化されずスキップされる（非パニック）。
    #[test]
    fn unresolved_binding_is_skipped() {
        let base = Path::new("shell/master");
        // atlas には known.png のみ焼く（bogus.png は未束縛＝None になる）。
        let atlas = bake_atlas(base, &["known.png"]);
        let known_id = atlas.resolve(SetId(0), "known.png").expect("known 解決");

        let shell = shell_of(vec![surface(
            1000,
            vec![elem(0, "known.png", 0, 0), elem(1, "bogus.png", 0, 0)],
        )]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, 1000);

        // bogus.png は None ゆえスキップ＝命令は known.png の1本のみ。
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].element, known_id);
    }

    /// テスト⑤（要件 4.2）: 命令の Transform は element の translate(x,y) と一致し、純平行移動である。
    #[test]
    fn transform_propagates_as_translation() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["p.png"]);

        let shell = shell_of(vec![surface(7, vec![elem(0, "p.png", 12, -8)])]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, 7);

        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].transform, Transform::translate(12, -8));
        assert_eq!(ops[0].transform.offset(), (12, -8));
        assert!(
            ops[0].transform.is_translation(),
            "M1 は単位行列＋平行移動（要件 4.2）"
        );
        assert_eq!(ops[0].method, ComposeMethod::Overlay);
    }

    /// 追記形の確認: 既存 ops を clear せず末尾追記する（スクラッチ再利用意図・要件 10.3）。
    #[test]
    fn appends_without_clearing() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["p.png"]);
        let shell = shell_of(vec![surface(1, vec![elem(0, "p.png", 0, 0)])]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let sentinel = BlitOp {
            element: ElementId(u32::MAX),
            transform: Transform::identity(),
            method: ComposeMethod::Overlay,
        };
        let mut ops = vec![sentinel.clone()];
        push_static_element_ops(&mut ops, &world, 1);

        // 先頭 sentinel が残り、末尾へ element 命令が追記される。
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], sentinel);
    }

    /// surface 不在では何も積まない（後続 task が SurfaceNotFound 分類を担う・本 task は非追記）。
    #[test]
    fn missing_surface_pushes_nothing() {
        let world = EmoWorld::build(&shell_of(Vec::new()));
        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, 9999);
        assert!(ops.is_empty());
    }
}
