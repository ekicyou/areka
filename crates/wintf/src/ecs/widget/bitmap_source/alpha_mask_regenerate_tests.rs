//! `AlphaMask::regenerate_from_pbgra32` の等価檻とバッファ再利用檻
//!
//! design.md `#### AlphaMask 再生成・共有（wintf・additive）` ／ `## Testing Strategy` Unit Tests 4
//! （要件 3.1・6.2・6.3）。
//!
//! 固定する主張は 2 つ:
//! 1. **等価**: `regenerate_from_pbgra32` は同一入力に対して `from_pbgra32` と完全に同一の
//!    マスク（`PartialEq`＝data／width／height の全一致）を作る。**寸法が縮む方向**でも前の
//!    内容が 1 ビットも残らない。
//! 2. **再利用**: 同寸または縮小方向の再生成でバッファを作り直さない（容量が減らず、確保
//!    位置も動かない）。task 4.1 の教訓に従い、**縮小方向の節**を必ず含める——同寸の反復
//!    だけでは毎回新規確保する実装が同じ容量に着地して素通りするため。
//!
//! 実時間は合否条件に使わない（要件 6.2）。純 x64・常設 `cargo test`・env ゲートなし・
//! `#[ignore]` なし（要件 6.3）。GPU は一切使わない（`AlphaMask` は純粋な CPU 構造）。

use super::*;

/// 決定論的な PBGRA32 画素列を作る。
///
/// α は `(x * 7 + y * 13) % 5 < 2` で 255／0 に振り分ける（透明・不透明が必ず混在し、
/// バイト境界（x=7,8）を跨いだ配置も再現される）。`stride` は行頭の起点にのみ使い、
/// 行末のパディング領域は 0 のまま残す。
fn deterministic_pixels(width: u32, height: u32, stride: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (stride * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let offset = (y * stride) as usize + (x as usize * 4);
            let opaque = (x * 7 + y * 13) % 5 < 2;
            pixels[offset + 3] = if opaque { 255 } else { 0 };
        }
    }
    pixels
}

/// 全画素が不透明な PBGRA32 画素列（縮小時の残留を最も検出しやすい前状態）
fn all_opaque_pixels(width: u32, height: u32, stride: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (stride * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let offset = (y * stride) as usize + (x as usize * 4);
            pixels[offset + 3] = 255;
        }
    }
    pixels
}

/// 全画素が透明な PBGRA32 画素列（α は全て 0 なので幅に依存しない）
fn all_transparent_pixels(height: u32, stride: u32) -> Vec<u8> {
    vec![0u8; (stride * height) as usize]
}

/// 検証に使う形状の一覧（幅はバイト境界・非バイト境界の両方を含む）
///
/// `(width, height, stride)`。stride は width*4（密）と、行末パディング付きの両方。
const SHAPES: &[(u32, u32, u32)] = &[
    (1, 1, 4),
    (2, 2, 8),
    (7, 3, 28),
    (8, 3, 32),
    (9, 3, 36),
    (16, 1, 64),
    (17, 5, 68),
    (32, 32, 128),
    (2, 2, 12),  // 行末パディング付き stride
    (9, 4, 44),  // 非バイト境界幅 + パディング
    (0, 0, 0),   // 退化ケース（空マスク）
    (4, 0, 16),  // height=0
];

/// マスクの全内容が一致することを assert する（`PartialEq` に加えて全座標の判定も突合）
fn assert_masks_identical(actual: &AlphaMask, expected: &AlphaMask, ctx: &str) {
    assert_eq!(actual, expected, "{ctx}: マスク全体（data/width/height）が不一致");
    assert_eq!(actual.width(), expected.width(), "{ctx}: width 不一致");
    assert_eq!(actual.height(), expected.height(), "{ctx}: height 不一致");
    // 全有効座標＋範囲外 1 歩ずつで判定が一致すること（内容の突合を座標側からも固定）
    for y in 0..=expected.height() {
        for x in 0..=expected.width() {
            assert_eq!(
                actual.is_hit(x, y),
                expected.is_hit(x, y),
                "{ctx}: is_hit({x},{y}) が不一致"
            );
        }
    }
}

// ============================================================================
// 1. 等価檻: regenerate_from_pbgra32 ≡ from_pbgra32
// ============================================================================

/// 空マスクからの再生成が `from_pbgra32` と等価（全形状）
#[test]
fn regenerate_from_empty_equals_from_pbgra32_for_all_shapes() {
    for &(w, h, stride) in SHAPES {
        let pixels = deterministic_pixels(w, h, stride);
        let expected = AlphaMask::from_pbgra32(&pixels, w, h, stride);

        let mut actual = AlphaMask::from_pbgra32(&[], 0, 0, 0);
        actual.regenerate_from_pbgra32(&pixels, w, h, stride);

        assert_masks_identical(&actual, &expected, &format!("空→{w}x{h} stride={stride}"));
    }
}

