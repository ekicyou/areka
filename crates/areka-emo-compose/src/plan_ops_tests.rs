use super::*;
use crate::plan_test_support::*;
use crate::world::EmoWorld;
use areka_emo_atlas::{AtlasTable, SetId};
use areka_parsers::shell::{Animation, Element, Interval, SortOrder, Surface};
use std::path::Path;

/// 受入基準・要件 4.1: layer [2,0,1]（登場順）→ ops は layer 昇順 0,1,2。
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
    let world = bound_world(&shell, &[], &atlas);

    let ops = ops(
        &world,
        &atlas,
        1000,
        &BindSet::default(),
        &PatternState::default(),
    );

    // layer 昇順: b(0) → c(1) → a(2)。
    assert_eq!(ops_to_paths(&ops, &map), vec!["b.png", "c.png", "a.png"]);
}

/// 要件 4.1: 同一 layer は登場（定義）順を保つ。
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
    let world = bound_world(&shell, &[], &atlas);

    let ops = ops(
        &world,
        &atlas,
        1,
        &BindSet::default(),
        &PatternState::default(),
    );

    assert_eq!(ops_to_paths(&ops, &map), vec!["first.png", "second.png"]);
}

/// 要件 4.3/6.3 前段: 未束縛（None）element は命令化されずスキップされる（非パニック）。
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
    let world = bound_world(&shell, &[], &atlas);

    let ops = ops(
        &world,
        &atlas,
        1000,
        &BindSet::default(),
        &PatternState::default(),
    );

    // bogus.png は None ゆえスキップ＝命令は known.png の1本のみ。
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].element, known_id);
}

/// 要件 4.2: 命令の Transform は element の translate(x,y) と一致し、純平行移動である。
#[test]
fn transform_propagates_as_translation() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["p.png"]);

    let shell = shell_of(vec![surface(7, vec![elem(0, "p.png", 12, -8)])]);
    let world = bound_world(&shell, &[], &atlas);

    let ops = ops(
        &world,
        &atlas,
        7,
        &BindSet::default(),
        &PatternState::default(),
    );

    assert_eq!(ops.len(), 1);
    assert_eq!(
        ops[0].transform,
        Transform::translate(12, -8),
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
    let world = bound_world(&shell, &[], &atlas);

    let sentinel = BlitOp {
        element: ElementId(u32::MAX),
        transform: Transform::identity(),
        method: ComposeMethod::Overlay,
    };
    let mut ops = vec![sentinel.clone()];
    derive_ops(
        &mut ops,
        &mut PlanScratch::default(),
        &world,
        &atlas,
        1,
        &BindSet::default(),
        &PatternState::default(),
    );

    // 先頭 sentinel が残り、末尾へ element 命令が追記される。
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0], sentinel);
}

/// surface 不在では何も積まない（SurfaceNotFound の分類は `build_plan` が担う）。
#[test]
fn missing_surface_pushes_nothing() {
    let world = EmoWorld::build(&shell_of(Vec::new()));
    // surface 不在ゆえ atlas は引かれない（中身は何でもよい）。
    let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);
    let ops = ops(
        &world,
        &atlas,
        9999,
        &BindSet::default(),
        &PatternState::default(),
    );
    assert!(ops.is_empty());
}

// ── 有効 bind の合成対象化＋animation-sort→ID 順の2段規則 ──────────

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
    let world = bound_world(&shell, &[], &atlas);
    (world, atlas)
}

/// 受入基準・要件 5.2/5.3/5.6: 複数有効 bind・sort 既定（descend）→ ID 昇順に描画。
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
        &[
            (3, 1300, "part3.png"),
            (1, 1100, "part1.png"),
            (2, 1200, "part2.png"),
        ],
        None, // 未指定＝既定 descend。
    );
    let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

    let binds = BindSet::from_ids([1, 2, 3]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // descend（既定）→ ID 昇順描画: part1(1) → part2(2) → part3(3)。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["part1.png", "part2.png", "part3.png"]
    );
}

