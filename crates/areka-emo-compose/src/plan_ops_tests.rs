use super::*;
use crate::world::EmoWorld;
use areka_emo_atlas::{AtlasTable, SetId};
use areka_parsers::shell::{
    Animation, DrawMethod, Element, Interval, Pattern, SortOrder, Surface,
};
use std::path::Path;
use super::test_support::*;

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
    push_static_element_ops(&mut ops, &world, &atlas, 1000, 0, 0);

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
    push_static_element_ops(&mut ops, &world, &atlas, 1, 0, 0);

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
    push_static_element_ops(&mut ops1, &world, &atlas, 1000, 0, 0);
    let mut ops2 = Vec::new();
    push_static_element_ops(&mut ops2, &world, &atlas, 1000, 0, 0);

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
    push_static_element_ops(&mut ops, &world, &atlas, 1000, 0, 0);

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
    push_static_element_ops(&mut ops, &world, &atlas, 7, 0, 0);

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
    push_static_element_ops(&mut ops, &world, &atlas, 1, 0, 0);

    // 先頭 sentinel が残り、末尾へ element 命令が追記される。
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0], sentinel);
}

/// surface 不在では何も積まない（後続 task が SurfaceNotFound 分類を担う・本 task は非追記）。
#[test]
fn missing_surface_pushes_nothing() {
    let world = EmoWorld::build(&shell_of(Vec::new()));
    // surface 不在ゆえ atlas は引かれない（空 atlas で足りる）。
    let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);
    let mut ops = Vec::new();
    push_static_element_ops(&mut ops, &world, &atlas, 9999, 0, 0);
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
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

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
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

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
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

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
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());
    assert!(!ops.is_empty(), "全 bind surface でも非空 bind 集合から可視層を生む");
    assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png", "part2.png"]);

    // 空 bind 集合 → bind 命令なし（静的 element も無いので空・非パニック）。
    let empty = BindSet::default();
    let mut ops_empty = Vec::new();
    derive_ops(&mut ops_empty, &mut Vec::new(), &world, &atlas, 1000, &empty, &PatternState::default());
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
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

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
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

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
    derive_ops(&mut ops1, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());
    let mut ops2 = Vec::new();
    derive_ops(&mut ops2, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

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
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].element, part_id);
    // element 自 (5,6) ＋ pattern0 offset (30,-20) ＝ (35, -14)。
    assert_eq!(ops[0].transform.offset(), (35, -14));
}

// ── task 5.3: 入れ子 surface 参照の多段 flatten と循環検出（visited 集合） ─────────────

/// bind animation を任意 (x,y) つきで組む（bind_anim の x,y=0 固定を拡張・pattern0 のみ有意）。
fn bind_anim_xy(id: u32, ref_surface_id: i64, x: i64, y: i64) -> Animation {
    bind_anim(id, ref_surface_id, x, y)
}

/// テスト5.3-①（受入基準・要件 7.1/7.2/7.3）: 自己参照 bind はスタックオーバーフローせず、
/// 部分結果（自 surface の静的 element）が得られ、循環枝は打ち切られる（warn・非パニック）。
///
/// surface 1000 は静的 element `self.png` を持ち、かつ自身の bind pattern0 が **surface 1000
/// 自身**を参照する（自己参照 = 循環）。derive_ops は無限再帰せず（テストが完走すれば O.K.）、
/// 静的層 self.png は積まれ、自己参照枝は visited で打ち切られる。
#[test]
fn self_reference_does_not_overflow_and_yields_partial_result() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["self.png"]);
    let self_id = atlas.resolve(SetId(0), "self.png").expect("self 解決");

    // surface 1000: 静的 element self.png ＋ bind id=1 が surface 1000 自身を参照。
    let host = surface_with_anims(
        1000,
        vec![elem(0, "self.png", 0, 0)],
        vec![bind_anim(1, 1000, 0, 0)], // 自己参照。
    );
    let shell = shell_of(vec![host]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops = Vec::new();
    // これが無限再帰すると本テストは完走できない（スタックオーバーフローで abort）。
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    // 静的層 self.png は積まれる（部分結果）。
    assert!(!ops.is_empty(), "自己参照でも静的層の部分結果が得られる");
    // ops は有界（自己参照枝は打ち切られる）。静的 self.png（1本）＋ bind pattern0 が
    // surface 1000 を参照→ visited で 1000 は既訪問ゆえ枝打ち切り＝追加 element なし。
    // よって self.png 1本のみ。
    let paths = ops_to_paths(&ops, &[(self_id, "self.png")]);
    assert_eq!(paths, vec!["self.png"], "自己参照枝は打ち切り＝静的層のみ");
}

