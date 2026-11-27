//! `Composer` ファサード結線の in-source テスト。
//!
//! fold（[`EmoWorld::build`]）→ bind（[`EmoWorld::bind_atlas`]）→ plan（[`Composer::plan`]）の全結線を、
//! `MemoryDecoder`+`bake` の COM/WIC 非依存経路で headless に検証する。Composer が計画を保持しないことを
//! 固定する（同じ入力の繰り返しで計画と作業場の容量が変わらないことは
//! `golden_tests_determinism_budget_tests`、失敗経路の `Err` と記録は `log_firing_tests`）。

use super::*;
use areka_emo_atlas::AtlasTable;
use areka_parsers::shell::SortOrder;
use std::path::Path;

use crate::plan_test_support::{
    bake_atlas_sized, bind_anim, bound_world, elem, frame, id_to_path, named_plan, shell_of,
    shell_of_with_sort, surface, surface_with_anims,
};

/// 40×30 の base（不透明青）を (0,0) に置き、有効 bind 1 が 200×10 の part（不透明赤）を (0,0) に置く面 1000。
fn blue_base_red_part() -> (EmoWorld, AtlasTable) {
    let atlas = bake_atlas_sized(
        Path::new("shell/master"),
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
    (bound_world(&shell_of(vec![host, part]), &[], &atlas), atlas)
}

/// 静的 element の無い面 1000 に bind 1（面 1100・青 4×4）と bind 2（面 1200・緑 2×2）。面 1300（赤 4×4）は
/// どの着せ替えの手順にも無い、コマの参照先。
fn two_binds_and_a_frame() -> (EmoWorld, AtlasTable) {
    let atlas = bake_atlas_sized(
        Path::new("shell/master"),
        &[
            ("p1.png", (4, 4), Some((200, 0, 0))), // 青＝id=1 の着せ替えの手順（surface1100）。
            ("p2.png", (2, 2), Some((0, 200, 0))), // 緑＝id=2 の着せ替えの手順（surface1200・小さめ）。
            ("p3.png", (4, 4), Some((0, 0, 200))), // 赤＝id=1 の現在コマ（surface1300）。
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
    (
        bound_world(&shell_of(vec![host, s1100, s1200, s1300]), &[], &atlas),
        atlas,
    )
}

// ── 有効入力で Ok(DrawPlan)・外形一致 ──

/// 静的 element 1 本＋有効 bind 1 本の surface の計画 → `Ok(DrawPlan)`。
///
/// 40×30 の base を (0,0)、200×10 の bind part を (0,0) に重ねる。外形は
/// 全 element＋全部の bind の描くコマの和集合＝w=max(40,200)=200・h=max(30,10)=30。
/// bind part は base の後（上）に描く。
#[test]
fn plan_returns_ok_draw_plan_for_valid_input() {
    let (world, atlas) = blue_base_red_part();

    let binds = BindSet::from_ids([1]);
    let plan = Composer::new()
        .plan(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("有効入力は Ok");

    assert_eq!((plan.extent.w, plan.extent.h), (200, 30));
    assert_eq!((plan.surface_id, plan.base_surface), (1000, 1000));
    let map = id_to_path(&atlas, &["base.png", "part.png"]);
    assert_eq!(
        named_plan(&plan, &map),
        [("base.png", 0, 0), ("part.png", 0, 0)],
        "base の上に bind part を描く"
    );
}

// ── Composer は計画を保持しない（挙動的） ──

/// 異なる surface を連続で計画して、各々が自 surface を反映する（古い結果を返さない）。
#[test]
fn composer_holds_no_result_no_stale_cache() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("a.png", (10, 10), Some((200, 0, 0))),
            ("b.png", (20, 20), Some((0, 200, 0))),
        ],
    );
    let sa = surface(100, vec![elem(0, "a.png", 0, 0)]);
    let sb = surface(200, vec![elem(0, "b.png", 0, 0)]);
    let world = bound_world(&shell_of(vec![sa, sb]), &[], &atlas);
    let map = id_to_path(&atlas, &["a.png", "b.png"]);

    let binds = BindSet::default();
    let mut composer = Composer::new();

    let plan_a = composer
        .plan(&world, &atlas, 100, &binds, &PatternState::default())
        .expect("A Ok");
    assert_eq!(plan_a.extent.w, 10);
    assert_eq!(named_plan(&plan_a, &map), [("a.png", 0, 0)]);

    let plan_b = composer
        .plan(&world, &atlas, 200, &binds, &PatternState::default())
        .expect("B Ok");
    assert_eq!(
        plan_b.extent.w, 20,
        "B の外形（20）を反映＝A の 10 を持ち越さない"
    );
    assert_eq!(named_plan(&plan_b, &map), [("b.png", 0, 0)]);

    let plan_a2 = composer
        .plan(&world, &atlas, 100, &binds, &PatternState::default())
        .expect("A2 Ok");
    assert_eq!(plan_a2, plan_a, "A の組み直しは初回 A と同じ計画");
}

// ── plan_into は使い回した out を必ず組み直す ──

/// 別の面の計画が入った `out` へ `plan_into` した結果が、新しい計画と等しい（前の項を残さない）。
#[test]
fn plan_into_a_used_plan_equals_a_fresh_one() {
    let (world, atlas) = blue_base_red_part();
    let binds = BindSet::from_ids([1]);
    let mut composer = Composer::new();
    let fresh = composer
        .plan(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("Ok");

    let mut used = composer
        .plan(&world, &atlas, 1100, &binds, &PatternState::default())
        .expect("Ok");
    assert_ne!(used, fresh, "検体: 使い回す計画は別の中身");
    composer
        .plan_into(
            &mut used,
            &world,
            &atlas,
            1000,
            &binds,
            &PatternState::default(),
        )
        .expect("Ok");
    assert_eq!(used, fresh);
}

// ── 正常空合成（失敗経路の伝播と記録は `log_firing_tests`）──

/// 全 element 全透明 → `Ok`・項の無い計画・非ゼロ外形。
#[test]
fn all_transparent_surface_is_ok_with_no_items_and_nonzero_extent() {
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
    let world = bound_world(&shell_of(vec![surf]), &[], &atlas);

    let plan = Composer::new()
        .plan(
            &world,
            &atlas,
            3000,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("全透明でもエラーにせず Ok");

    // 外形は原寸で非ゼロ（全透明でも original で寄与＝300×300 が支配）。
    assert_eq!((plan.extent.w, plan.extent.h), (300, 300));
    assert!(plan.items.is_empty(), "全透明 surface は描く項を持たない");
}

/// ComposeError の Display（thiserror `#[error]`）が正準文字列と厳密一致する（決定論的）。
#[test]
fn compose_error_display_strings_are_exact() {
    assert_eq!(
        ComposeError::SurfaceNotFound(7).to_string(),
        "surface 7 not found",
    );
    assert_eq!(
        ComposeError::EmptyComposition(7).to_string(),
        "surface 7 has an empty extent (width or height 0)",
    );
    assert_eq!(
        ComposeError::ExtentTooLarge {
            surface: 7,
            w: 16_385,
            h: 2
        }
        .to_string(),
        "surface 7 extent 16385x2 exceeds the size limit (side 16384 or area 33554432 pixels)",
    );
}

// ── multi-layer 実データの end-to-end 結線 ──

/// animation-sort=ascend の多層 + 複数 bind を実データ構造で end-to-end（fold→bind→plan）に計画し、
/// 描く順（ascend ゆえ ID 降順）が計画の項の順へ反映されることで結線を証明する。
///
/// hot path が `AtlasTable::resolve` を呼ばないことは、下の
/// [`hot_path_sources_do_not_call_atlas_resolve`] がソースを走査して確かめる。
#[test]
fn end_to_end_multilayer_ascend_wiring() {
    let base = Path::new("shell/master");
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
    let world = bound_world(
        &shell_of_with_sort(vec![host, p1, p2, p3], Some(SortOrder::Ascend)),
        &[],
        &atlas,
    );

    let binds = BindSet::from_ids([1, 2, 3]);
    let plan = Composer::new()
        .plan(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("Ok");

    assert_eq!((plan.extent.w, plan.extent.h), (4, 4));
    let map = id_to_path(&atlas, &["p1.png", "p2.png", "p3.png"]);
    assert_eq!(
        named_plan(&plan, &map),
        [("p3.png", 0, 0), ("p2.png", 0, 0), ("p1.png", 0, 0)],
        "ascend＝ID降順に描き、最後（最上層）が p1"
    );
}

/// 計画の道と外形の表を作る道のソースに `atlas.resolve(` が現れないことを静的に確認する。
///
/// 毎フレームの道の plan.rs／plan_always.rs／bind_steps.rs（着せ替えの手順）／plan_mix.rs／plan_cap.rs／
/// lib.rs（Composer）と、束縛のときに外形の表を作る plan_extent.rs・outline_walk.rs（合成からは呼ばない）の
/// 非テストコードは `atlas.entry`（O(1)）のみを引き、
/// `resolve` は bind_atlas（atlas_bind.rs）の名前引きに限られる。
/// 本テストは各ソースの非テスト領域文字列を検査し、hot path に `resolve` 呼び出しが無いことを固定する。
#[test]
fn hot_path_sources_do_not_call_atlas_resolve() {
    // 見るファイルの、中に書いたテストの mod より前に `.resolve(` が無いこと（plan.rs と
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
        (
            include_str!("bind_steps.rs"),
            "bind_steps.rs",
            "fn push_bind_contribution",
        ),
        (
            include_str!("outline_walk.rs"),
            "outline_walk.rs",
            "fn walk",
        ),
        (include_str!("plan_mix.rs"), "plan_mix.rs", "fn push_items"),
        (include_str!("plan_cap.rs"), "plan_cap.rs", "fn cap_pixels"),
    ] {
        let non_test = src.split("mod tests {").next().unwrap_or(src);
        assert!(
            non_test.contains(target),
            "{file}: 検査の的 {target} が在る"
        );
        assert!(
            !non_test.contains(".resolve("),
            "{file} の hot path（非テスト領域）は atlas.resolve を呼ばない"
        );
    }
    // lib.rs の Composer 定義領域（golden_tests/composer_tests 宣言前）に `.resolve(` が無いこと。
    let lib = include_str!("lib.rs");
    let composer_area = lib.split("#[cfg(test)]").next().unwrap_or(lib);
    assert!(
        composer_area.contains("fn plan_into"),
        "lib.rs: 検査の的 fn plan_into が在る"
    );
    assert!(
        !composer_area.contains(".resolve("),
        "lib.rs の Composer hot path は atlas.resolve を呼ばない"
    );
}

// ============================================================================
// PatternState の計画の檻（空 PatternState 等価・transient コマ合流）。
//
// 事後条件（`pattern.is_empty()` なら計画・外形ともコマの無い合成と完全一致）と、transient コマの ID 整列
// 合流（同 ID 置換・ID 整列不変）を、計画の項の (画像, 位置) の列で固定する。期待の列は入力の
// ジオメトリから独立に組む（tautology でない）。
// ============================================================================

/// 空 PatternState の計画が、入力から組んだ既知の項の列と一致する。
///
/// 着せ替えを持つ代表 surface（base 4×4 を (0,0)＋有効 bind の手順で part 2×2 を (1,1)）を
/// `PatternState::default()` で計画する。空 PatternState は no-op ゆえ、この一致は
/// 「PatternState の配線が空 pattern の出力を一切摂動しない」ことの回帰檻になる。
#[test]
fn empty_pattern_state_plan_equals_the_rebuilt_items() {
    let base = Path::new("shell/master");
    let atlas = bake_atlas_sized(
        base,
        &[
            ("base.png", (4, 4), Some((200, 0, 0))),
            ("part.png", (2, 2), Some((0, 0, 200))),
        ],
    );
    let host = surface_with_anims(
        1000,
        vec![elem(0, "base.png", 0, 0)],
        vec![bind_anim(1, 1100, 1, 1)], // 手順 → surface1100（part）を (1,1) へ。
    );
    let part = surface(1100, vec![elem(0, "part.png", 0, 0)]);
    let world = bound_world(&shell_of(vec![host, part]), &[], &atlas);

    let binds = BindSet::from_ids([1]);
    let plan = Composer::new()
        .plan(&world, &atlas, 1000, &binds, &PatternState::default())
        .expect("空 PatternState の計画は Ok");

    // 外形 4×4（base 4×4 ∪ part 2×2@(1,1)＝3×3 の和集合＝4×4・pattern は外形不寄与）。
    assert_eq!((plan.extent.w, plan.extent.h), (4, 4));
    let map = id_to_path(&atlas, &["base.png", "part.png"]);
    assert_eq!(
        named_plan(&plan, &map),
        [("base.png", 0, 0), ("part.png", 1, 1)],
        "空 PatternState の計画は既知の項の列と一致（空 pattern は no-op）"
    );
}

/// transient コマが同 ID の着せ替えの手順を置換し、animation-sort の ID 昇順描画が保たれる。
///
/// host(1000) に有効 bind 2 本（id=1→surface1100 p1 4×4／id=2→surface1200 p2 2×2）を置き、
/// `PatternState` で **id=1 の現在コマ = surface1300（p3 4×4）** を与える。
///
/// 検証:
///   (a) 置換: id=1 の層は 1100(p1) でなく 1300(p3)＝p1 は計画に現れない。
///   (b) 整列不変: id 昇順描画で id=2(p2) が id=1(コマ p3) の**後（上）**。
#[test]
fn transient_frame_replaces_same_id_bind_steps_and_preserves_painter_order() {
    let (world, atlas) = two_binds_and_a_frame();

    let binds = BindSet::from_ids([1, 2]);
    let mut pattern = PatternState::default();
    pattern.set(1, frame(1300, 0, 0));

    let plan = Composer::new()
        .plan(&world, &atlas, 1000, &binds, &pattern)
        .expect("transient コマ込みの計画は Ok");

    // 外形 4×4（外形の表は全部の bind の描くコマ＝1100(4×4)∪1200(2×2) の和集合・コマ 1300 は外形不寄与）。
    assert_eq!((plan.extent.w, plan.extent.h), (4, 4));
    let map = id_to_path(&atlas, &["p1.png", "p2.png", "p3.png"]);
    assert_eq!(
        named_plan(&plan, &map),
        [("p3.png", 0, 0), ("p2.png", 0, 0)],
        "id=1 はコマ(p3)に置き換わり、id=2(p2)がその上"
    );
}

/// transient 合流の非空虚性ガード: 空 PatternState では id=1 が手順(1100) のまま＝上檻の
/// 「p3 が置換」が pattern 由来であること（元々 p3 ではない）を対比で固定する。
#[test]
fn transient_merge_is_pattern_driven_not_static() {
    let (world, atlas) = two_binds_and_a_frame();

    let plan = Composer::new()
        .plan(
            &world,
            &atlas,
            1000,
            &BindSet::from_ids([1, 2]),
            &PatternState::default(),
        )
        .expect("Ok");
    let map = id_to_path(&atlas, &["p1.png", "p2.png", "p3.png"]);
    assert_eq!(
        named_plan(&plan, &map),
        [("p1.png", 0, 0), ("p2.png", 0, 0)],
        "空 PatternState では id=1 は手順(1100) のまま"
    );
}
