use super::*;
use crate::plan_test_support::*;
use crate::world::EmoWorld;
use areka_emo_atlas::SetId;
use std::path::Path;

// ── placement None スキップ＋静的キャンバス外形算出（有効 bind 非依存） ─────────
//
// 外形は `compute_extent` が全定義層（全 element ＋全 bind・`always` animation の描くコマ＝有効 bind
// 非依存）の「配置オフセット＋原寸」の和集合として算出する（要件 6.5・原点 (0,0) 固定・負方向
// クリップ）。`original` は bake が「登録画像の原寸」をそのまま記録する（trim.rs）ため、各 element
// の寄与サイズは register 時の (w,h) で制御できる。以下の土台ヘルパはサイズ可変・全透明を扱う。

/// テスト5.4-①（**受入基準**・要件 6.5）: 同一 surface へ空／部分／全 BindSet を渡しても
/// [`build_plan`] が返す [`Extent`] は不変（有効 bind 集合に依存しない静的量）。
///
/// host=1000 は静的 element base(40×30) を持ち、bind id=1→part1(200×10)、id=2→part2(10×150)、
/// id=3→part3(60×60) を各 pattern0 で参照。外形は全 element＋**全 bind の描くコマ**の和集合ゆえ
/// max(base.w, part1.w)=200・max(base.h, part2.h)=150＝Extent{200,150}。BindSet を空/部分/全に
/// 変えても同一でなければ「有効 bind 依存」バグ（6.5 違反）。
#[test]
fn extent_is_independent_of_bindset() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (40, 30), Some((64, 64, 64))),
            ("part1.png", (200, 10), Some((64, 64, 64))),
            ("part2.png", (10, 150), Some((64, 64, 64))),
            ("part3.png", (60, 60), Some((64, 64, 64))),
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

    // 全 element＋全 bind の描くコマ の和集合: w=max(40,200,10,60)=200・h=max(30,10,150,60)=150。
    let expected = Extent { w: 200, h: 150 };

    let extent_with = |binds: BindSet| {
        build_plan(
            &mut Vec::new(),
            &mut Default::default(),
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("合成できる")
    };
    assert_eq!(
        extent_with(BindSet::default()),
        expected,
        "空 BindSet でも全 bind の描くコマ を母集合に外形算出"
    );
    assert_eq!(
        extent_with(BindSet::from_ids([2])),
        expected,
        "部分 BindSet でも外形不変"
    );
    assert_eq!(
        extent_with(BindSet::from_ids([1, 2, 3])),
        expected,
        "全 BindSet でも外形不変"
    );
}

/// テスト5.4-②（要件 6.5・全 bind 母集合）: 非活性 bind（どの BindSet にも入れない）の
/// pattern0 が参照する巨大入れ子 surface が、それでも外形を支配する。
///
/// host=2000 の静的 element は小さい tiny(4×4) のみ。bind id=9 は**一度も activate されない**が、
/// その pattern0 が huge(500×400) を参照する。もし外形が「有効 bind のみ」を数えるバグなら tiny
/// の 4×4 になるはずだが、正しくは全 bind の描くコマ 母集合ゆえ 500×400。空 BindSet でも huge が支配。
#[test]
fn extent_unions_inactive_bind_contribution() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("tiny.png", (4, 4), Some((64, 64, 64))),
            ("huge.png", (500, 400), Some((64, 64, 64))),
        ],
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
    let extent = compute_extent(&world, &atlas, 2000);
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
            ("solid.png", (20, 20), Some((64, 64, 64))),
            ("ghost.png", (300, 300), None), // 全透明→ placement None・original は 300×300。
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
    let ops = ops(&world, &atlas, 3000, &binds, &PatternState::default());
    assert_eq!(
        ops.len(),
        1,
        "placement None（ghost）は命令からスキップ（要件 6.3）"
    );
    assert_eq!(ops[0].element, solid_id, "残る命令は不透明 solid のみ");

    // 外形: ghost の原寸 300×300 を数える（要件 6.5・全透明でも寄与）。
    let extent = compute_extent(&world, &atlas, 3000);
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
        &[
            ("base.png", (50, 40), Some((64, 64, 64))),
            ("small.png", (10, 10), Some((64, 64, 64))),
        ],
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
    let extent = compute_extent(&world, &atlas, 4000);
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
    let atlas = bake_atlas_sized(base, &[("only.png", (123, 45), Some((64, 64, 64)))]);

    let surf = surface(5000, vec![elem(0, "only.png", 0, 0)]);
    let shell = shell_of(vec![surf]);
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let extent = compute_extent(&world, &atlas, 5000);
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
        &[
            ("base.png", (40, 30), Some((64, 64, 64))),
            ("part.png", (200, 150), Some((64, 64, 64))),
        ],
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

    let e1 = compute_extent(&world, &atlas, 6000);
    let e2 = compute_extent(&world, &atlas, 6000);
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
        &mut Default::default(),
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
            ("ghost1.png", (300, 300), None), // 全透明→ placement None。
            ("ghost2.png", (20, 20), None),   // 全透明→ placement None。
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
        &mut Default::default(),
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
}