/// テスト5.3-②（要件 7.2）: 相互参照（A→B→A）はスタックオーバーフローせず、A 再入で
/// 打ち切られ部分結果が得られる（非パニック）。
///
/// surface A=1000 の bind→B=1100、B の bind→A=1000。A から derive すると
/// A 静的 → B 静的（B の bind 経由で A 再入は visited で打ち切り）。無限再帰しない。
#[test]
fn mutual_reference_cycle_is_pruned_without_overflow() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["a.png", "b.png"]);
    let a_id = atlas.resolve(SetId(0), "a.png").expect("a 解決");
    let b_id = atlas.resolve(SetId(0), "b.png").expect("b 解決");

    // A=1000: 静的 a.png ＋ bind id=1 → B=1100。
    let a = surface_with_anims(
        1000,
        vec![elem(0, "a.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0)],
    );
    // B=1100: 静的 b.png ＋ bind id=1 → A=1000（相互参照）。
    let b = surface_with_anims(
        1100,
        vec![elem(0, "b.png", 0, 0)],
        vec![bind_anim(1, 1000, 0, 0)],
    );
    let shell = shell_of(vec![a, b]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    // A 静的 → B 静的まで到達し、B→A の再入は打ち切り。a.png と b.png の各1本。
    let map = [(a_id, "a.png"), (b_id, "b.png")];
    let paths = ops_to_paths(&ops, &map);
    assert_eq!(paths, vec!["a.png", "b.png"], "相互参照は A 再入で打ち切り＝有界");
}

/// テスト5.3-③（要件 7.1・オフセット累積）: 非循環の多段入れ子 A→B→C でオフセットが累積する。
///
/// A=1000 の bind→B=1100 を (10,5)、B=1100 の bind→C=1200 を (100,50)、C=1200 の element は
/// 自 (2,3)。C の element の最終オフセット＝(10+100+2, 5+50+3)＝(112, 58)。多段累積を検証。
#[test]
fn offset_accumulates_across_multilevel_nesting() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["c.png"]);
    let c_id = atlas.resolve(SetId(0), "c.png").expect("c 解決");

    // A=1000: 静的 element なし・bind id=1 → B=1100 at (10,5)。
    let a = surface_with_anims(1000, Vec::new(), vec![bind_anim_xy(1, 1100, 10, 5)]);
    // B=1100: 静的 element なし・bind id=1 → C=1200 at (100,50)。
    let b = surface_with_anims(1100, Vec::new(), vec![bind_anim_xy(1, 1200, 100, 50)]);
    // C=1200: 静的 element c.png at (2,3)。
    let c = surface(1200, vec![elem(0, "c.png", 2, 3)]);
    let shell = shell_of(vec![a, b, c]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    assert_eq!(ops.len(), 1, "C の element 1 本のみ（A/B は静的 element なし）");
    assert_eq!(ops[0].element, c_id);
    // 累積: A→B (10,5) ＋ B→C (100,50) ＋ C element 自 (2,3) ＝ (112, 58)。
    assert_eq!(ops[0].transform.offset(), (112, 58), "多段オフセット累積");
}

/// テスト5.3-④（要件 7.1・ancestor-stack 規律）: 非循環で同一 surface を2回参照すると、
/// 双方とも展開される（visited は祖先スタックゆえ枝離脱で pop・偽陽性打ち切りをしない）。
///
/// A=1000 の bind id=1 → 共有子 S=1300 at (10,0)、bind id=2 → 中間 B=1100（B の bind→S=1300 at
/// (0,20)）。S は祖先でない2経路（A 直下・A→B 経由）から参照される。S の element は各経路で
/// 展開され、offset は (10,0) と (0,20) で異なる2命令になる（誤って一度きりに刈られない）。
#[test]
fn noncyclic_shared_child_expands_on_each_path() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["s.png"]);
    let s_id = atlas.resolve(SetId(0), "s.png").expect("s 解決");

    // A=1000: bind id=1 → S=1300 at (10,0)、bind id=2 → B=1100 at (0,0)。
    let a = surface_with_anims(
        1000,
        Vec::new(),
        vec![bind_anim_xy(1, 1300, 10, 0), bind_anim_xy(2, 1100, 0, 0)],
    );
    // B=1100: bind id=1 → S=1300 at (0,20)。
    let b = surface_with_anims(1100, Vec::new(), vec![bind_anim_xy(1, 1300, 0, 20)]);
    // S=1300: 共有子 element s.png at (0,0)。
    let s = surface(1300, vec![elem(0, "s.png", 0, 0)]);
    let shell = shell_of(vec![a, b, s]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1, 2]);
    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    // S は2経路で展開＝s.png が 2 命令。祖先スタック（pop-on-exit）ゆえ非循環重複は刈られない。
    assert_eq!(ops.len(), 2, "非循環の共有子は各経路で展開（祖先スタック規律）");
    assert!(ops.iter().all(|op| op.element == s_id));
    let offsets: Vec<(i64, i64)> = ops.iter().map(|op| op.transform.offset()).collect();
    // id 昇順描画（既定 descend）: id=1（S 直下 at (10,0)）→ id=2（B→S at (0,20)）。
    assert_eq!(offsets, vec![(10, 0), (0, 20)], "各経路で別オフセット＝重複展開");
}

