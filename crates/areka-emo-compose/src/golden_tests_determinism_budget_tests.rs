//! 決定性と計画の予算の fixture テーマ。

use super::test_support::{
    build_atlas_for_surface1000, parse_emo2_shell, plan_surface1000, shell_master_dir,
    surface1000_bind_parts,
};
use super::{BindSet, Composer, EmoWorld, PatternState, SetId};
use crate::plan_test_support::{bound_world, ops_and_extent};

// ============================================================================
// 決定性と計画の予算の fixture 検証テスト。
//
// composer_tests.rs は toy surface で決定性・計画の使い回しを固定する。ここは **emo2 fixture の本体
// surface（surface1000＋実 bind 群）**を入力に、次の項目を
// fixture レベルで固定する:
//   1. **計画の予算**: 同一 surface1000＋同一 BindSet を 1 つの Composer・1 つの計画で反復
//      plan_into し、計画の項の列の先頭と容量・命令列・作業場の列の容量が初回から変わらない
//      ことを **容量不変 assert** で固定する（同じ Composer の繰り返しが同じ計画を出すことも併せて）。
//      外形は束縛のときに作った外形の表から引くので、計画の中で外形をたどらない。
//   2. **命令数**: surface1000＋有効 bind 集合（3 本・1 本・0 本）で計画を建て、`ops.len()` が
//      **描画層数（＝解決した有効 bind 数）**に等しいことを assert する（1 つの検体の等値で、線形の証明ではない）。
//   3. **決定性**: 独立した 2 つの合成器の計画が等しいことを固定する。surface1000（複数 bind の入れ子
//      flatten を含む最も内容の濃い経路）を主対象とする。
//
// すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし）。surface1000 の bind パーツ
// 土台（surface1000_bind_parts / build_atlas_for_surface1000）を使う。
// ============================================================================

/// surface1000＋実 bind パーツ 3 種（1100/1200/1302）が「全て解決し描画命令を生む」有効 bind 集合。
///
/// [`surface1000_bind_parts`] の各 anim_id は [`build_atlas_for_surface1000`] が単色画像を挿入した
/// パーツで、`AtlasEntry.placement` が `Some`（不透明コアあり）に解決される。ゆえに build_plan は
/// 各有効 bind につき 1 命令ずつ生む。集合サイズ＝描画層数＝命令数（O(elements) の分母）になる。
fn surface1000_full_bindset() -> BindSet {
    BindSet::from_ids(surface1000_bind_parts().iter().map(|p| p.anim_id))
}

/// 決定性: emo2 surface1000＋固定 BindSet を 2 回計画 → 等しい計画。
///
/// 最も内容の濃い経路（静的 element ゼロ・複数有効 bind の入れ子 surface flatten＋animation-sort）
/// で、独立した 2 つの `Composer` による 2 回の計画が一致することを固定する。非空 bind 集合ゆえ計画は
/// 項を持ち、決定性 assert は空虚でない。
#[test]
fn surface1000_plan_is_deterministic() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    let first = plan_surface1000(&shell, &atlas, &binds);
    let second = plan_surface1000(&shell, &atlas, &binds);

    assert_eq!(
        first, second,
        "同一入力（surface1000＋固定 BindSet）→ 2 回の計画は等しい"
    );
    assert!(
        !first.items.is_empty(),
        "非空 bind 集合の計画は項を持つ（決定性 assert は空虚でない）"
    );
}

/// 計画の予算: emo2 surface1000 の反復 plan_into。
///
/// **容量不変 assert**を採る。#[global_allocator] を差し替えるやり方はプロセス全体を
/// 汚染し既存テストを不安定化しうるため採らない。初回 plan_into で計画／スクラッチが確定した後、
/// 後続呼び出しで
///   ① 計画の項の列の**先頭ポインタと容量が不変**（realloc が起きれば移動する＝再割り当てなしの直接証拠）、
///   ② Composer 内部 `ops` スクラッチの**容量が非成長**（毎回 clear+push で再利用）、
///   ③ Composer の作業場（`PlanScratch`）の全部の列と集合の**容量が非成長**（入れ子 flatten の
///      祖先スタック・段の積み荷・層 (ii) の id などの再利用。この検体で使う 6 つは 0 でないことも見る）、
///   ④ 計画と命令数が初回と同じ（同じ Composer の繰り返しでも決定的）、
/// を反復ループ全域で固定する。外形は束縛のときの表から引くので、計画の中で外形をたどらない。
/// 入力が変わるときの確保の有無は確かめていない。
#[test]
fn surface1000_replan_capacities_do_not_grow() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    let world = bound_world(&shell, &[], &atlas);

    let mut composer = Composer::new();
    let mut out = plan_surface1000(&shell, &atlas, &BindSet::default());

    // ウォームアップ（初回）: 計画／スクラッチの大きさがここで確定する。
    composer
        .plan_into(
            &mut out,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("初回 plan_into は Ok");

    let ptr0 = out.items.as_ptr();
    let cap0 = out.items.capacity();
    let plan0 = out.clone();
    let ops_len0 = composer.ops.len();
    let ops_cap0 = composer.ops.capacity();
    let caps0 = composer.scratch.capacities();
    let mut used = 0;
    for (name, cap) in caps0 {
        if matches!(
            name,
            "visited" | "on_path" | "steps" | "frames" | "merged_ids" | "anims"
        ) {
            assert!(cap > 0, "作業場の {name} を使っている（空虚でない）");
            used += 1;
        }
    }
    assert_eq!(used, 6, "確かめる作業場の列の名前が 6 つとも在る");

    // 定常状態: 同一 surface＋同一 BindSet を反復。初回で確定した容量から一切成長しないこと。
    for iter in 0..8 {
        composer
            .plan_into(
                &mut out,
                &world,
                &atlas,
                1000,
                &binds,
                &PatternState::default(),
            )
            .unwrap_or_else(|e| panic!("反復 {iter} 回目の plan_into は Ok: {e:?}"));

        assert_eq!(
            out.items.as_ptr(),
            ptr0,
            "反復 {iter}: 計画の項の列の先頭ポインタ不変＝realloc なし"
        );
        assert_eq!(out.items.capacity(), cap0, "反復 {iter}: 項の列の容量不変");
        assert_eq!(out, plan0, "反復 {iter}: 計画は初回と等しい");
        assert_eq!(composer.ops.len(), ops_len0, "反復 {iter}: 命令数は決定的");
        assert_eq!(
            composer.ops.capacity(),
            ops_cap0,
            "反復 {iter}: ops スクラッチ容量が非成長（clear+push 再利用）"
        );
        assert_eq!(
            composer.scratch.capacities(),
            caps0,
            "反復 {iter}: 作業場の全部の列と集合の容量が非成長（再利用）"
        );
    }

    // 計画は項を持つ（定常状態でも現に計画している＝空実装で容量安定を偽装していない）。
    assert!(!out.items.is_empty(), "反復後も計画は項を持つ");
}

