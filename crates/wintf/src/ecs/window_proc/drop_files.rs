//! ファイル・フォルダの投げ込み（`WM_DROPFILES`）の受け手
//!
//! 振り分け表の腕 [`WM_DROPFILES`]、OS から一覧を読む [`read_dropped_paths`]（`unsafe` は
//! このファイルだけ）、World 側の受け渡し [`deliver_dropped_files`]（OS を知らない・4 つの
//! 分かれ道）を置く。wintf は areka を知らない——窓に差された [`OnFilesDropped`] の関数へ
//! パスの一覧を渡すだけ。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::{debug, info, warn};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DragFinish, DragQueryFileW, HDROP};

use super::lifecycle::DESPAWNED_SKIP_TAG;
use crate::ecs::window::OnFilesDropped;
use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// 一覧を読めなかった理由。`files_dropped_read_failed` の `error` 欄に `Display` で載る。
#[derive(Debug, thiserror::Error)]
pub(crate) enum DropReadError {
    /// wParam が 0（取っ手が無い）。
    #[error("NullHandle: 取っ手が無い（wParam が 0）")]
    NullHandle,
    /// `index` 番目（0 起点）のパスの長さか中身が読めなかった（`total` 本のうち）。
    #[error("Query: 全 {total} 本のうち {index} 番目（0 起点）のパスの長さか中身が読めなかった")]
    Query { index: u32, total: u32 },
}

/// 途中で失敗しても `DragFinish` を必ず 1 回呼ぶ（`Drop`）。null の取っ手では作らない。
struct FinishOnDrop(HDROP);

impl Drop for FinishOnDrop {
    fn drop(&mut self) {
        // SAFETY: WM_DROPFILES で受けた null でない HDROP を、この守りが 1 回だけ返す。
        unsafe { DragFinish(self.0) };
    }
}

/// 振り分け表から呼ばれる腕。取っ手 0 → 返す資源が無いので `DragFinish` を呼ばず `warn!` 1 件。
/// それ以外は最初に [`FinishOnDrop`] を握り、読めなければ `warn!` 1 件で受け手を呼ばない。
/// 読めれば（0 件でも）[`deliver_dropped_files`] へ渡す。戻り値はどの腕も `Some(LRESULT(0))`。
#[inline]
pub(super) fn WM_DROPFILES(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    if wparam.0 == 0 {
        warn_read_failed(entity, &DropReadError::NullHandle);
        return Some(LRESULT(0));
    }
    let hdrop = HDROP(wparam.0 as _);
    let _finish = FinishOnDrop(hdrop);
    match read_dropped_paths(hdrop) {
        Ok(paths) => deliver_dropped_files(world, entity, paths),
        Err(e) => {
            warn_read_failed(entity, &e);
            Some(LRESULT(0))
        }
    }
}

/// 一覧を読めなかったときの `warn!`（`files_dropped_read_failed`）。受け手は呼ばない。
fn warn_read_failed(entity: Entity, error: &DropReadError) {
    warn!(
        event = "files_dropped_read_failed",
        entity = ?entity,
        error = %error,
        "[WM_DROPFILES] 落とされた物の一覧を OS から読めない → 受け手を呼ばず捨てる"
    );
}

/// null でない `HDROP` からパスの一覧を読む（OS 境界）。件数と長さを問うてから読み、
/// `OsString::from_wide` で非可逆変換を挟まない。0 件は `Ok(vec![])`。`DragFinish` は呼ばない。
fn read_dropped_paths(hdrop: HDROP) -> Result<Vec<PathBuf>, DropReadError> {
    // SAFETY: 呼び手が WM_DROPFILES で受けた null でない HDROP（DragFinish 前）を渡す。
    let total = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
    (0..total)
        .map(|index| {
            let err = DropReadError::Query { index, total };
            // SAFETY: 同上。バッファ無しの問いは終端を除く長さを返す。
            let len = unsafe { DragQueryFileW(hdrop, index, None) };
            if len == 0 {
                return Err(err);
            }
            let mut buf = vec![0u16; len as usize + 1];
            // SAFETY: 同上。buf は終端込みの長さを持つ。
            let got = unsafe { DragQueryFileW(hdrop, index, Some(&mut buf)) } as usize;
            if got == 0 {
                return Err(err);
            }
            Ok(PathBuf::from(OsString::from_wide(
                &buf[..got.min(len as usize)],
            )))
        })
        .collect()
}

/// 落とされた物の一覧を、窓に差された [`OnFilesDropped`] の関数へ World 借用中に渡す。
///
/// `WM_ENDSESSION` の受け手と同じ 4 つの分かれ道: World が借用中 → `warn!`（呼べない）／
/// entity が破棄済み（終了の直後に届いた）→ [`DESPAWNED_SKIP_TAG`] の `debug!`／
/// 部品なし → `debug!`／在れば `info!` の上で 1 回呼ぶ。戻り値はどの腕も `Some(LRESULT(0))`
/// （`WM_DROPFILES` を処理したら 0 を返す）。
pub(super) fn deliver_dropped_files(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    paths: Vec<PathBuf>,
) -> HandlerResult {
    let count = paths.len();
    let Ok(mut w) = world.try_borrow_mut() else {
        warn!(
            event = "files_dropped_world_busy",
            entity = ?entity,
            count,
            "[WM_DROPFILES] World が借用中で投げ込みの受け手を呼べない → この投げ込みは捨てる"
        );
        return Some(LRESULT(0));
    };
    if w.world().get_entity(entity).is_err() {
        debug!(
            entity = ?entity,
            count,
            "{DESPAWNED_SKIP_TAG} WM_DROPFILES: 対象 entity は既に破棄済み（despawn）→ \
             投げ込みを正常系として打ち切り"
        );
    } else if let Some(cb) = w.world().get::<OnFilesDropped>(entity).copied() {
        info!(
            event = "files_dropped",
            entity = ?entity,
            count,
            "[WM_DROPFILES] 落とされた物の一覧を利用側の関数へ渡す"
        );
        (cb.0)(w.world_mut(), entity, paths);
    } else {
        debug!(
            event = "files_dropped_no_receiver",
            entity = ?entity,
            count,
            "[WM_DROPFILES] 投げ込みの関数を持たない窓 → 何もしない"
        );
    }
    Some(LRESULT(0))
}

#[cfg(test)]
#[path = "drop_files_tests.rs"]
mod drop_files_tests;
