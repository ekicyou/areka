//! フォルダ選びの指定の選び分け（design「FolderPickerReadonly」・要件 8.6・11.6）。

use windows::Win32::UI::Shell::{FOS_DONTADDTORECENT, FOS_PICKFOLDERS};

use super::dialog_options;

/// 読み取りだけなら、足した指定が「最近使った所に足さない」を含み、元の指定も残す。
#[test]
fn readonly_adds_dont_add_to_recent() {
    let got = dialog_options(FOS_PICKFOLDERS, true);
    assert!(got.contains(FOS_DONTADDTORECENT));
    assert!(got.contains(FOS_PICKFOLDERS));
}

/// ふつうの起動なら、今の指定のまま。
#[test]
fn normal_run_keeps_base() {
    assert_eq!(dialog_options(FOS_PICKFOLDERS, false), FOS_PICKFOLDERS);
}