/// 要件 5.3: animation-sort=ascend → ID 降順に描画（小 ID が上）。
#[test]
fn active_binds_ascend_draws_in_id_descending_order() {
    let (world, atlas) = build_bind_world(
        1000,
        Vec::new(),
        &[],
        &[
            (3, 1300, "part3.png"),
            (1, 1100, "part1.png"),
            (2, 1200, "part2.png"),
        ],
        Some(SortOrder::Ascend),
    );
    let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

    let binds = BindSet::from_ids([1, 2, 3]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // ascend → ID 降順描画: part3(3) → part2(2) → part1(1)。
    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["part3.png", "part2.png", "part1.png"]
    );
}

/// 要件 5.2: BindSet に含まれない bind animation は合成対象から除外される。
#[test]
fn only_binds_in_bindset_are_included() {
    let (world, atlas) = build_bind_world(
        1000,
        Vec::new(),
        &[],
        &[
            (1, 1100, "part1.png"),
            (2, 1200, "part2.png"),
            (3, 1300, "part3.png"),
        ],
        None,
    );
    let map = id_to_path(&atlas, &["part1.png", "part2.png", "part3.png"]);

    // id=2 のみ有効（1,3 は非活性）。
    let binds = BindSet::from_ids([2]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // part2 のみ命令化される。
    assert_eq!(ops_to_paths(&ops, &map), vec!["part2.png"]);
}

/// 要件 5.4: 静的 element ゼロ・全パーツ bind の surface でも非空 bind 集合→非空 ops（空 bind 集合で命令が
/// ゼロになることは `plan_extent_tests` の空 BindSet の檻が見る）。
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
    let ops_full = ops(&world, &atlas, 1000, &binds, &PatternState::default());
    assert_eq!(
        ops_to_paths(&ops_full, &map),
        vec!["part1.png", "part2.png"]
    );
}

/// 要件 5.2・design 層列挙 i/ii: 静的 element 層が bind 層の**前（下）**に来る。
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
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // 静的 base（下）→ bind part1（上）。
    assert_eq!(ops_to_paths(&ops, &map), vec!["base.png", "part1.png"]);
}

/// 要件 5.5 前段: pattern0 の surface_id<0 はセンチネル＝命令を積まず skip（非パニック）。-2 を u32 へ
/// 丸めた番号 4294967294 の面を置いても、その絵は出ない（番号へ読み替えない）。
#[test]
fn sentinel_pattern0_is_skipped() {
    let base = Path::new("shell/master");
    // part1 は正常参照、bind id=2 の pattern0 は surface_id=-2（センチネル）。
    let atlas = bake_atlas(base, &["part1.png", "wrap.png"]);
    let map = id_to_path(&atlas, &["part1.png", "wrap.png"]);

    let host = surface_with_anims(
        1000,
        Vec::new(),
        vec![
            bind_anim(1, 1100, 0, 0), // 正常参照。
            bind_anim(2, -2, 0, 0),   // センチネル（非描画）。
        ],
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let wrapped = surface(u32::MAX - 1, vec![elem(0, "wrap.png", 0, 0)]);
    let shell = shell_of(vec![host, part1, wrapped]);
    let world = bound_world(&shell, &[], &atlas);

    let binds = BindSet::from_ids([1, 2]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // センチネル bind id=2 は積まれず、part1 のみ（非パニック・wrap.png は出ない）。
    assert_eq!(ops_to_paths(&ops, &map), vec!["part1.png"]);
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
    let world = bound_world(&shell, &[], &atlas);

    let binds = BindSet::from_ids([1]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].element, part_id);
    // element 自 (5,6) ＋ pattern0 offset (30,-20) ＝ (35, -14)。
    assert_eq!(ops[0].transform.offset(), (35, -14));
}

// ── 入れ子 surface 参照の多段 flatten と循環検出（祖先の経路 on_path） ─────────────

/// 受入基準・要件 7.1/7.2/7.3: 自己参照 bind はスタックオーバーフローせず、
/// 部分結果（自 surface の静的 element）が得られ、循環枝は打ち切られる（debug・非パニック）。
///
/// surface 1000 は静的 element `self.png` を持ち、かつ自身の bind pattern0 が **surface 1000
/// 自身**を参照する（自己参照 = 循環）。derive_ops は無限に回らず（テストが完走すれば O.K.）、
/// 静的層 self.png は積まれ、自己参照枝は祖先の経路（on_path）で打ち切られる。
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
    let world = bound_world(&shell, &[], &atlas);

    let binds = BindSet::from_ids([1]);
    // 循環を打ち切らなければ本テストは完走できない。
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // 静的層 self.png は積まれる（部分結果）。
    // ops は有界（自己参照枝は打ち切られる）。静的 self.png（1本）＋ bind pattern0 が
    // surface 1000 を参照→ 1000 は祖先の経路（on_path）に在るゆえ枝打ち切り＝追加 element なし。
    // よって self.png 1本のみ。
    let paths = ops_to_paths(&ops, &[(self_id, "self.png")]);
    assert_eq!(paths, vec!["self.png"], "自己参照枝は打ち切り＝静的層のみ");
}

