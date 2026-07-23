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

use crate::{BindSet, Composer, ComposedSurface, EmoWorld, PatternState};

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
        .compose(&world, &atlas, 0, &BindSet::default(), &PatternState::default())
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
        .compose(&world_ok, &atlas_ok, 0, &BindSet::default(), &PatternState::default())
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
        .compose(&world_other, &atlas_other, 0, &BindSet::default(), &PatternState::default())
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
        .compose(&world, atlas, 1000, binds, &PatternState::default())
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

// ============================================================================
// task 8.3: トリム等価の pixel テスト（要件 **11.3**・**6.2**）。
//
// bake は透明マージンを α-bbox トリムで切り落とし、原画像内の bbox 左上を
// `Placement.trim_offset`・原寸を `AtlasEntry.original` として記録する（`trim.rs`）。
// blit は転写先を `transform.offset() + placement.trim_offset` として算出する（`blit.rs`）。
// この 2 つが噛み合うと「不透明コアが原画像内で占めていた絶対位置」へコアが着弾し、
// トリムは合成結果を一切変えない＝**トリムありの合成 == トリムなし理論配置**（要件 6.2）。
//
// 本テストはこれを byte-equality で固定する（要件 11.3 の pixel テスト）:
//   - Scenario A（トリムあり）: 透明マージン付き 40×40 画像（不透明 10×10 コアが内側
//     オフセット (12,15)・残り α=0）を surface0.png として挿入 → bake がトリム
//     （uv=10×10・trim_offset=(12,15)・original=40×40）→ surface0 の element0（配置 (0,0)）
//     として合成する。
//   - Scenario B（トリムなし理論配置＝期待バッファ）: 原画像そのもの（40×40・コアは
//     (12,15) に premultiplied・残り 0）。element 配置 (0,0) ＋ trim_offset (12,15) = (12,15)
//     ＝原画像内でコアが占める位置ゆえ、透明マージンへの SourceOver は恒等で、合成結果は
//     **原画像バイトそのもの**になる。
//
// これは blit テスト④（`trim_offset_shifts_destination`）を fixture 駆動の full-pipeline
// 経路（parse→fold→bake トリム→bind→plan→blit）へ引き上げ、bake が現に記録する trim_offset を
// blit が現に再加算することを end-to-end で保証する（design「Integration Tests item 3」・
// 「BlitExecutor（転写先 = transform.offset()+trim_offset）」・Requirements Traceability 6.2/11.3）。
// すべて MemoryDecoder+bake の CPU-only 経路（COM/WIC/表示なし・要件 11.4）。
// ============================================================================

/// 透明マージン付き画像スペック（原寸・不透明コアのオフセット/寸法/色）。
///
/// 40×40 の全透明キャンバスに、内側 (INNER_X, INNER_Y) を左上とする CORE_W×CORE_H の
/// **全不透明（α=255）**単色コアを置く。全不透明ゆえ premultiplied 済み（色 ≤ α）で、
/// α=0 マージンは全バイト 0（premultiplied 透明）。この構成で bake は α-bbox トリムにより
/// マージンを落とし、trim_offset=(INNER_X, INNER_Y)・original=(ORIG_W, ORIG_H) を記録する。
const ORIG_W: u32 = 40;
const ORIG_H: u32 = 40;
const INNER_X: u32 = 12;
const INNER_Y: u32 = 15;
const CORE_W: u32 = 10;
const CORE_H: u32 = 10;
// コアの単色（判別可能・全不透明）。premultiplied（B,G,R ≤ A=255）。
const CORE_B: u8 = 30;
const CORE_G: u8 = 170;
const CORE_R: u8 = 220;

