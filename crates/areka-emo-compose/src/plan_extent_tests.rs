use super::test_support::*;
use super::*;
use crate::world::EmoWorld;
use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::Element;
use std::path::Path;

// ── task 5.4: placement None スキップ＋静的キャンバス外形算出（有効 bind 非依存） ─────────
//
// 外形は `compute_extent` が全定義層（全 element ＋全 bind animation の pattern0＝有効 bind
// 非依存）の「配置オフセット＋原寸」の和集合として算出する（要件 6.5・原点 (0,0) 固定・負方向
// クリップ）。`original` は bake が「登録画像の原寸」をそのまま記録する（trim.rs）ため、各 element
// の寄与サイズは register 時の (w,h) で制御できる。以下の土台ヘルパはサイズ可変・全透明を扱う。

/// tightly-packed w×h の premultiplied BGRA・不透明（α=255）画像スペックを組む。
///
/// bake は原寸（w,h）を `AtlasEntry.original` にそのまま記録するため、外形寄与サイズを
/// 登録側で制御できる（trim は α>0 の bbox＝ここでは全面ゆえ placement は Some）。
fn opaque_wxh(w: u32, h: u32) -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = w * 4;
    // 全画素不透明（BGR は識別不要ゆえ定数・α=255）。
    let bgra = vec![64u8; (stride * h) as usize];
    (w, h, stride, bgra, true)
}

/// tightly-packed w×h の premultiplied BGRA・**全透明**（α=0）画像スペックを組む。
///
/// 全 α==0 ゆえ trim は bbox を得られず `placement: None`（空エントリ）にするが、`original`
/// は (w,h) を記録する（trim.rs「原寸は全透明でも記録」）。命令からはスキップされ（要件 6.3）、
/// 外形へは (w,h) で寄与する（要件 6.5）ことを突く。
fn transparent_wxh(w: u32, h: u32) -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = w * 4;
    // premultiplied ゆえ α=0 なら BGR も 0（完全透明）。
    let bgra = vec![0u8; (stride * h) as usize];
    (w, h, stride, bgra, true)
}

/// (rel_path, (w,h), opaque?) 群から `AtlasTable` を bake する（サイズ・透過を個別指定）。
///
/// `bake_atlas` は全 element 2×2・不透明固定だが、本ヘルパは外形テスト用に原寸と透過を制御する。
fn bake_atlas_sized(base: &Path, specs: &[(&str, (u32, u32), bool)]) -> AtlasTable {
    let elements: Vec<Element> = specs.iter().map(|(r, _, _)| elem(0, r, 0, 0)).collect();
    let surfaces = vec![surface(0, elements)];
    let mut dec = MemoryDecoder::new();
    for (rel, (w, h), opaque) in specs {
        let (iw, ih, stride, bgra, has_alpha) = if *opaque {
            opaque_wxh(*w, *h)
        } else {
            transparent_wxh(*w, *h)
        };
        dec.insert(base.join(rel), iw, ih, stride, bgra, has_alpha);
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

/// テスト5.4-①（**受入基準**・要件 6.5）: 同一 surface へ空／部分／全 BindSet を渡しても
/// `compute_extent` が返す [`Extent`] は不変（有効 bind 集合に依存しない静的量）。
///
/// host=1000 は静的 element base(40×30) を持ち、bind id=1→part1(200×10)、id=2→part2(10×150)、
/// id=3→part3(60×60) を各 pattern0 で参照。外形は全 element＋**全 bind pattern0**の和集合ゆえ
/// max(base.w, part1.w)=200・max(base.h, part2.h)=150＝Extent{200,150}。BindSet を空/部分/全に
/// 変えても同一でなければ「有効 bind 依存」バグ（6.5 違反）。これが本 task の核心。
#[test]
fn extent_is_independent_of_bindset() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (40, 30), true),
            ("part1.png", (200, 10), true),
            ("part2.png", (10, 150), true),
            ("part3.png", (60, 60), true),
        ],
    );

    // host=1000: 静的 base ＋ bind id=1/2/3 が part1/part2/part3 を (0,0) 参照。
    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![
            bind_anim(1, 1100, 0, 0),
            bind_anim(2, 1200, 0, 0),
            bind_anim(3, 1300, 0, 0),
        ],
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let part2 = surface(1200, vec![elem(0, "part2.png", 0, 0)]);
    let part3 = surface(1300, vec![elem(0, "part3.png", 0, 0)]);
    let shell = shell_of(vec![host, part1, part2, part3]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // 全 element＋全 bind pattern0 の和集合: w=max(40,200,10,60)=200・h=max(30,10,150,60)=150。
    let expected = Extent { w: 200, h: 150 };

    let empty = compute_extent(&mut Vec::new(), &world, &atlas, 1000);
    let partial = {
        let _b = BindSet::from_ids([2]);
        compute_extent(&mut Vec::new(), &world, &atlas, 1000)
    };
    let full = {
        let _b = BindSet::from_ids([1, 2, 3]);
        compute_extent(&mut Vec::new(), &world, &atlas, 1000)
    };

    // compute_extent は BindSet を引数に取らない（＝構造的に有効 bind 非依存）。三者一致＋期待値一致。
    assert_eq!(
        empty, expected,
        "空 BindSet 相当でも全 bind pattern0 を母集合に外形算出"
    );
    assert_eq!(partial, expected, "部分 BindSet でも外形不変");
    assert_eq!(full, expected, "全 BindSet でも外形不変");
    assert_eq!(empty, partial);
    assert_eq!(partial, full);
}