/// 要件 7.2: 相互参照（A→B→A）はスタックオーバーフローせず、A 再入で
/// 打ち切られ部分結果が得られる（非パニック）。
///
/// surface A=1000 の bind→B=1100、B の bind→A=1000。A から derive すると
/// A 静的 → B 静的（B の bind 経由で A 再入は祖先の経路（on_path）で打ち切り）。無限に回らない。
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
    let world = bound_world(&shell, &[], &atlas);

    let binds = BindSet::from_ids([1]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // A 静的 → B 静的まで到達し、B→A の再入は打ち切り。a.png と b.png の各1本。
    let map = [(a_id, "a.png"), (b_id, "b.png")];
    let paths = ops_to_paths(&ops, &map);
    assert_eq!(
        paths,
        vec!["a.png", "b.png"],
        "相互参照は A 再入で打ち切り＝有界"
    );
}

/// 要件 7.1・オフセット累積: 非循環の多段入れ子 A→B→C でオフセットが累積する。
///
/// A=1000 の bind→B=1100 を (10,5)、B=1100 の bind→C=1200 を (100,50)、C=1200 の element は
/// 自 (2,3)。C の element の最終オフセット＝(10+100+2, 5+50+3)＝(112, 58)。多段累積を検証。
#[test]
fn offset_accumulates_across_multilevel_nesting() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["c.png"]);
    let c_id = atlas.resolve(SetId(0), "c.png").expect("c 解決");

    // A=1000: 静的 element なし・bind id=1 → B=1100 at (10,5)。
    let a = surface_with_anims(1000, Vec::new(), vec![bind_anim(1, 1100, 10, 5)]);
    // B=1100: 静的 element なし・bind id=1 → C=1200 at (100,50)。
    let b = surface_with_anims(1100, Vec::new(), vec![bind_anim(1, 1200, 100, 50)]);
    // C=1200: 静的 element c.png at (2,3)。
    let c = surface(1200, vec![elem(0, "c.png", 2, 3)]);
    let shell = shell_of(vec![a, b, c]);
    let world = bound_world(&shell, &[], &atlas);

    let binds = BindSet::from_ids([1]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    assert_eq!(
        ops.len(),
        1,
        "C の element 1 本のみ（A/B は静的 element なし）"
    );
    assert_eq!(ops[0].element, c_id);
    // 累積: A→B (10,5) ＋ B→C (100,50) ＋ C element 自 (2,3) ＝ (112, 58)。
    assert_eq!(ops[0].transform.offset(), (112, 58), "多段オフセット累積");
}

/// 要件 7.1・ancestor-stack 規律: 非循環で同一 surface を2回参照すると、
/// 双方とも展開される（祖先の経路（on_path）は枝を離れると外すので、偽陽性打ち切りをしない）。
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
        vec![bind_anim(1, 1300, 10, 0), bind_anim(2, 1100, 0, 0)],
    );
    // B=1100: bind id=1 → S=1300 at (0,20)。
    let b = surface_with_anims(1100, Vec::new(), vec![bind_anim(1, 1300, 0, 20)]);
    // S=1300: 共有子 element s.png at (0,0)。
    let s = surface(1300, vec![elem(0, "s.png", 0, 0)]);
    let shell = shell_of(vec![a, b, s]);
    let world = bound_world(&shell, &[], &atlas);

    let binds = BindSet::from_ids([1, 2]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    // S は2経路で展開＝s.png が 2 命令。祖先スタック（pop-on-exit）ゆえ非循環重複は刈られない。
    assert_eq!(
        ops.len(),
        2,
        "非循環の共有子は各経路で展開（祖先スタック規律）"
    );
    assert!(ops.iter().all(|op| op.element == s_id));
    let offsets: Vec<(i64, i64)> = ops.iter().map(|op| op.transform.offset()).collect();
    // id 昇順描画（既定 descend）: id=1（S 直下 at (10,0)）→ id=2（B→S at (0,20)）。
    assert_eq!(
        offsets,
        vec![(10, 0), (0, 20)],
        "各経路で別オフセット＝重複展開"
    );
}

