use areka_parsers::shell::{
    AliasKey, Animation, AppendTarget, Collision, DefRef, Element, Interval, Shell, Surface,
    SurfaceAlias, SurfaceAppend,
};
use bevy_ecs::world::World;

use crate::fold::{expand_targets_with, fold_shell_with};
use crate::method::ComposeMethod;
use crate::normalized::SurfaceMaster;
use crate::plan_test_support::{
    FoldSnapshot, coll, component_of, elem, fresh_world, shell_mixed, shell_of, snapshot_world,
    surface_def,
};
use crate::sample_test_support::emo2_shell;
use crate::world::{AliasMap, Records, SurfaceIndex};

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

/// animation 1 本（interval=bind・pattern 群空・id と識別のみに用いる）。
fn anim(id: u32) -> Animation {
    Animation {
        id,
        interval: Interval::Bind,
        interval_written: true,
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
    fold_shell_with(&mut world, &shell_of(vec![surf]), Records::Emit);

    let m0 = component_of::<SurfaceMaster>(&world, 0).expect("id=0 が常駐する");
    let m5 = component_of::<SurfaceMaster>(&world, 5).expect("id=5 が常駐する");
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
    fold_shell_with(&mut world, &shell_of(vec![surf]), Records::Emit);

    for id in [1u32, 2, 3] {
        let m = component_of::<SurfaceMaster>(&world, id)
            .unwrap_or_else(|| panic!("id={id} が常駐する"));
        assert_eq!(m.id, id);
        assert_eq!(m.elements.len(), 1);
    }
    assert!(component_of::<SurfaceMaster>(&world, 0).is_none());
    assert!(component_of::<SurfaceMaster>(&world, 4).is_none());
    assert_eq!(world.resource::<SurfaceIndex>().0.len(), 3);
}

/// (c) 単一形 `surface0`: 従来どおり 1 件だけ新設される。
#[test]
fn single_form_creates_one_surface() {
    let surf = surface_def(
        0,
        vec![AppendTarget::Single(0)],
        vec![elem(0, "s0.png", 1, 2)],
    );
    let mut world = fresh_world();
    fold_shell_with(&mut world, &shell_of(vec![surf]), Records::Emit);

    let m = component_of::<SurfaceMaster>(&world, 0).expect("id=0 が常駐する");
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
    assert_eq!(
        expand_targets_with(&surf.targets, Records::Emit),
        vec![2, 1],
        "記述の順"
    );
    let mut world = fresh_world();
    fold_shell_with(&mut world, &shell_of(vec![surf]), Records::Emit);

    assert!(component_of::<SurfaceMaster>(&world, 2).is_some());
    assert!(component_of::<SurfaceMaster>(&world, 1).is_some());
    assert_eq!(world.resource::<SurfaceIndex>().0.len(), 2);
}

// ── surface.append の展開＝既存 id 限定の追記 ──────────────────

/// (1) 受入基準: `surface.append10,2100-2110,2200-2210` 相当。
/// 事前に存在する id のみへ追記し、範囲内でも未存在 id は新設しない（要件 2.2）。
#[test]
fn append_reaches_existing_ids_only_not_creating_absent() {
    // 事前 plain surface: id=10, id=2100, id=2101（2102-2110 と 2200-2210 は未存在）。
    let s10 = surface_def(
        10,
        vec![AppendTarget::Single(10)],
        vec![elem(0, "base10.png", 0, 0)],
    );
    let s2100 = surface_def(
        2100,
        vec![AppendTarget::Single(2100)],
        vec![elem(0, "b2100.png", 0, 0)],
    );
    let s2101 = surface_def(
        2101,
        vec![AppendTarget::Single(2101)],
        vec![elem(0, "b2101.png", 0, 0)],
    );
    // append: 10, 2100-2110, 2200-2210（layer=1 の追記 element を持つ）。
    let ap = append_def(
        vec![
            AppendTarget::Single(10),
            AppendTarget::Range {
                start: 2100,
                end: 2110,
            },
            AppendTarget::Range {
                start: 2200,
                end: 2210,
            },
        ],
        vec![elem(1, "added.png", 3, 4)],
        Vec::new(),
        Vec::new(),
    );
    let mut world = fresh_world();
    // 登場順: Surface(10), Surface(2100), Surface(2101), Append(ap)。
    fold_shell_with(
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
        Records::Emit,
    );

    // 既存 id には追記 element が届く（base + added の 2 本）。
    for id in [10u32, 2100, 2101] {
        let m = component_of::<SurfaceMaster>(&world, id)
            .unwrap_or_else(|| panic!("id={id} が常駐する"));
        assert_eq!(m.elements.len(), 2, "id={id} に追記 element が届く");
        // 追記 element が含まれる。
        assert!(
            m.elements.iter().any(|e| e.path.as_str() == "added.png"),
            "id={id} に added.png が追記される"
        );
    }
    // 範囲内でも未存在だった id は新設されない。
    for id in [2102u32, 2110, 2200, 2205, 2210] {
        assert!(
            component_of::<SurfaceMaster>(&world, id).is_none(),
            "id={id} は新設されない"
        );
    }
    // entity 数は事前 3 件のまま（append は新設しない）。
    assert_eq!(world.resource::<SurfaceIndex>().0.len(), 3);
}

/// (2) element/collision マージ（要件 2.4）。
/// append 追記後、element は layer 昇順を保ち、collision は末尾連結される。
#[test]
fn append_merges_element_and_collision_layer_sorted() {
    // 事前 surface: layer=2 の element ＋ collision 1 本。
    let base = surface_full(
        5,
        vec![AppendTarget::Single(5)],
        vec![elem(2, "hi.png", 0, 0)],
        vec![coll(0, 0, 0, 10, 10, "Head")],
        Vec::new(),
    );
    // append: layer=0 の element（base より低い layer）＋ collision 1 本。
    let ap = append_def(
        vec![AppendTarget::Single(5)],
        vec![elem(0, "lo.png", 1, 1)],
        vec![coll(1, 0, 0, 10, 10, "Bust")],
        Vec::new(),
    );
    let mut world = fresh_world();
    fold_shell_with(
        &mut world,
        &shell_mixed(
            vec![base],
            vec![ap],
            vec![DefRef::Surface(0), DefRef::Append(0)],
        ),
        Records::Emit,
    );

    let m = component_of::<SurfaceMaster>(&world, 5).expect("id=5 が常駐する");
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

/// (3) 登場順（要件 2.3）。
/// append の後に定義された surface は遡って追記されない（append 実行時に存在する id のみ）。
#[test]
fn append_only_sees_earlier_defined_surfaces() {
    // 登場順: Surface(a)→id=1, Append(x)→{1,2}, Surface(b)→id=2。
    let sa = surface_def(
        1,
        vec![AppendTarget::Single(1)],
        vec![elem(0, "a.png", 0, 0)],
    );
    let sb = surface_def(
        2,
        vec![AppendTarget::Single(2)],
        vec![elem(0, "b.png", 0, 0)],
    );
    let ap = append_def(
        vec![AppendTarget::Single(1), AppendTarget::Single(2)],
        vec![elem(1, "x.png", 9, 9)],
        Vec::new(),
        Vec::new(),
    );
    let mut world = fresh_world();
    fold_shell_with(
        &mut world,
        &shell_mixed(
            vec![sa, sb],
            vec![ap],
            // Surface(0)=id1, Append(0)=x, Surface(1)=id2。
            vec![DefRef::Surface(0), DefRef::Append(0), DefRef::Surface(1)],
        ),
        Records::Emit,
    );

    // id=1 は append 実行時に存在 → x.png が届く（base + added の 2 本）。
    let m1 = component_of::<SurfaceMaster>(&world, 1).expect("id=1 が常駐する");
    assert_eq!(m1.elements.len(), 2);
    assert!(m1.elements.iter().any(|e| e.path.as_str() == "x.png"));
    // id=2 は append の後に定義 → 遡及されない（base のみ 1 本）。
    let m2 = component_of::<SurfaceMaster>(&world, 2).expect("id=2 が常駐する");
    assert_eq!(m2.elements.len(), 1);
    assert_eq!(m2.elements[0].path.as_str(), "b.png");
}

// ── 除外指定（`!N`/`!a-b`）の展開時減算 ─────────────────────────

/// (1) 単一除外: `Single(0),Single(5),Exclude(5)` は 0 を残し 5 を減算する（要件 2.5）。
#[test]
fn expand_targets_subtracts_single_exclude() {
    let ids = expand_targets_with(
        &[
            AppendTarget::Single(0),
            AppendTarget::Single(5),
            AppendTarget::Exclude(5),
        ],
        Records::Emit,
    );
    assert_eq!(ids, vec![0], "id=0 は残り、id=5 は除外される");
}

/// (2) 範囲除外: `Range{1,5}, ExcludeRange{2,4}` は両端含みで 2,3,4 を減算し {1,5}（要件 2.5）。
#[test]
fn expand_targets_subtracts_exclude_range_inclusive() {
    let ids = expand_targets_with(
        &[
            AppendTarget::Range { start: 1, end: 5 },
            AppendTarget::ExcludeRange { start: 2, end: 4 },
        ],
        Records::Emit,
    );
    assert_eq!(ids, vec![1, 5], "2,3,4 が両端含みで減算される");
}

/// (3) 除外なし回帰: `Range{1,3}` は減算ロジック導入後も {1,2,3} のまま変わらない。
#[test]
fn expand_targets_without_exclude_is_unchanged() {
    let ids = expand_targets_with(&[AppendTarget::Range { start: 1, end: 3 }], Records::Emit);
    assert_eq!(ids, vec![1, 2, 3]);
}

/// (4) 除外は記述位置に依らず集合全体へ効く（`!` 記述子が範囲の前でも減算される・design）。
#[test]
fn expand_targets_exclude_applies_regardless_of_position() {
    let ids = expand_targets_with(
        &[
            AppendTarget::Exclude(2),
            AppendTarget::Range { start: 1, end: 3 },
        ],
        Records::Emit,
    );
    assert_eq!(ids, vec![1, 3], "前置の除外でも展開結果から減算される");
}

/// (5) fold 経由の統合: 除外を含む plain surface を fold すると除外 id が entity 化されない。
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
    fold_shell_with(&mut world, &shell_of(vec![surf]), Records::Emit);

    for id in [1u32, 2, 4] {
        assert!(
            component_of::<SurfaceMaster>(&world, id).is_some(),
            "id={id} は新設される"
        );
    }
    // 除外 id は entity 化されない。
    assert!(
        component_of::<SurfaceMaster>(&world, 3).is_none(),
        "除外 id=3 は新設されない"
    );
    assert_eq!(world.resource::<SurfaceIndex>().0.len(), 3);
}

// ── kero.surface.alias の解決（AliasMap 収集・後勝ち・非パニック） ──

/// `Shell` を surface/alias 混在の登場順 `definitions` で組む（呼び手が DefRef を直接並べる）。
fn shell_with_aliases(
    surfaces: Vec<Surface>,
    aliases: Vec<SurfaceAlias>,
    defs: Vec<DefRef>,
) -> Shell {
    Shell {
        aliases,
        ..shell_mixed(surfaces, Vec::new(), defs)
    }
}

/// (1) 受入基準: 同一 alias キーが重複定義される（emo2 の `100,[2100]` が 2 回）とき、
/// fold 結果は後勝ちの一意な値を返す（要件 3.2）。
#[test]
fn duplicate_alias_key_is_last_wins() {
    // key="100" を 2 回定義: 前 [2100]、後 [2100,9999]（後勝ちを識別できるよう別内容）。
    let aliases = vec![
        alias_def("100", vec![2100]),
        alias_def("100", vec![2100, 9999]),
    ];
    let mut world = fresh_world();
    fold_shell_with(
        &mut world,
        &shell_with_aliases(
            Vec::new(),
            aliases,
            vec![DefRef::Alias(0), DefRef::Alias(1)],
        ),
        Records::Emit,
    );

    // 後の定義が勝つ（両者連結でなく一意な値）。
    assert_eq!(resolve_alias(&world, "100"), Some(&[2100, 9999][..]));
    // キーは 1 本のみ（重複しない）。
    assert_eq!(world.resource::<AliasMap>().0.len(), 1);
}

/// (1b) emo2 実相当: `100,[2100]` が 2 回同一値で定義されても後勝ちで一意（要件 3.2）。
#[test]
fn duplicate_alias_identical_value_collapses_to_one() {
    let aliases = vec![alias_def("100", vec![2100]), alias_def("100", vec![2100])];
    let mut world = fresh_world();
    fold_shell_with(
        &mut world,
        &shell_with_aliases(
            Vec::new(),
            aliases,
            vec![DefRef::Alias(0), DefRef::Alias(1)],
        ),
        Records::Emit,
    );

    assert_eq!(resolve_alias(&world, "100"), Some(&[2100][..]));
    assert_eq!(world.resource::<AliasMap>().0.len(), 1);
}

/// (2) 順序付き id リスト保持（要件 3.1）: `[30,10,20]` はソートも重複除去もせず順序のまま。
#[test]
fn alias_id_list_order_is_preserved() {
    let aliases = vec![alias_def("k", vec![30, 10, 20])];
    let mut world = fresh_world();
    fold_shell_with(
        &mut world,
        &shell_with_aliases(Vec::new(), aliases, vec![DefRef::Alias(0)]),
        Records::Emit,
    );

    assert_eq!(resolve_alias(&world, "k"), Some(&[30, 10, 20][..]));
}

/// (3) 未解決キーは None（パニックしない・要件 3.3）。
/// AliasMap が他キーで populate 済みでも、不在キーは None を返す。
#[test]
fn absent_alias_key_returns_none_non_panic() {
    let aliases = vec![alias_def("present", vec![1, 2])];
    let mut world = fresh_world();
    fold_shell_with(
        &mut world,
        &shell_with_aliases(Vec::new(), aliases, vec![DefRef::Alias(0)]),
        Records::Emit,
    );

    assert_eq!(resolve_alias(&world, "present"), Some(&[1, 2][..]));
    // 不在キーは None（populate 済みでも詰まらない）。
    assert_eq!(resolve_alias(&world, "nope"), None);
}

/// (4) single-pass 登場順収集: surface と alias が交互に並んでも両 alias が収集される（要件 1.7）。
#[test]
fn aliases_collected_when_interleaved_with_surfaces() {
    let s0 = surface_def(
        0,
        vec![AppendTarget::Single(0)],
        vec![elem(0, "s0.png", 0, 0)],
    );
    let s1 = surface_def(
        1,
        vec![AppendTarget::Single(1)],
        vec![elem(0, "s1.png", 0, 0)],
    );
    let a0 = alias_def("alpha", vec![100, 101]);
    let a1 = alias_def("beta", vec![200]);
    let mut world = fresh_world();
    // 登場順: Surface, Alias, Surface, Alias。
    fold_shell_with(
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
        Records::Emit,
    );

    // 両 alias が収集されている。
    assert_eq!(resolve_alias(&world, "alpha"), Some(&[100, 101][..]));
    assert_eq!(resolve_alias(&world, "beta"), Some(&[200][..]));
    // surface も並行して常駐する（alias 収集が surface fold を壊さない）。
    assert!(component_of::<SurfaceMaster>(&world, 0).is_some());
    assert!(component_of::<SurfaceMaster>(&world, 1).is_some());
    assert_eq!(world.resource::<AliasMap>().0.len(), 2);
}

/// (5) 範囲外 index は非パニックでスキップ（要件 1.4・Surface/Append と同じ寛容パターン）。
#[test]
fn alias_out_of_range_index_is_skipped() {
    let mut world = fresh_world();
    // aliases 空だが DefRef::Alias(0) を指す（範囲外）。
    fold_shell_with(
        &mut world,
        &shell_with_aliases(Vec::new(), Vec::new(), vec![DefRef::Alias(0)]),
        Records::Emit,
    );

    // パニックせず AliasMap は空のまま。
    assert_eq!(world.resource::<AliasMap>().0.len(), 0);
}

// ── 決定性（バイト等価）・登場順 single-pass の固定（要件 1.5/1.7/2.3/10.1）──

/// 同一 `Shell` を素の `World` へ2回 fold し、スナップショットが一致することを確かめるヘルパ。
fn assert_fold_twice_byte_equal(shell: &Shell) -> FoldSnapshot {
    let mut w1 = fresh_world();
    fold_shell_with(&mut w1, shell, Records::Emit);
    let mut w2 = fresh_world();
    fold_shell_with(&mut w2, shell, Records::Emit);
    let s1 = snapshot_world(&w1);
    let s2 = snapshot_world(&w2);
    assert_eq!(
        s1, s2,
        "同一 Shell の2回 fold はバイト等価な内容を生む（要件 1.5/10.1）"
    );
    s1
}

/// (1) 受入基準（要件 1.5/10.1）: 複数定義が重なる**手組み** Shell を2回 fold すると
/// 得られる EmoWorld 内容がバイト等価。emo2 の重なり構造（plain 群→多 id append→重複 alias）を
/// ミラーした入力で決定性を固定する（実 fixture とは別に、パーサ非依存で保証）。
#[test]
fn hand_built_overlap_shell_folds_byte_equal_twice() {
    // plain: 10, 2100, 2101（append の対象になり得る既存 id）。
    let s10 = surface_def(
        10,
        vec![AppendTarget::Single(10)],
        vec![elem(0, "b10.png", 0, 0)],
    );
    let s2100 = surface_full(
        2100,
        vec![AppendTarget::Single(2100)],
        vec![elem(0, "b2100.png", 0, 0)],
        vec![coll(0, 0, 0, 10, 10, "Head")],
        vec![anim(1)],
    );
    let s2101 = surface_def(
        2101,
        vec![AppendTarget::Single(2101)],
        vec![elem(0, "b2101.png", 0, 0)],
    );
    // 多 id 範囲 append（emo2 の `surface.append10,2100-2110,2200-2210` 相当）＝既存のみへ追記。
    let ap = append_def(
        vec![
            AppendTarget::Single(10),
            AppendTarget::Range {
                start: 2100,
                end: 2110,
            },
            AppendTarget::Range {
                start: 2200,
                end: 2210,
            },
        ],
        vec![elem(1, "added.png", 3, 4)],
        vec![coll(1, 0, 0, 10, 10, "Bust")],
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
    shell
        .definitions
        .extend([DefRef::Alias(0), DefRef::Alias(1), DefRef::Alias(2)]);

    let snap = assert_fold_twice_byte_equal(&shell);
    // fold が非自明（重なりが実際に効いている）ことを確かめる。
    let (surfaces, aliases) = &snap;
    // 常駐 id は昇順 [10, 2100, 2101]（append は新設しない）。
    let ids: Vec<u32> = surfaces.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        ids,
        vec![10, 2100, 2101],
        "append は既存3件のみに効き新設しない"
    );
    // append 対象の 2100 には base + added の element が届く。
    let m2100 = &surfaces.iter().find(|(id, _)| *id == 2100).unwrap().1;
    assert!(
        m2100
            .elements
            .iter()
            .any(|e| e.path.as_str() == "added.png"),
        "append element が 2100 へ届く"
    );
    // 重複 alias は後勝ちで一意（キー数＝distinct キー数）。
    assert_eq!(aliases.get("100"), Some(&vec![2100]));
    assert_eq!(
        aliases.len(),
        2,
        "重複キー 100 は畳まれ distinct 2 本（100/通常）"
    );
}

/// (2) 実 emo2 fixture の fold 決定性（要件 1.7/2.3/10.1）。
///
/// 実 `surfaces.txt` を parse → `EmoWorld::build` を2回。公開クエリ越しのスナップショット
/// （surface 群・alias 群・両 sort 順）がバイト等価であることを固定する。これは実登場順ストリーム
/// （plain 群 → 多 id range append → 重複キー alias）に対する single-pass fold が決定的である証左。
#[test]
fn emo2_fixture_folds_byte_equal_twice() {
    use crate::world::EmoWorld;

    let shell = emo2_shell();
    let a = EmoWorld::build(&shell);
    let b = EmoWorld::build(&shell);

    // 公開クエリ越しのスナップショット（surface_ids は昇順＝決定的）。
    let snap = |w: &EmoWorld| -> FoldSnapshot {
        let surfaces: Vec<(u32, SurfaceMaster)> = w
            .surface_ids()
            .map(|id| (id, w.surface(id).expect("列挙 id は常駐する").clone()))
            .collect();
        (surfaces, w.alias_snapshot())
    };
    assert_eq!(
        snap(&a),
        snap(&b),
        "実 emo2 の2回 fold はバイト等価（要件 10.1）"
    );

    // 非自明性: 既知 id が常駐し、重複 alias `100` が後勝ちで一意解決する。
    let ids: Vec<u32> = a.surface_ids().collect();
    assert!(ids.contains(&0), "surface0 が常駐する");
    assert!(
        ids.contains(&1000),
        "surface1000（着せ替えの本体）が常駐する"
    );
    assert!(ids.contains(&2100), "surface2100 が常駐する");
    // `100,[2100]` は2回定義されるが後勝ちで単一値に解決する（要件 3.2・重複ケース）。
    assert_eq!(
        a.resolve_alias("100"),
        Some(&[2100][..]),
        "重複 alias 100 は後勝ち一意"
    );
    // append 対象 2100 に collision（Head/Bust）が届く（append の重なりが観測できる・範囲内でも未存在の
    // id を新設しないことは手組みの検体で見る）。
    let m2100 = a.surface(2100).expect("surface2100 常駐");
    assert!(
        m2100.collisions.iter().any(|c| c.name.as_str() == "Head"),
        "surface.append の collision が 2100 へ届く（登場順の重なり適用）"
    );
}

/// (3) 登場順の決定的適用（要件 2.3）: 同一 surface id を surface→append→surface の順で
/// 交互に定義したとき、fold は登場順どおりに畳み込まれ、2回 fold して同一結果になる。
#[test]
fn interleaved_redefinition_is_appearance_order_deterministic() {
    // 登場順: Surface(id=5, old) → Append(id=5, +mid) → Surface(id=5, new・前へ足す)。
    let s_old = surface_def(
        5,
        vec![AppendTarget::Single(5)],
        vec![elem(0, "old.png", 0, 0)],
    );
    let ap = append_def(
        vec![AppendTarget::Single(5)],
        vec![elem(1, "mid.png", 1, 1)],
        Vec::new(),
        Vec::new(),
    );
    let s_new = surface_def(
        5,
        vec![AppendTarget::Single(5)],
        vec![elem(0, "new.png", 9, 9)],
    );
    let shell = shell_mixed(
        vec![s_old, s_new],
        vec![ap],
        // Surface(0)=old, Append(0)=+mid, Surface(1)=new。
        vec![DefRef::Surface(0), DefRef::Append(0), DefRef::Surface(1)],
    );

    let (surfaces, _) = assert_fold_twice_byte_equal(&shell);
    // 登場順の帰結: 最後の plain surface(new) は前へ足す → 同じ layer 0 は old・new の両方
    // （前の行が先）、途中 append の mid（layer 1）も残る。
    assert_eq!(surfaces.len(), 1);
    let (id, m) = &surfaces[0];
    assert_eq!(*id, 5);
    let layers: Vec<(u32, &str)> = m
        .elements
        .iter()
        .map(|e| (e.layer, e.path.as_str()))
        .collect();
    assert_eq!(
        layers,
        vec![(0, "old.png"), (0, "new.png"), (1, "mid.png")],
        "layer 0 は両方・append(mid) は残る（登場順の帰結）"
    );
}

/// 指摘 75: 除外の範囲は番号へ広げずに引く。`surface0,!0-4294967295` は短く終わり、0 は除外される。
#[test]
fn expand_targets_full_u32_exclude_range_is_not_expanded() {
    let ids = expand_targets_with(
        &[
            AppendTarget::Single(0),
            AppendTarget::ExcludeRange {
                start: 0,
                end: u32::MAX,
            },
        ],
        Records::Emit,
    );
    assert!(ids.is_empty(), "0 は除外される");
    let ids = expand_targets_with(
        &[
            AppendTarget::Range { start: 1, end: 9 },
            AppendTarget::Single(3),
            AppendTarget::ExcludeRange { start: 7, end: 5 },
            AppendTarget::Exclude(2),
            AppendTarget::ExcludeRange { start: 9, end: 9 },
        ],
        Records::Emit,
    );
    assert_eq!(
        ids,
        vec![1, 3, 4, 8, 3],
        "小さな範囲は前と同じ（並び・重複を保つ）"
    );
}

/// 指摘 75（重ね）: 単独の除外を 10 万個並べても、組をまとめて二分探索で引くので短く終わり、結果は正しい。
#[test]
fn expand_targets_many_single_excludes_subtract_correctly() {
    let mut targets = vec![AppendTarget::Range {
        start: 0,
        end: 8000,
    }];
    // 偶数 0〜199998 の単独の除外を 10 万個（逆順に並べ、まとめの並べ直しも突く）。
    targets.extend((0..100_000u32).rev().map(|n| AppendTarget::Exclude(n * 2)));
    let ids = expand_targets_with(&targets, Records::Emit);
    let expected: Vec<u32> = (0..=8000).filter(|n| n % 2 == 1).collect();
    assert_eq!(ids, expected, "偶数だけが除外され、並びは記述の順");
}
