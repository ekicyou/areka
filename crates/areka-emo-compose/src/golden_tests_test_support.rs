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

/// emo2 fixture 実資産のパスを組む。
/// `CARGO_MANIFEST_DIR` = `crates/areka-emo-compose`。fixtures はパイロット crate 配下。
fn emo2(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
        .join(rel)
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
    let content = std::fs::read_to_string(emo2("shell/master/surfaces.txt"))
        .expect("emo2 surfaces.txt must be readable (UTF-8)");
    let shell = areka_parsers::shell::parse(&content);
    assert!(
        !shell.surfaces.is_empty(),
        "parse produced surfaces from surfaces.txt"
    );
    shell
}

// ============================================================================
// task 8.2: surface1000（全パーツ MAYUNA bind の本体 surface）＋ 有効 bind 集合の
// golden 統合テスト（要件 **11.2**・**5.4**・**11.4**）。
//
// emo2 の surface1000 は静的 element を一切持たず、可視パーツが全て `interval,bind` の
// animation（各 pattern0 が 1100 系パーツ surface を overlay 参照）で構成される「着せ替え
// base surface」である。これは要件 5.4 の中核ケース＝静的 element ゼロでも有効 bind 集合が
// あれば非空ビットマップになる surface。ここでは:
//   ① 空 BindSet → 描画命令ゼロ → 全画素 α==0（全透明）だが、外形は bind 非依存の静的量
//      （全 bind pattern0 原寸の和集合・task 5.4）ゆえ非ゼロ。
//   ② 非空 BindSet → 参照パーツ画像が合成され α>0 の画素が生まれる（非空）。
//   ③ bind 数に応じた重なりを要点サンプリング（要件 11.2）: パーツごとに判別可能な色・サイズを
//      与え、特定パーツだけが覆う画素が「その bind が有効なときだけ不透明」になることで、合成が
//      固定画像でなく bind 集合を反映していることを証明する。
// すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし・要件 11.4）。
// ============================================================================

/// 単色・全不透明（α=255）の premultiplied BGRA 画像スペックを生成する（tightly-packed）。
///
/// 全画素同色ゆえパーツを覆う任意画素が同一色になり、サンプリングで「どのパーツか」を色で
/// 判別できる。α=255 なので premultiplied 済み（色 ≤ α）。判別可能な distinct 色をパーツ毎に
/// 割り当てる（`distinctive_opaque` は座標傾斜で位置ズレ検出用だが、8.2 は色で層を識別したいので
/// 単色を用いる）。
pub(super) fn solid_opaque(w: u32, h: u32, b: u8, g: u8, r: u8) -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = w * 4;
    let mut bgra = Vec::with_capacity((stride * h) as usize);
    for _ in 0..(w * h) {
        bgra.extend_from_slice(&[b, g, r, 255]);
    }
    (w, h, stride, bgra, true)
}

/// surface1000 の bind パーツ surface（1100 系）の element0 相対パスを解決するための対応表。
///
/// surfaces.txt の実データ（本テスト先頭コメント参照）に基づく (animation_id, part_element_rel)。
/// 各 bind animation `animationN.pattern0,overlay,N,0,0,0` は同番号のパーツ surface N を参照し、
/// その surface の element0 が下記 png を (0,0) 配置する（実 fixture より）。
/// 8.2 では判別のため、この rel パスへ**自作の単色画像**を MemoryDecoder へ挿入する。
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
        BindPart { anim_id: 1100, rel: "purple/0/base1.png", w: 80, h: 20, b: 255, g: 0, r: 0 },
        // 口（surface1200 → purple/2/a.png）: 小さい（30×30）・緑。
        BindPart { anim_id: 1200, rel: "purple/2/a.png", w: 30, h: 30, b: 0, g: 255, r: 0 },
        // 目（surface1302 → purple/4/normal.png）: 縦に長い（20×90）・赤。
        BindPart { anim_id: 1302, rel: "purple/4/normal.png", w: 20, h: 90, b: 0, g: 0, r: 255 },
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
        let (w, h, stride, bgra, has_alpha) = solid_opaque(p.w, p.h, p.b, p.g, p.r);
        dec.insert(base.join(p.rel), w, h, stride, bgra, has_alpha);
    }
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table
}

/// surface1000 を有効 bind 集合で合成した結果を得る（world 構築＋bind_atlas＋compose）。
pub(super) fn compose_surface1000(shell: &Shell, atlas: &AtlasTable, binds: &BindSet) -> ComposedSurface {
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
