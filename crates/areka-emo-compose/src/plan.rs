//! `BlitOp` / `PlanBuilder`: 正規化定義からバックエンド非依存の合成命令列を導出する。
//!
//! element レイヤ順・有効 bind 集合の `animation-sort`→animation ID 順の2段合成規則を確定し、
//! 各命令へ `AtlasTable` 解決結果（`ElementId`・`Placement`）と変換行列を含める。入れ子 surface
//! 参照はオフセット累積で再帰的に inline 展開（flatten）し、visited 集合で循環を検出する。
//! キャンバス外形を算出し、同一入力に対して決定的な命令列を生成する。
//!
//! # 本 module の到達点（task 5.1／5.2）とシーム
//!
//! - **task 5.1**: 静的 element 層の列挙（[`push_static_element_ops`]・layer 昇順）。
//! - **task 5.2（本 task）**: 有効 bind（`BindSet`）pattern0 層の合成対象化＋`animation-sort`→
//!   animation ID 順の 2 段規則（[`derive_ops`]）。静的層の後（上）へ、descend（既定）なら ID 昇順・
//!   ascend なら ID 降順で bind pattern0 を積む（design 決定5・画家のアルゴリズム）。全パーツが
//!   bind な surface でも非空 bind 集合から可視層を生成する。入れ子参照は **1 段** inline 展開する。
//!
//! 以下は明示的なシーム（後続 task が本 module を拡張して埋める）:
//!
//! - **入れ子 surface 参照の多段再帰 flatten＋循環検出（`visited`）** → task 5.3
//!   （本 task の [`derive_ops`] は入れ子を 1 段だけ inline 展開する）。
//! - **placement None スキップ＋静的キャンバス外形算出（`Extent`）** → task 5.4
//! - **描画可能命令ゼロの分類（`SurfaceNotFound`/`EmptyComposition`）** → task 5.5
//!
//! 本 module は stub で偽装せず、生成する命令列はすべて実挙動・決定的・テスト済みである。
//! design 署名 `build_plan`（out_ops/visited/binds/Extent/`Result<_, ComposeError>`）は後続
//! task（5.4/5.5）が [`derive_ops`] を wrap する形で導入する。

use areka_emo_atlas::ElementId;
use areka_parsers::shell::{Interval, SortOrder};
use bevy_ecs::entity::Entity;

use crate::bind::BindSet;
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
/// 消費は task 5.2 の [`derive_ops`] が本関数を wrap して行う（合成対象 surface 自身の静的層＝
/// (offset_x, offset_y)=(0,0)・bind pattern0 の入れ子参照＝pattern の (x,y) をオフセット）。
pub(crate) fn push_static_element_ops(
    out_ops: &mut Vec<BlitOp>,
    world: &EmoWorld,
    surface_id: u32,
    offset_x: i64,
    offset_y: i64,
) {
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
        // 入れ子参照の (x,y) オフセットを element 配置へ足し込む（1 段 inline 展開・要件 5.2 の
        // pattern0 入れ子参照。M1 は平行移動のみゆえ加算で足りる）。offset=(0,0) の合成対象自身の
        // 静的層では element の transform そのままになる。
        let (ex, ey) = element.transform.offset();
        out_ops.push(BlitOp {
            element: element_id,
            transform: Transform::translate(ex + offset_x, ey + offset_y),
            method: element.method.clone(),
        });
    }
}

