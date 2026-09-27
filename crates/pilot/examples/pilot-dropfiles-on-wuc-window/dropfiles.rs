//! 受け口の部品（拡張スタイルの読み分け・手当ての切替・管理者判定）。
//!
//! ログの行はすべて本文の先頭に目印 `[dropfiles]` を置く（design Key Decision 6・要件 3.6）。

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Weak;
use std::sync::atomic::{AtomicU32, Ordering};

use bevy_ecs::prelude::{Entity, Resource};
use windows::Win32::Foundation::{
    GetLastError, HWND, LPARAM, LRESULT, POINT, SetLastError, WIN32_ERROR, WPARAM,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, DragAcceptFiles, DragFinish, DragQueryFileW, DragQueryPoint, HDROP,
    IsUserAnAdmin, SetWindowSubclass,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WM_DROPFILES, WS_EX_ACCEPTFILES, WS_EX_LAYERED,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TRANSPARENT,
};
use wintf::ecs::PointF;
use wintf::ecs::hit_test_in_window;
use wintf::ecs::world::EcsWorld;

// ---------------------------------------------------------------------------
// 拡張スタイルの読み分け（要件 2.6・4.4）
// ---------------------------------------------------------------------------

/// 拡張スタイルの生の値と、見たい 4 ビットの有無。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExStyleBits {
    pub accept_files: bool,
    pub transparent: bool,
    pub layered: bool,
    pub noredirect: bool,
    pub raw: isize,
}

impl ExStyleBits {
    pub fn from_raw(raw: isize) -> Self {
        let has = |bit: u32| raw & bit as isize != 0;
        Self {
            accept_files: has(WS_EX_ACCEPTFILES.0),
            transparent: has(WS_EX_TRANSPARENT.0),
            layered: has(WS_EX_LAYERED.0),
            noredirect: has(WS_EX_NOREDIRECTIONBITMAP.0),
            raw,
        }
    }
}

/// `GWL_EXSTYLE` を読み戻して 1 行出す。`when` は読んだ場面（`created`／`fix`／`drop`）。
pub fn log_ex_style(hwnd: HWND, when: &'static str) -> ExStyleBits {
    // SAFETY: Win32 境界。読み取りのみ。無効な HWND なら 0 が返り、その値をそのまま記録する。
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let bits = ExStyleBits::from_raw(raw);
    tracing::info!(
        when,
        accept_files = bits.accept_files,
        transparent = bits.transparent,
        layered = bits.layered,
        noredirect = bits.noredirect,
        raw = format!("0x{raw:X}"),
        "[dropfiles] ex-style"
    );
    bits
}

// ---------------------------------------------------------------------------
// 手当ての切替（要件 2.3・5.4・design Key Decision 4）
// ---------------------------------------------------------------------------

/// 受け入れの宣言の手当て。既定は `None`（ビットを足すだけ・`DragAcceptFiles` は呼ばない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum Fix {
    None,
    /// 生成後に `GWL_EXSTYLE` へ `WS_EX_ACCEPTFILES` を付け直して `SWP_FRAMECHANGED`。
    Reapply,
    /// `DragAcceptFiles(hwnd, true)`。
    DragAccept,
}

impl Fix {
    pub const ENV: &'static str = "PILOT_DROPFILES_FIX";

    /// 未設定・空は `None`。`reapply`／`dragaccept` 以外は `warn!` して `None`。
    pub fn from_env_value(value: Option<&str>) -> Fix {
        let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) else {
            return Fix::None;
        };
        match v {
            "reapply" => Fix::Reapply,
            "dragaccept" => Fix::DragAccept,
            _ => {
                tracing::warn!(
                    value = v,
                    env = Fix::ENV,
                    "[dropfiles] 手当ての値が不明 — 手当てなしで走る"
                );
                Fix::None
            }
        }
    }
}

/// 手当てを適用し、適用したら読み戻しの行（`when=fix`）を出す。`Fix::None` は何もしない。
pub fn apply_fix(hwnd: HWND, fix: Fix) -> Result<(), String> {
    match fix {
        Fix::None => return Ok(()),
        Fix::Reapply => reapply_accept_files(hwnd)?,
        // SAFETY: Win32 境界。戻り値なし（失敗は読み戻しの行で見る）。
        Fix::DragAccept => unsafe { DragAcceptFiles(hwnd, true) },
    }
    log_ex_style(hwnd, "fix");
    Ok(())
}

