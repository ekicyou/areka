//! single-pass fold: parser の登場順定義ストリームを `EmoWorld` へ畳み込む。
//!
//! plain `surfaceN,M`／`N-M` は全 id を新設・`surface.append` は既存 id のみへ追記
//! （存在条件付き・ukadoc 意味論）。ターゲット記述子の単一・列挙・範囲を展開し、除外指定
//! （`!N`／`!a-b`）を展開時に減算適用する。複数定義が同一 surface に効く場合は登場順を保った
//! 順序で決定的に適用し、append ブロックが持つ element・collision・animation を対象 surface へ
//! 反映しつつ alias を収集する。参照 id が存在しない場合はパニックせず `warn` 以上で観測可能に扱う。

use areka_parsers::shell::{
    AppendTarget, DefRef, Element, Shell, Surface, SurfaceAlias, SurfaceAppend,
};
use bevy_ecs::world::World;

use crate::method::ComposeMethod;
use crate::normalized::{NormalizedElement, SurfaceMaster, Transform};
use crate::world::{AliasMap, SurfaceId, SurfaceIndex};

/// `Shell.definitions` を登場順に single-pass で走査し `World` へ畳み込む（要件 1.7）。
///
/// plain `surface` ヘッダ（[`DefRef::Surface`]）は全 id 新設、`surface.append`
/// （[`DefRef::Append`]）は展開後その時点でツリーに存在する id のみへ追記する（存在条件付き・
/// 要件 2.2）。登場順で走査するため、append は「その時点までに積まれた状態」に対して効き、後続で
/// 定義される surface へは遡及しない（要件 2.3・前方参照なし）。alias（[`DefRef::Alias`]）は
/// `AliasMap` へ収集し、同一キーは登場順で後勝ちとする（要件 3.1/3.2）。
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
            DefRef::Append(index) => match shell.appends.get(index) {
                Some(append) => fold_append(world, append),
                None => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Append が appends 範囲外を指す: スキップ"
                    );
                }
            },
            DefRef::Alias(index) => match shell.aliases.get(index) {
                Some(alias) => fold_alias(world, alias),
                None => {
                    // 転記層と定義ストリームの不整合（本来生じない）。パニックせず観測可能化する。
                    tracing::warn!(
                        target: "areka_emo_compose",
                        index,
                        "DefRef::Alias が aliases 範囲外を指す: スキップ"
                    );
                }
            },
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

/// `surface.append` 定義 1 件を展開し、**その時点で既存の id のみ**へ追記する（要件 2.2/2.4）。
///
/// ターゲット記述子を plain と同一規則で展開し（単一・列挙・両端含む範囲・[`expand_targets`]）、
/// 各 id が [`SurfaceIndex`] に存在する場合のみ対象 [`SurfaceMaster`] を in-place マージする
/// （despawn/respawn しない）。非存在 id は新設せず `warn` でスキップする（要件 1.4/2.2）。
/// 登場順の走査（[`fold_shell`]）ゆえ、後続で定義される surface へは遡及しない（要件 2.3）。
fn fold_append(world: &mut World, append: &SurfaceAppend) {
    // 追記 element は plain と同一の正規化（x,y→Transform・method=Overlay）で用意する。
    let append_elements: Vec<NormalizedElement> =
        append.elements.iter().map(normalize_element).collect();

    for id in expand_targets(&append.targets) {
        // 存在条件: その時点でツリーに存在する id のみ対象（非存在は新設しない・要件 2.2）。
        let Some(entity) = world.resource::<SurfaceIndex>().0.get(&id).copied() else {
            tracing::warn!(
                target: "areka_emo_compose",
                id,
                "surface.append 対象 id が未存在: 新設せずスキップ（存在条件付き）"
            );
            continue;
        };
        let Some(mut master) = world.get_mut::<SurfaceMaster>(entity) else {
            // SurfaceIndex に載るが component 欠落（本来生じない不整合）。観測可能化してスキップ。
            tracing::warn!(
                target: "areka_emo_compose",
                id,
                "SurfaceIndex は指すが SurfaceMaster component が欠落: スキップ"
            );
            continue;
        };

        // element: 末尾連結してから layer 昇順で安定ソート（不変条件を保つ・要件 2.4）。
        master.elements.extend(append_elements.iter().cloned());
        master.elements.sort_by_key(|e| e.layer);

        // collision: 末尾連結（転記のまま・要件 2.4）。
        master.collisions.extend(append.collisions.iter().cloned());

        // animation: 同一 id は後勝ち置換（+warn）・新 id は追加（要件 2.4）。
        merge_animations(id, &mut master.animations, &append.animations);
    }
}

/// `kero.surface.alias` の 1 エントリを `AliasMap` へ収集する（要件 3.1/3.2）。
///
/// alias キー → 順序付き数値 id リストを `BTreeMap` へ挿入する。`BTreeMap::insert` は同一キーで
/// 上書きするため、登場順 single-pass 走査（[`fold_shell`]）と併せて重複キーは後勝ちで決定的に
/// なる（要件 3.2）。id リストはソート・重複除去せず記述順のまま複製する（alias の順序付き
/// ターゲット列＝要件 3.1・BindSet の正規化とは別物）。
fn fold_alias(world: &mut World, alias: &SurfaceAlias) {
    world
        .resource_mut::<AliasMap>()
        .0
        .insert(alias.key.as_str().to_string(), alias.ids.clone());
}