/// テスト5.4-②（要件 6.5・全 bind 母集合）: 非活性 bind（どの BindSet にも入れない）の
/// pattern0 が参照する巨大入れ子 surface が、それでも外形を支配する。
///
/// host=2000 の静的 element は小さい tiny(4×4) のみ。bind id=9 は**一度も activate されない**が、
/// その pattern0 が huge(500×400) を参照する。もし外形が「有効 bind のみ」を数えるバグなら tiny
/// の 4×4 になるはずだが、正しくは全 bind pattern0 母集合ゆえ 500×400。空 BindSet でも huge が支配。
#[test]
fn extent_unions_inactive_bind_contribution() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[("tiny.png", (4, 4), true), ("huge.png", (500, 400), true)],
    );

    let host = surface_with_anims(
        2000,
        vec![elem(0, "tiny.png", 0, 0)],
        vec![bind_anim(9, 2900, 0, 0)], // id=9 は以降どの BindSet にも入れない。
    );
    let huge = surface(2900, vec![elem(0, "huge.png", 0, 0)]);
    let shell = shell_of(vec![host, huge]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // 有効 bind 非依存ゆえ、activate されない id=9 の huge も外形へ寄与＝500×400 が支配。
    let extent = compute_extent(&mut Vec::new(), &world, &atlas, 2000);
    assert_eq!(
        extent,
        Extent { w: 500, h: 400 },
        "非活性 bind の入れ子巨大 surface も外形を支配（全 bind 母集合・6.5）"
    );
}

/// テスト5.4-③（要件 6.3＋6.5）: placement None（全透明）element は命令からスキップされるが、
/// 外形には原寸で寄与する。
///
/// surface=3000 は静的 element を 2 本持つ: solid(20×20・不透明) と ghost(300×300・**全透明**＝
/// placement None)。`derive_ops` は ghost をスキップし solid の 1 命令のみ（要件 6.3）。一方
/// `compute_extent` は ghost の原寸 300×300 を数えて 300×300（要件 6.5・全透明でも原寸寄与）。
#[test]
fn placement_none_skipped_from_ops_but_counted_in_extent() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("solid.png", (20, 20), true),
            ("ghost.png", (300, 300), false), // 全透明→ placement None・original は 300×300。
        ],
    );
    let solid_id = atlas.resolve(SetId(0), "solid.png").expect("solid 解決");
    let ghost_id = atlas.resolve(SetId(0), "ghost.png").expect("ghost 解決");

    // 全透明 ghost は bake で placement None・原寸は保持（前提の実証）。
    assert!(
        atlas.entry(ghost_id).placement.is_none(),
        "全透明 element は bake で placement None（前提）"
    );
    assert_eq!(
        atlas.entry(ghost_id).original,
        areka_emo_atlas::Size { w: 300, h: 300 }
    );
    assert!(
        atlas.entry(solid_id).placement.is_some(),
        "不透明 element は placement Some"
    );

    let surf = surface(
        3000,
        vec![elem(0, "solid.png", 0, 0), elem(1, "ghost.png", 0, 0)],
    );
    let shell = shell_of(vec![surf]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // 命令: ghost はスキップされ solid の 1 本のみ（要件 6.3）。
    let binds = BindSet::default();
    let mut ops = Vec::new();
    derive_ops(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        3000,
        &binds,
        &PatternState::default(),
    );
    assert_eq!(
        ops.len(),
        1,
        "placement None（ghost）は命令からスキップ（要件 6.3）"
    );
    assert_eq!(ops[0].element, solid_id, "残る命令は不透明 solid のみ");

    // 外形: ghost の原寸 300×300 を数える（要件 6.5・全透明でも寄与）。
    let extent = compute_extent(&mut Vec::new(), &world, &atlas, 3000);
    assert_eq!(
        extent,
        Extent { w: 300, h: 300 },
        "placement None element も原寸で外形へ寄与（要件 6.5）"
    );
}