/// 透明マージン付き原画像を tightly-packed premultiplied BGRA で生成する。
///
/// 全画素をまず 0（透明）で埋め、内側 [INNER_X, INNER_X+CORE_W) × [INNER_Y, INNER_Y+CORE_H)
/// にだけ全不透明の単色コアを書く。返り値は Scenario B の「トリムなし理論配置＝期待バッファ」
/// そのものでもある（コアが原画像内で占める絶対位置に置かれた原画像）。
fn margined_core_image() -> (u32, u32, u32, Vec<u8>, bool) {
    let stride = ORIG_W * 4;
    let mut bgra = vec![0u8; (stride * ORIG_H) as usize];
    for y in INNER_Y..(INNER_Y + CORE_H) {
        for x in INNER_X..(INNER_X + CORE_W) {
            let off = (y * stride + x * 4) as usize;
            bgra[off] = CORE_B;
            bgra[off + 1] = CORE_G;
            bgra[off + 2] = CORE_R;
            bgra[off + 3] = 255;
        }
    }
    // has_alpha=true（α チャンネル採用腕＝use_self_alpha,On・normalize 恒等）。
    (ORIG_W, ORIG_H, stride, bgra, true)
}

/// margined_core_image を surface0.png として挿入した surface0 用 AtlasTable を構築する。
///
/// bake は α-bbox トリムでマージンを落とし、element0 エントリに
/// trim_offset=(INNER_X,INNER_Y)・original=(ORIG_W,ORIG_H)・uv=CORE_W×CORE_H を記録する。
/// 挿入した原画像バイト（Scenario B の期待バッファ）も併せて返す。
fn build_atlas_margined_surface0(shell: &Shell, base: &Path) -> (AtlasTable, Vec<u8>) {
    let mut dec = MemoryDecoder::new();
    let (w, h, stride, bgra, has_alpha) = margined_core_image();
    dec.insert(base.join("surface0.png"), w, h, stride, bgra.clone(), has_alpha);
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    let table = bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table;
    (table, bgra)
}

/// task 8.3（要件 11.3・6.2）: トリムありの合成結果 == トリムなし理論配置（原画像）とバイト等価。
///
/// surface0 の element0 は配置 (0,0)（実 fixture・既存テストで固定）。透明マージン付き 40×40 を
/// 挿入すると bake はトリム（trim_offset=(12,15)・original=40×40・uv=10×10）する。合成では
/// dest = 配置 (0,0) ＋ trim_offset (12,15) = (12,15) へ不透明コアが着弾し、その他は透明のまま。
/// これは原画像（コアが (12,15) に premultiplied・残り 0）とバイト単位で一致する。
/// あわせて外形が**原寸 40×40**（トリム後コア 10×10 ではない・task 5.4/要件 6.5）であることを固定する。
#[test]
fn surface0_trimmed_composite_equals_untrimmed_placement() {
    let shell = parse_emo2_shell();

    // surface0 が element0 単層・配置 (0,0)・bind 無しであることを実データで確認する（前提固定）。
    let s0 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 0)
        .expect("emo2 surfaces.txt に surface0 が存在する");
    assert_eq!(s0.elements.len(), 1, "surface0 は element0 単層");
    assert_eq!(
        (s0.elements[0].x, s0.elements[0].y),
        (0, 0),
        "element0 は原点配置（配置 (0,0)＋trim_offset で着弾位置が決まる）"
    );

    let base = shell_master_dir();
    let (atlas, original_bytes) = build_atlas_margined_surface0(&shell, &base);

    // bake が現に記録したトリム情報を診断的に固定する（trim_offset/original/uv が期待どおり）。
    // element0 は fold 後の登場順で ElementId(0)（surface0 単層・既存 8.1 harness と同一束縛）。
    let entry = atlas.entry(areka_emo_atlas::ElementId(0));
    assert_eq!(
        entry.original,
        areka_emo_atlas::Size { w: ORIG_W, h: ORIG_H },
        "original は原寸 40×40（トリム後コアではない・task 5.4 の外形母集合）"
    );
    let placement = entry
        .placement
        .as_ref()
        .expect("透明マージン付きでも不透明コアがあるので placement は Some");
    assert_eq!(
        placement.trim_offset,
        areka_emo_atlas::Point { x: INNER_X as i32, y: INNER_Y as i32 },
        "bake は原画像内 bbox 左上 (12,15) を trim_offset に記録"
    );
    assert_eq!(
        (placement.uv_rect.w, placement.uv_rect.h),
        (CORE_W, CORE_H),
        "uv はトリム後コア 10×10（マージンが落ちている＝現にトリムされた）"
    );

    // fold → bind → compose（surface0・空 BindSet）。
    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));
    let out = Composer::new()
        .compose(&world, &atlas, 0, &BindSet::default(), &PatternState::default())
        .expect("surface0 の単層合成は Ok（要件 11.1）");

    // 外形は原寸 40×40（有効 bind 非依存の静的外形＝原点+original・トリム後コアではない）。
    assert_eq!(out.width(), ORIG_W, "合成幅 == 原寸 40（トリム後 10 ではない・要件 6.5）");
    assert_eq!(out.height(), ORIG_H, "合成高 == 原寸 40（トリム後 10 ではない・要件 6.5）");
    assert_eq!(out.stride(), ORIG_W * 4, "stride == 原寸*4");

    // トリム等価の中核（要件 6.2/11.3）: トリムありの合成結果は原画像（トリムなし理論配置）と
    // バイト単位で一致する。配置 (0,0)＋trim_offset (12,15) がコアを原画像内の位置へ戻すため、
    // トリムは合成結果を一切変えない。
    assert_eq!(
        out.bytes(),
        original_bytes.as_slice(),
        "トリムありの合成結果 == トリムなし理論配置（原画像）とバイト等価（要件 6.2/11.3）"
    );

    // 着弾位置の要点確認: コア左上 (12,15) は不透明・素の配置 (0,0) は透明。
    let core_tl = ((INNER_Y * out.stride() + INNER_X * 4) as usize, [CORE_B, CORE_G, CORE_R, 255u8]);
    assert_eq!(
        &out.bytes()[core_tl.0..core_tl.0 + 4],
        &core_tl.1,
        "配置+trim=(12,15) に不透明コアが着弾"
    );
    assert_eq!(
        &out.bytes()[0..4],
        &[0, 0, 0, 0],
        "素の配置 (0,0)（trim を足さない位置）は透明のまま（トリムが見た目を変えない）"
    );
}

