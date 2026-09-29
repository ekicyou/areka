//! ファイル・フォルダの投げ込み（`WM_DROPFILES`）の受け手
//!
//! World 側の受け渡し [`deliver_dropped_files`]（OS を知らない・4 つの分かれ道）を置く。
//! wintf は areka を知らない——窓に差された [`OnFilesDropped`] の関数へパスの一覧を渡すだけ。

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use tracing::{debug, info, warn};
use windows::Win32::Foundation::LRESULT;

use super::lifecycle::DESPAWNED_SKIP_TAG;
use crate::ecs::window::OnFilesDropped;
use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// 落とされた物の一覧を、窓に差された [`OnFilesDropped`] の関数へ World 借用中に渡す。
///
/// `WM_ENDSESSION` の受け手と同じ 4 つの分かれ道: World が借用中 → `warn!`（呼べない）／
/// entity が破棄済み（終了の直後に届いた）→ [`DESPAWNED_SKIP_TAG`] の `debug!`／
/// 部品なし → `debug!`／在れば `info!` の上で 1 回呼ぶ。戻り値はどの腕も `Some(LRESULT(0))`
/// （`WM_DROPFILES` を処理したら 0 を返す）。
#[cfg_attr(not(test), allow(dead_code))] // WM_DROPFILES の腕（task 1.2）から呼ぶ
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
