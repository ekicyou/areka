//! `Composer` ファサード結線の in-source テスト（要件 5.1/5.5/6.6/9.1/9.3/9.4/10.3/10.4/10.5）。
//!
//! fold（[`EmoWorld::build`]）→ bind（[`EmoWorld::bind_atlas`]）→ plan（[`crate::plan::build_plan`]）
//! → execute（[`crate::blit::execute`]）の全結線を、`MemoryDecoder`+`bake` の COM/WIC 非依存経路で
//! headless に検証する。Composer が合成結果を保持しないこと（要件 9.4）・同じ入力の繰り返しで `out` の
//! 先頭と命令列の容量が変わらないこと（要件 10.3）・失敗経路が `Err` で伝播すること（要件 10.5）・
//! compose と compose_into が等価かつ決定的であること（要件 10.1）を固定する。

use super::*;
use areka_emo_atlas::{AtlasTable, SetId};
use areka_parsers::shell::{Shell, SortOrder};
use std::path::Path;

use crate::plan_test_support::{
    bake_atlas_sized, bind_anim, elem, px, shell_of, shell_of_with_sort, surface,
    surface_with_anims,
};

/// fold → bind 済み world と atlas を返す共通土台。
fn build_bound(shell: Shell, atlas: &AtlasTable) -> EmoWorld {
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(atlas, SetId(0));
    world
}

// ── テスト1（受入基準・要件 5.1/9.1）: 有効入力で Ok(ComposedSurface)・premultiplied BGRA・外形一致 ──

/// 静的 element 1 本＋有効 bind 1 本の surface を compose → `Ok(ComposedSurface)`。
///
/// premultiplied BGRA・width/height が外形一致（stride == w*4）・実合成が起きた画素を確認する。
/// 40×30 の base（不透明青）を (0,0)、200×10 の bind part（不透明赤）を (0,0) に重ねる。外形は
/// 全 element＋全部の bind の描くコマの和集合＝w=max(40,200)=200・h=max(30,10)=30。
#[test]
fn compose_returns_ok_composed_surface_for_valid_input() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (40, 30), Some((200, 0, 0))),  // 不透明青。
            ("part.png", (200, 10), Some((0, 0, 200))), // 不透明赤。
        ],
    );

    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0)],
    );
    let part = surface(1100, vec![elem(0, "part.png", 0, 0)]);
    let world = build_bound(shell_of(vec![host, part]), &atlas);

    let binds = BindSet::from_ids([1]);
    let mut composer = Composer::new();
    let out = composer
        .compose(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("有効入力は Ok（要件 5.1）");

    // 外形一致（要件 9.1）: w=200・h=30・stride == w*4。
    assert_eq!(out.width(), 200);
    assert_eq!(out.height(), 30);
    assert_eq!(out.stride(), out.width() * 4);
    assert_eq!(out.bytes().len() as u32, out.stride() * out.height());

    // 実合成の確認: bind part（赤）は (0,0) の 200×10 を覆う → (0,0) は赤（bind が base の上）。
    assert_eq!(
        px(&out, 0, 0),
        [0, 0, 200, 255],
        "上層 bind part（赤）が (0,0) を覆う"
    );
    // base（青）だけの領域 (0,20)（part の 10px 高さの外・base の 30px 高さ内）→ 青。
    assert_eq!(
        px(&out, 0, 20),
        [200, 0, 0, 255],
        "part 非被覆域は base（青）"
    );
}

// ── テスト2（要件 9.4）: Composer は合成結果を保持しない（構造的＋挙動的） ──

