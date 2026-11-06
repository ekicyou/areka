//! 要件 11.2・5.4・11.4: surface1000（全パーツ MAYUNA bind の本体 surface）＋有効 bind 集合の
//! golden テーマ。
//!
//! emo2 の surface1000 は静的 element を一切持たず、可視パーツが着せ替えの animation（`interval,bind`
//! の各 pattern0 が 1100 系パーツ surface を overlay 参照・まばたきの 1400 は `bind+random`）で構成される「着せ替え
//! base surface」である。これは要件 5.4 の中核ケース＝静的 element ゼロでも有効 bind 集合が
//! あれば非空ビットマップになる surface。ここでは:
//!   ① 空 BindSet → 描画命令ゼロ → 全画素 α==0（全透明）だが、外形は bind 非依存の静的量
//!      （全部の bind の描くコマの原寸の和集合）ゆえ非ゼロで、bind を全部入れたときと同じ。
//!   ② bind 数に応じた重なりを要点サンプリング（要件 11.2）: パーツごとに判別可能な色・サイズを
//!      与え、特定パーツだけが覆う画素が「その bind が有効なときだけ不透明」になることで、合成が
//!      固定画像でなく bind 集合を反映していることを証明する。
//! すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし・要件 11.4）。

use super::BindSet;
use super::test_support::{
    build_atlas_for_surface1000, compose_surface1000, opaque_pixel_count, parse_emo2_shell,
    shell_master_dir, surface1000_bind_parts,
};
use crate::plan_test_support::px;
use areka_parsers::shell::AppendTarget;

/// surface1000 が「静的 element ゼロ・全パーツ bind」の着せ替え base surface であることを実データで確認する。
///
/// 下の 2 本の前提（要件 5.4 の対象 surface である）を fixture で固定する診断テスト。
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

/// ①（要件 5.4・11.2）: 空 BindSet → 全透明（α==0）だが外形は非ゼロ。
///
/// surface1000 は静的 element ゼロゆえ、空 BindSet では描画可能命令が一切生まれず、合成結果は
/// 全画素 α==0（全透明）になる。一方、外形（[`Extent`]）は**有効 bind 非依存の静的量**（全部の bind
/// の描くコマの原寸の和集合）ゆえ、パーツ画像を挿入した以上は非ゼロで、代表 bind 3 本（1100/1200/1302）を
/// 入れたときと同じ。これは「全 bind surface でも外形が安定（bind on/off でサイズ不変）」を実データで固定する。
#[test]
fn surface1000_empty_bindset_is_fully_transparent_with_nonzero_extent() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);

    let out = compose_surface1000(&shell, &atlas, &BindSet::default());

    // 外形は非ゼロ（bind 非依存の静的外形＝挿入パーツ原寸の和集合）。
    assert!(
        out.width() > 0 && out.height() > 0,
        "外形は bind 非依存で非ゼロ"
    );
    assert_eq!(
        out.stride(),
        out.width() * 4,
        "premultiplied BGRA stride 契約"
    );

    // bind on/off で外形は変わらない（代表 bind 3 本を入れた合成と同じ幅・高さ）。
    let full = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1100, 1200, 1302]));
    assert_eq!(
        (out.width(), out.height()),
        (full.width(), full.height()),
        "外形は有効 bind に依らない"
    );

    // 全画素 α==0（描画命令ゼロ＝全透明）。α>0 の画素が 1 つも無いことを固定する。
    assert_eq!(
        opaque_pixel_count(&out),
        0,
        "空 BindSet では全画素が透明（α>0 の画素が存在しない・要件 5.4）"
    );
}

/// ②（要件 11.2・5.4）: bind 数に応じた重なりを要点サンプリング。
///
/// 各パーツに **判別可能な色・サイズ**を与え、特定パーツだけが覆う画素を「その bind が有効な
/// ときだけ不透明・無効なら透明」であることでサンプリング検証する。合成が固定画像でなく
/// **bind 集合を反映**していること、bind を増やすと重なり（不透明画素）が増えることを示す。
///
/// パーツ配置（全 pattern0 offset=(0,0)・パーツ element0=(0,0) ゆえ全て原点で重なる）:
///   - 腕 1100: 80×20（横に広い）→ x=70,y=5 は 1100 のみが覆う（口 30 幅・目 20 幅は届かない）。
///   - 目 1302: 20×90（縦に長い）→ x=5,y=80 は 1302 のみが覆う（腕 h=20・口 h=30 は届かない）。
#[test]
fn surface1000_bind_count_overlap_point_sampling() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);

    // 腕だけが覆う画素 (70,5)・目だけが覆う画素 (5,80) を選ぶ（他パーツの寸法外）。
    const ARM_ONLY: (u32, u32) = (70, 5); // 腕 1100（80×20）内・口/目の外。
    const EYE_ONLY: (u32, u32) = (5, 80); // 目 1302（20×90）内・腕/口の外。

    // (a) 腕 1100 のみ有効 → 腕専用画素は不透明・目専用画素は透明。
    let out_arm = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1100]));
    assert!(
        px(&out_arm, ARM_ONLY.0, ARM_ONLY.1)[3] > 0,
        "腕 1100 有効 → 腕専用画素 (70,5) は不透明"
    );
    assert_eq!(
        px(&out_arm, EYE_ONLY.0, EYE_ONLY.1)[3],
        0,
        "目 1302 無効 → 目専用画素 (5,80) は透明（合成は bind 集合を反映）"
    );

    // (b) 目 1302 のみ有効 → 目専用画素は不透明・腕専用画素は透明（対称の反証）。
    let out_eye = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1302]));
    assert!(
        px(&out_eye, EYE_ONLY.0, EYE_ONLY.1)[3] > 0,
        "目 1302 有効 → 目専用画素 (5,80) は不透明"
    );
    assert_eq!(
        px(&out_eye, ARM_ONLY.0, ARM_ONLY.1)[3],
        0,
        "腕 1100 無効 → 腕専用画素 (70,5) は透明"
    );

    // (c) 腕＋目 両方有効 → 両専用画素とも不透明。
    let out_both = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1100, 1302]));
    assert!(
        px(&out_both, ARM_ONLY.0, ARM_ONLY.1)[3] > 0
            && px(&out_both, EYE_ONLY.0, EYE_ONLY.1)[3] > 0,
        "腕＋目 有効 → 両専用画素とも不透明"
    );

    // bind 数に応じた重なり量: 1 bind < 2 bind < 3 bind（不透明画素は単調増加）。
    // 全パーツ (0,0) 配置で重なるが、サイズが異なるため和集合の面積は bind 追加で増える。
    let n_arm = opaque_pixel_count(&out_arm);
    let n_eye = opaque_pixel_count(&out_eye);
    let n_both = opaque_pixel_count(&out_both);
    // 腕(80×20=1600) と 目(20×90=1800) の和集合は各単独より真に大きい（(0,0) 重なりでも
    // 一方が他方を完全内包しない配置ゆえ）。
    assert!(n_both > n_arm, "腕＋目 は腕単独より真に多い不透明画素");
    assert!(n_both > n_eye, "腕＋目 は目単独より真に多い不透明画素");

    // 3 bind（腕＋口＋目）→ 2 bind より真に多い（口 1200 の 30×30 は (20..30, 20..30) を腕にも目にも
    // 覆われずに足す）。
    let out_three = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1100, 1200, 1302]));
    let n_three = opaque_pixel_count(&out_three);
    assert!(
        n_three > n_both,
        "3 bind の不透明画素数は 2 bind より真に多い（要件 11.2）"
    );
}
