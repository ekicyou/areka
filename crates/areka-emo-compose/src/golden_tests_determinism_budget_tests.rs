//! task 8.4（要件 10.1・10.3）: 決定性と再合成予算の fixture テーマ。

use super::{BindSet, ComposedSurface, Composer, EmoWorld, PatternState, SetId};
use super::test_support::{
    build_atlas_for_surface1000, compose_surface1000, opaque_pixel_count, parse_emo2_shell,
    shell_master_dir, surface1000_bind_parts,
};

// ============================================================================
// task 8.4: 決定性（要件 **10.1**）と再合成予算（要件 **10.3**）の fixture 検証テスト。
//
// composer_tests.rs（task 7）は toy surface で決定性・buffer 再利用を固定したが、本 task は
// **emo2 fixture の本体 surface（surface1000＋実 bind 群）**を入力に、design「Testing Strategy /
// Performance/Load」の 2 項目を fixture レベルで固定する:
//   1. **再合成予算（10.3・定常状態ゼロアロケーション）**: 同一 surface1000＋同一 BindSet を
//      1 つの Composer・1 つの out で反復 compose_into し、初回以降アロケーションが発生しない
//      ことを **容量不変 assert**（design 認可の「スクラッチ/バッファ再利用のカウンタ検証または
//      容量不変 assert」）で固定する。out バッファの先頭ポインタ・長さ・容量が安定（再割り当て
//      なし）であること、および build_plan の ops スクラッチ容量が定常状態で成長しないことを
//      両輪で押さえる。
//   2. **命令数 O(elements)（10.3）**: surface1000＋**全有効 bind 集合**で build_plan を直接
//      呼び、`ops.len()` が **描画層数（＝解決した有効 bind 数）**に等しく、surface 数・画素数に
//      比例しないこと（線形＝非二次）を assert する。
//   3. **決定性（10.1）**: 同一入力の 2 回 compose がバイト等価（bytes＋width＋height＋stride）で
//      あること、compose_into を 2 つの新規バッファへ行った結果もバイト等価であることを固定する。
//      surface1000（複数 bind の入れ子 flatten を含む最も内容の濃い経路）を主対象とする。
//
// すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし・要件 11.4）。task 8.2 の
// surface1000 bind パーツ土台（surface1000_bind_parts / build_atlas_for_surface1000 / on_params 等）を
// 再利用する。
// ============================================================================

use crate::plan::build_plan;

/// surface1000＋実 bind パーツ 3 種（1100/1200/1302）が「全て解決し描画命令を生む」有効 bind 集合。
///
/// [`surface1000_bind_parts`] の各 anim_id は [`build_atlas_for_surface1000`] が単色画像を挿入した
/// パーツで、`AtlasEntry.placement` が `Some`（不透明コアあり）に解決される。ゆえに build_plan は
/// 各有効 bind につき 1 命令ずつ生む。集合サイズ＝描画層数＝命令数（O(elements) の分母）になる。
fn surface1000_full_bindset() -> BindSet {
    BindSet::from_ids(surface1000_bind_parts().iter().map(|p| p.anim_id))
}

/// 2 つの `ComposedSurface` が外形（width/height/stride）＋全バイトで等価か。
fn composed_byte_equal(a: &ComposedSurface, b: &ComposedSurface) -> bool {
    a.width() == b.width()
        && a.height() == b.height()
        && a.stride() == b.stride()
        && a.bytes() == b.bytes()
}

/// task 8.4 決定性（要件 10.1）: emo2 surface1000＋固定 BindSet を 2 回 compose → バイト完全等価。
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
    assert!(composed_byte_equal(&first, &second), "外形＋全バイトで完全等価");

    // 非空虚性: 非空 bind 集合ゆえ結果は非空（α>0 の画素が存在する）。全透明同士の空虚一致でない。
    assert!(
        opaque_pixel_count(&first) > 0,
        "非空 bind 集合の合成結果は非空（決定性 assert は空虚でない）"
    );
}