/// 非空虚性ガード（要件 6.2/11.3）: 「trim_offset を足さない」誤配置は合成結果と一致しない。
///
/// byte-equality が tautology でないことを示すため、コアを **trim_offset を加算せず**素の配置
/// (0,0) に置いた「誤った期待バッファ」を手組みし、それが実際の合成結果と**不一致**になることを
/// 固定する。もし blit が trim_offset を再加算していなければ（要件 6.2 のバグ）、コアは (0,0) に
/// 着弾しこの誤バッファと一致してしまう——本 assert_ne! はその退行を検出する。
#[test]
fn surface0_trim_equivalence_is_non_vacuous() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let (atlas, _original) = build_atlas_margined_surface0(&shell, &base);

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));
    let out = Composer::new()
        .compose(&world, &atlas, 0, &BindSet::default(), &PatternState::default())
        .expect("Ok");

    // 誤った期待: コアを trim_offset を足さず原点 (0,0) に置いた 40×40 バッファ。
    // 実際の合成は配置+trim=(12,15) にコアを置くので、両者は一致してはならない。
    let stride = ORIG_W * 4;
    let mut wrong = vec![0u8; (stride * ORIG_H) as usize];
    for y in 0..CORE_H {
        for x in 0..CORE_W {
            let off = (y * stride + x * 4) as usize;
            wrong[off] = CORE_B;
            wrong[off + 1] = CORE_G;
            wrong[off + 2] = CORE_R;
            wrong[off + 3] = 255;
        }
    }

    assert_ne!(
        out.bytes(),
        wrong.as_slice(),
        "trim_offset を足さない誤配置は合成結果と不一致＝byte-equality は非空虚（blit が現に trim_offset を再加算・要件 6.2）"
    );

    // 誤バッファでコアが在る (0,0) は、実合成では透明（trim で (12,15) へ移動済み）＝差の直接証拠。
    assert_eq!(&wrong[0..4], &[CORE_B, CORE_G, CORE_R, 255], "誤バッファは (0,0) にコア");
    assert_eq!(&out.bytes()[0..4], &[0, 0, 0, 0], "実合成の (0,0) は透明（差が生じる画素）");
}

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

