//! task 8.2（要件 11.2・5.4・11.4）: surface1000（全パーツ MAYUNA bind の本体 surface）＋
//! 有効 bind 集合の golden テーマ。区画バナーは共有ヘルパ `solid_opaque` に付随するため
//! `test_support` 側に在る（本文一致の項目境界に従った結果）。

use super::test_support::{
    build_atlas_for_surface1000, compose_surface1000, opaque_pixel_count, parse_emo2_shell,
    shell_master_dir, surface1000_bind_parts,
};
use super::{BindSet, ComposedSurface};

/// 合成結果の (x,y) 画素の α バイト（BGRA の index+3）を読む。
fn alpha_at(s: &ComposedSurface, x: u32, y: u32) -> u8 {
    let i = (y * s.stride() + x * 4 + 3) as usize;
    s.bytes()[i]
}

/// surface1000 が「静的 element ゼロ・全パーツ bind」の着せ替え base surface であることを実データで確認する。
///
/// これは 8.2 の前提（要件 5.4 の対象 surface である）を fixture で固定する診断テスト。
#[test]
fn surface1000_is_all_bind_no_static_elements() {
    let shell = parse_emo2_shell();
    let s1000 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 1000)
        .expect("emo2 surfaces.txt に surface1000 が存在する");
    assert!(
        s1000.elements.is_empty(),
        "surface1000 は静的 element を持たない（全パーツ MAYUNA bind・要件 5.4 の対象）"
    );
    assert!(
        !s1000.animations.is_empty(),
        "surface1000 は bind animation 群を持つ"
    );
    // 本テストが使う代表 bind（1100/1200/1302）が実データに `interval,bind` として存在する。
    use areka_parsers::shell::Interval;
    for p in surface1000_bind_parts() {
        let anim = s1000
            .animations
            .iter()
            .find(|a| a.id == p.anim_id)
            .unwrap_or_else(|| panic!("surface1000 に animation{} が存在する", p.anim_id));
        assert!(
            matches!(anim.interval, Interval::Bind | Interval::BindRandom { .. }),
            "animation{} は bind 種 interval",
            p.anim_id
        );
        // pattern0 が同番号パーツ surface を overlay 参照する（surfaces.txt 実データ）。
        let pat0 = anim
            .patterns
            .iter()
            .min_by_key(|pt| pt.index)
            .expect("bind animation に pattern0 が存在する");
        assert_eq!(
            pat0.surface_id, p.anim_id as i64,
            "animation{} の pattern0 は同番号パーツ surface を参照",
            p.anim_id
        );
    }
}

/// task 8.2 ①（要件 5.4・11.2）: 空 BindSet → 全透明（α==0）だが外形は非ゼロ。
///
/// surface1000 は静的 element ゼロゆえ、空 BindSet では描画可能命令が一切生まれず、合成結果は
/// 全画素 α==0（全透明）になる。一方、外形（[`Extent`]）は**有効 bind 非依存の静的量**（全 bind
/// pattern0 原寸の和集合・task 5.4）ゆえ、パーツ画像を挿入した以上は非ゼロになる。これは
/// 「全 bind surface でも外形が安定（bind on/off でサイズ不変）」を実データで固定する。
#[test]
fn surface1000_empty_bindset_is_fully_transparent_with_nonzero_extent() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);

    let out = compose_surface1000(&shell, &atlas, &BindSet::default());

    // 外形は非ゼロ（bind 非依存の静的外形＝挿入パーツ原寸の和集合）。
    assert!(
        out.width() > 0 && out.height() > 0,
        "外形は bind 非依存で非ゼロ（task 5.4）"
    );
    assert_eq!(
        out.stride(),
        out.width() * 4,
        "premultiplied BGRA stride 契約"
    );

    // 全画素 α==0（描画命令ゼロ＝全透明）。α バイトを走査して 1 つも α>0 が無いことを固定する。
    let any_opaque = out.bytes().chunks_exact(4).any(|px| px[3] > 0);
    assert!(
        !any_opaque,
        "空 BindSet では全画素が透明（α>0 の画素が存在しない・要件 5.4）"
    );
    assert_eq!(opaque_pixel_count(&out), 0, "不透明画素は皆無");
}