/// 合成対象 surface の静的 element 層＋有効 bind pattern0 層の転写命令を `out_ops` へ導出する（要件 5.2/5.3/5.4/5.6）。
///
/// design「PlanBuilder（plan.rs）」の層列挙規則に厳密に従う:
///
/// 1. **静的 element 層**（i）: 合成対象 surface 自身の element を layer 昇順・同 layer は登場順で
///    [`push_static_element_ops`]（offset=(0,0)）により先頭へ積む（**下＝基底層**）。
/// 2. **有効 bind pattern0 層**（ii）: 合成対象 surface の `SurfaceMaster.animations` のうち、
///    interval が bind 種（[`Interval::Bind`]/[`Interval::BindRandom`]）**かつ** animation id が
///    `binds` に含まれる animation を「有効 bind」とし、その pattern0（index 0 の [`Pattern`]）を
///    **2 段ソート順**で静的層の**後（＝上）**へ積む。
///    - **段1（animation-sort・要件 5.3/5.6・design 決定5）**: [`EmoWorld::animation_sort`] が
///      [`SortOrder::Descend`]（既定）→ animation id **昇順**に描画（大 id が上）。[`SortOrder::Ascend`]
///      → id **降順**に描画（小 id が上）。これが画家のアルゴリズムの画素積層写像。
///    - **段2**: 上記方向の中で animation id をキーに整列する。
///
/// pattern0 の `surface_id >= 0` は**入れ子 surface 参照**として、参照先の静的 element 層のみを
/// pattern の (x,y) をオフセットして inline 展開する（本 task は **1 段**のみ・要件 5.2）。
/// `surface_id < 0` はレイヤクリア/停止センチネル＝**非描画 skip**（trace ログ・非パニック・要件 5.5 前段）。
///
/// # 決定性（要件 4.5/10.1）
///
/// 静的層は [`push_static_element_ops`] の安定順、bind 層は (animation id → 昇/降順) の全順序で
/// 整列するため、同一入力（World／surface_id／binds）に対し毎回同一命令列を append する。
///
/// # シーム（後続 task が本関数を拡張して埋める）
///
/// - **多段の再帰 flatten＋循環検出（visited 集合）** → task 5.3。本 task は入れ子参照を **1 段**だけ
///   inline 展開する（参照先が更に bind／入れ子を持つ場合は展開しない・emo2 の bind パーツ
///   surface＝1100 系は element のみで差が生じない）。5.3 が offset 累積つき多段再帰へ一般化する。
/// - **placement None スキップ＋静的キャンバス外形（`Extent`）算出** → task 5.4。
/// - **描画可能命令ゼロの Err 分類（`SurfaceNotFound`/`EmptyComposition`）** → task 5.5。本関数は
///   分類を返さず、不在 surface に対しては何も積まない（非パニック）。公開ファサード `build_plan`
///   （`Result<Extent, ComposeError>`）は 5.4/5.5 が本関数を wrap して導入する。
///
/// 本 task（5.2）では非テストの lib 経路からの呼び出し口（`build_plan` ファサード）が未導入ゆえ
/// `dead_code` になる。消費は task 5.4/5.5 の `build_plan` が本関数を wrap して行う（それまで
/// 意図的な未使用シーム・本 module 内の呼び出し鎖 `push_static_element_ops`/`surface_and_binding`/
/// `is_bind_interval` もこの一点から辿られるため、`allow` はここへ集約する）。
#[allow(dead_code)]
pub(crate) fn derive_ops(
    out_ops: &mut Vec<BlitOp>,
    world: &EmoWorld,
    surface_id: u32,
    binds: &BindSet,
) {
    // 層（i）: 合成対象 surface 自身の静的 element を基底層として先頭へ積む（offset=(0,0)）。
    push_static_element_ops(out_ops, world, surface_id, 0, 0);

    // 合成対象 surface 不在なら bind 層も積まない（後続 task が SurfaceNotFound 分類を担う）。
    let Some((master, _binding)) = surface_and_binding(world, surface_id) else {
        return;
    };

    // 層（ii）: 有効 bind（interval が bind 種 ∧ id ∈ binds）の animation を集める。
    // 走査順（登場順）に依らず全順序で並べ替えるため、まず有効 id を収集する。
    let mut active_ids: Vec<u32> = master
        .animations
        .iter()
        .filter(|a| is_bind_interval(&a.interval) && binds.contains(a.id))
        .map(|a| a.id)
        .collect();

    // 段1: animation-sort に応じて描画順を決める（design 決定5・画家のアルゴリズム写像）。
    //   Descend（既定）→ id 昇順に描画（大 id が上）。Ascend → id 降順に描画（小 id が上）。
    // 段2: その方向の中で id をキーに整列する。
    match world.animation_sort() {
        SortOrder::Descend => active_ids.sort_unstable(),
        SortOrder::Ascend => active_ids.sort_unstable_by(|a, b| b.cmp(a)),
        // `SortOrder` は `#[non_exhaustive]`。未知値は既定 Descend（id 昇順描画）へ倒す（非パニック）。
        other => {
            tracing::warn!(
                target: "areka_emo_compose",
                surface_id,
                sort = ?other,
                "未知の animation-sort: 既定 Descend（id 昇順描画）として扱う"
            );
            active_ids.sort_unstable();
        }
    }

    // 描画順に各有効 bind の pattern0 を静的層の後（＝上）へ積む。
    for id in active_ids {
        // 同 id の animation は fold 段で単一化済み（後勝ち）ゆえ find で足りる。
        let Some(anim) = master.animations.iter().find(|a| a.id == id) else {
            continue;
        };
        // pattern0＝index 昇順の先頭 pattern（疎 index 許容ゆえ最小 index を pattern0 とする）。
        let Some(pattern0) = anim.patterns.iter().min_by_key(|p| p.index) else {
            // pattern を持たない bind animation は積むべき層がない（非パニック・skip）。
            tracing::trace!(
                target: "areka_emo_compose",
                surface_id,
                animation_id = id,
                "有効 bind に pattern が無い: skip"
            );
            continue;
        };

        if pattern0.surface_id < 0 {
            // 負値＝レイヤクリア/停止センチネル。非描画ゆえ命令を積まない（要件 5.5 前段）。
            tracing::trace!(
                target: "areka_emo_compose",
                surface_id,
                animation_id = id,
                sentinel = pattern0.surface_id,
                "bind pattern0 がセンチネル（surface_id<0）: 非描画 skip"
            );
            continue;
        }

        // 入れ子 surface 参照: 参照先の静的 element 層のみを pattern0 の (x,y) をオフセットして
        // 1 段 inline 展開する（多段再帰＋循環検出は task 5.3）。
        let nested_id = pattern0.surface_id as u32;
        push_static_element_ops(out_ops, world, nested_id, pattern0.x, pattern0.y);
    }
}