/// Composer の内部フィールドが走査スクラッチのみ（ComposedSurface を持たない）ことを、異なる
/// surface を連続 compose して各々が自 surface を反映する（stale キャッシュがない）ことで示す。
///
/// surface A(青 10×10)・B(緑 20×20) を交互に compose し、B の結果が A でなく B を反映する。
#[test]
fn composer_holds_no_result_no_stale_cache() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("a.png", (10, 10), Some((200, 0, 0))), // 青。
            ("b.png", (20, 20), Some((0, 200, 0))), // 緑。
        ],
    );
    let sa = surface(100, vec![elem(0, "a.png", 0, 0)]);
    let sb = surface(200, vec![elem(0, "b.png", 0, 0)]);
    let world = build_bound(shell_of(vec![sa, sb]), &atlas);

    let binds = BindSet::default();
    let mut composer = Composer::new();

    let out_a = composer
        .compose(&world, &atlas, 100, &binds, &PatternState::default())
        .expect("A Ok");
    assert_eq!(out_a.width(), 10);
    assert_eq!(px(&out_a, 0, 0), [200, 0, 0, 255], "A は青");

    // 直後に B を compose → B（緑 20×20）を反映（A の残像・寸法を持ち越さない）。
    let out_b = composer
        .compose(&world, &atlas, 200, &binds, &PatternState::default())
        .expect("B Ok");
    assert_eq!(
        out_b.width(),
        20,
        "B の外形（20）を反映＝A の 10 を持ち越さない"
    );
    assert_eq!(
        px(&out_b, 0, 0),
        [0, 200, 0, 255],
        "B は緑（stale な A の青でない）"
    );

    // 再度 A → 依然 A（Composer は前回 B を保持しない）。
    let out_a2 = composer
        .compose(&world, &atlas, 100, &binds, &PatternState::default())
        .expect("A2 Ok");
    assert_eq!(
        out_a2.bytes(),
        out_a.bytes(),
        "A の再合成は初回 A とバイト等価（キャッシュ非依存）"
    );
}

// ── テスト3（要件 10.3・10.1）: compose_into は buffer を再利用し、繰り返しで再割り当てしない ──

/// 同一 out・同一 surface で compose_into を 2 回 → バッファ長不変・先頭ポインタ不変（再割り当て
/// なし）・結果バイト等価（決定性）。Composer 内部 ops スクラッチ容量も安定（定常状態ゼロアロケーション）。
#[test]
fn compose_into_reuses_buffer_across_calls() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(base, &[("p.png", (30, 20), Some((100, 110, 120)))]);
    let surf = surface(5000, vec![elem(0, "p.png", 0, 0)]);
    let world = build_bound(shell_of(vec![surf]), &atlas);

    let binds = BindSet::default();
    let mut composer = Composer::new();
    let mut out = ComposedSurface::new(0, 0);

    composer
        .compose_into(
            &mut out,
            &world,
            &atlas,
            5000,
            &binds,
            &PatternState::default(),
        )
        .expect("1 回目 Ok");
    let len1 = out.bytes().len();
    let ptr1 = out.bytes().as_ptr();
    let bytes1 = out.bytes().to_vec();
    let ops_cap1 = composer.ops.capacity();

    composer
        .compose_into(
            &mut out,
            &world,
            &atlas,
            5000,
            &binds,
            &PatternState::default(),
        )
        .expect("2 回目 Ok");
    let len2 = out.bytes().len();
    let ptr2 = out.bytes().as_ptr();
    let ops_cap2 = composer.ops.capacity();

    assert_eq!(len1, len2, "同一外形でバッファ長不変（要件 10.3）");
    assert_eq!(
        ptr1, ptr2,
        "容量再利用ゆえ先頭ポインタ不変＝再割り当てなし（要件 10.3）"
    );
    assert_eq!(
        bytes1,
        out.bytes(),
        "同一入力→バイト等価（決定性・要件 10.1）"
    );
    assert_eq!(
        ops_cap1, ops_cap2,
        "Composer 内部 ops スクラッチ容量も安定（定常状態ゼロアロケーション・要件 10.3）"
    );
}

// ── テスト3b（recompose-budget 要件 3.3／6.4）: compose_into は使い回した out を必ず塗り直す ──