/// **縮小方向**の再生成で前の内容が残らない（全不透明 → より小さい全透明）
///
/// これが本タスクが名指しする失敗形。clear を落とすと保持されたバイト列の前半が
/// 生き残り、`from_pbgra32` と一致しなくなる。
#[test]
fn regenerate_shrink_leaves_no_stale_content() {
    // 前状態: 32x32 全不透明（data は 4*32 = 128 バイトが全て 0xFF）
    let big = all_opaque_pixels(32, 32, 128);
    let mut mask = AlphaMask::from_pbgra32(&big, 32, 32, 128);
    assert!(mask.is_hit(31, 31), "前提: 前状態は全不透明");

    // 縮小: 8x8 全透明
    let small = all_transparent_pixels(8, 32);
    let expected = AlphaMask::from_pbgra32(&small, 8, 8, 32);
    mask.regenerate_from_pbgra32(&small, 8, 8, 32);

    assert_masks_identical(&mask, &expected, "32x32(全不透明) → 8x8(全透明)");
    // 非空虚性ガード: 縮小後は 1 画素もヒットしない（残留があればここで必ず割れる）
    for y in 0..8 {
        for x in 0..8 {
            assert!(!mask.is_hit(x, y), "縮小後に残留ヒット ({x},{y})");
        }
    }
}

/// 縮小方向の再生成が混在パターンでも `from_pbgra32` と等価
#[test]
fn regenerate_shrink_with_mixed_pattern_equals_fresh() {
    let big = all_opaque_pixels(17, 9, 68);
    let mut mask = AlphaMask::from_pbgra32(&big, 17, 9, 68);

    let small = deterministic_pixels(9, 3, 36);
    let expected = AlphaMask::from_pbgra32(&small, 9, 3, 36);
    mask.regenerate_from_pbgra32(&small, 9, 3, 36);

    assert_masks_identical(&mask, &expected, "17x9 → 9x3(混在)");
}

/// 拡大方向の再生成が `from_pbgra32` と等価
#[test]
fn regenerate_grow_equals_fresh() {
    let small = all_opaque_pixels(4, 4, 16);
    let mut mask = AlphaMask::from_pbgra32(&small, 4, 4, 16);

    let big = deterministic_pixels(33, 17, 132);
    let expected = AlphaMask::from_pbgra32(&big, 33, 17, 132);
    mask.regenerate_from_pbgra32(&big, 33, 17, 132);

    assert_masks_identical(&mask, &expected, "4x4 → 33x17");
}

/// 形状を次々に切り替えても、各段で常に `from_pbgra32` と等価であり続ける
///
/// 拡大・縮小・退化（0 外形）・同寸の反復を 1 本の系列に通し、前段の内容が後段へ
/// 漏れないことを全段で固定する。
#[test]
fn regenerate_sequence_stays_equivalent_at_every_step() {
    let mut mask = AlphaMask::from_pbgra32(&[], 0, 0, 0);

    // 大 → 小 → 0 → 中 → 同寸反復 の順で全形状を巡回（縮小が必ず含まれる並び）
    let sequence: &[(u32, u32, u32)] = &[
        (32, 32, 128),
        (9, 3, 36),
        (0, 0, 0),
        (17, 5, 68),
        (17, 5, 68),
        (1, 1, 4),
        (16, 1, 64),
        (2, 2, 12),
        (4, 0, 16),
        (8, 3, 32),
    ];

    for &(w, h, stride) in sequence {
        let pixels = deterministic_pixels(w, h, stride);
        let expected = AlphaMask::from_pbgra32(&pixels, w, h, stride);
        mask.regenerate_from_pbgra32(&pixels, w, h, stride);
        assert_masks_identical(&mask, &expected, &format!("系列 {w}x{h} stride={stride}"));
    }
}

/// 全不透明の前状態から同寸の全透明へ再生成しても残留しない（同寸でも clear が要る）
#[test]
fn regenerate_same_size_clears_previous_content() {
    let opaque = all_opaque_pixels(9, 4, 36);
    let mut mask = AlphaMask::from_pbgra32(&opaque, 9, 4, 36);

    let transparent = all_transparent_pixels(4, 36);
    let expected = AlphaMask::from_pbgra32(&transparent, 9, 4, 36);
    mask.regenerate_from_pbgra32(&transparent, 9, 4, 36);

    assert_masks_identical(&mask, &expected, "9x4 同寸(全不透明→全透明)");
}

// ============================================================================
// 2. 再利用檻: 同寸・縮小方向で確保し直さない
// ============================================================================

