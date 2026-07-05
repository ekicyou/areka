//! single-pass fold: parser の登場順定義ストリームを `EmoWorld` へ畳み込む。
//!
//! plain `surfaceN,M`／`N-M` は全 id を新設・`surface.append` は既存 id のみへ追記
//! （存在条件付き・ukadoc 意味論）。ターゲット記述子の単一・列挙・範囲を展開し、除外指定
//! （`!N`／`!a-b`）を展開時に減算適用する。複数定義が同一 surface に効く場合は登場順を保った
//! 順序で決定的に適用し、append ブロックが持つ element・collision・animation を対象 surface へ
//! 反映しつつ alias を収集する。参照 id が存在しない場合はパニックせず `warn` 以上で観測可能に扱う。

use areka_parsers::shell::{AppendTarget, DefRef, Element, Shell, Surface};
use bevy_ecs::world::World;

use crate::method::ComposeMethod;
use crate::normalized::{NormalizedElement, SurfaceMaster, Transform};
use crate::world::{SurfaceId, SurfaceIndex};

/// `Shell.definitions` を登場順に single-pass で走査し `World` へ畳み込む（要件 1.7）。
///
/// 本 task（3.2）が担うのは plain `surface` ヘッダ（[`DefRef::Surface`]）の展開＝全 id 新設のみ。
/// append（[`DefRef::Append`]）・alias（[`DefRef::Alias`]）は後続 task の領分であり、本段では
/// 素通り（無処理）にしてストリーム走査の骨格（前方参照なし・多パス不要）だけを確立する。
///
/// 欠落・不整合はパニックせず `warn` で観測可能化する（要件 1.4）。
pub(crate) fn fold_shell(world: &mut World, shell: &Shell) {
    for def in &shell.definitions {
        match *def {
            DefRef::Surface(index) => match shell.surfaces.get(index) {
                Some(surface) => fold_plain_surface(world, surface),
                None => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Surface が surfaces 範囲外を指す: スキップ"
                    );
                }
            },
            // append／alias は後続 task（3.3〜3.5）の領分。本 task では素通りする。
            DefRef::Append(_) | DefRef::Alias(_) => {}
            // `DefRef` は `#[non_exhaustive]`。未知の定義種別はパニックせず観測可能化する（要件 1.4）。
            other => {
                tracing::warn!(
                    target: "areka_emo_compose",
                    def = ?other,
                    "未知の DefRef 種別: スキップ"
                );
            }
        }
    }
}

/// plain `surface` 定義 1 件を展開し、各 id を新規 surface として常駐させる（要件 1.1/2.1）。
///
/// ターゲット記述子（単一・列挙・範囲）を記述順に展開し、共有ボディ（element/collision/animation）
/// から正規化 [`SurfaceMaster`] を id ごとに生成して登録する。既存 id は全置換（後勝ち・`warn`）。
fn fold_plain_surface(world: &mut World, surface: &Surface) {
    for id in expand_targets(&surface.targets) {
        let master = normalize_surface(id, surface);
        upsert_surface(world, id, master);
    }
}

/// ターゲット記述子を記述順に展開した id 列を返す（両端含む・要件 2.1）。
///
/// 本 task では除外（[`AppendTarget::Exclude`]／[`AppendTarget::ExcludeRange`]）の減算は行わず、
/// 包含ターゲット（`Single`／`Range`）のみを記述順に列挙する（除外減算は task 3.4 の領分・その
/// シームとして除外 variant は無視して素通りする）。
fn expand_targets(targets: &[AppendTarget]) -> Vec<u32> {
    let mut ids = Vec::new();
    for target in targets {
        match *target {
            AppendTarget::Single(id) => ids.push(id),
            AppendTarget::Range { start, end } => {
                // 記述子の向きに関わらず両端含みで昇順展開する（`a-b` は a..=b）。
                let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
                ids.extend(lo..=hi);
            }
            // 除外の減算適用は task 3.4。ここでは包含集合に影響させない（seam）。
            AppendTarget::Exclude(_) | AppendTarget::ExcludeRange { .. } => {}
            // `AppendTarget` は `#[non_exhaustive]`。未知の記述子はパニックせず観測可能化する（要件 1.4）。
            ref other => {
                tracing::warn!(
                    target: "areka_emo_compose",
                    target_desc = ?other,
                    "未知の AppendTarget 記述子: 包含集合から除外"
                );
            }
        }
    }
    ids
}

