// =============================================================================
// 鎖の適用系の決定論テスト用の替え玉（`#[cfg(test)]` でだけ結線される）
//
// design.md「File Structure Plan」の `zorder_chain_apply_test_support.rs`。
// 親の `zorder_chain_apply.rs` から `#[path]` で同じモジュール名 `double` のまま繋ぐ
// ——本番ファイルを 1 ファイル 1,000 行未満に保つための移し出しであり、中身は変えていない。
// =============================================================================

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use windows::Win32::Foundation::HWND;
use windows::core::HRESULT;

use super::SetWindowPosCommand;

/// 記録される実行環境の呼び出し 1 件（順序の主張に使う）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Call {
    /// 現在の所有者を読んだ。
    ReadOwner(usize),
    /// 所有関係を外した。
    ClearOwner(usize),
    /// 所有関係を張った（被所有側, 所有側）。
    SetOwner(usize, usize),
    /// 生の 1 つ奥を読んだ。
    RawNext(usize),
    /// 後押しを出した（差し直した窓, 挿入位置）。
    Nudge(usize, usize),
    /// 前面走査を行った。
    MeasureFront(usize),
}

/// 台本と、走行中に積まれた呼び出しの記録。
#[derive(Default)]
pub(crate) struct Script {
    /// 走行中に実際に呼ばれた窓口の列（**順序込み**）。
    pub calls: Vec<Call>,
    /// `owned` の現在の所有者（載っていない窓は所有者無し＝0）。
    pub owner_of: HashMap<usize, usize>,
    /// 所有者の読み取りを失敗させる窓。
    pub read_fails: HashSet<usize>,
    /// 撤去を失敗させる窓。
    pub clear_fails: HashSet<usize>,
    /// 付与を失敗させる窓（被所有側で指定する）。
    pub set_fails: HashSet<usize>,
    /// 生の 1 つ奥（載っていない窓は「奥に何も無い」＝`None`）。
    pub next_of: HashMap<usize, usize>,
    /// 後押しを失敗させる。
    pub nudge_fails: bool,
    /// 前面走査が返す列（起点 → 手前へ辿った順）。
    pub front_of: HashMap<usize, Vec<usize>>,
}

thread_local! {
    static SCRIPT: RefCell<Option<Script>> = const { RefCell::new(None) };
}

/// テスト用の失敗値。
fn failure() -> windows::core::Error {
    windows::core::Error::from(HRESULT(0x8007_0578u32 as i32))
}

/// 台本を据えて `body` を走らせ、走行後の台本（呼び出しの記録込み）を返す。
pub(crate) fn with_script<R>(script: Script, body: impl FnOnce() -> R) -> (R, Script) {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            SCRIPT.with(|cell| *cell.borrow_mut() = None);
        }
    }
    SCRIPT.with(|cell| *cell.borrow_mut() = Some(script));
    let _restore = Restore;
    let out = body();
    let used = SCRIPT.with(|cell| cell.borrow_mut().take());
    (out, used.expect("台本は走行中に落ちない"))
}

/// 台本が据わっていれば `f` を適用し、結果を返す（据わっていなければ `None`）。
fn with<R>(f: impl FnOnce(&mut Script) -> R) -> Option<R> {
    SCRIPT.with(|cell| cell.borrow_mut().as_mut().map(f))
}

pub(super) fn read_owner(hwnd: HWND) -> Option<windows::core::Result<HWND>> {
    with(|s| {
        let key = hwnd.0 as usize;
        s.calls.push(Call::ReadOwner(key));
        if s.read_fails.contains(&key) {
            return Err(failure());
        }
        let owner = s.owner_of.get(&key).copied().unwrap_or(0);
        Ok(HWND(owner as *mut _))
    })
}

pub(super) fn clear_owner(hwnd: HWND) -> Option<windows::core::Result<()>> {
    with(|s| {
        let key = hwnd.0 as usize;
        s.calls.push(Call::ClearOwner(key));
        if s.clear_fails.contains(&key) {
            return Err(failure());
        }
        s.owner_of.remove(&key);
        Ok(())
    })
}

pub(super) fn set_owner(owned: HWND, owner: HWND) -> Option<windows::core::Result<()>> {
    with(|s| {
        let key = owned.0 as usize;
        s.calls.push(Call::SetOwner(key, owner.0 as usize));
        if s.set_fails.contains(&key) {
            return Err(failure());
        }
        s.owner_of.insert(key, owner.0 as usize);
        Ok(())
    })
}

pub(super) fn raw_next(hwnd: HWND) -> Option<Option<HWND>> {
    with(|s| {
        let key = hwnd.0 as usize;
        s.calls.push(Call::RawNext(key));
        s.next_of.get(&key).map(|next| HWND(*next as *mut _))
    })
}

pub(super) fn nudge(cmd: &SetWindowPosCommand) -> Option<windows::core::Result<()>> {
    with(|s| {
        s.calls.push(Call::Nudge(
            cmd.hwnd.0 as usize,
            cmd.hwnd_insert_after.map_or(0, |h| h.0 as usize),
        ));
        if s.nudge_fails {
            return Err(failure());
        }
        Ok(())
    })
}

pub(super) fn measure_front(hwnd: HWND) -> Option<Vec<HWND>> {
    with(|s| {
        let key = hwnd.0 as usize;
        s.calls.push(Call::MeasureFront(key));
        s.front_of
            .get(&key)
            .map(|found| found.iter().map(|v| HWND(*v as *mut _)).collect())
            .unwrap_or_default()
    })
}
