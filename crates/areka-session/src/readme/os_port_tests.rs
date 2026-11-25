//! 関連付けの確かめ。判定の純関数だけを見る。
//! レジストリを引く `has_verb` は機械の登録に依るので常時のテストに入れない。

use super::*;
use std::path::Path;

fn check(file: &str, is_file: bool) -> AssocCheck {
    assoc_check(Path::new(file), is_file)
}

#[test]
fn url_and_scheme_are_not_checked() {
    assert_eq!(
        check("https://example.com/a.zzqqnoassoc", false),
        AssocCheck::Skip
    );
    assert_eq!(check("mailto:a@example.com", false), AssocCheck::Skip);
}

#[test]
fn directory_and_name_only_program_are_not_checked() {
    assert_eq!(check(r"C:\Windows", false), AssocCheck::Skip);
    // 名前だけ（OS のパス探索に任せる）・explorer.exe の /select は相対の名前。
    assert_eq!(check("explorer.exe", true), AssocCheck::Skip);
}

#[test]
fn absolute_file_is_checked_by_its_extension() {
    assert_eq!(
        check(r"C:\a\b.Zzqqnoassoc", true),
        AssocCheck::Ext(OsString::from(".Zzqqnoassoc"))
    );
}

#[test]
fn file_without_extension_has_no_association() {
    assert_eq!(check(r"C:\a\noext", true), AssocCheck::NoAssoc);
}

#[test]
fn dot_name_uses_the_whole_name_as_extension() {
    assert_eq!(
        check(r"C:\a\.gitignore", true),
        AssocCheck::Ext(".gitignore".into())
    );
}

#[test]
fn shortcut_is_left_to_the_shell() {
    // lnkfile は動詞を持たず、OS がリンク先を解く（関連付けの照会は常に失敗する）。
    assert_eq!(check(r"C:\a\b.LNK", true), AssocCheck::Skip);
}

#[test]
fn win32_code_unwraps_only_win32_facility_hresults() {
    assert_eq!(win32_code(0x8007_0002_u32 as i32), 2);
    assert_eq!(win32_code(0x8000_4005_u32 as i32), 0x8000_4005);
}
