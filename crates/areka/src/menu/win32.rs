//! メニューの OS 表示（areka-P0-popup-menu-minimal）。
//!
//! 計画から `HMENU` を組み立て、`TrackPopupMenuEx` で表示して選ばれた識別子を返す。
//! フォアグラウンドの作法とクライアント座標からスクリーン座標への変換もここに置く。
//! メニュー module の `unsafe` はこのファイルと、その読み戻しテスト（`win32_tests.rs`）だけに閉じる。
//!
//! 描画・キーボード操作・メニュー外クリックでの閉じ方は OS に任せ、自前の窓は作らない
//! （要件 1.2・7.5——持ち主はキャラクター窓そのもの）。記録は呼び手が知りえないことだけを
//! ここで出し、表示・選択・未選択・失敗の記録は呼び手（`trigger`）が戻り値から出す。

use windows::Win32::Foundation::{
    GetLastError, HWND, LPARAM, POINT, SetLastError, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, HMENU, MENU_ITEM_FLAGS, MF_CHECKED, MF_ENABLED, MF_GRAYED,
    MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, PostMessageW, SetForegroundWindow,
    TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TPM_TOPALIGN, TrackPopupMenuEx, WM_NULL,
};
use windows::core::{Error, HSTRING, Owned, Result};

use super::plan::{MenuPlan, PlanEntry};

/// 計画をメニューとして `screen_pos`（画面座標）に出し、閉じるまで待って結果を返す。
///
/// 戻り値で 3 つを区別する: `Ok(Some(id))`＝項目が選ばれた・`Ok(None)`＝何も選ばれずに
/// 閉じた・`Err`＝組み立てか表示に失敗した（何も表示されていない・要件 1.7）。
/// `owner` はキャラクター窓の実ハンドルで、メニューのための窓は別に作らない（要件 7.5）。
pub(crate) fn show(owner: HWND, screen_pos: (i32, i32), plan: &MenuPlan) -> Result<Option<u32>> {
    let menu = build(&plan.entries)?;
    let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_TOPALIGN;

    // 持ち主を前面にしてから出す。そうしないとメニューの外をクリックしても閉じない
    // （要件 1.3・Microsoft KB Q135788 の作法）。前面にできなくても表示は試みる。
    // SAFETY: 窓ハンドルを値で渡すだけで、無効なハンドルには API が失敗を返す。
    if !unsafe { SetForegroundWindow(owner) }.as_bool() {
        tracing::debug!(
            event = "menu_foreground_refused",
            "[menu] SetForegroundWindow refused: the menu may not close on an outside click"
        );
    }

    // `TPM_RETURNCMD` の戻り値 0 は「未選択」と「失敗」の両方を表すので、直前に最終エラーを
    // 0 にしておき、0 が返ったときの最終エラーで見分ける。読むのは戻った直後（間に別の
    // API を挟むと値が変わる）。
    // SAFETY: `menu` はこの関数が持つ有効なメニューで、呼び出しの間ずっと生存する。
    // 窓ハンドルは値で渡すだけで、無効なら API が失敗を返す。残りは値と None。
    let (selected, error) = unsafe {
        SetLastError(WIN32_ERROR(0));
        let selected = TrackPopupMenuEx(*menu, flags.0, screen_pos.0, screen_pos.1, owner, None);
        (selected.0 as u32, GetLastError())
    };

    // 同じ作法の後半: 閉じた後に持ち主へ無害なメッセージを 1 通送る。送れなくても結果は返す。
    // SAFETY: 窓ハンドルと値を渡すだけで、無効なハンドルには API が失敗を返す。
    if let Err(err) = unsafe { PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0)) } {
        tracing::debug!(
            event = "menu_null_post_failed",
            error = %err,
            "[menu] PostMessageW(WM_NULL) failed after the menu closed"
        );
    }

    match (selected, error) {
        (0, WIN32_ERROR(0)) => Ok(None),
        (0, error) => Err(Error::from(error)),
        (id, _) => Ok(Some(id)),
    }
}

/// 窓のクライアント座標を画面座標へ写す。窓が既に無いなどで写せなければ `None`
/// （呼び手はメニューの要求を捨てる）。
pub(crate) fn client_to_screen(hwnd: HWND, x: i32, y: i32) -> Option<(i32, i32)> {
    let mut point = POINT { x, y };
    // SAFETY: `point` は呼び出しの間だけ貸す書き込み先で、無効な窓ハンドルには API が失敗を返す。
    unsafe { ClientToScreen(hwnd, &mut point) }
        .as_bool()
        .then_some((point.x, point.y))
}

/// 計画の要素を順にメニューへ足す。途中で失敗したら作りかけは残さず `Err` を返す。
///
/// 戻り値の [`Owned`] は手放すときに `DestroyMenu` を呼ぶ。子メニューは親へ足せた時点で
/// 親のものになり、親を壊すと一緒に壊れる。だから子の [`Owned`] は足せた後にだけ手放し
/// （二重に壊さない）、足す前に失敗した子は自分の [`Owned`] が壊す（漏らさない）。
fn build(entries: &[PlanEntry]) -> Result<Owned<HMENU>> {
    // SAFETY: 作ったばかりのメニューは他の誰も持っておらず、ここが唯一の持ち主になる。
    let menu = unsafe { Owned::new(CreatePopupMenu()?) };
    for entry in entries {
        match entry {
            PlanEntry::Item {
                id,
                label,
                enabled,
                checked,
            } => {
                let checked = if *checked { MF_CHECKED } else { MF_UNCHECKED };
                let flags = MF_STRING | grayed(*enabled) | checked;
                append(*menu, flags, *id as usize, label)?;
            }
            PlanEntry::Submenu {
                label,
                enabled,
                children,
            } => {
                let child = build(children)?;
                // `MF_POPUP` のときは識別子の場所に子メニューのハンドルを渡す。
                append(
                    *menu,
                    MF_POPUP | grayed(*enabled),
                    (*child).0 as usize,
                    label,
                )?;
                std::mem::forget(child);
            }
            // 区切り線では識別子も文字列も無視される。
            PlanEntry::Separator => append(*menu, MF_SEPARATOR, 0, "")?,
        }
    }
    Ok(menu)
}

/// 選べない項目は灰色にする（要件 2.5）。
fn grayed(enabled: bool) -> MENU_ITEM_FLAGS {
    if enabled { MF_ENABLED } else { MF_GRAYED }
}

fn append(menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, label: &str) -> Result<()> {
    // 終端 0 付きの UTF-16。OS は呼び出しの中で写しを取る。
    let label = HSTRING::from(label);
    // SAFETY: `menu` は呼び手が持つ有効なメニューで、`label` は呼び出しの間ずっと生存する。
    unsafe { AppendMenuW(menu, flags, id, &label) }
}

#[cfg(test)]
#[path = "win32_tests.rs"]
mod win32_tests;