/// interval が bind 種（`Interval::Bind`/`Interval::BindRandom`）か（要件 5.2・design 層列挙）。
///
/// `Interval::Random`（純ランダム・非 bind）は有効 bind の対象にしない。`Interval` は
/// `#[non_exhaustive]` ゆえ未知 variant は bind でないものとして扱う（非パニック）。
fn is_bind_interval(interval: &Interval) -> bool {
    matches!(interval, Interval::Bind | Interval::BindRandom { .. })
}

/// 指定 surface id の [`SurfaceMaster`] と [`AtlasBinding`] を同時に読む（本 module 内補助）。
///
/// `SurfaceIndex`（疎 id→Entity・O(1)）で entity を引き、`SurfaceMaster` と `AtlasBinding` を
/// 併せて借用する。いずれか（surface 不在・binding 未挿入）が欠けると `None`。既存の公開
/// `world()` アクセサ経由で読むため、公開 API を広げない。
///
/// [`push_static_element_ops`]／[`derive_ops`] 経由で使う本 module 内補助。
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
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, Element, ElementPath, Interval, Pattern, Shell, SortOrder,
        Surface,
    };
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
        surface_with_anims(id, elements, Vec::new())
    }

    /// element＋animation を持つ surface（collision 空・単一ターゲット）。
    fn surface_with_anims(id: u32, elements: Vec<Element>, animations: Vec<Animation>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations,
        }
    }

    /// bind animation 1 本（interval=`Interval::Bind`・pattern0 が入れ子 surface_id を x,y で参照）。
    ///
    /// pattern index は 0 を pattern0（先頭）とし、識別用に index=5 の余分な pattern も加えて
    /// 「pattern0＝最小 index の pattern」を選ぶ実装を突く（疎 index 許容・model 契約）。
    fn bind_anim(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
        Animation {
            id,
            interval: Interval::Bind,
            patterns: vec![
                Pattern {
                    index: 0,
                    surface_id: ref_surface_id,
                    wait: 0,
                    x,
                    y,
                },
                // pattern0 以外は本 task では未使用（pattern0 のみ合成対象・要件 5.2）。
                Pattern {
                    index: 5,
                    surface_id: 999_999,
                    wait: 0,
                    x: 7,
                    y: 7,
                },
            ],
        }
    }

    /// 与えた surface 群を登場順 definitions（plain のみ）で包む `Shell`（animation-sort 未指定＝既定 descend）。
    fn shell_of(surfaces: Vec<Surface>) -> Shell {
        shell_of_with_sort(surfaces, None)
    }

    /// 与えた surface 群を `animation-sort` 指定つきで包む `Shell`。
    fn shell_of_with_sort(surfaces: Vec<Surface>, animation_sort: Option<SortOrder>) -> Shell {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort,
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
        push_static_element_ops(&mut ops, &world, 1000, 0, 0);

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
        push_static_element_ops(&mut ops, &world, 1, 0, 0);

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
        push_static_element_ops(&mut ops1, &world, 1000, 0, 0);
        let mut ops2 = Vec::new();
        push_static_element_ops(&mut ops2, &world, 1000, 0, 0);

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
        push_static_element_ops(&mut ops, &world, 1000, 0, 0);

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
        push_static_element_ops(&mut ops, &world, 7, 0, 0);

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
        push_static_element_ops(&mut ops, &world, 1, 0, 0);

        // 先頭 sentinel が残り、末尾へ element 命令が追記される。
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], sentinel);
    }

    /// surface 不在では何も積まない（後続 task が SurfaceNotFound 分類を担う・本 task は非追記）。
    #[test]
    fn missing_surface_pushes_nothing() {
        let world = EmoWorld::build(&shell_of(Vec::new()));
        let mut ops = Vec::new();
        push_static_element_ops(&mut ops, &world, 9999, 0, 0);
        assert!(ops.is_empty());
    }

    // ── task 5.2: 有効 bind pattern0 の合成対象化＋animation-sort→ID 順の2段規則 ──────────

    /// 命令列を element path 列へ逆写像する（ElementId→path が一意な bake 前提・テスト補助）。
    fn ops_to_paths(ops: &[BlitOp], map: &[(ElementId, &str)]) -> Vec<String> {
        ops.iter()
            .map(|op| {
                map.iter()
                    .find(|(id, _)| *id == op.element)
                    .map(|(_, p)| p.to_string())
                    .expect("op の ElementId は既知")
            })
            .collect()
    }

    /// 各 bind パーツ surface（element 1 本・path で識別可能）を持つ Shell を組む共通土台。
    ///
    /// 合成対象 surface `host_id` は静的 element を任意本持ち、各 bind animation の pattern0 が
    /// `bind_surfaces`（(animation_id, ref_surface_id, path) の並び）の各入れ子 surface を参照する。
    /// 戻り値は (world, atlas, id_to_path 表)。
    fn build_bind_world(
        host_id: u32,
        host_elements: Vec<Element>,
        host_element_rels: &[&str],
        bind_surfaces: &[(u32, u32, &str)], // (animation_id, ref_surface_id, part_path)
        animation_sort: Option<SortOrder>,
    ) -> (EmoWorld, AtlasTable) {
        let base = Path::new("shell/master");

        // 全 element path（host の静的分＋各 bind パーツ分）を bake する。
        let mut all_rels: Vec<&str> = host_element_rels.to_vec();
        for (_, _, path) in bind_surfaces {
            all_rels.push(path);
        }
        let atlas = bake_atlas(base, &all_rels);

        // host surface: 静的 element ＋ bind animation 群。
        let anims: Vec<Animation> = bind_surfaces
            .iter()
            .map(|(aid, ref_id, _)| bind_anim(*aid, *ref_id as i64, 0, 0))
            .collect();
        let host = surface_with_anims(host_id, host_elements, anims);

        // 各 bind パーツ surface: element 1 本（path で識別）。
        let parts: Vec<Surface> = bind_surfaces
            .iter()
            .map(|(_, ref_id, path)| surface(*ref_id, vec![elem(0, path, 0, 0)]))
            .collect();

        let mut surfaces = vec![host];
        surfaces.extend(parts);

        let shell = shell_of_with_sort(surfaces, animation_sort);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));
        (world, atlas)
    }

    /// テスト①（受入基準・要件 5.2/5.3/5.6）: 複数有効 bind・sort 既定（descend）→ ID 昇順に描画。
    ///
    /// animation id [3,1,2]（登場順）の各 pattern0 が別 surface（part3/part1/part2）を参照。
    /// animation-sort 未指定（既定 descend）ゆえ ID 昇順描画で bind 層は part1→part2→part3 の順。
    #[test]
    fn active_binds_descend_default_draws_in_id_ascending_order() {
        let (world, atlas) = build_bind_world(
            1000,
            Vec::new(),
            &[],
            // 登場順は 3,1,2（描画順とは別）。各 pattern0 が別 surface を参照。
            &[(3, 1300, "part3.png"), (1, 1100, "part1.png"), (2, 1200, "part2.png")],
            None, // 未指定＝既定 descend。
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

        let binds = BindSet::from_ids([1, 2, 3]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &world, 1000, &binds);

        // descend（既定）→ ID 昇順描画: part1(1) → part2(2) → part3(3)。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png", "part2.png", "part3.png"]);
    }

    /// テスト②（要件 5.3）: animation-sort=ascend → ID 降順に描画（小 ID が上）。
    #[test]
    fn active_binds_ascend_draws_in_id_descending_order() {
        let (world, atlas) = build_bind_world(
            1000,
            Vec::new(),
            &[],
            &[(3, 1300, "part3.png"), (1, 1100, "part1.png"), (2, 1200, "part2.png")],
            Some(SortOrder::Ascend),
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

        let binds = BindSet::from_ids([1, 2, 3]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &world, 1000, &binds);

        // ascend → ID 降順描画: part3(3) → part2(2) → part1(1)。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part3.png", "part2.png", "part1.png"]);
    }

    /// テスト③（要件 5.2）: BindSet に含まれない bind animation は合成対象から除外される。
    #[test]
    fn only_binds_in_bindset_are_included() {
        let (world, atlas) = build_bind_world(
            1000,
            Vec::new(),
            &[],
            &[(1, 1100, "part1.png"), (2, 1200, "part2.png"), (3, 1300, "part3.png")],
            None,
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

        // id=2 のみ有効（1,3 は非活性）。
        let binds = BindSet::from_ids([2]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &world, 1000, &binds);

        // part2 のみ命令化される。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part2.png"]);
    }

    /// テスト④（要件 5.4）: 静的 element ゼロ・全パーツ bind の surface でも非空 bind 集合→非空 ops。
    /// 空 bind 集合では bind 命令ゼロ（非パニック・全透明処理は 5.5/6.6）。
    #[test]
    fn bind_only_surface_produces_layers_from_nonempty_bindset() {
        let (world, atlas) = build_bind_world(
            1000, // emo2 surface1000 相当（static element なし・全 bind）。
            Vec::new(),
            &[],
            &[(1, 1100, "part1.png"), (2, 1200, "part2.png")],
            None,
        );
        let map = id_to_path(&atlas, &["part1.png", "part2.png"]);

        // 非空 bind 集合 → 可視層が生成される（空白にしない）。
        let binds = BindSet::from_ids([1, 2]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &world, 1000, &binds);
        assert!(!ops.is_empty(), "全 bind surface でも非空 bind 集合から可視層を生む");
        assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png", "part2.png"]);

        // 空 bind 集合 → bind 命令なし（静的 element も無いので空・非パニック）。
        let empty = BindSet::default();
        let mut ops_empty = Vec::new();
        derive_ops(&mut ops_empty, &world, 1000, &empty);
        assert!(ops_empty.is_empty(), "空 bind 集合では bind 命令ゼロ（非パニック）");
    }

    /// テスト⑤（要件 5.2・design 層列挙 i/ii）: 静的 element 層が bind 層の**前（下）**に来る。
    #[test]
    fn static_elements_precede_bind_layers() {
        let (world, atlas) = build_bind_world(
            1000,
            vec![elem(0, "base.png", 0, 0)], // 静的 element 1 本（基底）。
            &["base.png"],
            &[(1, 1100, "part1.png")],
            None,
        );
        let map = id_to_path(&atlas, &["base.png", "part1.png"]);

        let binds = BindSet::from_ids([1]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &world, 1000, &binds);

        // 静的 base（下）→ bind part1（上）。
        assert_eq!(ops_to_paths(&ops, &map), vec!["base.png", "part1.png"]);
    }

    /// テスト⑥（要件 5.5 前段）: pattern0 の surface_id<0 はセンチネル＝命令を積まず skip（非パニック）。
    #[test]
    fn sentinel_pattern0_is_skipped() {
        let base = Path::new("shell/master");
        // part1 は正常参照、bind id=2 の pattern0 は surface_id=-2（センチネル）。
        let atlas = bake_atlas(base, &["part1.png"]);
        let map = id_to_path(&atlas, &["part1.png"]);

        let host = surface_with_anims(
            1000,
            Vec::new(),
            vec![
                bind_anim(1, 1100, 0, 0),  // 正常参照。
                bind_anim(2, -2, 0, 0),    // センチネル（非描画）。
            ],
        );
        let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
        let shell = shell_of(vec![host, part1]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1, 2]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &world, 1000, &binds);

        // センチネル bind id=2 は積まれず、part1 のみ（非パニック）。
        assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png"]);
    }

    /// テスト⑦（要件 4.5/10.1）: 同一入力で 2 回導出→命令列がバイト等価（bind 経路の決定性）。
    #[test]
    fn bind_derivation_is_deterministic() {
        let (world, atlas) = build_bind_world(
            1000,
            vec![elem(0, "base.png", 0, 0)],
            &["base.png"],
            &[(3, 1300, "part3.png"), (1, 1100, "part1.png"), (2, 1200, "part2.png")],
            None,
        );
        let _ = &atlas; // atlas は bind_atlas 済み（以降の resolve は不要）。

        let binds = BindSet::from_ids([1, 2, 3]);
        let mut ops1 = Vec::new();
        derive_ops(&mut ops1, &world, 1000, &binds);
        let mut ops2 = Vec::new();
        derive_ops(&mut ops2, &world, 1000, &binds);

        assert_eq!(ops1, ops2, "同一入力→同一 ops（バイト等価）");
        // base(静的1) ＋ bind 3 本＝4 命令。
        assert_eq!(ops1.len(), 4);
    }

    /// pattern0 の (x,y) が入れ子参照 element の配置へオフセット加算される（要件 5.2・1 段 inline 展開）。
    #[test]
    fn nested_pattern0_offset_is_applied_to_element_transform() {
        let base = Path::new("shell/master");
        let atlas = bake_atlas(base, &["part.png"]);
        let part_id = atlas.resolve(SetId(0), "part.png").expect("part 解決");

        // bind id=1 の pattern0 が surface 1100 を (30, -20) で参照。part の element は自 (5, 6)。
        let host = surface_with_anims(1000, Vec::new(), vec![bind_anim(1, 1100, 30, -20)]);
        let part = surface(1100, vec![elem(0, "part.png", 5, 6)]);
        let shell = shell_of(vec![host, part]);
        let mut world = EmoWorld::build(&shell);
        world.bind_atlas(&atlas, SetId(0));

        let binds = BindSet::from_ids([1]);
        let mut ops = Vec::new();
        derive_ops(&mut ops, &world, 1000, &binds);

        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].element, part_id);
        // element 自 (5,6) ＋ pattern0 offset (30,-20) ＝ (35, -14)。
        assert_eq!(ops[0].transform.offset(), (35, -14));
    }
}