/// 外形の表: 束縛の前は表が無く外形 0×0＝`EmptyComposition`（今までの前提どおり）。束縛すると
/// 表から引いた外形で合成でき、その値は直にたどった外形と同じ。
#[test]
fn extent_comes_from_the_table_made_by_bind_atlas() {
    use crate::error::ComposeError;
    use crate::plan::compute_extent;

    let shell = parse_emo2_shell();
    let atlas = build_atlas_for_surface1000(&shell, &shell_master_dir());
    let binds = surface1000_full_bindset();
    let mut world = EmoWorld::build(&shell);
    let plan =
        |world: &EmoWorld| ops_and_extent(world, &atlas, 1000, &binds, &PatternState::default()).1;
    assert_eq!(
        plan(&world),
        Err(ComposeError::EmptyComposition(1000)),
        "束縛の前は外形 0×0"
    );
    world.bind_atlas(&atlas, SetId(0));
    let extent = plan(&world).expect("束縛の後は合成できる");
    assert_eq!(extent, compute_extent(&world, &atlas, 1000));
}

/// 命令数: surface1000＋全有効 bind → 命令数 == 描画層数。
///
/// surface1000 は静的 element ゼロ・全パーツ bind の本体 surface。有効 bind 集合を
/// [`surface1000_full_bindset`]（解決する 3 パーツ 1100/1200/1302）にすると、build_plan は各有効
/// bind の pattern0 入れ子参照を 1 層ずつ flatten し、**有効 bind ごとに厳密に 1 命令**を生む。
/// よって `ops.len() == 有効 bind 数`（＝描画層数）を固定する。
///
/// 1 つの検体の等値なので線形の証明ではないが、二次（層×層）や画素比例（外形 w*h ≈ 数万）で命令を
/// 生む実装ならこの等値が破れる。各 bind パーツ surface（1100/1200/1302）は element 1 本の単純
/// surface で、入れ子 flatten は 1 段。ゆえに命令数 = Σ(有効 bind の可視層数) = 有効 bind 数。
#[test]
fn surface1000_instruction_count_equals_bound_layers() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    let world = bound_world(&shell, &[], &atlas);

    let (ops, result) = ops_and_extent(&world, &atlas, 1000, &binds, &PatternState::default());
    result.expect("build_plan は Ok（surface1000 存在＋外形非ゼロ）");

    assert_eq!(
        ops.len(),
        3,
        "解決した 3 有効 bind（1100/1200/1302）→ 3 命令"
    );

    // 各命令の method は overlay（M1 実装対象）。
    for op in &ops {
        assert_eq!(
            op.method,
            crate::method::ComposeMethod::Overlay,
            "全命令は overlay 層"
        );
    }

    let one = BindSet::from_ids([surface1000_bind_parts()[0].anim_id]);
    let (ops1, result) = ops_and_extent(&world, &atlas, 1000, &one, &PatternState::default());
    result.expect("単一 bind でも Ok");
    assert_eq!(ops1.len(), 1, "有効 bind 1 本 → 命令 1 本（層数に線形）");

    let (ops0, result) = ops_and_extent(
        &world,
        &atlas,
        1000,
        &BindSet::default(),
        &PatternState::default(),
    );
    result.expect("空 bind でも surface 存在＋外形非ゼロで Ok");
    assert_eq!(ops0.len(), 0, "空 bind 集合 → 描画命令ゼロ（層数 0）");
}