/// 事前に 0 以外で塗った `out` へ `compose_into` した結果が、まっさらな `out` への結果と
/// **バイト等価**であること（合成先に前コマの残像を残さない）。
///
/// # なぜこの 1 本が要るか
///
/// [`crate::blit::execute`] が呼ぶ `resize_and_clear` はゼロ埋めを**除去してはならない**——
/// 合成は画布へ重ねる意味論で、どの命令にも覆われない領域は「書かれずに 0 のまま」であることに
/// 依存している。ところが実測では、`blit` 側へこの除去を誤って適用してもこのクレートの
/// 既存テストは**全て緑のまま**通り、別クレート（emo-present）の等価檻でしか赤にならなかった。
/// 除去してはならない側の防壁を、その `resize_and_clear` を所有するクレート自身に置く。
///
/// # fixture の前提（偽の緑を作らないための自己検査つき）
///
/// 外形 200×30 に対し base（40×30）と bind part（200×10）を置くので、右下の
/// `x∈[40,200) × y∈[10,30)` は**どの命令にも覆われない**＝正解が BGRA 全 0 になる。
/// 「正解の出力に 0 バイトが実在し、かつ全部が 0 ではない」ことを檻の中で確認してから
/// 残留の主張を立てる。
#[test]
fn compose_into_a_prepainted_buffer_is_byte_equal_to_a_fresh_one() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (40, 30), Some((200, 0, 0))),  // 不透明青。
            ("part.png", (200, 10), Some((0, 0, 200))), // 不透明赤。
        ],
    );
    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0)],
    );
    let part = surface(1100, vec![elem(0, "part.png", 0, 0)]);
    let world = build_bound(shell_of(vec![host, part]), &atlas);

    let binds = BindSet::from_ids([1]);
    let mut composer = Composer::new();

    // 対照（まっさらな出力先）。
    let mut fresh = ComposedSurface::new(0, 0);
    composer
        .compose_into(
            &mut fresh,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("まっさらな出力先への compose_into は Ok");
    let (w, h) = (fresh.width(), fresh.height());
    let want = fresh.bytes().to_vec();

    // fixture の前提: 正解の出力に 0 バイトが実在し、かつ全部が 0 ではない。
    assert!(
        want.contains(&0),
        "fixture 前提が崩れた（命令に覆われない領域が無いと残留を弁別できない）"
    );
    assert!(
        want.iter().any(|&b| b != 0),
        "fixture 前提が崩れた（出力が全ゼロでは何も主張していない）"
    );

    // 使い回した出力先（同じ外形・0 以外で事前に塗る）へ書いても 1 バイトも違わない。
    let mut reused = ComposedSurface::new(w, h);
    reused.bytes_mut().fill(0xAA);
    composer
        .compose_into(
            &mut reused,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("使い回した出力先への compose_into は Ok");
    assert_eq!(
        (reused.width(), reused.height()),
        (w, h),
        "使い回しで外形が事後条件どおりでない"
    );
    assert_eq!(
        reused.bytes(),
        want.as_slice(),
        "命令に覆われない領域へ 0xAA の残留が残った（合成先のクリアが失われている）"
    );
}

// ── テスト4（要件 10.5・6.6）: 失敗経路の伝播と正常空合成 ──

/// 不在 surface → `Err(SurfaceNotFound)`（error ログ済み・要件 10.5・非パニック）。
#[test]
fn absent_surface_propagates_surface_not_found() {
    let world = EmoWorld::build(&shell_of(Vec::new()));
    let atlas = bake_atlas_sized(
        Path::new("shell/master"),
        &[("dummy.png", (2, 2), Some((1, 2, 3)))],
    );

    let binds = BindSet::default();
    let mut composer = Composer::new();
    let result = composer.compose(&world, &atlas, 9999, &binds, &PatternState::default());
    assert_eq!(
        result.err(),
        Some(ComposeError::SurfaceNotFound(9999)),
        "不在 surface は SurfaceNotFound（要件 10.5）"
    );
}

/// 定義層皆無で外形 0×0 の退化 → `Err(EmptyComposition)`（要件 10.5）。
#[test]
fn no_layers_degenerate_propagates_empty_composition() {
    let surf = surface(7000, Vec::new()); // element ゼロ・animation ゼロ。
    let atlas = bake_atlas_sized(
        Path::new("shell/master"),
        &[("dummy.png", (2, 2), Some((1, 2, 3)))],
    );
    let world = build_bound(shell_of(vec![surf]), &atlas);

    let binds = BindSet::default();
    let mut composer = Composer::new();
    let result = composer.compose(&world, &atlas, 7000, &binds, &PatternState::default());
    assert_eq!(
        result.err(),
        Some(ComposeError::EmptyComposition(7000)),
        "定義層皆無で 0×0 は EmptyComposition（要件 10.5）"
    );
}

/// 全 element 全透明 → `Ok`・全透明バッファ・非ゼロ外形（要件 6.6・議題2裁定）。
#[test]
fn all_transparent_surface_is_ok_transparent_nonzero_extent() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("ghost1.png", (300, 300), None), // 全透明→ placement None。
            ("ghost2.png", (20, 20), None),
        ],
    );
    let surf = surface(
        3000,
        vec![elem(0, "ghost1.png", 0, 0), elem(1, "ghost2.png", 0, 0)],
    );
    let world = build_bound(shell_of(vec![surf]), &atlas);

    let binds = BindSet::default();
    let mut composer = Composer::new();
    let out = composer
        .compose(&world, &atlas, 3000, &binds, &PatternState::default())
        .expect("全透明でもエラーにせず Ok（要件 6.6）");

    // 外形は原寸で非ゼロ（全透明でも original で寄与＝300×300 が支配）。
    assert_eq!(out.width(), 300);
    assert_eq!(out.height(), 300);
    // 描画可能命令ゼロゆえ全画素透明（残像なし）。
    assert!(
        out.bytes().iter().all(|&b| b == 0),
        "全透明 surface → 全透明バッファ（要件 6.6）"
    );
}

