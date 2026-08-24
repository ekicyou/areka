//! `AlphaMaskResource` の内部 `Arc` 化と `set_shared` の檻
//!
//! design.md `#### AlphaMask 再生成・共有（wintf・additive）`（要件 3.1・6.2・6.3）。
//!
//! 固定する主張は 2 つ:
//! 1. **既存の見た目と挙動が不変**: `new` / `default` / `set` / `mask()` の観測結果が
//!    内部表現の `Arc` 化で変わらない（`set` は値を受け取り、`mask()` は `Option<&AlphaMask>`）。
//! 2. **`set_shared` は `set` と同一の観測結果を与える**（同一内容なら判定も一致）。加えて
//!    `set_shared` は渡された確保をそのまま保持する（深いコピーを作らない＝共有供給の要件）。
//!
//! 実時間は合否条件に使わない（要件 6.2）。純 x64・常設 `cargo test`・env ゲートなし・
//! `#[ignore]` なし（要件 6.3）。GPU も World も使わない（`AlphaMaskResource` 単体の檻）。
//! ヒットテスト経路からの読み取り優先順位は既存の `tests_ex/alpha_mask_resource.rs` が
//! 引き続き固定しており、本ファイルはその前段（供給口そのもの）だけを見る。

use super::*;
use std::sync::Arc;

/// 決定論的な混在パターンのマスクを作る
fn mixed_mask(width: u32, height: u32) -> AlphaMask {
    let stride = width * 4;
    let mut pixels = vec![0u8; (stride * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let offset = (y * stride) as usize + (x as usize * 4);
            if (x * 3 + y * 5) % 4 < 2 {
                pixels[offset + 3] = 255;
            }
        }
    }
    AlphaMask::from_pbgra32(&pixels, width, height, stride)
}

/// 2 つのマスクが全座標で同一判定であることを assert する
fn assert_same_hits(a: &AlphaMask, b: &AlphaMask, ctx: &str) {
    assert_eq!(a.width(), b.width(), "{ctx}: width 不一致");
    assert_eq!(a.height(), b.height(), "{ctx}: height 不一致");
    for y in 0..=b.height() {
        for x in 0..=b.width() {
            assert_eq!(
                a.is_hit(x, y),
                b.is_hit(x, y),
                "{ctx}: is_hit({x},{y}) 不一致"
            );
        }
    }
}

// ============================================================================
// 既存 API の観測不変（new / default / set / mask）
// ============================================================================

/// `new()` と `default()` はマスク未設定（従来どおり `None`）
#[test]
fn new_and_default_have_no_mask() {
    assert!(AlphaMaskResource::new().mask().is_none(), "new() は未設定");
    assert!(
        AlphaMaskResource::default().mask().is_none(),
        "default() は未設定"
    );
}

/// `set` → `mask()` の観測が従来どおり（値で受け取り、参照で返す）
#[test]
fn set_then_mask_returns_the_same_mask() {
    let mask = mixed_mask(9, 4);
    let mut res = AlphaMaskResource::new();
    res.set(mask.clone());

    let got = res.mask().expect("set 後は Some");
    assert_eq!(got, &mask, "set したマスクと異なる内容が返った");
    assert_same_hits(got, &mask, "set → mask()");
}

/// `set` の再呼び出しで置換される（後勝ち・従来挙動）
#[test]
fn set_replaces_previous_mask() {
    let first = mixed_mask(16, 4);
    let second = mixed_mask(5, 7);

    let mut res = AlphaMaskResource::new();
    res.set(first.clone());
    res.set(second.clone());

    let got = res.mask().expect("set 後は Some");
    assert_eq!(got, &second, "後から set したマスクに置換されていない");
    assert_ne!(got, &first, "前のマスクが残っている");
}

