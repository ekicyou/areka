//! areka の終了の統合操作（areka-P0-app-lifetime-separation・design「quit_app と ExitOrigin」）。
//!
//! 「終了のために全窓を閉じる」と「終了を指示する」を [`quit_app`] 1 つの操作にまとめ、
//! 片方だけを呼べない形にする（裁定 2）。全窓を閉じる部品 [`despawn_app_windows`] は私有で、
//! クレート内のどこからも単独では呼べない（要件 3.6 を構造で守る）。
//!
//! 出所 [`ExitOrigin`] は記録の語彙であり、受け手は出所で分岐しない（要件 3.7）。

use areka_kanade::KanadeStopCause;
use bevy_ecs::prelude::*;
use wintf::AppExit;

use crate::DummyWindowMarker;
use crate::placement::diag::DESPAWNED_SKIP_TAG;
use crate::placement::spawn::GhostWindowMarker;

/// どの終了操作から来たか（記録の語彙・受け手は分岐しない）。
///
/// `Debug` 出力がそのまま実機ログの検索語になる（`origin=KanadeStopped(Quit)` のように
/// `ghost_quit` の `cause` と同じ語に揃う）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitOrigin {
    /// kanade の終了系列の完了（メニューの終了・別れの台詞のあと・中断のあと）。
    KanadeStopped(KanadeStopCause),
    /// 強制退避（Ctrl+Shift+左ダブルクリック）。
    Escape,
    /// ダミー窓の左ダブルクリック。
    DummyWindow,
    /// smoke の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）。
    Smoke,
    /// ダミー窓への OS の閉鎖要求（ゴースト窓の閉鎖要求は `KanadeStopped` 経由で来る）。
    OsClose,
}

/// 全窓（ダミー窓＋ゴースト窓）を閉じ、終了を指示する。戻り値は標的として拾った窓の数。
///
/// 閉じた数が 0 でも指示する（要件 3.2・分岐を置かない）。出所と閉じた数は `info` で残す。
/// 受け口 [`AppExit`] が World に無いとき（本番では `WinApp` が必ず挿すので配線の誤り）は
/// `error!` を残して戻る——窓は閉じたが終了は指示できない。
pub(crate) fn quit_app(world: &mut World, origin: ExitOrigin) -> usize {
    let closed = despawn_app_windows(world);
    let Some(exit) = world.get_non_send::<AppExit>() else {
        tracing::error!(
            event = "app_exit_unwired",
            origin = ?origin,
            closed,
            "[quit_app] 終了の受け口が World に無い——窓は閉じたが終了を指示できない"
        );
        return closed;
    };
    tracing::info!(
        event = "app_exit",
        origin = ?origin,
        closed,
        "[quit_app] 全窓を閉じ、終了を指示した"
    );
    exit.request_exit();
    closed
}

/// 全窓（`DummyWindowMarker`／`GhostWindowMarker`）を despawn する私有部品。
/// 戻り値は標的として拾った件数（標的なしは 0・no-op 安全）。bare `World` だけで動く。
///
/// # 存在確認
///
/// query で集めた標的は**ループ実行中に**破棄済みへ変わり得る——bevy の連鎖 despawn
/// （`Children` は `LINKED_SPAWN` の関係対象＝親の despawn が子孫へ再帰する）を先行の
/// 1 体が引き起こせば、後続のイテレーションは既に無効な `Entity` を叩き、`World::despawn`
/// は `log` の `warn!`（`Could not despawn entity: …`）を出す。これは終了処理の正常系で
/// あり、警告として残すと良性ノイズが本物の異常を埋める。ゆえに entity 不在の標的は
/// [`DESPAWNED_SKIP_TAG`] の `debug!` で打ち切り、**残りの標的は処理し切る**。
///
/// 戻り値の意味は「標的として拾った件数」——連鎖で消えた標的も掃除後には存在しないため、
/// 「消えた窓の数」と一致する。
fn despawn_app_windows(world: &mut World) -> usize {
    let targets: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<DummyWindowMarker>, With<GhostWindowMarker>)>>()
        .iter(world)
        .collect();
    let count = targets.len();
    for e in targets {
        if world.get_entity(e).is_err() {
            tracing::debug!(
                entity = ?e,
                "{} [quit_app] 全窓の破棄: 標的 entity は既に破棄済み（despawn・連鎖破棄）→ \
                 正常系として打ち切り（残りの標的は継続）",
                DESPAWNED_SKIP_TAG
            );
            continue;
        }
        world.despawn(e);
    }
    count
}

#[cfg(test)]
#[path = "app_exit_tests.rs"]
mod tests;
