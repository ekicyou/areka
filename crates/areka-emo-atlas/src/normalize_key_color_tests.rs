//! 抜き色（キーカラー透過）の単体テスト（spec: areka-P0-shell-implicit-surface 要件 4.1〜4.10・5.4・7.3）。
//!
//! α チャンネルを持たない絵は、左上の 1 画素（座標 0,0）と同じ 4 バイトの画素を
//! すべて完全に透明（`0,0,0,0`）にし、それ以外の画素を 1 バイトも変えない。
//! 画素を決め打ちした小さな絵だけで走り、ファイルにも検体にも依存しない。
//!
//! 離れた同じ色が抜かれること・α 付きが素通しなこと・`plan` が抜く色を持つ場合は、
//! `normalize_rule_tests.rs` の表の行（同じ入力と期待）が見るので、ここには置かない。

use super::*;
use crate::decode::DecodedImage;
use crate::test_support::{image, params};

/// 抜き色の腕（`On` ＋ α なし）を通す。
fn key_out(img: DecodedImage) -> NormalizedImage {
    Normalizer.normalize(img, params(UseSelfAlpha::On))
}

/// 要件 4.2: 許容幅 0。1 成分だけ 1 違う色は不透明のまま 1 バイトも変わらない。
#[test]
fn color_off_by_one_in_a_single_component_stays_untouched() {
    let key = [10u8, 20, 30, 255];
    // B・G・R のそれぞれを 1 だけずらした 3 色（いずれも抜かれてはならない）。
    let near_b = [11u8, 20, 30, 255];
    let near_g = [10u8, 21, 30, 255];
    let near_r = [10u8, 20, 31, 255];
    let out = key_out(image(2, 2, false, &[key, near_b, near_g, near_r]));

    assert_eq!(&out.pbgra[0..4], &[0, 0, 0, 0], "左上だけが透明になる");
    assert_eq!(&out.pbgra[4..8], &near_b, "B が 1 違う色は不透明のまま");
    assert_eq!(&out.pbgra[8..12], &near_g, "G が 1 違う色は不透明のまま");
    assert_eq!(&out.pbgra[12..16], &near_r, "R が 1 違う色は不透明のまま");
}

/// 要件 4.7: 全画素が左上と同じ色の絵は失敗にならず、全画素が透明になる。
#[test]
fn image_of_a_single_color_is_ok_and_fully_transparent() {
    let key = [7u8, 7, 7, 255];
    let out = key_out(image(3, 2, false, &[key; 6]));

    assert!(
        out.pbgra.iter().all(|&b| b == 0),
        "全画素が `0,0,0,0` になる（要件 4.7）"
    );
    assert_eq!(out.width, 3);
    assert_eq!(out.height, 2);
    assert_eq!(out.pbgra.len(), (out.stride * out.height) as usize);
}

/// 行の詰め物（`stride > width * 4`）を読まず、書きもしない。
#[test]
fn row_padding_is_neither_read_nor_written() {
    let key = [10u8, 20, 30, 255];
    let other = [200u8, 100, 50, 255];
    // 幅 2・高さ 2・stride 12（画素 8 バイト＋詰め物 4 バイト）。詰め物は抜き色と同じ並び。
    let stride = 12u32;
    let mut bgra = Vec::new();
    for row in [[key, other], [other, other]] {
        for px in row {
            bgra.extend_from_slice(&px);
        }
        bgra.extend_from_slice(&key); // 詰め物
    }
    let src = DecodedImage {
        width: 2,
        height: 2,
        stride,
        bgra,
        has_alpha: false,
    };

    let out = key_out(src);

    assert_eq!(&out.pbgra[0..4], &[0, 0, 0, 0], "左上は透明");
    assert_eq!(&out.pbgra[4..8], &other);
    assert_eq!(
        &out.pbgra[8..12],
        &key,
        "1 行目の詰め物は 1 バイトも変わらない"
    );
    assert_eq!(&out.pbgra[12..16], &other);
    assert_eq!(&out.pbgra[16..20], &other);
    assert_eq!(
        &out.pbgra[20..24],
        &key,
        "2 行目の詰め物は 1 バイトも変わらない"
    );
}

/// 要件 4.8: `tRNS` 相当（左上が完全に透明・他に半透明の画素）で、半透明はそのまま残る。
#[test]
fn trns_like_semi_transparent_pixels_survive() {
    // 読み手が α を持たないとした絵に α<255 の画素が混じって届いた場合の備え（パレットの番号
    // ごとの透明度を持つ絵は、emo-atlas-palette-alpha の後は `has_alpha` が真で、この腕へ来ない）。
    let transparent = [0u8, 0, 0, 0]; // 左上＝tRNS で完全に透明
    let semi = [64u8, 0, 0, 128]; // 半透明（乗算済み）
    let opaque = [200u8, 100, 50, 255];
    let out = key_out(image(
        2,
        2,
        false,
        &[transparent, semi, opaque, transparent],
    ));

    assert_eq!(&out.pbgra[0..4], &[0, 0, 0, 0], "左上はそのまま透明");
    assert_eq!(&out.pbgra[4..8], &semi, "半透明は生きる（要件 4.8）");
    assert_eq!(&out.pbgra[8..12], &opaque, "不透明はそのまま");
    assert_eq!(
        &out.pbgra[12..16],
        &[0, 0, 0, 0],
        "左上と同じ 4 バイトの画素は透明のまま"
    );
}

/// 幅か高さが 0 の絵は抜き色を持てないので、そのまま渡す（失敗にしない）。
#[test]
fn zero_sized_image_passes_through() {
    let src = image(0, 0, false, &[]);
    let out = key_out(src);
    assert!(out.pbgra.is_empty());
    assert_eq!(out.width, 0);
    assert_eq!(out.height, 0);
}