/// `Clone` した `AlphaMaskResource` が同一内容のマスクを返す（derive の観測不変）
#[test]
fn clone_preserves_observable_mask() {
    let mask = mixed_mask(17, 3);
    let mut res = AlphaMaskResource::new();
    res.set(mask.clone());

    let cloned = res.clone();
    let got = cloned.mask().expect("clone 後も Some");
    assert_eq!(got, &mask, "clone でマスク内容が変わった");
    assert_same_hits(got, &mask, "clone → mask()");
}

// ============================================================================
// set_shared（additive）
// ============================================================================

/// `set_shared` は `set` と同一の観測結果を与える（同一内容ならマスクも判定も一致）
#[test]
fn set_shared_matches_set_observably() {
    let mask = mixed_mask(33, 6);

    let mut via_set = AlphaMaskResource::new();
    via_set.set(mask.clone());

    let mut via_shared = AlphaMaskResource::new();
    via_shared.set_shared(Arc::new(mask.clone()));

    let a = via_set.mask().expect("set 後は Some");
    let b = via_shared.mask().expect("set_shared 後は Some");
    assert_eq!(a, b, "set と set_shared で観測されるマスクが異なる");
    assert_same_hits(a, b, "set vs set_shared");
    assert_eq!(b, &mask, "set_shared が渡した内容と異なるマスクを返した");
}

/// `set_shared` は渡された確保をそのまま保持する（深いコピーを作らない＝共有供給）
///
/// これが `set_shared` の存在理由（下流供給を参照カウント増だけで済ませる）。内部で
/// `AlphaMask` を複製する実装だと確保位置が一致せず割れる。
#[test]
fn set_shared_stores_the_same_allocation() {
    let arc = Arc::new(mixed_mask(9, 9));
    let arc_ptr = Arc::as_ptr(&arc);

    let mut res = AlphaMaskResource::new();
    res.set_shared(Arc::clone(&arc));

    let got = res.mask().expect("set_shared 後は Some") as *const AlphaMask;
    assert_eq!(got, arc_ptr, "set_shared が別の確保へ複製している");
    assert_eq!(
        Arc::strong_count(&arc),
        2,
        "set_shared が参照カウントを増やしていない（共有していない）"
    );

    drop(res);
    assert_eq!(
        Arc::strong_count(&arc),
        1,
        "リソース破棄後も参照カウントが戻らない"
    );
}

/// `set` と `set_shared` は相互に置換できる（どちらの順序でも後勝ち）
#[test]
fn set_and_set_shared_replace_each_other() {
    let a = mixed_mask(8, 2);
    let b = mixed_mask(12, 5);

    // set → set_shared
    let mut res = AlphaMaskResource::new();
    res.set(a.clone());
    res.set_shared(Arc::new(b.clone()));
    assert_eq!(
        res.mask().expect("Some"),
        &b,
        "set → set_shared で後勝ちしない"
    );

    // set_shared → set
    let mut res = AlphaMaskResource::new();
    res.set_shared(Arc::new(b.clone()));
    res.set(a.clone());
    assert_eq!(
        res.mask().expect("Some"),
        &a,
        "set_shared → set で後勝ちしない"
    );
}

/// 同一 `Arc` を複数のリソースへ供給しても各々が同じ内容を観測する（共有の非破壊性）
#[test]
fn shared_mask_can_supply_multiple_resources() {
    let arc = Arc::new(mixed_mask(20, 4));

    let mut r1 = AlphaMaskResource::new();
    let mut r2 = AlphaMaskResource::new();
    r1.set_shared(Arc::clone(&arc));
    r2.set_shared(Arc::clone(&arc));

    assert_eq!(Arc::strong_count(&arc), 3, "共有の参照カウントが合わない");
    let m1 = r1.mask().expect("Some");
    let m2 = r2.mask().expect("Some");
    assert_eq!(m1, m2, "同一 Arc から異なる内容が観測された");
    assert_eq!(
        m1 as *const AlphaMask, m2 as *const AlphaMask,
        "同一 Arc なのに確保位置が異なる"
    );
    assert_same_hits(m1, &arc, "共有マスクの判定一致");
}