/// task 8.2 ②（要件 5.4・11.2）: 非空 BindSet → 非空（α>0 の画素が存在する）。
///
/// 静的 element ゼロの surface1000 でも、有効 bind 集合を与えれば参照パーツ画像が合成され、
/// 少なくとも 1 画素が α>0 になる（要件 5.4「静的 element ゼロ＋非空 bind 集合 → 可視ビットマップ・
/// 空白でない」）。空 BindSet（全透明）との対比が bind 経路が現に駆動されている証拠（RED/GREEN）。
#[test]
fn surface1000_nonempty_bindset_is_non_blank() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_surface1000(&shell, &base);

    // 腕 1100 のみ有効。
    let binds = BindSet::from_ids([1100]);
    let out = compose_surface1000(&shell, &atlas, &binds);

    let opaque = opaque_pixel_count(&out);
    assert!(
        opaque > 0,
        "非空 BindSet では α>0 の画素が存在する（非空・要件 5.4）"
    );
    // 腕パーツ（80×20 全不透明）が原点 (0,0) に着弾するので、その内側の代表画素が不透明。
    assert!(alpha_at(&out, 0, 0) > 0, "パーツ左上 (0,0) は不透明");
    assert!(alpha_at(&out, 40, 10) > 0, "腕パーツ内部の代表画素は不透明");
}

/// task 8.2 ③（要件 11.2・5.4）: bind 数に応じた重なりを要点サンプリング。
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
        alpha_at(&out_arm, ARM_ONLY.0, ARM_ONLY.1) > 0,
        "腕 1100 有効 → 腕専用画素 (70,5) は不透明"
    );
    assert_eq!(
        alpha_at(&out_arm, EYE_ONLY.0, EYE_ONLY.1),
        0,
        "目 1302 無効 → 目専用画素 (5,80) は透明（合成は bind 集合を反映）"
    );

    // (b) 目 1302 のみ有効 → 目専用画素は不透明・腕専用画素は透明（対称の反証）。
    let out_eye = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1302]));
    assert!(
        alpha_at(&out_eye, EYE_ONLY.0, EYE_ONLY.1) > 0,
        "目 1302 有効 → 目専用画素 (5,80) は不透明"
    );
    assert_eq!(
        alpha_at(&out_eye, ARM_ONLY.0, ARM_ONLY.1),
        0,
        "腕 1100 無効 → 腕専用画素 (70,5) は透明"
    );

    // (c) 腕＋目 両方有効 → 両専用画素とも不透明。
    let out_both = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1100, 1302]));
    assert!(
        alpha_at(&out_both, ARM_ONLY.0, ARM_ONLY.1) > 0
            && alpha_at(&out_both, EYE_ONLY.0, EYE_ONLY.1) > 0,
        "腕＋目 有効 → 両専用画素とも不透明"
    );

    // bind 数に応じた重なり量: 1 bind < 2 bind < 3 bind（不透明画素は単調増加）。
    // 全パーツ (0,0) 配置で重なるが、サイズが異なるため和集合の面積は bind 追加で増える。
    let n_arm = opaque_pixel_count(&out_arm);
    let n_eye = opaque_pixel_count(&out_eye);
    let n_both = opaque_pixel_count(&out_both);
    assert!(
        n_both >= n_arm && n_both >= n_eye,
        "2 bind の不透明画素数は各単独 bind 以上（重なりの和集合・要件 11.2）"
    );
    // 腕(80×20=1600) と 目(20×90=1800) の和集合は各単独より真に大きい（(0,0) 重なりでも
    // 一方が他方を完全内包しない配置ゆえ）。
    assert!(n_both > n_arm, "腕＋目 は腕単独より真に多い不透明画素");
    assert!(n_both > n_eye, "腕＋目 は目単独より真に多い不透明画素");

    // 3 bind（腕＋口＋目）→ 2 bind 以上（口 1200 追加で不透明画素が減らない）。
    let out_three = compose_surface1000(&shell, &atlas, &BindSet::from_ids([1100, 1200, 1302]));
    let n_three = opaque_pixel_count(&out_three);
    assert!(
        n_three >= n_both,
        "3 bind の不透明画素数は 2 bind 以上（単調・要件 11.2）"
    );
}
