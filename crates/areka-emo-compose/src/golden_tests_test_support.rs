//! `golden_tests` の複数テーマから参照される共有 fixture ヘルパ（テスト関数を持たない）。
//!
//! emo2 fixture のパス解決・surfaces.txt パース・`AlphaParams`・画像スペック生成・
//! surface1000 の bind パーツ土台（`BindPart` / `surface1000_bind_parts` /
//! `build_atlas_for_surface1000` / `compose_surface1000`）・不透明画素の計数を集約する。
//! 単一テーマからしか参照されないヘルパは各テーマファイル側へ残してある。

use super::{
    AlphaParams, AtlasTable, BindSet, ComposedSurface, Composer, EmoWorld, MemoryDecoder,
    PackConfig, Path, PathBuf, PatternState, SetId, Shell, SurfaceSet, UseSelfAlpha, bake,
};

/// emo2 検体の実資産のパスを組む（在処は窓口 `sample-ghost-kit` が持つ）。
fn emo2(rel: &str) -> PathBuf {
    crate::sample_test_support::emo2_root().join(rel)
}

/// emo2 の shell/master 基準 dir（surfaces.txt の element 相対パス起点）。
pub(super) fn shell_master_dir() -> PathBuf {
    emo2("shell/master")
}

/// AlphaParams { use_self_alpha: On }（emo2 の descript は seriko.use_self_alpha,1）。
pub(super) fn on_params() -> AlphaParams {
    AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    }
}

/// emo2 の surfaces.txt をパースして `Shell` を得る（`emo2_golden.rs` の loader 経路と同一）。
pub(super) fn parse_emo2_shell() -> Shell {
    let shell = crate::sample_test_support::emo2_shell();
    assert!(
        !shell.surfaces.is_empty(),
        "parse produced surfaces from surfaces.txt"
    );
    shell
}

/// surface1000 の bind パーツ surface（1100 系）の element0 相対パスを解決するための対応表。
///
/// surfaces.txt の実データ（`golden_tests_surface1000_bind_tests.rs` の先頭の説明）に基づく
/// (animation_id, part_element_rel)。
/// 各 bind animation `animationN.pattern0,overlay,N,0,0,0` は同番号のパーツ surface N を参照し、
/// その surface の element0 が下記 png を (0,0) 配置する（実 fixture より）。
/// テストでは判別のため、この rel パスへ**自作の単色画像**を MemoryDecoder へ挿入する。
pub(super) struct BindPart {
    /// surface1000 の bind animation id（= 参照先パーツ surface id と同番号）。
    pub(super) anim_id: u32,
    /// パーツ surface の element0 相対パス（shell/master 起点）。
    rel: &'static str,
    /// 挿入する単色画像の (w, h, b, g, r)。
    w: u32,
    h: u32,
    b: u8,
    g: u8,
    r: u8,
}

/// emo2 surface1000 の代表 bind パーツ 3 種（腕 1100 / 口 1200 / 目 1302）。
///
/// パーツごとに **異なるサイズ**と**異なる色**を与える。全パーツの element0 は (0,0) 配置ゆえ
/// キャンバス原点で重なるが、サイズ差により「大きいパーツだけが覆う画素」が生じる。これを
/// 要点サンプリングに使う（例: 幅 80 の 1100 は x=70 を覆うが、幅 30 の 1200 は覆わない）。
pub(super) fn surface1000_bind_parts() -> [BindPart; 3] {
    [
        // 腕（surface1100 → purple/0/base1.png）: 横に広い（80×20）・青。
        BindPart {
            anim_id: 1100,
            rel: "purple/0/base1.png",
            w: 80,
            h: 20,
            b: 255,
            g: 0,
            r: 0,
        },
        // 口（surface1200 → purple/2/a.png）: 小さい（30×30）・緑。
        BindPart {
            anim_id: 1200,
            rel: "purple/2/a.png",
            w: 30,
            h: 30,
            b: 0,
            g: 255,
            r: 0,
        },
        // 目（surface1302 → purple/4/normal.png）: 縦に長い（20×90）・赤。
        BindPart {
            anim_id: 1302,
            rel: "purple/4/normal.png",
            w: 20,
            h: 90,
            b: 0,
            g: 0,
            r: 255,
        },
    ]
}

/// surface1000 用 `AtlasTable` を構築する（COM/WIC 非依存・要件 11.4）。
///
/// surfaces.txt 全体を bake するが、MemoryDecoder には [`surface1000_bind_parts`] の各 rel のみ
/// 既知の単色画像を挿入する。他パーツ（1101/1201/…）は `NotFound` として bake の errors に
/// 記録されるだけで、本テストが与える有効 bind の合成には影響しない（有効 bind は挿入済み
/// パーツのみを指す）。
pub(super) fn build_atlas_for_surface1000(shell: &Shell, base: &Path) -> AtlasTable {
    let mut dec = MemoryDecoder::new();
    for p in surface1000_bind_parts() {
        let (w, h, stride, bgra, has_alpha) =
            crate::plan_test_support::opaque_wxh(p.w, p.h, p.b, p.g, p.r);
        dec.insert(base.join(p.rel), w, h, stride, bgra, has_alpha);
    }
    bake_emo2(shell, base, &dec)
}

/// `shell` の全部の面を `dec` の絵で焼いて表を返す（`use_self_alpha` は On・焼けない絵は errors に
/// 残るだけ）。
pub(super) fn bake_emo2(shell: &Shell, base: &Path, dec: &MemoryDecoder) -> AtlasTable {
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    bake(std::slice::from_ref(&set), dec, PackConfig::default()).table
}

/// surface1000 を有効 bind 集合で合成した結果を得る（world 構築＋bind_atlas＋compose）。
pub(super) fn compose_surface1000(
    shell: &Shell,
    atlas: &AtlasTable,
    binds: &BindSet,
) -> ComposedSurface {
    let mut world = EmoWorld::build(shell);
    world.bind_atlas(atlas, SetId(0));
    Composer::new()
        .compose(&world, atlas, 1000, binds, &PatternState::default())
        .expect("surface1000 の合成は Ok（surface 存在＋静的外形非ゼロ・要件 6.6）")
}

/// α>0 の画素数を数える（非空虚性・重なり量の要点指標）。
pub(super) fn opaque_pixel_count(s: &ComposedSurface) -> usize {
    s.bytes().chunks_exact(4).filter(|px| px[3] > 0).count()
}