/// 共有ボディから id 固有の正規化 [`SurfaceMaster`] を生成する（要件 4.2/4.4）。
///
/// element は layer 昇順（同 layer は登場順）に安定ソートし、x,y を [`Transform::translate`] へ、
/// method は M1 契約により常に [`ComposeMethod::Overlay`] とする。collision/animation は転記のまま
/// 複製する。
fn normalize_surface(id: u32, surface: &Surface) -> SurfaceMaster {
    let mut elements: Vec<NormalizedElement> = surface
        .elements
        .iter()
        .map(normalize_element)
        .collect();
    // layer 昇順・同 layer は登場順（安定ソート）。
    elements.sort_by_key(|e| e.layer);

    SurfaceMaster {
        id,
        elements,
        collisions: surface.collisions.clone(),
        animations: surface.animations.clone(),
    }
}

/// 転記 element を正規化 element へ写す（x,y→[`Transform`]・method は M1 固定 [`Overlay`]）。
///
/// [`Overlay`]: ComposeMethod::Overlay
fn normalize_element(element: &Element) -> NormalizedElement {
    NormalizedElement {
        layer: element.layer,
        path: element.path.clone(),
        transform: Transform::translate(element.x, element.y),
        method: ComposeMethod::Overlay,
    }
}

/// id→entity を登録する。既存 id は全置換（後勝ち）＋ `warn`（要件 2.1・ukadoc 明文規則なし＝de-facto）。
fn upsert_surface(world: &mut World, id: u32, master: SurfaceMaster) {
    let existing = world.resource::<SurfaceIndex>().0.get(&id).copied();
    if let Some(old_entity) = existing {
        tracing::warn!(
            target: "areka_emo_compose",
            id,
            "surface id 重複: 既存定義を全置換する（後勝ち）"
        );
        // 全置換のため旧 entity を除去してから新設する（画素バッファは持たない・要件 10.6）。
        world.despawn(old_entity);
    }
    let entity = world.spawn((SurfaceId(id), master)).id();
    world.resource_mut::<SurfaceIndex>().0.insert(id, entity);
}

#[cfg(test)]
mod tests {
    use areka_parsers::shell::{
        AppendTarget, DefRef, Element, ElementPath, Shell, Surface,
    };
    use bevy_ecs::world::World;

    use crate::fold::fold_shell;
    use crate::method::ComposeMethod;
    use crate::world::{SurfaceId, SurfaceIndex};
    use crate::normalized::SurfaceMaster;

    /// 既定リソースを備えた空 World を用意する（`EmoWorld::build` と同じ初期化）。
    fn fresh_world() -> World {
        let mut world = World::new();
        world.insert_resource(SurfaceIndex::default());
        world
    }

