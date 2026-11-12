//! `AlphaMaskResource::set_shared` の檻
//!
//! design.md `#### AlphaMask 再生成・共有（wintf・additive）`（要件 3.1・6.2・6.3）。
//!
//! 固定する主張は 1 つ: `set_shared` は渡された確保をそのまま保持する（深いコピーを
//! 作らない＝共有供給の要件）。`set` / `mask()` の観測とヒットテスト経路からの読み取りは
//! `tests_ex/alpha_mask_resource.rs` が固定している。
//!
//! 純 x64・常設 `cargo test`・env ゲートなし・`#[ignore]` なし（要件 6.3）。
//! GPU も World も使わない（`AlphaMaskResource` 単体の檻）。

use super::*;
use std::sync::Arc;

/// `set_shared` は渡された確保をそのまま保持する（深いコピーを作らない＝共有供給）
///
/// これが `set_shared` の存在理由（下流供給を参照カウント増だけで済ませる）。内部で
/// `AlphaMask` を複製する実装だと確保位置が一致せず割れる。
#[test]
fn set_shared_stores_the_same_allocation() {
    let arc = Arc::new(super::tests::mask_2x2(255));
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
