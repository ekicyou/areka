//! 要件 10.1・10.3: 決定性と再合成予算の fixture テーマ。

use super::test_support::{
    build_atlas_for_surface1000, compose_surface1000, opaque_pixel_count, parse_emo2_shell,
    shell_master_dir, surface1000_bind_parts,
};
use super::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState, SetId};

// ============================================================================
// 決定性（要件 **10.1**）と再合成予算（要件 **10.3**）の fixture 検証テスト。
//
// composer_tests.rs は toy surface で決定性・buffer 再利用を固定する。ここは **emo2 fixture の本体
// surface（surface1000＋実 bind 群）**を入力に、design「Testing Strategy / Performance/Load」の項目を
// fixture レベルで固定する:
//   1. **再合成予算（10.3）**: 同一 surface1000＋同一 BindSet を 1 つの Composer・1 つの out で反復
//      compose_into し、out バッファの先頭ポインタ・長さと命令列・作業場の列の容量が初回から変わらない
//      ことを **容量不変 assert** で固定する（同じ Composer の繰り返しが同じバイトを出すことも併せて）。
//   2. **命令数 O(elements)（10.3）**: surface1000＋**全有効 bind 集合**で build_plan を直接
//      呼び、`ops.len()` が **描画層数（＝解決した有効 bind 数）**に等しく、surface 数・画素数に
//      比例しないこと（線形＝非二次）を assert する。
//   3. **決定性（10.1）**: 独立した 2 つの合成器の compose がバイト等価（bytes＋width＋height＋stride）で
//      あることを固定する。surface1000（複数 bind の入れ子 flatten を含む最も内容の濃い経路）を主対象とする。
//
// すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし・要件 11.4）。surface1000 の bind パーツ
// 土台（surface1000_bind_parts / build_atlas_for_surface1000 / on_params 等）を使う。
// ============================================================================

use crate::plan::{PlanScratch, build_plan};

/// surface1000＋実 bind パーツ 3 種（1100/1200/1302）が「全て解決し描画命令を生む」有効 bind 集合。
///
/// [`surface1000_bind_parts`] の各 anim_id は [`build_atlas_for_surface1000`] が単色画像を挿入した
/// パーツで、`AtlasEntry.placement` が `Some`（不透明コアあり）に解決される。ゆえに build_plan は
/// 各有効 bind につき 1 命令ずつ生む。集合サイズ＝描画層数＝命令数（O(elements) の分母）になる。
fn surface1000_full_bindset() -> BindSet {
    BindSet::from_ids(surface1000_bind_parts().iter().map(|p| p.anim_id))
}

/// 決定性（要件 10.1）: emo2 surface1000＋固定 BindSet を 2 回 compose → バイト完全等価。
///
/// 最も内容の濃い経路（静的 element ゼロ・複数有効 bind の入れ子 surface flatten＋animation-sort）
/// で、独立した 2 つの `Composer` による 2 回の合成が width/height/stride/bytes すべてで一致する
/// ことを固定する。非空 bind 集合ゆえ結果は非空（全透明でない）で、決定性 assert は空虚でない。
#[test]
fn surface1000_compose_is_byte_deterministic() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    // fold+bind 済み world を独立に 2 つ構築（構築経路も含めて決定的であることを押さえる）。
    let first = compose_surface1000(&shell, &atlas, &binds);
    let second = compose_surface1000(&shell, &atlas, &binds);

    assert_eq!(first.width(), second.width(), "決定性: 幅一致");
    assert_eq!(first.height(), second.height(), "決定性: 高さ一致");
    assert_eq!(first.stride(), second.stride(), "決定性: stride 一致");
    assert_eq!(
        first.bytes(),
        second.bytes(),
        "同一入力（surface1000＋固定 BindSet）→ 2 回合成はバイト等価（要件 10.1）"
    );

    // 非空虚性: 非空 bind 集合ゆえ結果は非空（α>0 の画素が存在する）。全透明同士の空虚一致でない。
    assert!(
        opaque_pixel_count(&first) > 0,
        "非空 bind 集合の合成結果は非空（決定性 assert は空虚でない）"
    );
}