    /// 指定 id をキーに登録済み entity の `SurfaceMaster` を引く。
    fn master_of<'w>(world: &'w World, id: u32) -> Option<&'w SurfaceMaster> {
        let entity = *world.resource::<SurfaceIndex>().0.get(&id)?;
        world.get::<SurfaceMaster>(entity)
    }

    /// ターゲット記述子とボディ element 群から plain surface 定義を組み立てる。
    fn surface_def(id: u32, targets: Vec<AppendTarget>, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets,
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    /// element 1 本（layer/x/y 指定・パスは id 由来のダミー）。
    fn elem(layer: u32, path: &str, x: i64, y: i64) -> Element {
        Element {
            layer,
            path: ElementPath::new(path.to_string()),
            x,
            y,
        }
    }

    /// surfaces と definitions を 1 対 1（登場順）で組んだ `Shell`。
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

    /// (a) 列挙 `surface0,5`: id=0 と id=5 の両方が同一の共有ボディを持って常駐する（要件 1.1/2.1）。
    #[test]
    fn enumeration_creates_all_ids_with_shared_body() {
        let body = vec![elem(0, "arm.png", 10, 20)];
        let surf = surface_def(
            0,
            vec![AppendTarget::Single(0), AppendTarget::Single(5)],
            body,
        );
        let mut world = fresh_world();
        fold_shell(&mut world, &shell_of(vec![surf]));

        let m0 = master_of(&world, 0).expect("id=0 が常駐する");
        let m5 = master_of(&world, 5).expect("id=5 が常駐する");
        // それぞれの component の id フィールドは自 id を持つ。
        assert_eq!(m0.id, 0);
        assert_eq!(m5.id, 5);
        // ボディ（element）は共有＝同一内容。
        assert_eq!(m0.elements, m5.elements);
        assert_eq!(m0.elements.len(), 1);
        assert_eq!(m0.elements[0].layer, 0);
        assert_eq!(m0.elements[0].path.as_str(), "arm.png");
        assert_eq!(m0.elements[0].transform.offset(), (10, 20));
        assert_eq!(m0.elements[0].method, ComposeMethod::Overlay);
        // ちょうど 2 entity。
        assert_eq!(world.resource::<SurfaceIndex>().0.len(), 2);
    }

    /// (b) 範囲 `surface1-3`: id 1,2,3 が両端含みで新設される（要件 1.1/2.1）。
    #[test]
    fn range_creates_inclusive_ids() {
        let surf = surface_def(
            1,
            vec![AppendTarget::Range { start: 1, end: 3 }],
            vec![elem(0, "r.png", 0, 0)],
        );
        let mut world = fresh_world();
        fold_shell(&mut world, &shell_of(vec![surf]));

        for id in [1u32, 2, 3] {
            let m = master_of(&world, id).unwrap_or_else(|| panic!("id={id} が常駐する"));
            assert_eq!(m.id, id);
            assert_eq!(m.elements.len(), 1);
        }
        assert!(master_of(&world, 0).is_none());
        assert!(master_of(&world, 4).is_none());
        assert_eq!(world.resource::<SurfaceIndex>().0.len(), 3);
    }

    /// (c) 重複 id は全置換（後勝ち）: 後の定義が前の定義を丸ごと差し替える（要件 2.1）。
    #[test]
    fn duplicate_id_is_replaced_last_wins() {
        let first = surface_def(7, vec![AppendTarget::Single(7)], vec![elem(0, "old.png", 0, 0)]);
        let second =
            surface_def(7, vec![AppendTarget::Single(7)], vec![elem(0, "new.png", 5, 6)]);
        let mut world = fresh_world();
        fold_shell(&mut world, &shell_of(vec![first, second]));

        let m = master_of(&world, 7).expect("id=7 が常駐する");
        // 後勝ち: new.png / (5,6) が残る。
        assert_eq!(m.elements.len(), 1);
        assert_eq!(m.elements[0].path.as_str(), "new.png");
        assert_eq!(m.elements[0].transform.offset(), (5, 6));
        // 置換のため entity は 1 件のみ（古い entity は残さない）。
        assert_eq!(world.resource::<SurfaceIndex>().0.len(), 1);
        let live: Vec<u32> = world
            .query::<&SurfaceId>()
            .iter(&world)
            .map(|s| s.0)
            .collect();
        assert_eq!(live, vec![7]);
    }

    /// (d) 単一形 `surface0`: 従来どおり 1 件だけ新設される。
    #[test]
    fn single_form_creates_one_surface() {
        let surf = surface_def(0, vec![AppendTarget::Single(0)], vec![elem(0, "s0.png", 1, 2)]);
        let mut world = fresh_world();
        fold_shell(&mut world, &shell_of(vec![surf]));

        let m = master_of(&world, 0).expect("id=0 が常駐する");
        assert_eq!(m.id, 0);
        assert_eq!(m.elements[0].path.as_str(), "s0.png");
        assert_eq!(world.resource::<SurfaceIndex>().0.len(), 1);
    }

    /// 記述順展開: `surface2,1` は 2→1 の順に生成され、両 id が常駐する（決定的・design）。
    #[test]
    fn enumeration_expands_in_description_order() {
        let surf = surface_def(
            2,
            vec![AppendTarget::Single(2), AppendTarget::Single(1)],
            vec![elem(0, "d.png", 0, 0)],
        );
        let mut world = fresh_world();
        fold_shell(&mut world, &shell_of(vec![surf]));

        assert!(master_of(&world, 2).is_some());
        assert!(master_of(&world, 1).is_some());
        assert_eq!(world.resource::<SurfaceIndex>().0.len(), 2);
    }
}