/// **縮小方向**の再生成でバッファを作り直さない（容量が減らず、確保位置も動かない）
///
/// task 4.1 の教訓（`scale_resample_tests.rs:629` の節）と同型。同寸の反復だけでは
/// 毎回 `vec![0u8; len]` する実装が毎回同じ容量に着地して素通りするため、**より小さい
/// 寸法での再生成で容量が縮まないこと**を檻の要にする。
#[test]
fn regenerate_reuses_buffer_on_shrink() {
    // 32x32 → row_bytes=4、data.len() = 128
    let big = deterministic_pixels(32, 32, 128);
    let mut mask = AlphaMask::from_pbgra32(&big, 32, 32, 128);
    let cap_after_big = mask.data.capacity();
    let ptr_after_big = mask.data.as_ptr();
    assert!(cap_after_big >= 128, "前提: 大きい方の容量は 128 以上");

    // 縮小: 8x8 → row_bytes=1、data.len() = 8
    let small = deterministic_pixels(8, 8, 32);
    mask.regenerate_from_pbgra32(&small, 8, 8, 32);

    assert_eq!(
        mask.data.capacity(),
        cap_after_big,
        "縮小方向の再生成で容量が変化した（＝確保し直している）"
    );
    assert_eq!(
        mask.data.as_ptr(),
        ptr_after_big,
        "縮小方向の再生成で確保位置が動いた（＝確保し直している）"
    );
    assert_eq!(mask.data.len(), 8, "縮小後の長さは row_bytes*height");
}

/// 容量内での拡大（縮小後に元の寸法へ戻す）でも確保し直さない
#[test]
fn regenerate_reuses_buffer_when_growing_back_within_capacity() {
    let big = deterministic_pixels(32, 32, 128);
    let mut mask = AlphaMask::from_pbgra32(&big, 32, 32, 128);
    let cap = mask.data.capacity();
    let ptr = mask.data.as_ptr();

    let small = deterministic_pixels(8, 8, 32);
    mask.regenerate_from_pbgra32(&small, 8, 8, 32);
    // 容量内へ戻す
    mask.regenerate_from_pbgra32(&big, 32, 32, 128);

    assert_eq!(mask.data.capacity(), cap, "容量内の再拡大で容量が変化した");
    assert_eq!(mask.data.as_ptr(), ptr, "容量内の再拡大で確保位置が動いた");
    assert_eq!(mask.data.len(), 128);
}

/// 縮小・拡大を交互に繰り返しても容量が単調に増え続けない（定常での再確保なし）
#[test]
fn regenerate_alternating_shapes_does_not_grow_capacity() {
    let a = deterministic_pixels(32, 32, 128);
    let b = deterministic_pixels(9, 3, 36);

    let mut mask = AlphaMask::from_pbgra32(&a, 32, 32, 128);
    mask.regenerate_from_pbgra32(&b, 9, 3, 36);
    let cap = mask.data.capacity();
    let ptr = mask.data.as_ptr();

    for i in 0..8 {
        mask.regenerate_from_pbgra32(&a, 32, 32, 128);
        mask.regenerate_from_pbgra32(&b, 9, 3, 36);
        assert_eq!(mask.data.capacity(), cap, "反復 {i} で容量が増えた");
        assert_eq!(mask.data.as_ptr(), ptr, "反復 {i} で確保位置が動いた");
    }
}

// ============================================================================
// 3. PartialEq derive の意味（等価檻の比較器そのものの較正）
// ============================================================================

/// `PartialEq` が内容差を検出する（比較器が恒真でないことの陽性・陰性対照）
///
/// 等価檻は `assert_eq!` にぶら下がっているため、比較器が差を見落とすなら檻全体が
/// 偽合格になる。同内容が等しく・1 ビット差／寸法差が等しくないことを対で固定する。
#[test]
fn partial_eq_detects_content_and_dimension_differences() {
    let pixels = deterministic_pixels(9, 3, 36);
    let same_a = AlphaMask::from_pbgra32(&pixels, 9, 3, 36);
    let same_b = AlphaMask::from_pbgra32(&pixels, 9, 3, 36);
    assert_eq!(same_a, same_b, "同一入力のマスクは等しい");

    // 1 画素だけ反転（内容差）
    let mut flipped = pixels.clone();
    let offset = (1 * 36usize) + (2 * 4) + 3;
    flipped[offset] = 255 - flipped[offset];
    let content_diff = AlphaMask::from_pbgra32(&flipped, 9, 3, 36);
    assert_ne!(same_a, content_diff, "1 画素の差を PartialEq が見落とした");

    // 寸法差（同じ data 長になりうる形でも width/height が違えば不一致）
    let dim_diff = AlphaMask::from_pbgra32(&pixels, 9, 2, 36);
    assert_ne!(same_a, dim_diff, "寸法差を PartialEq が見落とした");
}