/// task 8.4 決定性（要件 10.1）: `compose_into` を 2 つの**新規バッファ**へ行った結果がバイト等価。
///
/// `compose_into` は呼び手のバッファへ書く経路。空の 2 バッファへ同一入力を書き、両者が
/// width/height/stride/bytes で一致することを固定する（buffer 経路でも決定的）。
#[test]
fn surface1000_compose_into_two_fresh_buffers_are_byte_equal() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let mut composer = Composer::new();

    let mut out_a = ComposedSurface::new(0, 0);
    composer
        .compose_into(&mut out_a, &world, &atlas, 1000, &binds, &PatternState::default())
        .expect("compose_into A は Ok");

    let mut out_b = ComposedSurface::new(0, 0);
    composer
        .compose_into(&mut out_b, &world, &atlas, 1000, &binds, &PatternState::default())
        .expect("compose_into B は Ok");

    assert!(
        composed_byte_equal(&out_a, &out_b),
        "compose_into を 2 つの新規バッファへ → バイト等価（要件 10.1）"
    );
    assert!(opaque_pixel_count(&out_a) > 0, "非空（空虚でない）");
}

/// task 8.4 再合成予算（要件 10.3・定常状態ゼロアロケーション）: emo2 surface1000 の反復 compose_into。
///
/// **アプローチ (A)（容量不変 assert・design 認可）**を採る。#[global_allocator] を差し替える
/// アプローチ (B) はプロセス全体を汚染し既存テストを不安定化しうるため採らない。定常状態の
/// アロケーション不在は、初回 compose_into でバッファ／スクラッチが確定した後、後続呼び出しで
///   ① out.bytes() の**先頭ポインタが不変**（realloc が起きれば移動する＝再割り当てなしの直接証拠）、
///   ② out.bytes() の**長さが不変**（外形一定ゆえ resize が伸長を起こさない。バッファは
///      `bytes()` がスライスを返すため容量は観測できないが、①のポインタ不変が realloc 不在を担保する）、
///   ③ Composer 内部 `ops` スクラッチの**容量が単調・非成長**（毎フレーム clear+push で再利用）、
///   ④ Composer 内部 `visited` スクラッチの**容量が非成長**（入れ子 flatten の祖先スタック再利用）、
/// を反復ループ全域で固定することで示す。realloc する実装ならポインタ／容量が動いて破れる。
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
        .compose_into(&mut out, &world, &atlas, 1000, &binds, &PatternState::default())
        .expect("初回 compose_into は Ok");

    let ptr0 = out.bytes().as_ptr();
    let len0 = out.bytes().len();
    let ops_cap0 = composer.ops.capacity();
    let visited_cap0 = composer.visited.capacity();

    // 定常状態: 同一 surface＋同一 BindSet を反復。初回で確定した容量から一切成長しないこと。
    for iter in 0..8 {
        composer
            .compose_into(&mut out, &world, &atlas, 1000, &binds, &PatternState::default())
            .unwrap_or_else(|e| panic!("反復 {iter} 回目の compose_into は Ok: {e:?}"));

        assert_eq!(
            out.bytes().as_ptr(),
            ptr0,
            "反復 {iter}: out バッファ先頭ポインタ不変＝realloc なし（要件 10.3）"
        );
        assert_eq!(out.bytes().len(), len0, "反復 {iter}: バッファ長不変（外形一定）");
        assert_eq!(
            composer.ops.capacity(),
            ops_cap0,
            "反復 {iter}: ops スクラッチ容量が非成長（clear+push 再利用・要件 10.3）"
        );
        assert_eq!(
            composer.visited.capacity(),
            visited_cap0,
            "反復 {iter}: visited スクラッチ容量が非成長（祖先スタック再利用・要件 10.3）"
        );
    }

    // 結果は依然非空（定常状態でも実合成が起きている＝空実装で容量安定を偽装していない）。
    assert!(
        opaque_pixel_count(&out) > 0,
        "反復後も非空（定常状態で現に合成している）"
    );
}