/// テスト5.3-⑤（要件 4.5/10.1）: 循環を含む入力でも 2 回導出→バイト等価（決定性・有界）。
#[test]
fn cyclic_derivation_is_deterministic() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["a.png", "b.png"]);

    let a = surface_with_anims(
        1000,
        vec![elem(0, "a.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0)],
    );
    let b = surface_with_anims(
        1100,
        vec![elem(0, "b.png", 0, 0)],
        vec![bind_anim(1, 1000, 0, 0)], // B→A 相互参照。
    );
    let shell = shell_of(vec![a, b]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops1 = Vec::new();
    derive_ops(&mut ops1, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());
    let mut ops2 = Vec::new();
    derive_ops(&mut ops2, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    assert_eq!(ops1, ops2, "循環入力でも同一入力→同一 ops（バイト等価・有界）");
}

// ── task 11.2: 静的合成の pattern0＝厳密 index==0 選択（実機第2欠陥・要件 9.1/9.2/9.5）──────
//
// 実機サインオフ#2: むらさきの目が常時閉じている。真因＝`flatten_surface` の pattern 選択が
// 最小 index フォールバック（min_by_key）だったため、pattern0 を持たない まばたき animation
// （emo2 1400=`interval,bind+random`・pattern1/2/3、1403=`interval,bind`・pattern2）の閉じ目
// フレーム（surface 1412/1414）が静的土台へ積まれ、ベースの目（1302）を覆っていた。canon では
// pattern0（index==0）を持たない bind animation は seriko-loop（M-life）が再生する再生専用
// フレームで、静的土台には寄与しない。修正＝index==0 厳密選択・不在は良性 skip（DEBUG）。

/// bind animation を「指定 index の pattern 1 本だけ」で組む（pattern0=index0 の有無を制御する）。
///
/// `bind_anim` は index=0 と index=5 を持つが、本ヘルパは **単一 index** の pattern のみを載せる。
/// index を 1 以上にすれば「pattern0（index==0）を持たない再生アニメ」（まばたき相当）を作れる。
fn single_pattern_anim(id: u32, interval: Interval, index: u32, ref_surface_id: i64) -> Animation {
    Animation {
        id,
        interval,
        patterns: vec![Pattern {
            index,
            method: DrawMethod::new("overlay".to_string()),
            surface_id: ref_surface_id,
            wait: 0,
            x: 0,
            y: 0,
        }],
    }
}

/// テスト11.2-①（**RED 核**・受入基準・要件 9.1/9.2）: pattern0（index==0）を持たない有効 bind は
/// 静的合成へ**一切寄与しない**（生成描画命令に現れない）。
///
/// bind id=1400 は pattern0 を持たず index=1 のみ（surface 1412＝閉じ目相当を参照）。旧実装
/// （`min_by_key(index)`）は index=1 を pattern0 として採り surface 1412 を合成する→この assert が
/// 破れる（RED）。修正後（`find(index==0)`）は index==0 不在ゆえ skip され、命令ゼロになる（GREEN）。
#[test]
fn pattern0_less_bind_contributes_no_ops() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["closed_eye.png"]);
    let map = id_to_path(&atlas, &["closed_eye.png"]);

    // pattern0 なし・index=1 のみが surface 1412（閉じ目相当）を参照する再生アニメ。
    let blink = single_pattern_anim(1400, Interval::BindRandom { k: 4 }, 1, 1412);
    let host = surface_with_anims(1000, Vec::new(), vec![blink]);
    let part = surface(1412, vec![elem(0, "closed_eye.png", 0, 0)]);
    let shell = shell_of(vec![host, part]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1400]);
    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    // pattern0 を持たない有効 bind は静的合成へ寄与しない＝closed_eye.png 命令が現れない。
    assert!(
        ops_to_paths(&ops, &map).is_empty(),
        "pattern0（index==0）なし bind は描画命令に現れない（旧 min_by_key は pattern1 を合成し RED）"
    );
    assert!(ops.is_empty(), "静的 element も無いので命令ゼロ");
}