/// `apply_click_through`（`crates/wintf/src/win_style.rs`）と同じレシピで宣言のビットを付け直す。
fn reapply_accept_files(hwnd: HWND) -> Result<(), String> {
    // SAFETY: Win32 境界。SetLastError(0) で残留エラーを消してから呼び、
    // 「戻り値 0 かつ GetLastError が非 0」だけを失敗と読む（前の値が 0 でも成功はありうる）。
    unsafe {
        SetLastError(WIN32_ERROR(0));
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let err = GetLastError();
        if current == 0 && err.0 != 0 {
            return Err(format!("GetWindowLongPtrW(GWL_EXSTYLE) が失敗: {err:?}"));
        }
        SetLastError(WIN32_ERROR(0));
        let prev = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current | WS_EX_ACCEPTFILES.0 as isize);
        let err = GetLastError();
        if prev == 0 && err.0 != 0 {
            return Err(format!("SetWindowLongPtrW(GWL_EXSTYLE) が失敗: {err:?}"));
        }
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|e| format!("SetWindowPos(SWP_FRAMECHANGED) が失敗: {e}"))
    }
}

// ---------------------------------------------------------------------------
// 受け口（要件 2.8・3.1〜3.6・4.1・4.4）
// ---------------------------------------------------------------------------

/// 受け取った回数（World の外で数える・`seq` の元）。
pub static DROP_COUNT: AtomicU32 = AtomicU32::new(0);

/// `SetWindowSubclass` の識別子（この example は 1 つしか重ねない）。
const SUBCLASS_ID: usize = 1;

/// 受け口が持つ文脈。`world` は wintf の `EcsWorldSelfRef` の複製。
pub struct Context {
    pub world: Weak<RefCell<EcsWorld>>,
    pub window: Entity,
}

/// 窓へ受け口を重ね掛けする。失敗（`FALSE`）は呼び手へ返す。
pub fn install(hwnd: HWND, ctx: Context) -> Result<(), String> {
    // ponytail: 文脈の Box は解放しない（窓 1 枚・使い捨て）。窓を作り直すなら WM_NCDESTROY で Box::from_raw して外す。
    let refdata = Box::into_raw(Box::new(ctx));
    // SAFETY: Win32 境界。refdata はプロセスの終わりまで生きる。
    if unsafe { SetWindowSubclass(hwnd, Some(subclass_proc), SUBCLASS_ID, refdata as usize) }
        .as_bool()
    {
        Ok(())
    } else {
        // SAFETY: 設置に失敗したので誰も refdata を持っていない。取り戻して落とす。
        drop(unsafe { Box::from_raw(refdata) });
        Err("SetWindowSubclass が FALSE を返した".to_string())
    }
}

