//! 宣言 4 通り × 絵の場合から画素の扱いを決める表（`Normalizer::plan`）と、その扱いを
//! 画素に当てる処理（`apply`）の単体テスト（spec: areka-P0-self-alpha-declaration
//! 要件 3・4・5・8.2・9）。表の 7 行を 1 行 1 テストで固定する。
//! 画素を決め打ちした小さな絵だけで走り、ファイルにも検体にも依存しない。

use super::*;
use crate::decode::DecodedImage;

/// 画素の並び（4 バイト単位）と寸法から `DecodedImage` を組む。`stride` は `width * 4`。
fn image(width: u32, height: u32, has_alpha: bool, pixels: &[[u8; 4]]) -> DecodedImage {
    assert_eq!(
        pixels.len(),
        (width * height) as usize,
        "画素数と寸法が合わない"
    );
    DecodedImage {
        width,
        height,
        stride: width * 4,
        bgra: pixels.iter().flat_map(|px| px.iter().copied()).collect(),
        has_alpha,
    }
}

fn params(use_self_alpha: UseSelfAlpha) -> AlphaParams {
    AlphaParams { use_self_alpha }
}

/// `plan` で決めて `apply` で当てた画素と、決めた扱いを返す。
fn plan_and_apply(img: &DecodedImage, decl: UseSelfAlpha) -> (AlphaRule, Vec<u8>) {
    let rule = Normalizer::plan(img, params(decl));
    let mut bgra = img.bgra.clone();
    apply(&mut bgra, img.width, img.height, img.stride, rule);
    (rule, bgra)
}

/// 乗算済みの α あり 2×2: 半透明・不透明・完全な透明を混ぜる。
const PREMUL_RED_128: [u8; 4] = [0, 0, 128, 128];
const PREMUL_GREEN_64: [u8; 4] = [0, 64, 0, 64];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// 全画素不透明の 2×2。左上と右下が同じ地色。
const KEY: [u8; 4] = [10, 20, 30, 255];
const OTHER: [u8; 4] = [200, 100, 50, 255];

/// 表の 1 行目（要件 3.1・3.3）: `On` × α あり → 扱いなし・画素は 1 バイトも変わらない。
/// `normalize` を通しても、今までの期待（入力のバイトそのまま）と一致する。
#[test]
fn row1_on_with_alpha_keeps_pixels_as_today() {
    let img = image(2, 2, true, &[PREMUL_RED_128, PREMUL_GREEN_64, WHITE, CLEAR]);
    let (rule, out) = plan_and_apply(&img, UseSelfAlpha::On);

    assert_eq!(
        rule,
        AlphaRule {
            opaque: false,
            key: None
        }
    );
    let expected: Vec<u8> = [PREMUL_RED_128, PREMUL_GREEN_64, WHITE, CLEAR].concat();
    assert_eq!(out, expected, "入力と出力のバイトが同じ");
    let normalized = Normalizer.normalize(img, params(UseSelfAlpha::On));
    assert_eq!(normalized.pbgra, expected, "正規化の出力も今の期待と同じ");
}

/// 表の 2 行目（要件 3.2・3.3）: `On` × α なし → 左上の 4 バイトを抜く。
/// `normalize` を通しても、今までの抜き色の期待とバイト単位で一致する。
#[test]
fn row2_on_without_alpha_clears_top_left_color_as_today() {
    let img = image(2, 2, false, &[KEY, OTHER, OTHER, KEY]);
    let (rule, out) = plan_and_apply(&img, UseSelfAlpha::On);

    assert_eq!(
        rule,
        AlphaRule {
            opaque: false,
            key: Some(KEY)
        }
    );
    let expected: Vec<u8> = [CLEAR, OTHER, OTHER, CLEAR].concat();
    assert_eq!(out, expected, "左上と同じ色だけが 0,0,0,0");
    let normalized = Normalizer.normalize(img, params(UseSelfAlpha::On));
    assert_eq!(normalized.pbgra, expected, "正規化の出力も今の期待と同じ");
}