/// テスト5.5-③（要件 6.6）: bind のみ surface＋空 BindSet → `Ok(非ゼロ Extent)`＋空 ops。
///
/// host=1000 は静的 element なし・全パーツ bind。空 BindSet ゆえ有効 bind ゼロ＝命令ゼロだが、
/// 外形は全 bind の描くコマ 母集合（有効 bind 非依存）ゆえ非ゼロ。エラーでなく正常空合成。
#[test]
fn build_plan_empty_bindset_bind_only_is_ok_empty_ops_nonzero_extent() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("part1.png", (200, 10), Some((64, 64, 64))),
            ("part2.png", (10, 150), Some((64, 64, 64))),
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
        &mut Default::default(),
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
    // 外形は全 bind の描くコマ の和集合（有効 bind 非依存）: w=max(200,10)=200・h=max(10,150)=150。
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
        compute_extent(&world, &atlas, 7000),
        Extent { w: 0, h: 0 },
        "定義層皆無 → 外形 0×0（前提）"
    );

    let binds = BindSet::default();
    let mut ops = Vec::new();
    let result = build_plan(
        &mut ops,
        &mut Default::default(),
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

/// テスト5.5-⑤（要件 6.6・10.3・10.1）: 可視 element を持つ通常 surface → `Ok`＋非空 ops。out_ops は
/// エントリで clear される（スクラッチ再利用）・決定的。
#[test]
fn build_plan_clears_scratch_and_is_deterministic() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(base, &[("visible.png", (80, 60), Some((64, 64, 64)))]);
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
        &mut Default::default(),
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
    assert_eq!(e1, Extent { w: 80, h: 60 }, "外形＝element 原寸");

    // 2 回目（別スクラッチ）→ バイト等価・同一 Extent（決定性）。
    let mut ops2 = Vec::new();
    let e2 = build_plan(
        &mut ops2,
        &mut Default::default(),
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

// ── audit-emo-compose 指摘 2: 外形が GPU の面の最大寸（一辺 16,384）を越えたら確保せずに失敗する ──

/// `top` の計画を建て、結果と捕まえた記録の ERROR の行数を返す。
fn plan_with_errors(
    world: &EmoWorld,
    atlas: &AtlasTable,
    top: u32,
) -> (Result<Extent, ComposeError>, usize) {
    let mut result = None;
    let logs = crate::log_capture::capture_logs(|| {
        result = Some(build_plan(
            &mut Vec::new(),
            &mut Default::default(),
            world,
            atlas,
            top,
            &BindSet::default(),
            &PatternState::default(),
        ));
    });
    (result.unwrap(), logs.matches("level=ERROR").count())
}

/// pattern を (100000, 100000) に置く純 `bind` の面は、外形が約 100000×100000（約 40 GB）になる。
/// 合成は確保の前に `ExtentTooLarge` を返し、出力先の絵の容量は 0 のまま・`error!` は 1 行。
#[test]
fn far_bind_pattern_fails_without_allocating() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (4, 4), Some((64, 64, 64))),
            ("part.png", (4, 4), Some((64, 64, 64))),
        ],
    );
    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 100_000, 100_000)],
    );
    let part = surface(1100, vec![elem(0, "part.png", 0, 0)]);
    let mut world = EmoWorld::build(&shell_of(vec![host, part]));
    world.bind_atlas(&atlas, SetId(0));
    let too_large = || ComposeError::ExtentTooLarge {
        surface: 1000,
        w: 100_004,
        h: 100_004,
    };

    // 計画の段で止まることを先に見る（直しが無いと下の合成が約 40 GB を確保しにいく）。
    let (planned, errors) = plan_with_errors(&world, &atlas, 1000);
    assert_eq!(planned, Err(too_large()));
    assert_eq!(errors, 1, "error! は 1 行");

    let mut out = crate::ComposedSurface::new(0, 0);
    let composed = crate::Composer::new().compose_into(
        &mut out,
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1]),
        &PatternState::default(),
    );
    assert_eq!(composed, Err(too_large()));
    assert_eq!(out.bytes_capacity(), 0, "絵は確保しない");
}