/// テスト11.2-②（要件 9.2）: index==0 と index==1 が共存 → **index==0（pattern0）のみ**合成し、
/// index==1 フレームは静的合成に現れない（index==0 厳密選択の固定）。
#[test]
fn coexisting_pattern0_and_pattern1_composes_only_index0() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["open_eye.png", "closed_eye.png"]);
    let map = id_to_path(&atlas, &["open_eye.png", "closed_eye.png"]);

    // bind id=1: pattern0(index0)→open(surface 1100)、pattern1(index1)→closed(surface 1200)。
    let anim = Animation {
        id: 1,
        interval: Interval::Bind,
        patterns: vec![
            Pattern { index: 0, method: DrawMethod::new("overlay".to_string()), surface_id: 1100, wait: 0, x: 0, y: 0 },
            Pattern { index: 1, method: DrawMethod::new("overlay".to_string()), surface_id: 1200, wait: 0, x: 0, y: 0 },
        ],
    };
    let host = surface_with_anims(1000, Vec::new(), vec![anim]);
    let open = surface(1100, vec![elem(0, "open_eye.png", 0, 0)]);
    let closed = surface(1200, vec![elem(0, "closed_eye.png", 0, 0)]);
    let shell = shell_of(vec![host, open, closed]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    // index==0（open_eye）のみ合成。index==1（closed_eye）は静的合成に現れない。
    assert_eq!(ops_to_paths(&ops, &map), vec!["open_eye.png"]);
}

// ── task 7.2: PatternState 合流と method ゲート（層(ii) の transient コマ合流・要件 4.2/4.6/5.3/8.4）─
//
// flatten_surface（top-level のみ）で「有効 bind pattern0 の集合 ∪ PatternState のコマ集合」を
// 既存 animation-sort 整列へ合流し、同 id はコマが pattern0 寄与を置換する。コマ・pattern0 双方に
// method ゲート（is_implemented()＝Overlay のみ駆動）を適用し、非 Overlay は warn!（method 名込み）
// ＋不描画とする（8.4）。warn 発火は log_capture で檻に入れる。

use crate::log_capture::capture_logs;
use crate::pattern::PatternFrame;

/// 現在コマ（PatternFrame）を 1 本組む（テスト補助・任意 method／オフセット）。
fn koma(surface_id: u32, method: ComposeMethod, x: i64, y: i64) -> PatternFrame {
    PatternFrame { surface_id, method, x, y }
}

/// pattern0（index==0）1 本だけを持つ bind animation を任意 method で組む（pattern0 method ゲート檻用）。
fn bind_anim_method(id: u32, ref_surface_id: i64, method: &str) -> Animation {
    Animation {
        id,
        interval: Interval::Bind,
        patterns: vec![Pattern {
            index: 0,
            method: DrawMethod::new(method.to_string()),
            surface_id: ref_surface_id,
            wait: 0,
            x: 0,
            y: 0,
        }],
    }
}

