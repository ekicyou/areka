//! emo2 fixture の pixel 観測 golden テスト（in-source `#[cfg(test)]`・`MemoryDecoder`+bake 経路）。
//!
//! 実上流（実行時エンジン）非依存に、fixture／正規化モデル直入力で合成結果を観測する。
//! 本モジュールは task 8.1（要件 **11.1**・**11.4**）を担う: emo2 fixture の surfaces.txt を
//! パースし、COM/WIC/表示に一切依存しない `MemoryDecoder`＋`bake` 経路で `AtlasTable` を構築、
//! `Composer::compose` でパイプライン全段（parse → fold → bake → bind → plan → blit）を駆動して
//! surface0（`element0,overlay,surface0.png,0,0` の単層 base surface）の合成結果が、`MemoryDecoder`
//! へ挿入した決定的な既知画像と**バイト等価**であることを検証する。
//!
//! ## なぜ単層 base surface が「挿入画像とバイト等価」になるか
//! surface0 は `element0` 一本のみを持つ base surface で、その element は原点 (0,0) の overlay。
//! 有効 bind 集合が空（surface0 は着せ替え bind を一切持たない）ゆえ、合成は「全透明キャンバスへ
//! element0 単層を (0,0) で SourceOver する」＝単なるコピーに帰着する。挿入画像を
//! **全不透明・透明マージン無し**にすることで α-bbox トリムが恒等（`trim_offset=(0,0)`・
//! `uv size == original`）となり、合成結果はキャンバス外形 == 画像外形の premultiplied BGRA
//! バッファそのものになる。よって `composed.bytes() == inserted_premultiplied_bytes`。
//!
//! ## 決定性
//! - 挿入画像は本テスト内で構成する固定パターンゆえ、実行環境に依らず不変。
//! - `bake`／トリム／packing は純粋な整数演算（`emo2_golden.rs` 参照）。
//! - COM/WIC/表示なし（`MemoryDecoder`+`bake` は CPU-only・要件 11.4）。

use std::path::{Path, PathBuf};

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_parsers::shell::Shell;

use crate::{BindSet, Composer, ComposedSurface, EmoWorld};

/// emo2 fixture 実資産のパスを組む。
/// `CARGO_MANIFEST_DIR` = `crates/areka-emo-compose`。fixtures はパイロット crate 配下。
fn emo2(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pilot/examples/shiori-host-32/fixtures/emo2")
        .join(rel)
}

/// emo2 の shell/master 基準 dir（surfaces.txt の element 相対パス起点）。
fn shell_master_dir() -> PathBuf {
    emo2("shell/master")
}

/// AlphaParams { use_self_alpha: On }（emo2 の descript は seriko.use_self_alpha,1）。
fn on_params() -> AlphaParams {
    AlphaParams {
        use_self_alpha: UseSelfAlpha::On,
    }
}

/// emo2 の surfaces.txt をパースして `Shell` を得る（`emo2_golden.rs` の loader 経路と同一）。
fn parse_emo2_shell() -> Shell {
    let content = std::fs::read_to_string(emo2("shell/master/surfaces.txt"))
        .expect("emo2 surfaces.txt must be readable (UTF-8)");
    let shell = areka_parsers::shell::parse(&content);
    assert!(
        !shell.surfaces.is_empty(),
        "parse produced surfaces from surfaces.txt"
    );
    shell
}

/// 決定的な既知画像を tightly-packed premultiplied BGRA で生成する。
///
/// 全画素 **不透明（α=255）**・透明マージン無しゆえ、α-bbox トリムは恒等
/// （`trim_offset=(0,0)`・`uv size == original`）になる。画素は行列インデックスから
/// 導く**判別可能な傾斜パターン**（B/G/R が座標で変化）で、誤った挿入画像なら
/// バイト等価が壊れる（golden の非空虚性）。`MemoryDecoder::insert` は premultiplied
/// BGRA を期待する（`DecodedImage` 契約）ため、α=255 の各色は既に premultiplied。
fn distinctive_opaque(w: u32, h: u32) -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = w * 4;
    let mut bgra = Vec::with_capacity((stride * h) as usize);
    for y in 0..h {
        for x in 0..w {
            // 座標から決定的に生成する傾斜（一様色でないので位置ズレも検出可能）。
            let b = ((x * 7 + y * 3) % 251) as u8;
            let g = ((x * 3 + y * 11) % 241) as u8;
            let r = ((x * 13 + y * 5) % 233) as u8;
            bgra.extend_from_slice(&[b, g, r, 255]);
        }
    }
    (w, h, stride, bgra, true)
}