/// テスト5.4-④（要件 6.5・原点固定／負方向クリップ）: 負オフセットの入れ子層は原点 (0,0) を
/// 上へ動かさず、外形を基底より縮めない。
///
/// host=4000 の静的 base(50×40)。bind id=1 の pattern0 が small(10×10) を **(-100,-100)** で参照。
/// 負オフセット層の寄与は max(0, -100+10)=0 ゆえ外形へ効かず、原点は (0,0) のまま・外形は base の
/// 50×40 を下回らない（負方向はみ出しは転写時クリップ・議題2裁定 (A)）。
#[test]
fn negative_offset_is_clipped_and_origin_stays_fixed() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[("base.png", (50, 40), true), ("small.png", (10, 10), true)],
    );

    let host = surface_with_anims(
        4000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 4100, -100, -100)], // 負オフセット参照。
    );
    let small = surface(4100, vec![elem(0, "small.png", 0, 0)]);
    let shell = shell_of(vec![host, small]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // small は (-100+10, -100+10)=(-90,-90) ゆえ max(0,·)=0 で寄与せず、base の 50×40 が残る。
    let extent = compute_extent(&mut Vec::new(), &world, &atlas, 4000);
    assert_eq!(
        extent,
        Extent { w: 50, h: 40 },
        "負オフセット層は原点クリップ＝外形を縮めない（要件 6.5・原点 (0,0) 固定）"
    );
}

/// テスト5.4-⑤（要件 6.5・基底一致）: (0,0) 原寸 (W,H) の単一 element・より大きい層なし→
/// 外形は原寸ちょうど。
#[test]
fn extent_equals_base_original_when_no_larger_layers() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(base, &[("only.png", (123, 45), true)]);

    let surf = surface(5000, vec![elem(0, "only.png", 0, 0)]);
    let shell = shell_of(vec![surf]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let extent = compute_extent(&mut Vec::new(), &world, &atlas, 5000);
    assert_eq!(
        extent,
        Extent { w: 123, h: 45 },
        "単一 element・(0,0)→外形＝原寸"
    );
}

/// テスト5.4-⑥（要件 10.1・決定性）: 同一入力で 2 回算出→ [`Extent`] が同値。
#[test]
fn extent_is_deterministic() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[("base.png", (40, 30), true), ("part.png", (200, 150), true)],
    );
    let host = surface_with_anims(
        6000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 6100, 5, 7)],
    );
    let part = surface(6100, vec![elem(0, "part.png", 0, 0)]);
    let shell = shell_of(vec![host, part]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let e1 = compute_extent(&mut Vec::new(), &world, &atlas, 6000);
    let e2 = compute_extent(&mut Vec::new(), &world, &atlas, 6000);
    assert_eq!(e1, e2, "同一入力→同一 Extent（決定的）");
    // 参考: part は (5,7) 参照ゆえ w=max(40, 5+200)=205・h=max(30, 7+150)=157。
    assert_eq!(e1, Extent { w: 205, h: 157 });
}

// ── task 5.5: 命令ゼロ時の 3 分類（正常空合成／対象不在／退化データ）─────────────────
//
// build_plan は derive_ops（有効 bind 依存の命令列）＋ compute_extent（有効 bind 非依存の
// 静的外形）を wrap し、design「Error Handling」表の 3 状態を厳密に区別する:
//   - 対象 surface 不在        → Err(SurfaceNotFound)（error ログ・要件 10.5）。
//   - surface 存在・命令ゼロでも → Ok(非ゼロ Extent)＋空 ops（正常全透明・要件 6.6・議題2裁定）。
//   - 定義層皆無で外形 0×0      → Err(EmptyComposition)（error ログ・要件 10.5・唯一の真の失敗退化）。
// 非パニック（要件 1.4）。out_ops はエントリで clear する再利用スクラッチ（要件 10.3）。