/// 再合成予算（要件 10.3）: emo2 surface1000 の反復 compose_into。
///
/// **容量不変 assert**（design 認可）を採る。#[global_allocator] を差し替えるやり方はプロセス全体を
/// 汚染し既存テストを不安定化しうるため採らない。初回 compose_into でバッファ／スクラッチが確定した後、
/// 後続呼び出しで
///   ① out.bytes() の**先頭ポインタが不変**（realloc が起きれば移動する＝再割り当てなしの直接証拠）、
///   ② out.bytes() の**長さが不変**（外形一定ゆえ resize が伸長を起こさない。バッファは
///      `bytes()` がスライスを返すため容量は観測できないが、①のポインタ不変が realloc 不在を担保する）、
///   ③ Composer 内部 `ops` スクラッチの**容量が単調・非成長**（毎フレーム clear+push で再利用）、
///   ④ Composer の作業場（`PlanScratch`）の全部の列と集合の**容量が非成長**（入れ子 flatten の
///      祖先スタック・段の積み荷・層 (ii) の id などの再利用。この検体で使う 6 つは 0 でないことも見る）、
///   ⑤ 絵のバイトと命令数が初回と同じ（同じ Composer の繰り返しでも決定的・要件 10.1）、
/// を反復ループ全域で固定する。外形は束縛のときの表から引くので、合成の中で外形をたどらない（指摘 8）。
/// realloc する実装ならポインタ／容量が動いて破れる。入力が変わるときの確保の有無は確かめていない。
#[test]
fn surface1000_recompose_steady_state_zero_allocation() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let mut composer = Composer::new();
    let mut out = ComposedSurface::new(0, 0);

    // ウォームアップ（初回）: バッファ／スクラッチのサイズがここで確定する。
    composer
        .compose_into(
            &mut out,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("初回 compose_into は Ok");

    let ptr0 = out.bytes().as_ptr();
    let len0 = out.bytes().len();
    let bytes0 = out.bytes().to_vec();
    let ops_len0 = composer.ops.len();
    let ops_cap0 = composer.ops.capacity();
    let caps0 = composer.scratch.capacities();
    for (name, cap) in caps0 {
        if matches!(
            name,
            "visited" | "on_path" | "steps" | "frames" | "merged_ids" | "anims"
        ) {
            assert!(cap > 0, "作業場の {name} を使っている（空虚でない）");
        }
    }

    // 定常状態: 同一 surface＋同一 BindSet を反復。初回で確定した容量から一切成長しないこと。
    for iter in 0..8 {
        composer
            .compose_into(
                &mut out,
                &world,
                &atlas,
                1000,
                &binds,
                &PatternState::default(),
            )
            .unwrap_or_else(|e| panic!("反復 {iter} 回目の compose_into は Ok: {e:?}"));

        assert_eq!(
            out.bytes().as_ptr(),
            ptr0,
            "反復 {iter}: out バッファ先頭ポインタ不変＝realloc なし（要件 10.3）"
        );
        assert_eq!(
            out.bytes().len(),
            len0,
            "反復 {iter}: バッファ長不変（外形一定）"
        );
        assert!(
            out.bytes() == bytes0,
            "反復 {iter}: 絵は初回とバイト等価（要件 10.1）"
        );
        assert_eq!(composer.ops.len(), ops_len0, "反復 {iter}: 命令数は決定的");
        assert_eq!(
            composer.ops.capacity(),
            ops_cap0,
            "反復 {iter}: ops スクラッチ容量が非成長（clear+push 再利用・要件 10.3）"
        );
        assert_eq!(
            composer.scratch.capacities(),
            caps0,
            "反復 {iter}: 作業場の全部の列と集合の容量が非成長（再利用・要件 10.3）"
        );
    }

    // 結果は依然非空（定常状態でも実合成が起きている＝空実装で容量安定を偽装していない）。
    assert!(
        opaque_pixel_count(&out) > 0,
        "反復後も非空（定常状態で現に合成している）"
    );
}