/// append の animation を対象 surface の animation 群へマージする（要件 2.4）。
///
/// 同一 animation id が既存にあれば後勝ち置換（既存を append 版で差し替え・単一に保つ）、
/// 新 id なら末尾へ追加する。置換は ukadoc 明文規則が無い de-facto 挙動ゆえ `warn` で記録する。
fn merge_animations(
    surface_id: u32,
    existing: &mut Vec<areka_parsers::shell::Animation>,
    additions: &[areka_parsers::shell::Animation],
) {
    for add in additions {
        if let Some(slot) = existing.iter_mut().find(|a| a.id == add.id) {
            tracing::warn!(
                target: "areka_emo_compose",
                surface_id,
                animation_id = add.id,
                "append が既存 animation id を再定義: 後勝ちで置換する（de-facto）"
            );
            *slot = add.clone();
        } else {
            existing.push(add.clone());
        }
    }
}

/// ターゲット記述子を記述順に展開し、除外指定（`!N`／`!a-b`）を減算した id 列を返す（要件 2.5）。
///
/// 二相構成: (1) 包含記述子（[`AppendTarget::Single`]／[`AppendTarget::Range`]・両端含む）を
/// **記述順**に列挙し、(2) 除外記述子（[`AppendTarget::Exclude`]／[`AppendTarget::ExcludeRange`]・
/// 両端含む）を集合として集め、包含列から除外集合に属する id を取り除く。除外は記述子の登場位置に
/// 依らず展開結果全体へ効く（ukadoc: `!` は集合から除去）。生存 id の記述順・重複は保つ（design
/// 「展開結果の適用順は記述順」）。
///
/// emo2 は除外を使用しないため実処理は型シームだが、記述子の口は保持し減算まで実装する（要件 2.5/12.3）。
fn expand_targets(targets: &[AppendTarget]) -> Vec<u32> {
    use std::collections::BTreeSet;

    // (1) 包含 id を記述順に列挙（重複はそのまま保つ）。
    let mut included: Vec<u32> = Vec::new();
    // (2) 除外 id を集合へ集める（記述位置に依らず全体へ効かせるため先に全走査）。
    let mut excluded: BTreeSet<u32> = BTreeSet::new();

    for target in targets {
        match *target {
            AppendTarget::Single(id) => included.push(id),
            AppendTarget::Range { start, end } => {
                // 記述子の向きに関わらず両端含みで昇順展開する（`a-b` は a..=b）。
                let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
                included.extend(lo..=hi);
            }
            AppendTarget::Exclude(id) => {
                excluded.insert(id);
            }
            AppendTarget::ExcludeRange { start, end } => {
                // 除外範囲も両端含み（記述子の向き不問）。
                let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
                excluded.extend(lo..=hi);
            }
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

    // 減算: 除外集合に属さない id のみを記述順・重複保持で残す。
    included.retain(|id| !excluded.contains(id));
    included
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
        AliasKey, Animation, AppendTarget, Collision, CollisionName, DefRef, Element, ElementPath,
        Interval, Shell, Surface, SurfaceAlias, SurfaceAppend,
    };
    use bevy_ecs::world::World;

    use crate::fold::{expand_targets, fold_shell};
    use crate::method::ComposeMethod;
    use crate::world::{AliasMap, SurfaceId, SurfaceIndex};
    use crate::normalized::SurfaceMaster;

    /// 既定リソースを備えた空 World を用意する（`EmoWorld::build` と同じ初期化）。
    fn fresh_world() -> World {
        let mut world = World::new();
        world.insert_resource(SurfaceIndex::default());
        world.insert_resource(AliasMap::default());
        world
    }

    /// alias キーの解決（`EmoWorld::resolve_alias` と同じ AliasMap 参照経路）。
    fn resolve_alias<'w>(world: &'w World, key: &str) -> Option<&'w [u32]> {
        world.resource::<AliasMap>().0.get(key).map(Vec::as_slice)
    }

    /// alias 1 エントリ（KEY,[id,...]）を組み立てる。
    fn alias_def(key: &str, ids: Vec<u32>) -> SurfaceAlias {
        SurfaceAlias {
            key: AliasKey::new(key.to_string()),
            ids,
        }
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

    /// collision 1 本（矩形＋領域名）。
    fn coll(index: u32, name: &str) -> Collision {
        Collision {
            index,
            left: 0,
            top: 0,
            right: 10,
            bottom: 10,
            name: CollisionName::new(name.to_string()),
        }
    }

    /// animation 1 本（interval=bind・pattern 群空・id と識別のみに用いる）。
    fn anim(id: u32) -> Animation {
        Animation {
            id,
            interval: Interval::Bind,
            patterns: Vec::new(),
        }
    }

    /// element/collision/animation を任意に持つ append 定義を組み立てる。
    fn append_def(
        targets: Vec<AppendTarget>,
        elements: Vec<Element>,
        collisions: Vec<Collision>,
        animations: Vec<Animation>,
    ) -> SurfaceAppend {
        SurfaceAppend {
            targets,
            elements,
            collisions,
            animations,
        }
    }

    /// surface 定義（element/collision/animation を任意に持つ）を組み立てる。
    fn surface_full(
        id: u32,
        targets: Vec<AppendTarget>,
        elements: Vec<Element>,
        collisions: Vec<Collision>,
        animations: Vec<Animation>,
    ) -> Surface {
        Surface {
            id,
            targets,
            elements,
            collisions,
            animations,
        }
    }

    /// `Shell` を surface/append を混在させた登場順 `definitions` で組む。
    ///
    /// `defs` は登場順に `DefRef` を並べたもの（呼び手が `DefRef::Surface(i)`／`DefRef::Append(i)`
    /// を直接指定する）。surfaces/appends の各 Vec には index が既に対応している前提。
    fn shell_mixed(
        surfaces: Vec<Surface>,
        appends: Vec<SurfaceAppend>,
        defs: Vec<DefRef>,
    ) -> Shell {
        Shell {
            surfaces,
            appends,
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions: defs,
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

    // ── task 3.3: surface.append の展開＝既存 id 限定の追記 ──────────────────

    /// (3.3-1) 受入基準: `surface.append10,2100-2110,2200-2210` 相当。
    /// 事前に存在する id のみへ追記し、範囲内でも未存在 id は新設しない（要件 2.2）。
    #[test]
    fn append_reaches_existing_ids_only_not_creating_absent() {
        // 事前 plain surface: id=10, id=2100, id=2101（2102-2110 と 2200-2210 は未存在）。
        let s10 = surface_def(10, vec![AppendTarget::Single(10)], vec![elem(0, "base10.png", 0, 0)]);
        let s2100 =
            surface_def(2100, vec![AppendTarget::Single(2100)], vec![elem(0, "b2100.png", 0, 0)]);
        let s2101 =
            surface_def(2101, vec![AppendTarget::Single(2101)], vec![elem(0, "b2101.png", 0, 0)]);
        // append: 10, 2100-2110, 2200-2210（layer=1 の追記 element を持つ）。
        let ap = append_def(
            vec![
                AppendTarget::Single(10),
                AppendTarget::Range { start: 2100, end: 2110 },
                AppendTarget::Range { start: 2200, end: 2210 },
            ],
            vec![elem(1, "added.png", 3, 4)],
            Vec::new(),
            Vec::new(),
        );
        let mut world = fresh_world();
        // 登場順: Surface(10), Surface(2100), Surface(2101), Append(ap)。
        fold_shell(
            &mut world,
            &shell_mixed(
                vec![s10, s2100, s2101],
                vec![ap],
                vec![
                    DefRef::Surface(0),
                    DefRef::Surface(1),
                    DefRef::Surface(2),
                    DefRef::Append(0),
                ],
            ),
        );

        // 既存 id には追記 element が届く（base + added の 2 本）。
        for id in [10u32, 2100, 2101] {
            let m = master_of(&world, id).unwrap_or_else(|| panic!("id={id} が常駐する"));
            assert_eq!(m.elements.len(), 2, "id={id} に追記 element が届く");
            // 追記 element が含まれる。
            assert!(
                m.elements.iter().any(|e| e.path.as_str() == "added.png"),
                "id={id} に added.png が追記される"
            );
        }
        // 範囲内でも未存在だった id は新設されない。
        for id in [2102u32, 2110, 2200, 2205, 2210] {
            assert!(master_of(&world, id).is_none(), "id={id} は新設されない");
        }
        // entity 数は事前 3 件のまま（append は新設しない）。
        assert_eq!(world.resource::<SurfaceIndex>().0.len(), 3);
    }

    /// (3.3-2) element/collision マージ（要件 2.4）。
    /// append 追記後、element は layer 昇順を保ち、collision は末尾連結される。
    #[test]
    fn append_merges_element_and_collision_layer_sorted() {
        // 事前 surface: layer=2 の element ＋ collision 1 本。
        let base = surface_full(
            5,
            vec![AppendTarget::Single(5)],
            vec![elem(2, "hi.png", 0, 0)],
            vec![coll(0, "Head")],
            Vec::new(),
        );
        // append: layer=0 の element（base より低い layer）＋ collision 1 本。
        let ap = append_def(
            vec![AppendTarget::Single(5)],
            vec![elem(0, "lo.png", 1, 1)],
            vec![coll(1, "Bust")],
            Vec::new(),
        );
        let mut world = fresh_world();
        fold_shell(
            &mut world,
            &shell_mixed(
                vec![base],
                vec![ap],
                vec![DefRef::Surface(0), DefRef::Append(0)],
            ),
        );

        let m = master_of(&world, 5).expect("id=5 が常駐する");
        // element は 2 本＝マージされている。
        assert_eq!(m.elements.len(), 2);
        // layer 昇順（追記後も不変条件を保つ）: lo(layer=0) → hi(layer=2)。
        assert_eq!(m.elements[0].layer, 0);
        assert_eq!(m.elements[0].path.as_str(), "lo.png");
        assert_eq!(m.elements[1].layer, 2);
        assert_eq!(m.elements[1].path.as_str(), "hi.png");
        // collision は末尾連結（Head → Bust）。
        assert_eq!(m.collisions.len(), 2);
        assert_eq!(m.collisions[0].name.as_str(), "Head");
        assert_eq!(m.collisions[1].name.as_str(), "Bust");
    }

    /// (3.3-3) animation 後勝ち置換＋新 id 追加（要件 2.4）。
    /// 対象に既存 animation id=N があり append が同 id=N を持つ → append 版に置換（単一）。
    /// 新 id=M は追加される。
    #[test]
    fn append_animation_last_wins_and_new_id_added() {
        // 事前 surface: animation id=1（interval=bind, pattern 空）。
        let base = surface_full(
            8,
            vec![AppendTarget::Single(8)],
            Vec::new(),
            Vec::new(),
            vec![anim(1)],
        );
        // append: 同 id=1 を別内容（interval=random,k=5）で持ち、新 id=2 を追加。
        let replacement = Animation {
            id: 1,
            interval: Interval::Random { k: 5 },
            patterns: Vec::new(),
        };
        let ap = append_def(
            vec![AppendTarget::Single(8)],
            Vec::new(),
            Vec::new(),
            vec![replacement.clone(), anim(2)],
        );
        let mut world = fresh_world();
        fold_shell(
            &mut world,
            &shell_mixed(
                vec![base],
                vec![ap],
                vec![DefRef::Surface(0), DefRef::Append(0)],
            ),
        );

        let m = master_of(&world, 8).expect("id=8 が常駐する");
        // id=1 は 1 本のみ（重複しない）＝後勝ち置換。
        let id1: Vec<&Animation> = m.animations.iter().filter(|a| a.id == 1).collect();
        assert_eq!(id1.len(), 1, "id=1 は置換されて 1 本");
        assert_eq!(id1[0].interval, Interval::Random { k: 5 }, "append 版の interval に置換");
        // id=2 が追加されている。
        assert!(m.animations.iter().any(|a| a.id == 2), "新 id=2 が追加される");
        // 合計 2 本（id=1 置換 ＋ id=2 追加）。
        assert_eq!(m.animations.len(), 2);
    }

    /// (3.3-4) 登場順（要件 2.3）。
    /// append の後に定義された surface は遡って追記されない（append 実行時に存在する id のみ）。
    #[test]
    fn append_only_sees_earlier_defined_surfaces() {
        // 登場順: Surface(a)→id=1, Append(x)→{1,2}, Surface(b)→id=2。
        let sa = surface_def(1, vec![AppendTarget::Single(1)], vec![elem(0, "a.png", 0, 0)]);
        let sb = surface_def(2, vec![AppendTarget::Single(2)], vec![elem(0, "b.png", 0, 0)]);
        let ap = append_def(
            vec![AppendTarget::Single(1), AppendTarget::Single(2)],
            vec![elem(1, "x.png", 9, 9)],
            Vec::new(),
            Vec::new(),
        );
        let mut world = fresh_world();
        fold_shell(
            &mut world,
            &shell_mixed(
                vec![sa, sb],
                vec![ap],
                // Surface(0)=id1, Append(0)=x, Surface(1)=id2。
                vec![DefRef::Surface(0), DefRef::Append(0), DefRef::Surface(1)],
            ),
        );

        // id=1 は append 実行時に存在 → x.png が届く（base + added の 2 本）。
        let m1 = master_of(&world, 1).expect("id=1 が常駐する");
        assert_eq!(m1.elements.len(), 2);
        assert!(m1.elements.iter().any(|e| e.path.as_str() == "x.png"));
        // id=2 は append の後に定義 → 遡及されない（base のみ 1 本）。
        let m2 = master_of(&world, 2).expect("id=2 が常駐する");
        assert_eq!(m2.elements.len(), 1);
        assert!(!m2.elements.iter().any(|e| e.path.as_str() == "x.png"));
        assert_eq!(m2.elements[0].path.as_str(), "b.png");
    }

    // ── task 3.4: 除外指定（`!N`/`!a-b`）の展開時減算 ─────────────────────────

    /// (3.4-1) 単一除外: `Single(0),Single(5),Exclude(5)` は 0 を残し 5 を減算する（要件 2.5）。
    #[test]
    fn expand_targets_subtracts_single_exclude() {
        let ids = expand_targets(&[
            AppendTarget::Single(0),
            AppendTarget::Single(5),
            AppendTarget::Exclude(5),
        ]);
        assert!(ids.contains(&0), "id=0 は残る");
        assert!(!ids.contains(&5), "id=5 は除外される");
        assert_eq!(ids, vec![0]);
    }

    /// (3.4-2) 範囲除外: `Range{1,5}, ExcludeRange{2,4}` は両端含みで 2,3,4 を減算し {1,5}（要件 2.5）。
    #[test]
    fn expand_targets_subtracts_exclude_range_inclusive() {
        let ids = expand_targets(&[
            AppendTarget::Range { start: 1, end: 5 },
            AppendTarget::ExcludeRange { start: 2, end: 4 },
        ]);
        assert_eq!(ids, vec![1, 5], "2,3,4 が両端含みで減算される");
    }

    /// (3.4-3) 除外なし回帰: `Range{1,3}` は減算ロジック導入後も {1,2,3} のまま変わらない。
    #[test]
    fn expand_targets_without_exclude_is_unchanged() {
        let ids = expand_targets(&[AppendTarget::Range { start: 1, end: 3 }]);
        assert_eq!(ids, vec![1, 2, 3]);
    }

    /// (3.4-4) 除外は記述位置に依らず集合全体へ効く（`!` 記述子が範囲の前でも減算される・design）。
    #[test]
    fn expand_targets_exclude_applies_regardless_of_position() {
        let ids = expand_targets(&[
            AppendTarget::Exclude(2),
            AppendTarget::Range { start: 1, end: 3 },
        ]);
        assert_eq!(ids, vec![1, 3], "前置の除外でも展開結果から減算される");
    }

    /// (3.4-5) fold 経由の統合: 除外を含む plain surface を fold すると除外 id が entity 化されない。
    /// `expand_targets` とその fold 呼び手の双方が除外を honor することを EmoWorld/SurfaceIndex で検証。
    #[test]
    fn fold_plain_surface_honors_exclusion_through_build() {
        // `surface1-4 !3` 相当（1,2,4 を新設・3 を除外）。
        let surf = surface_def(
            1,
            vec![
                AppendTarget::Range { start: 1, end: 4 },
                AppendTarget::Exclude(3),
            ],
            vec![elem(0, "s.png", 0, 0)],
        );
        let mut world = fresh_world();
        fold_shell(&mut world, &shell_of(vec![surf]));

        for id in [1u32, 2, 4] {
            assert!(master_of(&world, id).is_some(), "id={id} は新設される");
        }
        // 除外 id は entity 化されない。
        assert!(master_of(&world, 3).is_none(), "除外 id=3 は新設されない");
        assert_eq!(world.resource::<SurfaceIndex>().0.len(), 3);
    }

    // ── task 3.5: kero.surface.alias の解決（AliasMap 収集・後勝ち・非パニック） ──

    /// `Shell` を surface/alias 混在の登場順 `definitions` で組む（呼び手が DefRef を直接並べる）。
    fn shell_with_aliases(
        surfaces: Vec<Surface>,
        aliases: Vec<SurfaceAlias>,
        defs: Vec<DefRef>,
    ) -> Shell {
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases,
            animation_sort: None,
            collision_sort: None,
            definitions: defs,
        }
    }

    /// (3.5-1) 受入基準: 同一 alias キーが重複定義される（emo2 の `100,[2100]` が 2 回）とき、
    /// fold 結果は後勝ちの一意な値を返す（要件 3.2）。
    #[test]
    fn duplicate_alias_key_is_last_wins() {
        // key="100" を 2 回定義: 前 [2100]、後 [2100,9999]（後勝ちを識別できるよう別内容）。
        let aliases = vec![
            alias_def("100", vec![2100]),
            alias_def("100", vec![2100, 9999]),
        ];
        let mut world = fresh_world();
        fold_shell(
            &mut world,
            &shell_with_aliases(
                Vec::new(),
                aliases,
                vec![DefRef::Alias(0), DefRef::Alias(1)],
            ),
        );

        // 後の定義が勝つ（両者連結でなく一意な値）。
        assert_eq!(resolve_alias(&world, "100"), Some(&[2100, 9999][..]));
        // キーは 1 本のみ（重複しない）。
        assert_eq!(world.resource::<AliasMap>().0.len(), 1);
    }

    /// (3.5-1b) emo2 実相当: `100,[2100]` が 2 回同一値で定義されても後勝ちで一意（要件 3.2）。
    #[test]
    fn duplicate_alias_identical_value_collapses_to_one() {
        let aliases = vec![alias_def("100", vec![2100]), alias_def("100", vec![2100])];
        let mut world = fresh_world();
        fold_shell(
            &mut world,
            &shell_with_aliases(
                Vec::new(),
                aliases,
                vec![DefRef::Alias(0), DefRef::Alias(1)],
            ),
        );

        assert_eq!(resolve_alias(&world, "100"), Some(&[2100][..]));
        assert_eq!(world.resource::<AliasMap>().0.len(), 1);
    }

    /// (3.5-2) 順序付き id リスト保持（要件 3.1）: `[30,10,20]` はソートも重複除去もせず順序のまま。
    #[test]
    fn alias_id_list_order_is_preserved() {
        let aliases = vec![alias_def("k", vec![30, 10, 20])];
        let mut world = fresh_world();
        fold_shell(
            &mut world,
            &shell_with_aliases(Vec::new(), aliases, vec![DefRef::Alias(0)]),
        );

        assert_eq!(resolve_alias(&world, "k"), Some(&[30, 10, 20][..]));
    }

    /// (3.5-3) 未解決キーは None（パニックしない・要件 3.3）。
    /// AliasMap が他キーで populate 済みでも、不在キーは None を返す。
    #[test]
    fn absent_alias_key_returns_none_non_panic() {
        let aliases = vec![alias_def("present", vec![1, 2])];
        let mut world = fresh_world();
        fold_shell(
            &mut world,
            &shell_with_aliases(Vec::new(), aliases, vec![DefRef::Alias(0)]),
        );

        assert_eq!(resolve_alias(&world, "present"), Some(&[1, 2][..]));
        // 不在キーは None（populate 済みでも詰まらない）。
        assert_eq!(resolve_alias(&world, "nope"), None);
    }

    /// (3.5-4) single-pass 登場順収集: surface と alias が交互に並んでも両 alias が収集される（要件 1.7）。
    #[test]
    fn aliases_collected_when_interleaved_with_surfaces() {
        let s0 = surface_def(0, vec![AppendTarget::Single(0)], vec![elem(0, "s0.png", 0, 0)]);
        let s1 = surface_def(1, vec![AppendTarget::Single(1)], vec![elem(0, "s1.png", 0, 0)]);
        let a0 = alias_def("alpha", vec![100, 101]);
        let a1 = alias_def("beta", vec![200]);
        let mut world = fresh_world();
        // 登場順: Surface, Alias, Surface, Alias。
        fold_shell(
            &mut world,
            &shell_with_aliases(
                vec![s0, s1],
                vec![a0, a1],
                vec![
                    DefRef::Surface(0),
                    DefRef::Alias(0),
                    DefRef::Surface(1),
                    DefRef::Alias(1),
                ],
            ),
        );

        // 両 alias が収集されている。
        assert_eq!(resolve_alias(&world, "alpha"), Some(&[100, 101][..]));
        assert_eq!(resolve_alias(&world, "beta"), Some(&[200][..]));
        // surface も並行して常駐する（alias 収集が surface fold を壊さない）。
        assert!(master_of(&world, 0).is_some());
        assert!(master_of(&world, 1).is_some());
        assert_eq!(world.resource::<AliasMap>().0.len(), 2);
    }

    /// (3.5-5) 範囲外 index は非パニックでスキップ（要件 1.4・Surface/Append と同じ寛容パターン）。
    #[test]
    fn alias_out_of_range_index_is_skipped() {
        let mut world = fresh_world();
        // aliases 空だが DefRef::Alias(0) を指す（範囲外）。
        fold_shell(
            &mut world,
            &shell_with_aliases(Vec::new(), Vec::new(), vec![DefRef::Alias(0)]),
        );

        // パニックせず AliasMap は空のまま。
        assert_eq!(world.resource::<AliasMap>().0.len(), 0);
    }

    // ── task 3.7: 決定性（バイト等価）・登場順 single-pass の固定（要件 1.5/1.7/2.3/10.1）──

    /// fold 結果の観測可能内容スナップショット（`EmoWorld` は derive Eq を持たないため手組み）。
    ///
    /// (1) surface id 昇順（`SurfaceIndex` のキーを整列＝`EmoWorld::surface_ids` と同じ決定的順）に
    /// `(id, SurfaceMaster)` を集める。`SurfaceMaster` は `PartialEq`（element/collision/animation の
    /// 深い等価）。(2) alias は `AliasMap`（`BTreeMap`＝キー順が決定的）を丸ごと複製する。
    /// この2値が等しければ、公開クエリ（`surface(id)`／`resolve_alias(key)`／`surface_ids()`）越しに
    /// 観測できる内容がバイト等価であることを意味する（要件 1.5/10.1）。
    type FoldSnapshot = (Vec<(u32, SurfaceMaster)>, std::collections::BTreeMap<String, Vec<u32>>);

    /// 素の `World`（`fold_shell` 直呼び）から決定性比較用スナップショットを抽出する。
    fn snapshot_world(world: &World) -> FoldSnapshot {
        // surface id を昇順に整列（`SurfaceIndex` の HashMap 反復順に依らず決定的）。
        let mut ids: Vec<u32> = world.resource::<SurfaceIndex>().0.keys().copied().collect();
        ids.sort_unstable();
        let surfaces: Vec<(u32, SurfaceMaster)> = ids
            .into_iter()
            .map(|id| (id, master_of(world, id).expect("id が常駐する").clone()))
            .collect();
        // alias は BTreeMap ゆえ複製すればキー順まで決定的。
        let aliases = world.resource::<AliasMap>().0.clone();
        (surfaces, aliases)
    }

    /// 同一 `Shell` を素の `World` へ2回 fold し、スナップショットが一致することを確かめるヘルパ。
    fn assert_fold_twice_byte_equal(shell: &Shell) -> FoldSnapshot {
        let mut w1 = fresh_world();
        fold_shell(&mut w1, shell);
        let mut w2 = fresh_world();
        fold_shell(&mut w2, shell);
        let s1 = snapshot_world(&w1);
        let s2 = snapshot_world(&w2);
        assert_eq!(s1, s2, "同一 Shell の2回 fold はバイト等価な内容を生む（要件 1.5/10.1）");
        s1
    }

    /// emo2 fixture（実上流シェル）の `surfaces.txt` を UTF-8 で読み parse して `Shell` を得る。
    ///
    /// `emo2_e2e.rs`（emo-atlas）の読込経路と同一（`read_to_string`＋`areka_parsers::shell::parse`）を
    /// 本 crate から辿る。COM／WIC／atlas bake は経由しない純粋な parse+fold ゆえ headless で走る。
    fn emo2_shell() -> Shell {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
        areka_parsers::shell::parse(&content)
    }

    /// (3.7-1) 受入基準（要件 1.5/10.1）: 複数定義が重なる**手組み** Shell を2回 fold すると
    /// 得られる EmoWorld 内容がバイト等価。emo2 の重なり構造（plain 群→多 id append→重複 alias）を
    /// ミラーした入力で決定性を固定する（実 fixture とは別に、パーサ非依存で保証）。
    #[test]
    fn hand_built_overlap_shell_folds_byte_equal_twice() {
        // plain: 10, 2100, 2101（append の対象になり得る既存 id）。
        let s10 = surface_def(10, vec![AppendTarget::Single(10)], vec![elem(0, "b10.png", 0, 0)]);
        let s2100 = surface_full(
            2100,
            vec![AppendTarget::Single(2100)],
            vec![elem(0, "b2100.png", 0, 0)],
            vec![coll(0, "Head")],
            vec![anim(1)],
        );
        let s2101 =
            surface_def(2101, vec![AppendTarget::Single(2101)], vec![elem(0, "b2101.png", 0, 0)]);
        // 多 id 範囲 append（emo2 の `surface.append10,2100-2110,2200-2210` 相当）＝既存のみへ追記。
        let ap = append_def(
            vec![
                AppendTarget::Single(10),
                AppendTarget::Range { start: 2100, end: 2110 },
                AppendTarget::Range { start: 2200, end: 2210 },
            ],
            vec![elem(1, "added.png", 3, 4)],
            vec![coll(1, "Bust")],
            vec![anim(2)],
        );
        // 重複 alias（emo2 の `100,[2100]` が2回）＝後勝ちで一意。
        let mut shell = shell_mixed(
            vec![s10, s2100, s2101],
            vec![ap],
            vec![
                DefRef::Surface(0),
                DefRef::Surface(1),
                DefRef::Surface(2),
                DefRef::Append(0),
            ],
        );
        shell.aliases = vec![
            alias_def("100", vec![2100]),
            alias_def("通常", vec![2100]),
            alias_def("100", vec![2100]),
        ];
        // 登場順: plain×3 → append → alias×3。
        shell.definitions.extend([
            DefRef::Alias(0),
            DefRef::Alias(1),
            DefRef::Alias(2),
        ]);

        let snap = assert_fold_twice_byte_equal(&shell);
        // fold が非自明（重なりが実際に効いている）ことを確かめる。
        let (surfaces, aliases) = &snap;
        // 常駐 id は昇順 [10, 2100, 2101]（append は新設しない）。
        let ids: Vec<u32> = surfaces.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![10, 2100, 2101], "append は既存3件のみに効き新設しない");
        // append 対象の 2100 には base + added の element が届く。
        let m2100 = &surfaces.iter().find(|(id, _)| *id == 2100).unwrap().1;
        assert!(
            m2100.elements.iter().any(|e| e.path.as_str() == "added.png"),
            "append element が 2100 へ届く"
        );
        // 重複 alias は後勝ちで一意（キー数＝distinct キー数）。
        assert_eq!(aliases.get("100"), Some(&vec![2100]));
        assert_eq!(aliases.len(), 2, "重複キー 100 は畳まれ distinct 2 本（100/通常）");
    }

    /// (3.7-2) 実 emo2 fixture の fold 決定性（要件 1.7/2.3/10.1）。
    ///
    /// 実 `surfaces.txt` を parse → `EmoWorld::build` を2回。公開クエリ越しのスナップショット
    /// （surface 群・alias 群・両 sort 順）がバイト等価であることを固定する。これは実登場順ストリーム
    /// （plain 群 → 多 id range append → 重複キー alias）に対する single-pass fold が決定的である証左。
    #[test]
    fn emo2_fixture_folds_byte_equal_twice() {
        use crate::world::EmoWorld;

        let shell = emo2_shell();
        // fold は非自明: 実 fixture は多数の surface と alias を含む。
        assert!(!shell.surfaces.is_empty(), "emo2 は surface を含む");
        assert!(!shell.definitions.is_empty(), "emo2 は登場順ストリームを持つ");

        let a = EmoWorld::build(&shell);
        let b = EmoWorld::build(&shell);

        // 公開クエリ越しのスナップショット（surface_ids は昇順＝決定的）。
        let snap = |w: &EmoWorld| -> FoldSnapshot {
            let surfaces: Vec<(u32, SurfaceMaster)> = w
                .surface_ids()
                .map(|id| (id, w.surface(id).expect("列挙 id は常駐する").clone()))
                .collect();
            // alias は既知キーのうち代表を安定順で引く（BTreeMap 実体の全複製は非公開のため
            // 公開 resolve_alias 経路で決定性を観測する）。
            let mut aliases = std::collections::BTreeMap::new();
            for key in ["0", "通常", "100", "200", "10"] {
                if let Some(ids) = w.resolve_alias(key) {
                    aliases.insert(key.to_string(), ids.to_vec());
                }
            }
            (surfaces, aliases)
        };
        assert_eq!(snap(&a), snap(&b), "実 emo2 の2回 fold はバイト等価（要件 10.1）");

        // sort 順も決定的（emo2 は sort 未指定＝既定）。
        assert_eq!(a.animation_sort(), b.animation_sort());
        assert_eq!(a.collision_sort(), b.collision_sort());

        // 非自明性: 既知 id が常駐し、重複 alias `100` が後勝ちで一意解決する。
        let ids: Vec<u32> = a.surface_ids().collect();
        assert!(ids.contains(&0), "surface0 が常駐する");
        assert!(ids.contains(&1000), "surface1000（全 bind）が常駐する");
        assert!(ids.contains(&2100), "surface2100 が常駐する");
        // `100,[2100]` は2回定義されるが後勝ちで単一値に解決する（要件 3.2・重複ケース）。
        assert_eq!(a.resolve_alias("100"), Some(&[2100][..]), "重複 alias 100 は後勝ち一意");
        // 多 id range append は既存 2100-2110 へ collision を届けるが、範囲外の未存在は新設しない。
        assert!(!ids.contains(&2211), "append 範囲外の未存在 id は新設されない");
        // append 対象 2100 に collision（Head/Bust）が届く（append の重なりが観測できる）。
        let m2100 = a.surface(2100).expect("surface2100 常駐");
        assert!(
            m2100.collisions.iter().any(|c| c.name.as_str() == "Head"),
            "surface.append の collision が 2100 へ届く（登場順の重なり適用）"
        );
    }

    /// (3.7-3) 登場順の決定的適用（要件 2.3）: 同一 surface id を surface→append→surface の順で
    /// 交互に定義したとき、fold は登場順どおりに畳み込まれ、2回 fold して同一結果になる。
    #[test]
    fn interleaved_redefinition_is_appearance_order_deterministic() {
        // 登場順: Surface(id=5, old) → Append(id=5, +mid) → Surface(id=5, new・全置換)。
        let s_old = surface_def(5, vec![AppendTarget::Single(5)], vec![elem(0, "old.png", 0, 0)]);
        let ap = append_def(
            vec![AppendTarget::Single(5)],
            vec![elem(1, "mid.png", 1, 1)],
            Vec::new(),
            Vec::new(),
        );
        let s_new = surface_def(5, vec![AppendTarget::Single(5)], vec![elem(0, "new.png", 9, 9)]);
        let shell = shell_mixed(
            vec![s_old, s_new],
            vec![ap],
            // Surface(0)=old, Append(0)=+mid, Surface(1)=new。
            vec![DefRef::Surface(0), DefRef::Append(0), DefRef::Surface(1)],
        );

        let (surfaces, _) = assert_fold_twice_byte_equal(&shell);
        // 登場順の帰結: 最後の plain surface(new) が全置換 → old も途中 append の mid も消える。
        assert_eq!(surfaces.len(), 1);
        let (id, m) = &surfaces[0];
        assert_eq!(*id, 5);
        assert_eq!(m.elements.len(), 1, "最後の全置換で element は1本のみ");
        assert_eq!(m.elements[0].path.as_str(), "new.png");
        assert!(
            !m.elements.iter().any(|e| e.path.as_str() == "mid.png"),
            "全置換前の append(mid) は残らない（登場順の帰結）"
        );
    }

    /// (3.7-4) 前方参照なし・single-pass（要件 1.7）: 後で定義される id を狙う append は遡及せず、
    /// かつ 2回 fold して結果が安定する。
    #[test]
    fn forward_reference_append_does_not_apply_and_is_stable() {
        // 登場順: Append(狙い id=3) → Surface(id=3)。append 時点で 3 は未存在ゆえ届かない。
        let ap = append_def(
            vec![AppendTarget::Single(3)],
            vec![elem(1, "ghost.png", 0, 0)],
            Vec::new(),
            Vec::new(),
        );
        let s3 = surface_def(3, vec![AppendTarget::Single(3)], vec![elem(0, "base3.png", 0, 0)]);
        let shell = shell_mixed(
            vec![s3],
            vec![ap],
            // Append(0) が Surface(0) より先＝前方参照。
            vec![DefRef::Append(0), DefRef::Surface(0)],
        );

        let (surfaces, _) = assert_fold_twice_byte_equal(&shell);
        // id=3 は base のみ（前方参照 append は適用されない）。
        assert_eq!(surfaces.len(), 1);
        let (id, m) = &surfaces[0];
        assert_eq!(*id, 3);
        assert_eq!(m.elements.len(), 1, "前方参照 append は適用されない（single-pass）");
        assert_eq!(m.elements[0].path.as_str(), "base3.png");
        assert!(
            !m.elements.iter().any(|e| e.path.as_str() == "ghost.png"),
            "後定義 id を狙う前方参照 append は遡及しない"
        );
    }
}
