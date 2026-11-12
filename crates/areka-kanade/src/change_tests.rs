//! 切替の語彙の綴りのテスト（areka-P0-ghost-shell-balloon-switch 要件 2.1）。
//!
//! `OnGhostChanging` の Ref1 に載る出どころの 2 語を固定する。`KanadeConfig::new` が入れる
//! 起動の由来の既定は `msg_tests.rs` の既定値のテストが見る。

use super::ChangeOrigin;

#[test]
fn change_origin_reference_spellings_are_manual_and_automatic() {
    assert_eq!(ChangeOrigin::Manual.as_ref_str(), "manual");
    assert_eq!(ChangeOrigin::Automatic.as_ref_str(), "automatic");
}