/// emo2 surface0 の element0 パス（`surface0.png`・shell/master 相対）を、`base_dir.join(rel)` の
/// 実パスで `MemoryDecoder` へ挿入する。挿入画像スペック（w,h,premultiplied BGRA）も返す。
fn register_surface0_image(dec: &mut MemoryDecoder, base: &Path) -> (u32, u32, Vec<u8>) {
    // element0 の相対パスは surfaces.txt の `element0,overlay,surface0.png,0,0` に対応。
    let rel = "surface0.png";
    // emo2 の実 surface0.png 寸法に縛られない任意の決定的サイズ（合成は挿入画像で駆動される）。
    let (w, h, stride, bgra, has_alpha) = distinctive_opaque(37, 29);
    dec.insert(base.join(rel), w, h, stride, bgra.clone(), has_alpha);
    // stride==w*4（tightly-packed）を担保。
    assert_eq!(stride, w * 4);
    (w, h, bgra)
}

/// emo2 surface0 の element0 を含む `AtlasTable` を COM/WIC 非依存に構築する（要件 11.4）。
///
/// surfaces.txt 全体を `SurfaceSet` として bake するが、`MemoryDecoder` には surface0 の
/// element0 パスのみ既知画像を登録する。他 element は `NotFound` として `bake` の errors に
/// 記録されるだけで、surface0 の合成には影響しない（surface0 は element0 単層のみ）。
fn build_atlas_for_surface0(shell: &Shell, base: &Path) -> (AtlasTable, u32, u32, Vec<u8>) {
    let mut dec = MemoryDecoder::new();
    let (w, h, bgra) = register_surface0_image(&mut dec, base);

    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    let result = bake(std::slice::from_ref(&set), &dec, PackConfig::default());
    (result.table, w, h, bgra)
}

/// task 8.1 受入基準（要件 11.1・11.4）: emo2 surface0 の element0 単層合成が挿入画像とバイト等価。
///
/// 経路: surfaces.txt を parse → `EmoWorld::build`（fold）→ `MemoryDecoder`+`bake` で `AtlasTable`
/// 構築（COM/WIC/表示なし）→ `bind_atlas` → `Composer::compose(surface0, EMPTY BindSet)`。
/// surface0 は `element0,overlay,surface0.png,0,0` の単層 base surface で bind を持たないため、
/// 合成結果は全透明キャンバスへ element0 を (0,0) SourceOver した「コピー」＝挿入した premultiplied
/// BGRA 画像そのものになる。外形（w/h/stride）と全バイトの等価を検証する。
#[test]
fn surface0_element0_single_layer_equals_inserted_image() {
    let shell = parse_emo2_shell();

    // surface0 が「element0 単層・bind 無し」の base surface であることを実データで確認する。
    let s0 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 0)
        .expect("emo2 surfaces.txt に surface0 が存在する");
    assert_eq!(
        s0.elements.len(),
        1,
        "emo2 surface0 は element0 単層（`element0,overlay,surface0.png,0,0`）"
    );
    assert_eq!(
        s0.elements[0].path.as_str(),
        "surface0.png",
        "element0 の相対パスは surface0.png"
    );
    assert_eq!((s0.elements[0].x, s0.elements[0].y), (0, 0), "element0 は原点配置");
    assert!(
        s0.animations.is_empty(),
        "surface0 は bind/animation を持たない（純 base surface）"
    );

    let base = shell_master_dir();
    let (atlas, iw, ih, inserted) = build_atlas_for_surface0(&shell, &base);

    // fold → bind（構築時に一度きり resolve・以降 hot path は entry O(1)）。
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // 空 BindSet で surface0 を合成（着せ替え bind を一切適用しない単層経路）。
    let mut composer = Composer::new();
    let out: ComposedSurface = composer
        .compose(&world, &atlas, 0, &BindSet::default())
        .expect("surface0 の単層合成は Ok（要件 11.1）");

    // 外形一致: キャンバス外形 == 挿入画像外形（trim 恒等・原点配置ゆえ）。
    assert_eq!(out.width(), iw, "合成幅 == 挿入画像幅");
    assert_eq!(out.height(), ih, "合成高 == 挿入画像高");
    assert_eq!(out.stride(), iw * 4, "stride == w*4（premultiplied BGRA）");
    assert_eq!(
        out.bytes().len(),
        inserted.len(),
        "バッファ長 == 挿入画像バイト長"
    );

    // バイト等価（要件 11.1 の中核）: 単層 (0,0) SourceOver onto 透明キャンバス == コピー。
    assert_eq!(
        out.bytes(),
        inserted.as_slice(),
        "surface0 element0 単層の合成結果は挿入した premultiplied BGRA 画像とバイト等価（要件 11.1）"
    );
}

