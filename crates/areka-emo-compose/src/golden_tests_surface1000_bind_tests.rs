//! surface1000（全パーツ MAYUNA bind の本体 surface）＋有効 bind 集合の
//! golden テーマ。
//!
//! emo2 の surface1000 は静的 element を一切持たず、可視パーツが着せ替えの animation（`interval,bind`
//! の各 pattern0 が 1100 系パーツ surface を overlay 参照・まばたきの 1400 は `bind+random`）で構成される「着せ替え
//! base surface」である。これは中核ケース＝静的 element ゼロでも有効 bind 集合が
//! あれば描く項を持つ surface。ここでは空 BindSet → 描く項ゼロだが、
//! 外形は bind 非依存の静的量（全部の bind の描くコマの原寸の和集合）ゆえ非ゼロで、bind を全部入れたときと
//! 同じことを固める。bind の画素の点は WARP の見本（`areka-emo-present` の `offscreen_emo2_tests.rs`）が持つ。
//! すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし）。

use super::BindSet;
use super::test_support::{
    build_atlas_for_surface1000, parse_emo2_shell, plan_surface1000, shell_master_dir,
    surface1000_bind_parts,
};
use areka_parsers::shell::AppendTarget;

/// surface1000 が「静的 element ゼロ・全パーツ bind」の着せ替え base surface であることを実データで確認する。
///
/// 下のテストと WARP の画素の点の見本の前提を fixture で固定する診断テスト。
#[test]
fn surface1000_is_all_bind_no_static_elements() {
    let shell = parse_emo2_shell();
    let s1000 = shell
        .surfaces
        .iter()
        .find(|s| s.targets == [AppendTarget::Single(1000)])
        .expect("emo2 surfaces.txt に surface1000 が存在する");
    assert!(
        s1000.elements.is_empty(),
        "surface1000 は静的 element を持たない（全パーツ MAYUNA bind・要件 5.4 の対象）"
    );
    assert!(
        !s1000.animations.is_empty(),
        "surface1000 は bind animation 群を持つ"
    );
    // 本テストが使う代表 bind（1100/1200/1302）が実データに純 `bind` として存在する。
    use areka_parsers::shell::Interval;
    for p in surface1000_bind_parts() {
        let anim = s1000
            .animations
            .iter()
            .find(|a| a.id == p.anim_id)
            .unwrap_or_else(|| panic!("surface1000 に animation{} が存在する", p.anim_id));
        assert_eq!(
            anim.interval,
            Interval::Bind,
            "animation{} は純 bind",
            p.anim_id
        );
        // pattern0 が同番号パーツ surface を overlay 参照する（surfaces.txt 実データ）。
        let pat0 = anim
            .patterns
            .iter()
            .find(|pt| pt.index == 0)
            .expect("bind animation に pattern0 が存在する");
        assert_eq!(
            pat0.surface_id, p.anim_id as i64,
            "animation{} の pattern0 は同番号パーツ surface を参照",
            p.anim_id
        );
    }
}

/// 空 BindSet → 描く項ゼロだが外形は非ゼロ。
///
/// surface1000 は静的 element ゼロゆえ、空 BindSet では描く項が一切生まれない。一方、外形（[`Extent`]）は**有効 bind 非依存の静的量**（全部の bind
/// の描くコマの原寸の和集合）ゆえ、パーツ画像を挿入した以上は非ゼロで、代表 bind 3 本（1100/1200/1302）を
/// 入れたときと同じ。これは「全 bind surface でも外形が安定（bind on/off でサイズ不変）」を実データで固定する。
#[test]
fn surface1000_empty_bindset_has_no_items_with_nonzero_extent() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);

    let empty = plan_surface1000(&shell, &atlas, &BindSet::default());
    assert!(
        empty.extent.w > 0 && empty.extent.h > 0,
        "外形は bind 非依存で非ゼロ"
    );

    let full = plan_surface1000(&shell, &atlas, &BindSet::from_ids([1100, 1200, 1302]));
    assert_eq!(empty.extent, full.extent, "外形は有効 bind に依らない");
    assert!(empty.items.is_empty(), "空 BindSet では描く項が無い");
    assert_eq!(full.items.len(), 3, "対照: 3 本の bind は 3 項を描く");
}