/// `WM_DROPFILES` だけ消費し、他は `DefSubclassProc` へ無条件に流す。
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    refdata: usize,
) -> LRESULT {
    if msg != WM_DROPFILES {
        // SAFETY: 受けた引数をそのまま連鎖の次へ渡す。
        return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: refdata は install が Box::into_raw した Context で、解放されない。
    let ctx = unsafe { &*(refdata as *const Context) };
    let hdrop = HDROP(wparam.0 as _);
    if catch_unwind(AssertUnwindSafe(|| handle_drop(ctx, hwnd, hdrop))).is_err() {
        tracing::error!("[dropfiles] 受け口の中で panic — 片付けて続行");
    }
    LRESULT(0)
}

/// 途中で失敗（panic を含む）しても `DragFinish` を必ず呼ぶ。
struct FinishOnDrop(HDROP);

impl Drop for FinishOnDrop {
    fn drop(&mut self) {
        // SAFETY: WM_DROPFILES で受けた HDROP を 1 回だけ解放する。
        unsafe { DragFinish(self.0) };
    }
}

/// 1 回の到着を記録する。位置による分岐は置かない（要件 4.1）。
fn handle_drop(ctx: &Context, hwnd: HWND, hdrop: HDROP) {
    let _finish = FinishOnDrop(hdrop);
    let seq = DROP_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let mut pt = POINT::default();
    // SAFETY: Win32 境界。pt は有効な書き込み先。
    let in_client = unsafe { DragQueryPoint(hdrop, &mut pt) }.as_bool();
    let opaque = opaque_label(hit_opaque(ctx, pt));
    let bits = log_ex_style(hwnd, "drop");
    // SAFETY: Win32 境界。0xFFFF_FFFF で本数を問う。
    let n = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
    tracing::info!(
        seq,
        x = pt.x,
        y = pt.y,
        in_client,
        opaque,
        transparent = bits.transparent,
        accept_files = bits.accept_files,
        layered = bits.layered,
        noredirect = bits.noredirect,
        n,
        "[dropfiles] 到着"
    );
    for i in 0..n {
        match query_path(hdrop, i) {
            Some(path) => tracing::info!(seq, i, path = %path, "[dropfiles] ファイル"),
            None => tracing::error!(seq, i, "[dropfiles] パスの取り出しに失敗"),
        }
    }
}

/// 物理 px のまま透過機構と同じ判定器に聞く。World を借りられなければ `None`（借用はここで終える）。
fn hit_opaque(ctx: &Context, pt: POINT) -> Option<bool> {
    let world = ctx.world.upgrade()?;
    let ecs = world.try_borrow().ok()?;
    let hit = hit_test_in_window(
        ecs.world(),
        ctx.window,
        PointF::new(pt.x as f32, pt.y as f32),
    );
    Some(hit.is_some())
}

fn opaque_label(hit: Option<bool>) -> &'static str {
    match hit {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

/// `i` 番目のパス。長さを問うてから取り出す。どちらかが 0 なら `None`。
fn query_path(hdrop: HDROP, i: u32) -> Option<String> {
    // SAFETY: Win32 境界。None で必要な長さ（終端を除く）を問う。
    let len = unsafe { DragQueryFileW(hdrop, i, None) };
    if len == 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize + 1];
    // SAFETY: buf は長さ＋終端ぶん確保済み。
    let got = unsafe { DragQueryFileW(hdrop, i, Some(&mut buf)) };
    (got != 0).then(|| String::from_utf16_lossy(&buf[..got as usize]))
}

// ---------------------------------------------------------------------------
// 管理者判定（要件 5.2）
// ---------------------------------------------------------------------------

/// 管理者として動いているか（`IsUserAnAdmin`）。
pub fn is_admin() -> bool {
    // SAFETY: Win32 境界。引数なしの問い合わせ。
    unsafe { IsUserAnAdmin() }.as_bool()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ex_style_bits_reads_each_of_the_four_bits() {
        let none = ExStyleBits::from_raw(0);
        assert!(!none.accept_files && !none.transparent && !none.layered && !none.noredirect);
        assert!(ExStyleBits::from_raw(0x10).accept_files);
        assert!(ExStyleBits::from_raw(0x20).transparent);
        assert!(ExStyleBits::from_raw(0x80000).layered);
        assert!(ExStyleBits::from_raw(0x200000).noredirect);
        // 1 ビットだけのとき他の 3 つは立たない。
        let only_accept = ExStyleBits::from_raw(0x10);
        assert!(!only_accept.transparent && !only_accept.layered && !only_accept.noredirect);
    }

    #[test]
    fn ex_style_bits_reads_combinations_and_keeps_raw() {
        // 生成直後の見込み（ACCEPTFILES | TOOLWINDOW | NOREDIRECTIONBITMAP）。
        let created = ExStyleBits::from_raw(0x10 | 0x80 | 0x200000);
        assert_eq!(
            created,
            ExStyleBits {
                accept_files: true,
                transparent: false,
                layered: false,
                noredirect: true,
                raw: 0x200090,
            }
        );
        // 透過が付いた後（pilot-clickthrough-alpha-toggle の実測 0x280028 に ACCEPTFILES を足した形）。
        let toggled = ExStyleBits::from_raw(0x280028 | 0x10);
        assert_eq!(
            (
                toggled.accept_files,
                toggled.transparent,
                toggled.layered,
                toggled.noredirect
            ),
            (true, true, true, true)
        );
        // 宣言が消えた場合も読み分けられる。
        let lost = ExStyleBits::from_raw(0x280028);
        assert_eq!(
            (
                lost.accept_files,
                lost.transparent,
                lost.layered,
                lost.noredirect
            ),
            (false, true, true, true)
        );
    }

    #[test]
    fn opaque_label_maps_hit_result() {
        assert_eq!(opaque_label(Some(true)), "true");
        assert_eq!(opaque_label(Some(false)), "false");
        assert_eq!(opaque_label(None), "unknown");
    }

    #[test]
    fn fix_from_env_value_maps_each_value() {
        assert_eq!(Fix::ENV, "PILOT_DROPFILES_FIX");
        assert_eq!(Fix::from_env_value(None), Fix::None);
        assert_eq!(Fix::from_env_value(Some("")), Fix::None);
        assert_eq!(Fix::from_env_value(Some("  ")), Fix::None);
        assert_eq!(Fix::from_env_value(Some("reapply")), Fix::Reapply);
        assert_eq!(Fix::from_env_value(Some("dragaccept")), Fix::DragAccept);
        assert_eq!(Fix::from_env_value(Some("DragAccept")), Fix::None);
        assert_eq!(Fix::from_env_value(Some("bogus")), Fix::None);
    }
}