// ============================================================================
// task 11.2: emo2 まばたき bind（1400/1403・pattern0 なし）の静的不活性回帰檻
// （要件 **9.1**・**9.5**・11.2）。
//
// 実機サインオフ#2: むらさきの目が常時閉じている。真因＝`flatten_surface` の pattern 選択が
// 最小 index フォールバック（min_by_key）だったため、pattern0（index==0）を持たない まばたき
// animation 1400（`interval,bind+random`・pattern1/2/3）と 1403（`interval,bind`・pattern2）の
// 閉じ目フレーム（surface 1412/1414）が静的土台に積まれ、ベースの目（1302）を覆っていた。実 pasta は
// まばたきを on-repeat で bind するため BindSet に残る。canon では pattern0 を持たない bind
// animation は seriko-loop（M-life）が再生する再生専用フレームで、静的土台には寄与しない。
//
// 本檻: emo2 surface1000 を [1302]（目/通常・pattern0 あり）と [1302,1400,1403]（＋まばたき）で
// build_plan し、生成描画命令列（BlitOp）が**同一**であることを固定する。まばたき bind を足しても
// 静的合成が一切変わらない＝「常時閉じ目」退行を直接ガードする。
//
// 非空虚性: 閉じ目パーツ（1412=eyebase+toji・1414=null）の画像を atlas へ挿入するため、旧
// min_by_key 実装なら 1400→surface1412・1403→surface1414 が composed され命令列が伸びて assert が
// 破れる。修正後（find(index==0)）はこれらが skip され命令列が [1302] と一致する。
// ============================================================================

/// まばたき檻用の `AtlasTable` を構築する（COM/WIC 非依存・要件 11.4）。
///
/// [`build_atlas_for_surface1000`] と異なり、目/通常（1302→purple/4/normal.png）に加えて **閉じ目
/// パーツ**の画像（surface1412 の eyebase/toji・surface1414 の null）も MemoryDecoder へ挿入する。
/// これにより旧 min_by_key 実装なら 1400/1403 の閉じ目フレームが解決され命令が生じる（＝非空虚）。
fn build_atlas_for_blink_cage(shell: &Shell, base: &Path) -> AtlasTable {
    let mut dec = MemoryDecoder::new();
    // 目/通常（1302 = surface1302 → purple/4/normal.png・pattern0 あり）。
    let (w, h, s, b, a) = solid_opaque(20, 90, 0, 0, 255);
    dec.insert(base.join("purple/4/normal.png"), w, h, s, b, a);
    // 閉じ目パーツ（1400→surface1412=eyebase+toji、1403→surface1414=null）を解決可能にする。
    // これらが解決できることで、旧 min_by_key 実装なら閉じ目が composed され命令列が伸びる（非空虚）。
    for rel in ["purple/a/eyebase.png", "purple/4/toji.png", "purple/a/null.png"] {
        let (w, h, s, b, a) = solid_opaque(20, 90, 128, 128, 128);
        dec.insert(base.join(rel), w, h, s, b, a);
    }
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: base,
        alpha_params: on_params(),
    };
    bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table
}