/// element定義の子を `u32::MAX + 10` の位置に置く面は、`u32` への切り詰めで小さな外形に化けず
/// `ExtentTooLarge` を返す（飽和の外形は u32::MAX）。
#[test]
fn child_beyond_u32_does_not_wrap_to_small_extent() {
    let far = u32::MAX as u64 + 10;
    let text = format!(
        "surface0\n{{\nelement0,overlay,base.png,0,0\nelement1,overlay,10,{far},0\n}}\n\
         surface10\n{{\nelement0,overlay,part.png,0,0\n}}\n"
    );
    let atlas = bake_atlas_sized(
        Path::new("shell/master"),
        &[
            ("base.png", (4, 4), Some((64, 64, 64))),
            ("part.png", (4, 4), Some((64, 64, 64))),
        ],
    );
    let mut world =
        EmoWorld::build_with_images(&areka_parsers::shell::parse(&text), &Default::default());
    world.bind_atlas(&atlas, SetId(0));

    let (planned, errors) = plan_with_errors(&world, &atlas, 0);
    assert_eq!(
        planned,
        Err(ComposeError::ExtentTooLarge {
            surface: 0,
            w: u32::MAX,
            h: 4,
        })
    );
    assert_eq!(errors, 1, "error! は 1 行");
}

/// 広がりの予算（指摘 3）: 予算 2 の外形は element定義の子 3 つのうち 2 つ目まで入って 3 つ目を数えず、
/// `error!` を 1 行だけ出す。予算 3 なら 3 つとも数える。
#[test]
fn spent_budget_leaves_children_not_yet_entered_out_of_the_extent() {
    let text = "surface0\n{\nelement0,overlay,10,0,0\nelement1,overlay,10,10,0\n\
                element2,overlay,10,20,0\n}\n\
                surface10\n{\nelement0,overlay,eye.png,0,0\n}\n";
    let atlas = bake_atlas_sized(
        Path::new("shell/master"),
        &[("eye.png", (4, 4), Some((64, 64, 64)))],
    );
    let mut world = EmoWorld::build(&areka_parsers::shell::parse(text));
    world.bind_atlas(&atlas, SetId(0));
    let run = |limit| {
        let mut extent = None;
        let logs = crate::log_capture::capture_logs(|| {
            extent = Some(compute_extent_limited(
                &mut Default::default(),
                &world,
                &atlas,
                0,
                limit,
                None,
            ));
        });
        (extent.expect("外形"), logs)
    };
    let (extent, logs) = run(2);
    assert_eq!(extent, Extent { w: 14, h: 4 });
    assert_eq!(logs.matches("level=ERROR").count(), 1, "{logs}");
    assert!(
        logs.contains("walk=\"extent\"") && logs.contains("root=0"),
        "{logs}"
    );
    let (extent, logs) = run(3);
    assert_eq!(extent, Extent { w: 24, h: 4 });
    assert!(!logs.contains("level=ERROR"), "{logs}");
}

/// 指摘 32: pattern の面の番号が u32 を越える（4294967297）なら、外形は先が無いと数え、
/// 切り詰めた番号（1）の面を数えない（ほかの走査と同じ `u32::try_from`）。
#[test]
fn extent_does_not_truncate_pattern_surface_beyond_u32() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("tiny.png", (4, 4), Some((64, 64, 64))),
            ("huge.png", (500, 400), Some((64, 64, 64))),
        ],
    );
    let host = surface_with_anims(
        2000,
        vec![elem(0, "tiny.png", 0, 0)],
        vec![bind_anim(9, 4_294_967_297, 0, 0)],
    );
    let one = surface(1, vec![elem(0, "huge.png", 0, 0)]);
    let mut world = EmoWorld::build(&shell_of(vec![host, one]));
    world.bind_atlas(&atlas, SetId(0));
    let extent = compute_extent(&world, &atlas, 2000);
    assert_eq!(extent, Extent { w: 4, h: 4 }, "面 1 を数えない");
}