/// テスト7.2-①（**受入基準**・要件 8.4）: 非 Overlay method の現在コマは warn!（method 名込み）＋
/// 不描画。かつコマは同 id の pattern0 静的寄与を置換するゆえ、その pattern0 も現れない（4.2）。
///
/// host=1000 は静的 base ＋ bind id=1 pattern0→part1(overlay)。id=1 の現在コマは surface 1500
/// （koma）method=Replace（非 Overlay）。コマは非駆動で不描画・pattern0(part1) も置換で不描画ゆえ、
/// 残るは base のみ。warn は method=Replace を載せる。
#[test]
fn non_overlay_koma_warns_and_is_not_drawn_replacing_pattern0() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["base.png", "part1.png", "koma.png"]);
    let map = id_to_path(&atlas, &["base.png", "part1.png", "koma.png"]);

    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0)], // pattern0 は overlay。
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let koma_surface = surface(1500, vec![elem(0, "koma.png", 0, 0)]);
    let shell = shell_of(vec![host, part1, koma_surface]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut pattern = PatternState::default();
    pattern.set(1, koma(1500, ComposeMethod::Replace, 0, 0)); // 非 Overlay コマ。

    let mut ops = Vec::new();
    let logs = capture_logs(|| {
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &pattern);
    });

    // コマ（koma）は非駆動で不描画・pattern0（part1）はコマに置換され不描画 → base のみ。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["base.png"],
        "非 Overlay コマは不描画・pattern0 も置換で不描画（残るは base）"
    );
    // warn! が method 名込みで発火する（要件 8.4・完全形保持のまま非駆動）。
    assert!(logs.contains("level=WARN"), "非 Overlay コマは WARN: {logs}");
    assert!(logs.contains("target=areka_emo_compose"), "target: {logs}");
    assert!(logs.contains("method=Replace"), "method 名（判別子）を載せる: {logs}");
    assert!(logs.contains("animation_id=1"), "対象 animation id を載せる: {logs}");
}

/// テスト7.2-②（要件 8.4・D-5 是正）: 非 Overlay の **bind pattern0**（静的経路）も warn!（method
/// 名込み）＋不描画。parser の overlay フィルタ撤去で非 overlay pattern0 が流入し得るため。
///
/// host=1000 は静的 base ＋ bind id=1 の pattern0 method="replace"→part1。空 PatternState。
/// pattern0 は非 Overlay ゆえ不描画（warn method=Replace）で、残るは base のみ。
#[test]
fn non_overlay_pattern0_warns_and_is_not_drawn() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["base.png", "part1.png"]);
    let map = id_to_path(&atlas, &["base.png", "part1.png"]);

    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim_method(1, 1100, "replace")], // 非 Overlay pattern0。
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let shell = shell_of(vec![host, part1]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops = Vec::new();
    let logs = capture_logs(|| {
        derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());
    });

    // 非 Overlay pattern0（part1）は不描画 → base のみ。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["base.png"],
        "非 Overlay pattern0 は不描画（残るは base）"
    );
    assert!(logs.contains("level=WARN"), "非 Overlay pattern0 は WARN: {logs}");
    assert!(logs.contains("method=Replace"), "method 名（判別子）を載せる: {logs}");
    assert!(logs.contains("animation_id=1"), "対象 animation id を載せる: {logs}");
}

/// テスト7.2-③（**受入基準**・要件 4.2）: Overlay の現在コマは描画され、同 id の pattern0 静的
/// 寄与を**置換**する（pattern0 の surface は現れず、コマの surface が現れる）。
///
/// host=1000 は bind id=1 pattern0→open(1100)。id=1 の現在コマは closed(1200) method=Overlay。
/// コマが pattern0 を置換 → closed_eye のみ現れ open_eye は現れない。
#[test]
fn overlay_koma_is_drawn_and_replaces_same_id_pattern0() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["open_eye.png", "closed_eye.png"]);
    let map = id_to_path(&atlas, &["open_eye.png", "closed_eye.png"]);

    let host = surface_with_anims(1000, Vec::new(), vec![bind_anim(1, 1100, 0, 0)]);
    let open = surface(1100, vec![elem(0, "open_eye.png", 0, 0)]);
    let closed = surface(1200, vec![elem(0, "closed_eye.png", 0, 0)]);
    let shell = shell_of(vec![host, open, closed]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut pattern = PatternState::default();
    pattern.set(1, koma(1200, ComposeMethod::Overlay, 0, 0)); // Overlay コマ。

    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &pattern);

    // コマ（closed_eye）が pattern0（open_eye）を置換 → closed_eye のみ。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["closed_eye.png"],
        "Overlay コマが同 id の pattern0 を置換（open_eye は現れない）"
    );
}