/// 非空虚性ガード: 「間違った挿入画像」なら byte-equality が壊れることを示す。
///
/// golden の assert が意味を持つこと（tautology でないこと）を、意図的に異なる画像を挿入した
/// 合成結果が上の期待画像とバイト不一致になることで実証する。
#[test]
fn surface0_golden_is_non_vacuous() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();

    // 正しい合成（distinctive_opaque(37,29) を挿入）。
    let (atlas_ok, _, _, correct) = build_atlas_for_surface0(&shell, &base);
    let mut world_ok = EmoWorld::build(&shell);
    world_ok.bind_atlas(&atlas_ok, SetId(0));
    let out_ok = Composer::new()
        .compose(&world_ok, &atlas_ok, 0, &BindSet::default())
        .expect("Ok");
    assert_eq!(out_ok.bytes(), correct.as_slice(), "正しい挿入画像とはバイト等価");

    // 異なる画像（別サイズ・別パターン）を挿入した場合の合成。
    let mut dec = MemoryDecoder::new();
    let (w2, h2, stride2, bgra2, has_alpha2) = distinctive_opaque(20, 24);
    dec.insert(base.join("surface0.png"), w2, h2, stride2, bgra2, has_alpha2);
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: &base,
        alpha_params: on_params(),
    };
    let atlas_other = bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table;
    let mut world_other = EmoWorld::build(&shell);
    world_other.bind_atlas(&atlas_other, SetId(0));
    let out_other = Composer::new()
        .compose(&world_other, &atlas_other, 0, &BindSet::default())
        .expect("Ok");

    // 異なる挿入画像 → 合成結果は「正しい期待画像」と一致しない（byte-equality が識別する）。
    assert_ne!(
        out_other.bytes(),
        correct.as_slice(),
        "異なる挿入画像は正しい期待画像とバイト不一致＝golden の assert は非空虚"
    );
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
fn solid_opaque(w: u32, h: u32, b: u8, g: u8, r: u8) -> (u32, u32, u32, Vec<u8>, bool) {
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
struct BindPart {
    /// surface1000 の bind animation id（= 参照先パーツ surface id と同番号）。
    anim_id: u32,
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
fn surface1000_bind_parts() -> [BindPart; 3] {
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
fn build_atlas_for_surface1000(shell: &Shell, base: &Path) -> AtlasTable {
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
fn compose_surface1000(shell: &Shell, atlas: &AtlasTable, binds: &BindSet) -> ComposedSurface {
    let mut world = EmoWorld::build(shell);
    world.bind_atlas(atlas, SetId(0));
    Composer::new()
        .compose(&world, atlas, 1000, binds)
        .expect("surface1000 の合成は Ok（surface 存在＋静的外形非ゼロ・要件 6.6）")
}

/// 合成結果の (x,y) 画素の α バイト（BGRA の index+3）を読む。
fn alpha_at(s: &ComposedSurface, x: u32, y: u32) -> u8 {
    let i = (y * s.stride() + x * 4 + 3) as usize;
    s.bytes()[i]
}

/// α>0 の画素数を数える（非空虚性・重なり量の要点指標）。
fn opaque_pixel_count(s: &ComposedSurface) -> usize {
    s.bytes().chunks_exact(4).filter(|px| px[3] > 0).count()
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
    assert!(out.width() > 0 && out.height() > 0, "外形は bind 非依存で非ゼロ（task 5.4）");
    assert_eq!(out.stride(), out.width() * 4, "premultiplied BGRA stride 契約");

    // 全画素 α==0（描画命令ゼロ＝全透明）。α バイトを走査して 1 つも α>0 が無いことを固定する。
    let any_opaque = out.bytes().chunks_exact(4).any(|px| px[3] > 0);
    assert!(!any_opaque, "空 BindSet では全画素が透明（α>0 の画素が存在しない・要件 5.4）");
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
    assert!(opaque > 0, "非空 BindSet では α>0 の画素が存在する（非空・要件 5.4）");
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
    assert!(n_three >= n_both, "3 bind の不透明画素数は 2 bind 以上（単調・要件 11.2）");
}
