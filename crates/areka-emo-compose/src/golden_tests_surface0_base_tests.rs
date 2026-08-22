//! task 8.1（要件 11.1・11.4）: emo2 surface0 の単層 base surface 合成が、`MemoryDecoder` へ
//! 挿入した既知画像とバイト等価であることを固定する golden テーマ。

use super::test_support::{on_params, parse_emo2_shell, shell_master_dir};
use super::{
    AtlasTable, BindSet, ComposedSurface, Composer, EmoWorld, MemoryDecoder, PackConfig, Path,
    PatternState, SetId, Shell, SurfaceSet, bake,
};

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
    assert_eq!(
        (s0.elements[0].x, s0.elements[0].y),
        (0, 0),
        "element0 は原点配置"
    );
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
        .compose(
            &world,
            &atlas,
            0,
            &BindSet::default(),
            &PatternState::default(),
        )
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
        .compose(
            &world_ok,
            &atlas_ok,
            0,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("Ok");
    assert_eq!(
        out_ok.bytes(),
        correct.as_slice(),
        "正しい挿入画像とはバイト等価"
    );

    // 異なる画像（別サイズ・別パターン）を挿入した場合の合成。
    let mut dec = MemoryDecoder::new();
    let (w2, h2, stride2, bgra2, has_alpha2) = distinctive_opaque(20, 24);
    dec.insert(
        base.join("surface0.png"),
        w2,
        h2,
        stride2,
        bgra2,
        has_alpha2,
    );
    let set = SurfaceSet {
        surfaces: &shell.surfaces,
        base_dir: &base,
        alpha_params: on_params(),
    };
    let atlas_other = bake(std::slice::from_ref(&set), &dec, PackConfig::default()).table;
    let mut world_other = EmoWorld::build(&shell);
    world_other.bind_atlas(&atlas_other, SetId(0));
    let out_other = Composer::new()
        .compose(
            &world_other,
            &atlas_other,
            0,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("Ok");

    // 異なる挿入画像 → 合成結果は「正しい期待画像」と一致しない（byte-equality が識別する）。
    assert_ne!(
        out_other.bytes(),
        correct.as_slice(),
        "異なる挿入画像は正しい期待画像とバイト不一致＝golden の assert は非空虚"
    );
}