// ── 着せ替えのコマの静的な積み方（要件 9.1/9.2/9.5）──────────────────────────────────────
//
// コマの無い有効な純 `bind` は全部のコマを番号の順に重ねる（seriko-pattern-calls 6.8・SSP の測り M6）。
// `bind+random` はコマのあるときだけそのコマを描き、コマが無いあいだは何も積まない（良性 skip・DEBUG・
// animated-surface-log-flood 件 26）。実機サインオフ#2（むらさきの目が常時閉じている）は、まばたきの
// `bind+random`（emo2 1400）の閉じ目フレーム（surface 1412）が静的土台に積まれてベースの目を覆った件。

/// 要件 9.1/9.2: pattern0（index==0）を持たない有効な純 `bind` も、持っているコマを
/// 静的に積む（純 `bind` は全部のコマを番号の順に重ねる・seriko-pattern-calls 6.8・SSP の測り M6 で
/// 期待を直した）。pattern0 で絞ると閉じ目（surface 1412）が消える。`bind+random` がコマの無いあいだ
/// 何も積まないことは次のテストが見る。
#[test]
fn pattern0_less_pure_bind_draws_its_patterns() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["closed_eye.png"]);
    let map = id_to_path(&atlas, &["closed_eye.png"]);

    // pattern0 なし・index=1 のみが surface 1412（閉じ目相当）を参照する純 `bind`。
    let blink = single_pattern_anim(1400, Interval::Bind, 1, 1412);
    let host = surface_with_anims(1000, Vec::new(), vec![blink]);
    let part = surface(1412, vec![elem(0, "closed_eye.png", 0, 0)]);
    let shell = shell_of(vec![host, part]);
    let world = bound_world(&shell, &[], &atlas);

    let binds = BindSet::from_ids([1400]);
    let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());

    assert_eq!(
        ops_to_paths(&ops, &map),
        vec!["closed_eye.png"],
        "pattern0 なしの純 bind も pattern1 を積む（M6）"
    );
}

/// 要件 9.2: index==0 と index==1 が共存 → `bind+random` はコマが無いあいだ何も
/// 静的に積まない（animated-surface-log-flood 件 26 の決めで期待を直した）。純 `bind` は全部のコマを
/// 番号の順に重ねる（seriko-pattern-calls 6.8・SSP の測り M6 で期待を直した）。
#[test]
fn coexisting_patterns_bind_random_draws_none_pure_bind_draws_all() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas(base, &["open_eye.png", "closed_eye.png"]);
    let map = id_to_path(&atlas, &["open_eye.png", "closed_eye.png"]);

    // bind id=1: pattern0(index0)→open(surface 1100)、pattern1(index1)→closed(surface 1200)。
    let anim = |interval| Animation {
        id: 1,
        interval,
        patterns: vec![pattern(0, "overlay", 1100), pattern(1, "overlay", 1200)],
    };
    let paths = |interval| {
        let host = surface_with_anims(1000, Vec::new(), vec![anim(interval)]);
        let open = surface(1100, vec![elem(0, "open_eye.png", 0, 0)]);
        let closed = surface(1200, vec![elem(0, "closed_eye.png", 0, 0)]);
        let shell = shell_of(vec![host, open, closed]);
        let world = bound_world(&shell, &[], &atlas);

        let binds = BindSet::from_ids([1]);
        let ops = ops(&world, &atlas, 1000, &binds, &PatternState::default());
        ops_to_paths(&ops, &map)
    };

    // `bind+random`: コマが無いあいだは pattern0 も積まない（animated-surface-log-flood 件 26）。
    assert!(paths(Interval::BindRandom { k: 4 }).is_empty());
    // 純 `bind`: 全部のコマを番号の順に（M6）。
    assert_eq!(
        paths(Interval::Bind),
        vec!["open_eye.png", "closed_eye.png"]
    );
}