/// 表の 3 行目（要件 4.1）: `Full` × α あり → α をそのまま使う。
#[test]
fn row3_full_with_alpha_keeps_pixels() {
    let img = image(2, 2, true, &[PREMUL_RED_128, PREMUL_GREEN_64, WHITE, CLEAR]);
    let (rule, out) = plan_and_apply(&img, UseSelfAlpha::Full);

    assert_eq!(
        rule,
        AlphaRule {
            opaque: false,
            key: None
        }
    );
    assert_eq!(out, img.bgra);
}

/// 表の 4 行目（要件 4.2・4.5）: `Full` × α なし → 全画素 α=255。左上と同じ色も抜かれない。
#[test]
fn row4_full_without_alpha_is_fully_opaque_and_keeps_top_left_color() {
    let img = image(2, 2, false, &[KEY, OTHER, OTHER, KEY]);
    let (rule, out) = plan_and_apply(&img, UseSelfAlpha::Full);

    assert_eq!(
        rule,
        AlphaRule {
            opaque: true,
            key: None
        }
    );
    assert_eq!(out, img.bgra, "左上と同じ色の画素も α=255 のまま残る");
}

/// 表の 4 行目の備え（要件 4.2）: α の印が偽なのに α<255 の画素を持つ絵（パレットの PNG の
/// 透明の情報など）も、`Full` では全画素 α=255 になる。色のバイトは触らない。
#[test]
fn row4_full_without_alpha_flag_forces_translucent_pixels_opaque() {
    let img = image(
        2,
        2,
        false,
        &[PREMUL_RED_128, PREMUL_GREEN_64, WHITE, CLEAR],
    );
    let (_, out) = plan_and_apply(&img, UseSelfAlpha::Full);

    let expected: Vec<u8> = [[0, 0, 128, 255], [0, 64, 0, 255], WHITE, [0, 0, 0, 255]].concat();
    assert_eq!(out, expected);
}

/// 表の 5 行目の α なし（要件 5.1・5.4）: `0` × α なし → 左上と同じ色を抜く。色が 1 違う画素は抜かれない。
#[test]
fn row5_off_without_alpha_clears_only_exact_top_left_color() {
    let near = [11u8, 20, 30, 255];
    let img = image(2, 2, false, &[KEY, near, OTHER, KEY]);
    let (rule, out) = plan_and_apply(&img, UseSelfAlpha::Off);

    assert_eq!(
        rule,
        AlphaRule {
            opaque: true,
            key: Some(KEY)
        }
    );
    let expected: Vec<u8> = [CLEAR, near, OTHER, CLEAR].concat();
    assert_eq!(out, expected, "B が 1 違う画素は抜かれない（許容幅なし）");
}

/// 表の 5 行目の α あり（要件 5.2・5.3・5.4・5.6・5.8）: `0` × α あり → α を捨てて全画素を
/// 不透明にし、その後の左上と 4 バイトとも同じ画素だけを抜く。α は 255 か 0 だけ。
#[test]
fn row5_off_with_alpha_drops_alpha_then_clears_top_left_color() {
    // 左上は半透明の赤（乗算済み 0,0,128,α=128）→ α を捨てると 0,0,128,255。
    // 2 つ目は同じ色で完全に透明だった画素 → 0,0,128,255 になり抜かれる。
    // 3 つ目は R が 1 違う画素 → 不透明のまま残る。4 つ目は完全な透明 → 0,0,0,255 で残る。
    let same_color_clear = [0u8, 0, 128, 0];
    let near = [0u8, 0, 129, 128];
    let img = image(2, 2, true, &[PREMUL_RED_128, same_color_clear, near, CLEAR]);
    let (rule, out) = plan_and_apply(&img, UseSelfAlpha::Off);

    assert_eq!(
        rule,
        AlphaRule {
            opaque: true,
            key: Some([0, 0, 128, 255])
        }
    );
    let expected: Vec<u8> = [CLEAR, CLEAR, [0, 0, 129, 255], [0, 0, 0, 255]].concat();
    assert_eq!(out, expected);
    for px in out.as_chunks::<4>().0 {
        assert!(px[3] == 255 || px[3] == 0, "半透明の画素が残った: {px:?}");
    }
}