// ── テスト5（等価性）: compose == compose_into into a fresh buffer ──

/// `compose` の結果は、同一入力を新規バッファへ `compose_into` した結果とバイト等価。
#[test]
fn compose_equals_compose_into_fresh_buffer() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (40, 30), Some((10, 20, 30))),
            ("part.png", (25, 25), Some((90, 80, 70))),
        ],
    );
    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 5, 3)],
    );
    let part = surface(1100, vec![elem(0, "part.png", 0, 0)]);
    let world = build_bound(shell_of(vec![host, part]), &atlas);

    let binds = BindSet::from_ids([1]);
    let mut composer = Composer::new();

    let via_compose = composer
        .compose(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("compose Ok");

    let mut fresh = ComposedSurface::new(0, 0);
    composer
        .compose_into(
            &mut fresh,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("compose_into Ok");

    assert_eq!(via_compose.width(), fresh.width());
    assert_eq!(via_compose.height(), fresh.height());
    assert_eq!(
        via_compose.bytes(),
        fresh.bytes(),
        "compose と compose_into は等価"
    );
}

// ── テスト6（要件 10.1）: 決定性 — 同一入力の 2 回合成はバイト等価 ──

#[test]
fn compose_is_deterministic() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (40, 30), Some((10, 20, 30))),
            ("p1.png", (60, 20), Some((1, 2, 3))),
            ("p2.png", (20, 60), Some((4, 5, 6))),
        ],
    );
    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 0, 0), bind_anim(2, 1200, 0, 0)],
    );
    let p1 = surface(1100, vec![elem(0, "p1.png", 0, 0)]);
    let p2 = surface(1200, vec![elem(0, "p2.png", 0, 0)]);
    let world = build_bound(shell_of(vec![host, p1, p2]), &atlas);

    let binds = BindSet::from_ids([1, 2]);
    let mut c1 = Composer::new();
    let mut c2 = Composer::new();
    let a = c1
        .compose(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("Ok");
    let b = c2
        .compose(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("Ok");
    assert_eq!(a.width(), b.width());
    assert_eq!(a.height(), b.height());
    assert_eq!(
        a.bytes(),
        b.bytes(),
        "同一入力→バイト等価（決定性・要件 10.1）"
    );
}

// ── テスト7（要件 4.3・hot path resolve-free）: multi-layer 実データの end-to-end 結線 ──

/// animation-sort=ascend の多層 + 複数 bind を実データ構造で end-to-end（fold→bind→plan→blit）合成し、
/// 描画順（ascend ゆえ ID 降順）が画素へ反映されることで結線を証明する。
///
/// hot path が `AtlasTable::resolve` を呼ばないことは、下の
/// [`hot_path_sources_do_not_call_atlas_resolve`] がソースを走査して確かめる。
#[test]
fn end_to_end_multilayer_ascend_wiring() {
    let base = Path::new("shell/master");
    // 同一 (0,0)・同一 4×4 サイズの 3 パーツを重ねる。ascend ゆえ ID 降順描画（小 ID が上）。
    let atlas = bake_atlas_sized(
        base,
        &[
            ("p1.png", (4, 4), Some((11, 0, 0))),
            ("p2.png", (4, 4), Some((0, 22, 0))),
            ("p3.png", (4, 4), Some((0, 0, 33))),
        ],
    );
    let host = surface_with_anims(
        1000,
        Vec::new(),
        vec![
            bind_anim(1, 1100, 0, 0),
            bind_anim(2, 1200, 0, 0),
            bind_anim(3, 1300, 0, 0),
        ],
    );
    let p1 = surface(1100, vec![elem(0, "p1.png", 0, 0)]);
    let p2 = surface(1200, vec![elem(0, "p2.png", 0, 0)]);
    let p3 = surface(1300, vec![elem(0, "p3.png", 0, 0)]);
    let mut world = EmoWorld::build(&shell_of_with_sort(
        vec![host, p1, p2, p3],
        Some(SortOrder::Ascend),
    ));
    world.bind_atlas(&atlas, SetId(0));

    let binds = BindSet::from_ids([1, 2, 3]);
    let mut composer = Composer::new();
    let out = composer
        .compose(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("Ok");

    assert_eq!(out.width(), 4);
    assert_eq!(out.height(), 4);
    // ascend → ID 降順描画: p3(3) 下 → p2(2) → p1(1) 上。全不透明ゆえ最上層 p1（B=11）が見える。
    assert_eq!(
        px(&out, 0, 0),
        [11, 0, 0, 255],
        "ascend＝ID降順描画で最上層 p1 が見える（結線実証）"
    );
}

/// hot path（compose/compose_into 経路）のソースに `atlas.resolve(` が現れないことを静的に確認する。
///
/// plan.rs／plan_extent.rs／plan_always.rs／blit.rs／blit_rect.rs／lib.rs（Composer）の非テストコードは
/// `atlas.entry`（O(1)）のみを引き、
/// `resolve` は事前の bind_atlas（atlas_bind.rs）に限られる（要件 4.3・design 主要決定）。
/// 本テストは各ソースの非テスト領域文字列を検査し、hot path に `resolve` 呼び出しが無いことを固定する。
#[test]
fn hot_path_sources_do_not_call_atlas_resolve() {
    // 毎フレームの道のファイルの、中に書いたテストの mod より前に `.resolve(` が無いこと（plan.rs と
    // plan_extent.rs は頭に `#[cfg(test)]` の口を持つので、そこで切ると頭の数十行しか見ない）。
    // 各ファイルの的の関数（切った領域に在ることを先に確かめ、空の領域で黙って通らないようにする）。
    for (src, file, target) in [
        (include_str!("plan.rs"), "plan.rs", "fn build_plan"),
        (
            include_str!("plan_extent.rs"),
            "plan_extent.rs",
            "fn build_extents",
        ),
        (
            include_str!("plan_always.rs"),
            "plan_always.rs",
            "fn film_op",
        ),
        (include_str!("blit.rs"), "blit.rs", "fn execute"),
        (
            include_str!("blit_rect.rs"),
            "blit_rect.rs",
            "fn execute_reusing",
        ),
    ] {
        let non_test = src.split("mod tests {").next().unwrap_or(src);
        assert!(
            non_test.contains(target),
            "{file}: 検査の的 {target} が在る"
        );
        assert!(
            !non_test.contains(".resolve("),
            "{file} の hot path（非テスト領域）は atlas.resolve を呼ばない（要件 4.3）"
        );
    }
    // lib.rs の Composer 定義領域（golden_tests/composer_tests 宣言前）に `.resolve(` が無いこと。
    let lib = include_str!("lib.rs");
    let composer_area = lib.split("#[cfg(test)]").next().unwrap_or(lib);
    assert!(
        !composer_area.contains(".resolve("),
        "lib.rs の Composer hot path は atlas.resolve を呼ばない（要件 4.3）"
    );
}

// ============================================================================
// PatternState の合成 golden 檻（空 PatternState 等価・transient コマ合流）。
//
// design「合成合流と method ゲート」の Postconditions（`pattern.is_empty()` なら出力命令列・外形
// ともコマの無い合成と完全一致・要件 5.4）と、transient コマの ID 整列合流（同 ID 置換・ID 整列不変・
// 要件 5.3/4.2）を、composer_tests の headless bake 経路（`bake_atlas_sized` の全不透明単色画像）で
// **byte / pixel 単位**に固定する。全不透明画像ゆえ premultiplied SourceOver は「上層優先」に帰着し、
// 期待バッファを入力ジオメトリから**独立に再構成**できる（golden は tautology でない）。
//
// 空 PatternState の合成が既知バイトへ完全一致することで「PatternState の配線が空 pattern の出力を
// 摂動しない」ことを、非空 PatternState の合成が「コマが同 ID の pattern0 を置換しつつ animation-sort
// 整列を保つ」ことで transient 合流の中核挙動（要件 5.3）を、それぞれ回帰ガードする。
// ============================================================================

/// 要件 5.4・design Postconditions: 空 PatternState の合成が、入力ジオメトリから組んだ既知バイトと
/// **byte 完全一致**する golden。
///
/// bind pattern0 を持つ代表 surface（base 青 4×4 全不透明＋有効 bind pattern0 で赤 2×2 を (1,1) へ
/// overlay）を `PatternState::default()` で合成し、合成バイトが入力ジオメトリから独立再構成した
/// 期待バッファと byte-for-byte 一致することを固定する。空 PatternState は no-op ゆえ、この一致は
/// 「PatternState の配線が空 pattern の出力を一切摂動しない」ことの回帰檻になる。
#[test]
fn empty_pattern_state_compose_is_byte_identical_to_pre_extension_golden() {
    let base = Path::new("shell/master");
    // base(青 4×4 全不透明) を (0,0)、有効 bind pattern0（赤 2×2）を (1,1) へ overlay。全不透明ゆえ
    // SourceOver は上層優先＝期待バッファを独立再構成できる（tautology でない golden）。
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (4, 4), Some((200, 0, 0))), // 青。
            ("part.png", (2, 2), Some((0, 0, 200))), // 赤（bind pattern0）。
        ],
    );
    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 1, 1)], // pattern0 → surface1100（赤 part）を (1,1) へ。
    );
    let part = surface(1100, vec![elem(0, "part.png", 0, 0)]);
    let world = build_bound(shell_of(vec![host, part]), &atlas);

    let binds = BindSet::from_ids([1]);
    let mut composer = Composer::new();
    let out = composer
        .compose(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("空 PatternState の合成は Ok（要件 5.4）");

    // 外形 4×4（base 4×4 ∪ part 2×2@(1,1)＝3×3 の和集合＝4×4・pattern は外形不寄与）。
    assert_eq!(
        (out.width(), out.height(), out.stride()),
        (4, 4, 16),
        "外形は静的母集合どおり 4×4（PatternState は外形へ寄与しない・design 不変条件）"
    );

    // 独立再構成した期待バッファ: base 青全面、その上に赤 part を (1,1)-(2,2) へ（上層優先）。
    let mut expected: Vec<u8> = Vec::with_capacity(4 * 4 * 4);
    for y in 0..4u32 {
        for x in 0..4u32 {
            let red_covered = (1..3).contains(&x) && (1..3).contains(&y);
            if red_covered {
                expected.extend_from_slice(&[0, 0, 200, 255]); // 赤 part（上層）。
            } else {
                expected.extend_from_slice(&[200, 0, 0, 255]); // 青 base。
            }
        }
    }
    assert_eq!(
        out.bytes(),
        expected.as_slice(),
        "空 PatternState の合成は既知バイトと byte 完全一致（要件 5.4・空 pattern は no-op）"
    );
}

