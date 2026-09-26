//! 切替の語彙の綴りと、起動の由来を運ぶ設定の欄の既定値のテスト
//! （areka-P0-ghost-shell-balloon-switch 要件 2.1・4.1・11.7）。
//!
//! `OnGhostChanging` の Ref1 に載る出どころの 2 語と、`KanadeConfig::new` が入れる
//! 既定（ふつうの起動・シェルのフォルダ名はシェル名の写し）を固定する。

use super::{BootOrigin, ChangeOrigin};
use crate::msg::KanadeConfig;

#[test]
fn change_origin_reference_spellings_are_manual_and_automatic() {
    assert_eq!(ChangeOrigin::Manual.as_ref_str(), "manual");
    assert_eq!(ChangeOrigin::Automatic.as_ref_str(), "automatic");
}

#[test]
fn kanade_config_new_defaults_to_plain_boot_and_shell_folder_copied_from_shell_name() {
    let config = KanadeConfig::new("master", "1.0.0");
    assert_eq!(config.boot_origin, BootOrigin::Plain);
    assert_eq!(config.shell_folder, "master");
}