/// build_plan を直接反復し、ops/visited スクラッチが定常状態で成長しないことを固定する（要件 10.3）。
///
/// `compose_into` 経由（上テスト）に加え、design が示唆する「reused `ops`/`visited` で build_plan を
/// 直接呼び ops.capacity() が成長しない」経路を明示的に押さえる。初回で確定した容量から、後続
/// 反復で ops.capacity()・visited.capacity() が一切増えないことを assert する。
#[test]
fn surface1000_build_plan_scratch_capacity_is_stable() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);
    let binds = surface1000_full_bindset();

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let mut ops = Vec::new();
    let mut visited = Vec::new();

    // 初回: 容量確定。
    let extent0 = build_plan(&mut ops, &mut visited, &world, &atlas, 1000, &binds, &PatternState::default())
        .expect("初回 build_plan は Ok（surface1000 存在＋外形非ゼロ）");
    let ops_cap0 = ops.capacity();
    let visited_cap0 = visited.capacity();
    let ops_len0 = ops.len();

    for iter in 0..8 {
        let extent = build_plan(&mut ops, &mut visited, &world, &atlas, 1000, &binds, &PatternState::default())
            .unwrap_or_else(|e| panic!("反復 {iter} 回目の build_plan は Ok: {e:?}"));

        // 外形・命令数は決定的（毎回同一）。
        assert_eq!(extent, extent0, "反復 {iter}: 外形は決定的（要件 10.1）");
        assert_eq!(ops.len(), ops_len0, "反復 {iter}: 命令数は決定的");
        // スクラッチ容量は初回から非成長（定常状態ゼロアロケーション・要件 10.3）。
        assert_eq!(
            ops.capacity(),
            ops_cap0,
            "反復 {iter}: ops スクラッチ容量が非成長（再利用・要件 10.3）"
        );
        assert_eq!(
            visited.capacity(),
            visited_cap0,
            "反復 {iter}: visited スクラッチ容量が非成長（再利用・要件 10.3）"
        );
    }
}

/// task 8.4 命令数 O(elements)（要件 10.3）: surface1000＋全有効 bind → 命令数 == 描画層数（線形）。
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
    let active_bind_count = binds.ids().len(); // 解決する有効 bind 数（＝描画層数）。

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    let mut ops = Vec::new();
    let mut visited = Vec::new();
    let extent = build_plan(&mut ops, &mut visited, &world, &atlas, 1000, &binds, &PatternState::default())
        .expect("build_plan は Ok（surface1000 存在＋外形非ゼロ）");

    // 命令数 == 描画層数（有効 bind 数）。線形＝非二次・非画素比例。
    assert_eq!(
        ops.len(),
        active_bind_count,
        "surface1000＋全有効 bind の命令数 == 描画層数（O(elements)・要件 10.3）"
    );
    // 具体値の固定（回帰検出）: 3 有効 bind → 3 命令。数十命令規模（design 記述）の下端。
    assert_eq!(ops.len(), 3, "解決した 3 有効 bind（1100/1200/1302）→ 3 命令");

    // 命令数が「画素数」に比例しないことの直接反証: 外形は数千〜数万画素だが命令は 3 本のみ。
    let pixels = extent.w as usize * extent.h as usize;
    assert!(
        pixels > ops.len() * 100,
        "外形画素数（{pixels}）≫ 命令数（{}）＝命令は画素比例でない（O(elements)）",
        ops.len()
    );

    // 各命令の method は overlay（M1 実装対象）で、命令は実描画層（センチネル skip 済み）。
    for op in &ops {
        assert_eq!(op.method, crate::method::ComposeMethod::Overlay, "全命令は overlay 層");
    }

    // 有効 bind を 1 本に絞ると命令も 1 本（層数に厳密比例＝線形係数 1 の追加確証）。
    let one = BindSet::from_ids([surface1000_bind_parts()[0].anim_id]);
    let mut ops1 = Vec::new();
    let mut visited1 = Vec::new();
    build_plan(&mut ops1, &mut visited1, &world, &atlas, 1000, &one, &PatternState::default())
        .expect("単一 bind でも Ok");
    assert_eq!(ops1.len(), 1, "有効 bind 1 本 → 命令 1 本（層数に線形）");

    // 空 bind 集合 → 命令ゼロ（静的 element ゼロゆえ描画層皆無・外形は非ゼロで Ok）。
    let mut ops0 = Vec::new();
    let mut visited0 = Vec::new();
    build_plan(&mut ops0, &mut visited0, &world, &atlas, 1000, &BindSet::default(), &PatternState::default())
        .expect("空 bind でも surface 存在＋外形非ゼロで Ok");
    assert_eq!(ops0.len(), 0, "空 bind 集合 → 描画命令ゼロ（層数 0）");
}