/// 要件 5.3・4.2: transient コマが同 ID の pattern0 を置換し、animation-sort の
/// ID 昇順描画（画家のアルゴリズム）が保たれる golden。
///
/// host(1000) に有効 bind 2 本（id=1→surface1100 青 4×4／id=2→surface1200 緑 2×2）を置き、
/// `PatternState` で **id=1 の現在コマ = surface1300（赤 4×4）** を与える。合流規則（design 決定）は
/// 対象 id 集合＝{ 有効 bind pattern0 id } ∪ { コマ id } を既存 animation-sort（Descend 既定＝id 昇順
/// 描画）で整列し、コマがあれば同 ID の pattern0 静的寄与を置換する（4.2）。
///
/// 検証:
///   (a) 置換: id=1 の層は 1100(青) でなく 1300(赤)＝青は 1 画素も現れない。
///   (b) 整列不変: id 昇順描画で id=2(緑) が id=1(コマ赤) の**上**＝(0,0) は緑・緑外側 (2,2) は赤。
#[test]
fn transient_frame_replaces_same_id_pattern0_and_preserves_painter_order() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("p1.png", (4, 4), Some((200, 0, 0))), // 青＝id=1 pattern0（surface1100）。
            ("p2.png", (2, 2), Some((0, 200, 0))), // 緑＝id=2 pattern0（surface1200・小さめ）。
            ("p3.png", (4, 4), Some((0, 0, 200))), // 赤＝id=1 の現在コマ（surface1300）。
        ],
    );
    let host = surface_with_anims(
        1000,
        Vec::new(), // 静的 element なし（全パーツ bind＋コマ）。
        vec![bind_anim(1, 1100, 0, 0), bind_anim(2, 1200, 0, 0)],
    );
    let s1100 = surface(1100, vec![elem(0, "p1.png", 0, 0)]);
    let s1200 = surface(1200, vec![elem(0, "p2.png", 0, 0)]);
    let s1300 = surface(1300, vec![elem(0, "p3.png", 0, 0)]); // コマ参照先（pattern0 を持たない transient）。
    let world = build_bound(shell_of(vec![host, s1100, s1200, s1300]), &atlas);

    let binds = BindSet::from_ids([1, 2]);

    // id=1 の現在コマ = surface1300（赤）を (0,0) へ。method=Overlay ゆえ駆動される。
    let mut pattern = PatternState::default();
    pattern.set(
        1,
        PatternFrame {
            surface_id: 1300,
            method: ComposeMethod::Overlay,
            x: 0,
            y: 0,
        },
    );

    let mut composer = Composer::new();
    let out = composer
        .compose(&world, &atlas, 1000, &binds, &pattern)
        .expect("transient コマ込みの合成は Ok（要件 5.3）");

    // 外形 4×4（外形の表は全部の bind の描くコマ＝1100(4×4)∪1200(2×2) の和集合・コマ 1300 は外形不寄与）。
    assert_eq!(
        (out.width(), out.height()),
        (4, 4),
        "外形は bind の描くコマの母集合どおり 4×4"
    );

    // (a) コマが pattern0 を置換: id=1 の層は 1100(青) でなく 1300(赤)。緑(2×2)の外側 (2,2) は赤。
    assert_eq!(
        px(&out, 2, 2),
        [0, 0, 200, 255],
        "id=1 の現在コマ(赤 surface1300)が pattern0(青 surface1100)を置換（要件 4.2）"
    );
    // 置換された青 pattern0 は合成に 1 画素も現れない（コマが完全に肩代わり）。
    assert!(
        !out.bytes().chunks_exact(4).any(|p| p == [200, 0, 0, 255]),
        "置換された pattern0(青 surface1100)は合成へ寄与しない（要件 4.2・現在コマ 1 枚）"
    );

    // (b) animation-sort 整列不変（Descend 既定＝id 昇順描画）: id=2(緑) が id=1(コマ赤) の上。
    assert_eq!(
        px(&out, 0, 0),
        [0, 200, 0, 255],
        "id 昇順描画で id=2(緑)が上層＝合流後も整列規則そのものは不変（要件 5.3）"
    );
}

