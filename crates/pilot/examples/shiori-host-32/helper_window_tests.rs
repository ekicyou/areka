//! `helper_window.rs` の終了コードの決まりのテスト（兄弟のファイル）。

use super::*;

/// (UNLOAD で抜けたか, pasta を読めたか) → 終了コード。上限で抜けたら読めたかに関係なく 4。
#[test]
fn exit_code_table() {
    assert_eq!(exit_code(true, true), 0);
    assert_eq!(exit_code(false, true), 4);
    assert_eq!(exit_code(false, false), 4);
    assert_eq!(exit_code(true, false), 5);
}