/// テスト7.2-④（要件 5.3・画家のアルゴリズム）: 合流後も id 昇順描画順が保たれる（既定 descend）。
///
/// bind id=1 pattern0→partA(1100)、**コマのみ** id=2→partB(1200)（bind animation なし）、
/// bind id=3 pattern0→partC(1300)。合流集合 {1,3}∪{2}={1,2,3}。id 昇順描画で partA→partB→partC。
/// コマ専用 id=2 が bind ids の間へ id 順で正しく挿入されることを突く。
#[test]
fn merged_ids_preserve_id_ascending_painter_order() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["partA.png", "partB.png", "partC.png"]);
    let map = id_to_path(&atlas, &["partA.png", "partB.png", "partC.png"]);

    // id=2 の bind animation は無い（コマ専用）。bind は id=1,3 のみ。
    let host = surface_with_anims(
        1000,
        Vec::new(),
        vec![bind_anim(1, 1100, 0, 0), bind_anim(3, 1300, 0, 0)],
    );
    let part_a = surface(1100, vec![elem(0, "partA.png", 0, 0)]);
    let part_b = surface(1200, vec![elem(0, "partB.png", 0, 0)]);
    let part_c = surface(1300, vec![elem(0, "partC.png", 0, 0)]);
    let shell = shell_of(vec![host, part_a, part_b, part_c]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // bind は 1,3 のみ有効。コマ id=2 は BindSet 非依存で合流される。
    let binds = BindSet::from_ids([1, 3]);
    let mut pattern = PatternState::default();
    pattern.set(2, koma(1200, ComposeMethod::Overlay, 0, 0));

    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &pattern);

    // 既定 descend → id 昇順描画: partA(1) → partB(2・コマ) → partC(3)。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["partA.png", "partB.png", "partC.png"],
        "合流後も id 昇順描画（コマ専用 id が bind ids の間へ id 順で挿入）"
    );
}

/// テスト7.2-⑤（要件 5.4・byte 等価 sanity）: 空 PatternState は合流前と同一 ops を生む
/// （full golden は task 7.3・ここでは merge 導入で pattern0 経路が退行しないことの即時檻）。
#[test]
fn empty_pattern_state_yields_identical_ops_to_pre_merge() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["base.png", "part1.png"]);
    let map = id_to_path(&atlas, &["base.png", "part1.png"]);

    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0)],
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let shell = shell_of(vec![host, part1]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1]);
    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &PatternState::default());

    // 空 PatternState → 静的 base（下）＋ bind pattern0 part1（上）＝合流前と同一。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["base.png", "part1.png"],
        "空 PatternState は pattern0 経路そのまま（merge 導入で退行しない）"
    );
}

/// テスト7.2-⑥（要件 4.6・5.3）: bind pattern0 を持たない id の Overlay コマ単独でも描画される
/// （合流集合が union ゆえ・コマ専用 id）。まばたき再生（pattern0 なし bind の再生コマ）を突く。
#[test]
fn koma_only_id_without_bind_pattern0_is_drawn() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["blink.png"]);
    let map = id_to_path(&atlas, &["blink.png"]);

    // host は bind animation を一切持たない（静的 element も無し）。
    let host = surface_with_anims(1000, Vec::new(), Vec::new());
    let blink = surface(1412, vec![elem(0, "blink.png", 0, 0)]);
    let shell = shell_of(vec![host, blink]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // BindSet は空。コマだけが surface 1412 を Overlay で駆動する。
    let binds = BindSet::default();
    let mut pattern = PatternState::default();
    pattern.set(1400, koma(1412, ComposeMethod::Overlay, 0, 0));

    let mut ops = Vec::new();
    derive_ops(&mut ops, &mut Vec::new(), &world, &atlas, 1000, &binds, &pattern);

    // pattern0 が無くてもコマは駆動される（union 合流・4.6 まばたき再生）。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["blink.png"],
        "bind pattern0 を持たない id のコマ単独でも描画（union 合流）"
    );
}