/// transient 合流の非空虚性ガード: 空 PatternState では id=1 が青 pattern0(1100) のまま＝上檻の
/// 「赤が置換」が pattern 由来であること（元々赤ではない）を対比で固定する。
#[test]
fn transient_merge_is_pattern_driven_not_static() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("p1.png", (4, 4), Some((200, 0, 0))),
            ("p2.png", (2, 2), Some((0, 200, 0))),
            ("p3.png", (4, 4), Some((0, 0, 200))),
        ],
    );
    let host = surface_with_anims(
        1000,
        Vec::new(),
        vec![bind_anim(1, 1100, 0, 0), bind_anim(2, 1200, 0, 0)],
    );
    let s1100 = surface(1100, vec![elem(0, "p1.png", 0, 0)]);
    let s1200 = surface(1200, vec![elem(0, "p2.png", 0, 0)]);
    let s1300 = surface(1300, vec![elem(0, "p3.png", 0, 0)]);
    let world = build_bound(shell_of(vec![host, s1100, s1200, s1300]), &atlas);

    // 空 PatternState → id=1 は青 pattern0(1100) のまま。緑外側 (2,2) は青（赤ではない）。
    let out = Composer::new()
        .compose(
            &world,
            &atlas,
            1000,
            &BindSet::from_ids([1, 2]),
            &PatternState::default(),
        )
        .expect("Ok");
    assert_eq!(
        px(&out, 2, 2),
        [200, 0, 0, 255],
        "空 PatternState では id=1 は青 pattern0(1100) のまま（赤置換は pattern 由来＝上檻は非空虚）"
    );
    // 赤(コマ surface1300)は空 pattern では 1 画素も現れない。
    assert!(
        !out.bytes().chunks_exact(4).any(|p| p == [0, 0, 200, 255]),
        "空 PatternState では transient コマ(赤 1300)は合成に現れない"
    );
}