/// 表の 6 行目（要件 9.1）: 宣言なし × α<255 の画素が在る → α をそのまま使う。
/// `has_alpha` が偽でも、届いた画素に透明が在れば α の側になる（印を見ない）。
#[test]
fn row6_undeclared_with_translucent_pixel_keeps_pixels_ignoring_alpha_flag() {
    for has_alpha in [true, false] {
        let img = image(2, 2, has_alpha, &[KEY, OTHER, OTHER, PREMUL_GREEN_64]);
        let (rule, out) = plan_and_apply(&img, UseSelfAlpha::Undeclared);

        assert_eq!(
            rule,
            AlphaRule {
                opaque: false,
                key: None
            },
            "has_alpha={has_alpha}"
        );
        assert_eq!(out, img.bgra, "has_alpha={has_alpha}");
    }
}

/// 表の 7 行目（要件 9.2・9.3・9.5・8.2）: 宣言なし × 全画素不透明 → 左上の色を抜く。
/// α の印が真でも全画素不透明なら抜き色の側（9.2）。α なしの絵の扱いは `On` × α なしと等しい。
#[test]
fn row7_undeclared_without_translucent_pixel_matches_on_without_alpha() {
    for has_alpha in [true, false] {
        let img = image(2, 2, has_alpha, &[KEY, OTHER, OTHER, KEY]);
        let (rule, out) = plan_and_apply(&img, UseSelfAlpha::Undeclared);

        assert_eq!(
            rule,
            AlphaRule {
                opaque: false,
                key: Some(KEY)
            },
            "has_alpha={has_alpha}"
        );
        assert_eq!(
            out,
            [CLEAR, OTHER, OTHER, CLEAR].concat(),
            "has_alpha={has_alpha}"
        );
    }
    let no_alpha = image(2, 2, false, &[KEY, OTHER, OTHER, KEY]);
    assert_eq!(
        Normalizer::plan(&no_alpha, params(UseSelfAlpha::Undeclared)),
        Normalizer::plan(&no_alpha, params(UseSelfAlpha::On)),
        "宣言なし × α なしは 1 × α なしと同じ扱い（8.2・9.5）"
    );
}

/// 宣言なしの走査は行の詰め物を読まない: 詰め物に α<255 のバイトが在っても抜き色の側。
/// `apply` も詰め物を 1 バイトも変えない（`opaque` と `key` の両方で）。
#[test]
fn padding_is_neither_scanned_nor_written() {
    // 1×2・stride 8（各行 4 バイトの詰め物）。詰め物は 0,0,0,0 で「透明」に見える。
    let pad = [9u8, 9, 9, 0];
    let bgra: Vec<u8> = [KEY, pad, OTHER, pad].concat();
    let img = DecodedImage {
        width: 1,
        height: 2,
        stride: 8,
        bgra,
        has_alpha: false,
    };

    let (rule, out) = plan_and_apply(&img, UseSelfAlpha::Undeclared);
    assert_eq!(
        rule,
        AlphaRule {
            opaque: false,
            key: Some(KEY)
        },
        "詰め物は走査しない"
    );
    assert_eq!(out, [CLEAR, pad, OTHER, pad].concat());

    let (_, out) = plan_and_apply(&img, UseSelfAlpha::Off);
    assert_eq!(
        out,
        [CLEAR, pad, OTHER, pad].concat(),
        "詰め物の α は書かない"
    );
}

/// 幅か高さが 0 の絵は `key` を持たない（今と同じ）。
#[test]
fn empty_image_has_no_key() {
    let img = DecodedImage {
        width: 0,
        height: 0,
        stride: 0,
        bgra: Vec::new(),
        has_alpha: false,
    };
    for decl in [
        UseSelfAlpha::On,
        UseSelfAlpha::Off,
        UseSelfAlpha::Undeclared,
    ] {
        assert_eq!(Normalizer::plan(&img, params(decl)).key, None, "{decl:?}");
    }
}