use crate::error::ComposeError;

/// テスト5.5-①（要件 10.5）: 対象 surface 不在 → `Err(SurfaceNotFound)`・ops 空・非パニック。
#[test]
fn build_plan_absent_surface_is_surface_not_found() {
    // surface を一切持たない World（9999 は不在）。
    let world = EmoWorld::build(&shell_of(Vec::new()));
    let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);

    let binds = BindSet::default();
    let mut ops = Vec::new();
    let result = build_plan(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        9999,
        &binds,
        &PatternState::default(),
    );

    assert_eq!(
        result,
        Err(ComposeError::SurfaceNotFound(9999)),
        "不在 surface は SurfaceNotFound（要件 10.5）"
    );
    assert!(ops.is_empty(), "不在 surface では命令を積まない");
}

/// テスト5.5-②（**受入基準**・要件 6.6）: 全 element が全透明の surface → `Ok(非ゼロ Extent)`＋空 ops。
///
/// surface=3000 は element を 2 本持つがいずれも全透明（α=0 → placement None）。描画可能命令は
/// ゼロだが、外形は原寸（300×300 が支配）で非ゼロ。これはエラーでなく **正常系**（議題2裁定）。
#[test]
fn build_plan_all_transparent_is_ok_empty_ops_nonzero_extent() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("ghost1.png", (300, 300), false), // 全透明→ placement None。
            ("ghost2.png", (20, 20), false),   // 全透明→ placement None。
        ],
    );

    let surf = surface(
        3000,
        vec![elem(0, "ghost1.png", 0, 0), elem(1, "ghost2.png", 0, 0)],
    );
    let shell = shell_of(vec![surf]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::default();
    let mut ops = Vec::new();
    let result = build_plan(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        3000,
        &binds,
        &PatternState::default(),
    );

    // 描画可能命令ゼロでも Err にしない（要件 6.6・議題2裁定）。
    let extent = result.expect("全透明でもエラーにせず Ok（要件 6.6）");
    assert!(
        ops.is_empty(),
        "全 element 全透明 → 空 ops（描画可能命令ゼロ）"
    );
    // 外形は原寸で非ゼロ（全透明でも original で寄与＝300×300 が支配）。
    assert_eq!(
        extent,
        Extent { w: 300, h: 300 },
        "非ゼロの静的外形を返す（要件 6.6）"
    );
    assert_ne!(extent.w, 0);
    assert_ne!(extent.h, 0);
}

/// テスト5.5-③（要件 6.6）: bind のみ surface＋空 BindSet → `Ok(非ゼロ Extent)`＋空 ops。
///
/// host=1000 は静的 element なし・全パーツ bind。空 BindSet ゆえ有効 bind ゼロ＝命令ゼロだが、
/// 外形は全 bind pattern0 母集合（有効 bind 非依存）ゆえ非ゼロ。エラーでなく正常空合成。
#[test]
fn build_plan_empty_bindset_bind_only_is_ok_empty_ops_nonzero_extent() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("part1.png", (200, 10), true),
            ("part2.png", (10, 150), true),
        ],
    );

    // 静的 element なし・bind id=1/2 が part1/part2 を参照。
    let host = surface_with_anims(
        1000,
        Vec::new(),
        vec![bind_anim(1, 1100, 0, 0), bind_anim(2, 1200, 0, 0)],
    );
    let part1 = surface(1100, vec![elem(0, "part1.png", 0, 0)]);
    let part2 = surface(1200, vec![elem(0, "part2.png", 0, 0)]);
    let shell = shell_of(vec![host, part1, part2]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // 空 BindSet → 有効 bind ゼロ＝描画可能命令ゼロ。
    let binds = BindSet::default();
    let mut ops = Vec::new();
    let result = build_plan(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        1000,
        &binds,
        &PatternState::default(),
    );

    let extent = result.expect("空 BindSet でも正常（描画可能命令ゼロは失敗でない・要件 6.6）");
    assert!(
        ops.is_empty(),
        "空 BindSet → bind 命令ゼロ・静的 element も無し → 空 ops"
    );
    // 外形は全 bind pattern0 の和集合（有効 bind 非依存）: w=max(200,10)=200・h=max(10,150)=150。
    assert_eq!(
        extent,
        Extent { w: 200, h: 150 },
        "有効 bind 非依存の非ゼロ外形"
    );
}