/// 外形の表（指摘 8）: 束縛の前は表が無く外形 0×0＝`EmptyComposition`（今までの前提どおり）。束縛すると
/// 表から引いた外形で合成でき、その値は直にたどった外形と同じ。
#[test]
fn extent_comes_from_the_table_made_by_bind_atlas() {
    use crate::error::ComposeError;
    use crate::plan::compute_extent;

    let shell = parse_emo2_shell();
    let atlas = build_atlas_for_surface1000(&shell, &shell_master_dir());
    let binds = surface1000_full_bindset();
    let mut world = EmoWorld::build(&shell);
    let (mut ops, mut scratch) = (Vec::new(), PlanScratch::default());
    let plan = |world: &EmoWorld, ops: &mut Vec<_>, scratch: &mut PlanScratch| {
        build_plan(
            ops,
            scratch,
            world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
    };
    assert_eq!(
        plan(&world, &mut ops, &mut scratch),
        Err(ComposeError::EmptyComposition(1000)),
        "束縛の前は外形 0×0"
    );
    world.bind_atlas(&atlas, SetId(0));
    let extent = plan(&world, &mut ops, &mut scratch).expect("束縛の後は合成できる");
    assert_eq!(extent, compute_extent(&world, &atlas, 1000));
    assert!(extent.w > 0 && extent.h > 0);
}

/// 命令数 O(elements)（要件 10.3）: surface1000＋全有効 bind → 命令数 == 描画層数（線形）。
///
/// surface1000 は静的 element ゼロ・全パーツ bind の本体 surface。有効 bind 集合を
/// [`surface1000_full_bindset`]（解決する 3 パーツ 1100/1200/1302）にすると、build_plan は各有効
/// bind の pattern0 入れ子参照を 1 層ずつ flatten し、**有効 bind ごとに厳密に 1 命令**を生む。
/// よって `ops.len() == 有効 bind 数`（＝描画層数）であり、命令数は **surface 数・画素数でなく
/// 描画層数に線形（O(elements)）**である。
///
/// ## なぜこの assert が O(elements) の証明になるか
/// - 上界を有効 bind 数（3）に固定する。二次（層×層）や画素比例（外形 w*h ≈ 数万）で命令を生む
///   実装なら `ops.len()` はこの上界を大きく超え、この等値 assert が破れる。
/// - 各 bind パーツ surface（1100/1200/1302）は element 1 本の単純 surface で、入れ子 flatten は
///   1 段。ゆえに命令数 = Σ(有効 bind の可視層数) = 有効 bind 数。線形係数 1。
#[test]
fn surface1000_instruction_count_is_linear_in_layers() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let mut ops = Vec::new();
    let mut visited = PlanScratch::default();
    let extent = build_plan(
        &mut ops,
        &mut visited,
        &world,
        &atlas,
        1000,
        &binds,
        &PatternState::default(),
    )
    .expect("build_plan は Ok（surface1000 存在＋外形非ゼロ）");

    // 命令数 == 描画層数（有効 bind 数）。線形＝非二次・非画素比例（O(elements)・要件 10.3）。
    assert_eq!(
        ops.len(),
        3,
        "解決した 3 有効 bind（1100/1200/1302）→ 3 命令"
    );

    // 命令数が「画素数」に比例しないことの直接反証: 外形は数千〜数万画素だが命令は 3 本のみ。
    let pixels = extent.w as usize * extent.h as usize;
    assert!(
        pixels > ops.len() * 100,
        "外形画素数（{pixels}）≫ 命令数（{}）＝命令は画素比例でない（O(elements)）",
        ops.len()
    );

    // 各命令の method は overlay（M1 実装対象）で、命令は実描画層（センチネル skip 済み）。
    for op in &ops {
        assert_eq!(
            op.method,
            crate::method::ComposeMethod::Overlay,
            "全命令は overlay 層"
        );
    }

    // 有効 bind を 1 本に絞ると命令も 1 本（層数に厳密比例＝線形係数 1 の追加確証）。
    let one = BindSet::from_ids([surface1000_bind_parts()[0].anim_id]);
    let mut ops1 = Vec::new();
    let mut visited1 = PlanScratch::default();
    build_plan(
        &mut ops1,
        &mut visited1,
        &world,
        &atlas,
        1000,
        &one,
        &PatternState::default(),
    )
    .expect("単一 bind でも Ok");
    assert_eq!(ops1.len(), 1, "有効 bind 1 本 → 命令 1 本（層数に線形）");

    // 空 bind 集合 → 命令ゼロ（静的 element ゼロゆえ描画層皆無・外形は非ゼロで Ok）。
    let mut ops0 = Vec::new();
    let mut visited0 = PlanScratch::default();
    build_plan(
        &mut ops0,
        &mut visited0,
        &world,
        &atlas,
        1000,
        &BindSet::default(),
        &PatternState::default(),
    )
    .expect("空 bind でも surface 存在＋外形非ゼロで Ok");
    assert_eq!(ops0.len(), 0, "空 bind 集合 → 描画命令ゼロ（層数 0）");
}
