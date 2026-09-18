//! 計画から `HMENU` への写し（[`build`]）の読み戻しテスト（要件 1.2・2.4・2.5）。
//!
//! 組み立てた `HMENU` を OS の読み出し API（項目数・識別子・状態・子メニュー・文字列）で
//! 読み戻す。窓もメッセージループも要らず、画面には何も出ない。表示（[`show`]）は
//! 操作を待って止まるのでここからは呼ばない（確認は実機）。
//!
//! 期待値はすべて書き下した値で、[`build`] 自身から導かない。

use windows::Win32::UI::WindowsAndMessaging::{
    GetMenuItemCount, GetMenuItemID, GetMenuState, GetMenuStringW, GetSubMenu, HMENU,
    MF_BYPOSITION, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
};

use super::*;

fn item(id: u32, label: &str, enabled: bool, checked: bool) -> PlanEntry {
    PlanEntry::Item {
        id,
        label: label.to_owned(),
        enabled,
        checked,
    }
}

/// 位置で指した項目の状態。下位 8 ビットが状態の旗、サブメニューなら上位 8 ビットが子の数。
fn state(menu: HMENU, pos: u32) -> u32 {
    // SAFETY: `menu` はこのテストが組み立てて保持している有効なメニューで、範囲外の位置には
    // API が `u32::MAX` を返すだけである。
    unsafe { GetMenuState(menu, pos, MF_BYPOSITION) }
}

fn has(state: u32, flag: MENU_ITEM_FLAGS) -> bool {
    state & flag.0 != 0
}

fn id_at(menu: HMENU, pos: i32) -> u32 {
    // SAFETY: `state` と同じ（有効なメニュー・範囲外は `u32::MAX`）。
    unsafe { GetMenuItemID(menu, pos) }
}

fn count(menu: HMENU) -> i32 {
    // SAFETY: `state` と同じ（有効なメニュー）。
    unsafe { GetMenuItemCount(Some(menu)) }
}

fn label_at(menu: HMENU, pos: u32) -> String {
    let mut buffer = [0u16; 64];
    // SAFETY: `buffer` は呼び出しの間だけ貸す書き込み先で、長さは API へ一緒に渡る。
    let len = unsafe { GetMenuStringW(menu, pos, Some(&mut buffer), MF_BYPOSITION) };
    String::from_utf16_lossy(&buffer[..len as usize])
}

#[test]
fn build_copies_order_ids_flags_and_labels_into_the_hmenu() {
    let entries = vec![
        item(1, "ふつう", true, false),
        item(2, "説明書(&R)", false, false),
        item(3, "現在", true, true),
        PlanEntry::Separator,
        PlanEntry::Submenu {
            label: "ゴースト".to_owned(),
            enabled: true,
            children: vec![item(4, "子一", true, true), item(5, "子二", false, false)],
        },
        PlanEntry::Submenu {
            label: "シェル".to_owned(),
            enabled: false,
            children: vec![],
        },
    ];

    let menu = build(&entries).expect("the menu builds");

    assert_eq!(count(*menu), 6);

    // 識別子は計画のものがそのまま載る。サブメニューの見出しは識別子を持たない。
    assert_eq!(id_at(*menu, 0), 1);
    assert_eq!(id_at(*menu, 1), 2);
    assert_eq!(id_at(*menu, 2), 3);
    assert_eq!(id_at(*menu, 4), u32::MAX);

    // 有効・無効・チェックの写し（要件 2.4・2.5）。
    let plain = state(*menu, 0);
    assert!(!has(plain, MF_GRAYED) && !has(plain, MF_CHECKED) && !has(plain, MF_SEPARATOR));
    let disabled = state(*menu, 1);
    assert!(has(disabled, MF_GRAYED) && !has(disabled, MF_CHECKED));
    let checked = state(*menu, 2);
    assert!(has(checked, MF_CHECKED) && !has(checked, MF_GRAYED));
    assert!(has(state(*menu, 3), MF_SEPARATOR));

    // 文字列は UTF-16 へ写り、`&` はそのまま渡る。
    assert_eq!(label_at(*menu, 0), "ふつう");
    assert_eq!(label_at(*menu, 1), "説明書(&R)");
    assert_eq!(label_at(*menu, 4), "ゴースト");

    // サブメニュー: 見出しは子メニューを開く項目で、子は登記順に並ぶ。
    let heading = state(*menu, 4);
    assert!(has(heading, MF_POPUP) && !has(heading, MF_GRAYED));
    assert_eq!(heading >> 8, 2, "the high byte carries the child count");
    // SAFETY: `state` と同じ（有効なメニュー・子メニューが無い位置には無効なハンドルが返る）。
    let child = unsafe { GetSubMenu(*menu, 4) };
    assert!(!child.is_invalid());
    assert_eq!(count(child), 2);
    assert_eq!(id_at(child, 0), 4);
    assert_eq!(id_at(child, 1), 5);
    assert!(has(state(child, 0), MF_CHECKED));
    assert!(has(state(child, 1), MF_GRAYED));
    assert_eq!(label_at(child, 1), "子二");

    // 無効な見出しは灰色で、子が無くても子メニューを開く項目のままである。
    let disabled_heading = state(*menu, 5);
    assert!(has(disabled_heading, MF_POPUP) && has(disabled_heading, MF_GRAYED));
}