/// テスト5.5-④（要件 10.5・議題2裁定）: 定義層皆無で外形 0×0 → `Err(EmptyComposition)`。
///
/// surface=7000 は EXISTS するが element ゼロ・bind ゼロ（外形へ寄与する層が皆無）。
/// compute_extent が {0,0} を返す唯一の真の失敗退化ケース。
#[test]
fn build_plan_no_layers_degenerate_is_empty_composition() {
    // element ゼロ・animation ゼロの surface（存在はするが定義層が皆無）。
    let surf = surface(7000, Vec::new());
    let shell = shell_of(vec![surf]);
    let mut world = EmoWorld::build(&shell);
    // atlas は引かれない（element がない）が bind_atlas は呼ぶ（binding 挿入）。
    let atlas = bake_atlas(Path::new("shell/master"), &["dummy.png"]);
    world.bind_atlas(&atlas, SetId(0));

    // 前提の実証: 外形が 0×0（定義層皆無）。
    assert_eq!(
        compute_extent(&mut Vec::new(), &world, &atlas, 7000),
        Extent { w: 0, h: 0 },
        "定義層皆無 → 外形 0×0（前提）"
    );

    let binds = BindSet::default();
    let mut ops = Vec::new();
    let result = build_plan(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        7000,
        &binds,
        &PatternState::default(),
    );

    assert_eq!(
        result,
        Err(ComposeError::EmptyComposition(7000)),
        "定義層皆無で 0×0 の退化のみ EmptyComposition（要件 10.5・議題2裁定）"
    );
    assert!(ops.is_empty());
}

/// テスト5.5-⑤（sanity・要件 6.6 対比）: 可視 element を持つ通常 surface → `Ok`＋非空 ops。
#[test]
fn build_plan_populated_surface_is_ok_with_nonempty_ops() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(base, &[("visible.png", (80, 60), true)]);
    let visible_id = atlas
        .resolve(SetId(0), "visible.png")
        .expect("visible 解決");

    let surf = surface(5000, vec![elem(0, "visible.png", 0, 0)]);
    let shell = shell_of(vec![surf]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::default();
    let mut ops = Vec::new();
    let result = build_plan(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        5000,
        &binds,
        &PatternState::default(),
    );

    let extent = result.expect("通常 surface は Ok");
    assert_eq!(ops.len(), 1, "可視 element 1 本 → 命令 1 本");
    assert_eq!(ops[0].element, visible_id);
    assert_eq!(extent, Extent { w: 80, h: 60 }, "外形＝element 原寸");
}

/// テスト5.5-⑥（要件 10.3・10.1）: out_ops はエントリで clear される（スクラッチ再利用）・決定的。
#[test]
fn build_plan_clears_scratch_and_is_deterministic() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(base, &[("visible.png", (80, 60), true)]);
    let visible_id = atlas
        .resolve(SetId(0), "visible.png")
        .expect("visible 解決");

    let surf = surface(5000, vec![elem(0, "visible.png", 0, 0)]);
    let shell = shell_of(vec![surf]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::default();

    // 事前にゴミを詰めた out_ops → build_plan がエントリで clear する（ゴミは消える）。
    let junk = BlitOp {
        element: ElementId(u32::MAX),
        transform: Transform::identity(),
        method: ComposeMethod::Overlay,
    };
    let mut ops = vec![junk.clone(), junk.clone(), junk];
    let e1 = build_plan(
        &mut ops,
        &mut Vec::new(),
        &world,
        &atlas,
        5000,
        &binds,
        &PatternState::default(),
    )
    .expect("Ok");

    assert_eq!(
        ops.len(),
        1,
        "エントリで clear ＝ この surface の命令のみが残る"
    );
    assert_eq!(ops[0].element, visible_id, "ゴミは残らない");

    // 2 回目（別スクラッチ）→ バイト等価・同一 Extent（決定性）。
    let mut ops2 = Vec::new();
    let e2 = build_plan(
        &mut ops2,
        &mut Vec::new(),
        &world,
        &atlas,
        5000,
        &binds,
        &PatternState::default(),
    )
    .expect("Ok");
    assert_eq!(ops, ops2, "同一入力→同一 ops（バイト等価）");
    assert_eq!(e1, e2, "同一入力→同一 Extent");
}