/// task 11.2（要件 9.1・9.5・11.2）: emo2 まばたき bind（1400/1403・pattern0 なし）は静的合成へ
/// 寄与しない＝BindSet に足しても BlitOp 列が [1302] のときと**同一**。
///
/// 「むらさきの目が常時閉じている」実機第2欠陥の直接回帰檻。まばたき（閉じ目）フレームが静的
/// 土台へ積まれない（pattern0 を持たないゆえ）ことを命令列の同一性で固定する。
#[test]
fn emo2_blink_binds_are_statically_inactive() {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_blink_cage(&shell, &base);

    let mut world = EmoWorld::build(&shell);
    world.bind_atlas(&atlas, SetId(0));

    // [1302] のみ（目/通常・pattern0 あり）。
    let mut ops_base = Vec::new();
    let mut vis_base = Vec::new();
    build_plan(&mut ops_base, &mut vis_base, &world, &atlas, 1000, &BindSet::from_ids([1302]), &PatternState::default())
        .expect("目/通常のみでも surface1000 存在＋外形非ゼロで Ok");

    // [1302,1400,1403]（＋まばたき・pattern0 なし）。
    let mut ops_blink = Vec::new();
    let mut vis_blink = Vec::new();
    build_plan(
        &mut ops_blink,
        &mut vis_blink,
        &world,
        &atlas,
        1000,
        &BindSet::from_ids([1302, 1400, 1403]),
        &PatternState::default(),
    )
    .expect("まばたき込みでも Ok");

    // まばたき bind（1400/1403・pattern0 なし）は静的合成へ寄与しない＝BlitOp 列が完全一致。
    assert_eq!(
        ops_base, ops_blink,
        "まばたき bind を足しても BlitOp 列は不変（pattern0 なしは静的不活性・要件 9.1/9.5）",
    );
    // 非空虚: 目/通常（1302）は pattern0 ありゆえ現に 1 命令を生む（空同士の空虚一致でない）。
    assert_eq!(ops_base.len(), 1, "目/通常 1302 は pattern0 ありで 1 命令（檻は空虚でない）");
}

/// まばたき檻の非空虚性の実データ確認: 1400/1403 が pattern0（index==0）を持たず、閉じ目パーツ
/// 画像が atlas に解決される（旧実装なら composed されうる）ことを固定する。
///
/// これにより上檻 [`emo2_blink_binds_are_statically_inactive`] の assert_eq が「まばたきが元々
/// 何も生まないから一致」という空虚一致でないこと（旧 min_by_key なら閉じ目が composed され破れる）
/// を実データで裏づける。
#[test]
fn emo2_blink_binds_lack_pattern0_but_frames_are_resolvable() {
    use areka_parsers::shell::Interval;

    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let atlas = build_atlas_for_blink_cage(&shell, &base);

    let s1000 = shell
        .surfaces
        .iter()
        .find(|s| s.id == 1000)
        .expect("emo2 surfaces.txt に surface1000 が存在する");

    // 1400（bind+random）と 1403（bind）は pattern0（index==0）を持たない再生アニメである。
    for (aid, closed_frame_surface) in [(1400u32, 1412i64), (1403, 1414)] {
        let anim = s1000
            .animations
            .iter()
            .find(|a| a.id == aid)
            .unwrap_or_else(|| panic!("surface1000 に animation{aid} が存在する"));
        assert!(
            matches!(anim.interval, Interval::Bind | Interval::BindRandom { .. }),
            "animation{aid} は bind 種 interval（BindSet に載る）",
        );
        assert!(
            !anim.patterns.iter().any(|p| p.index == 0),
            "animation{aid} は pattern0（index==0）を持たない（まばたき再生アニメ）",
        );
        // 旧 min_by_key が採るであろう最小 index pattern の参照先（閉じ目 surface）が存在する。
        let min_pat = anim
            .patterns
            .iter()
            .min_by_key(|p| p.index)
            .expect("pattern を 1 本以上持つ");
        assert_eq!(
            min_pat.surface_id, closed_frame_surface,
            "animation{aid} の最小 index pattern は閉じ目 surface{closed_frame_surface} を参照（旧実装ならこれを合成）",
        );
    }

    // 閉じ目パーツ surface（1412=eyebase+toji）の element 画像が atlas に解決される（placement Some）。
    // ＝旧実装なら実際に composed され得た＝上檻の assert_eq は非空虚。
    for rel in ["purple/a/eyebase.png", "purple/4/toji.png", "purple/a/null.png"] {
        let id = atlas
            .resolve(SetId(0), rel)
            .unwrap_or_else(|| panic!("閉じ目パーツ {rel} が atlas に解決される"));
        assert!(
            atlas.entry(id).placement.is_some(),
            "{rel} は不透明コアあり＝placement Some（旧実装なら composed され得た）",
        );
    }
}
