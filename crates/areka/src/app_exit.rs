//! areka の終了の統合操作（areka-P0-app-lifetime-separation・design「quit_app と ExitOrigin」）。
//!
//! 「終了のために全窓を閉じる」と「終了を指示する」を [`quit_app`] 1 つの操作にまとめ、
//! 片方だけを呼べない形にする（裁定 2）。全窓を閉じる部品 [`despawn_app_windows`] は私有で、
//! クレート内のどこからも単独では呼べない（要件 3.6 を構造で守る）。
//!
//! 出所 [`ExitOrigin`] は記録の語彙であり、受け手は出所で分岐しない（要件 3.7）。

use areka_kanade::{CloseReason, KanadeStopCause, ShioriFault};
use bevy_ecs::prelude::*;
use wintf::AppExit;
use wintf::ecs::WindowHandle;
use wintf::ecs::window::OnCloseRequest;

use crate::input_events::MouseWiring;
use crate::placement::diag::DESPAWNED_SKIP_TAG;
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker, GhostWindowMarker};

/// どの終了操作から来たか（記録の語彙・受け手は分岐しない）。
///
/// `Debug` 出力がそのまま実機ログの検索語になる（`origin=KanadeStopped(Quit)` のように
/// `ghost_quit` の `cause` と同じ語に揃う）。Fault は種類と理由（`String`）を運ぶので `Copy` ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExitOrigin {
    /// kanade の終了系列の完了（メニューの終了・別れの台詞のあと・中断のあと）。
    KanadeStopped(KanadeStopCause),
    /// 強制退避（Ctrl+Shift+左ダブルクリック）。
    Escape,
    /// smoke の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）。
    Smoke,
    /// kanade 未結線の起動でのゴースト窓への OS の閉鎖要求
    /// （結線済みならゴースト窓の閉鎖要求は `KanadeStopped` 経由で来る）。
    OsClose,
}

/// 最初に終了を指示した出所（World に 1 つ・書き込みは 1 度）。
///
/// [`quit_app`] が無ければ挿し、2 度目以降は上書きせず `debug!(event="app_exit_again")` で流す
/// （[`AppExit::request_exit`] の「最初が勝つ」と同じ規則）。`run()` の後で `main` が読み、
/// [`fault_of`] で告知と終了コードを決める。
#[derive(Resource)]
pub(crate) struct FirstExit(pub(crate) ExitOrigin);

/// 告知するか・終了コードを 1 にするかの判定（呼び手は分けて判断しない）。
///
/// 失敗の中身を返すのは「kanade の停止で原因が Fault」だけ。他の停止原因・強制退避・smoke の
/// 自動終了・OS の閉鎖要求は `None`（告知なし・終了コード 0）。
pub(crate) fn fault_of(origin: &ExitOrigin) -> Option<&ShioriFault> {
    match origin {
        ExitOrigin::KanadeStopped(KanadeStopCause::Fault(f)) => Some(f),
        ExitOrigin::KanadeStopped(
            KanadeStopCause::Quit
            | KanadeStopCause::Forced
            | KanadeStopCause::CloseSilent
            | KanadeStopCause::DeadlineExceeded,
        )
        | ExitOrigin::Escape
        | ExitOrigin::Smoke
        | ExitOrigin::OsClose => None,
    }
}

/// 全窓（ゴースト窓）を閉じ、終了を指示する。戻り値は標的として拾った窓の数。
///
/// 閉じた数が 0 でも指示する（要件 3.2・分岐を置かない）。出所と閉じた数は `info` で残す。
/// 最初の出所は受け口の有無に依らず [`FirstExit`] に残す（2 度目は `debug` で流すだけ）。
/// 受け口 [`AppExit`] が World に無いとき（本番では `WinApp` が必ず挿すので配線の誤り）は
/// `error!` を残して戻る——窓は閉じたが終了は指示できない。
pub(crate) fn quit_app(world: &mut World, origin: ExitOrigin) -> usize {
    let closed = despawn_app_windows(world);
    if world.contains_resource::<FirstExit>() {
        tracing::debug!(
            event = "app_exit_again",
            origin = ?origin,
            "[quit_app] 終了は指示済み——最初の出所を残し、この出所は記録だけ"
        );
    } else {
        world.insert_resource(FirstExit(origin.clone()));
    }
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

/// 全窓（`GhostWindowMarker`）を despawn する私有部品。
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
        .query_filtered::<Entity, With<GhostWindowMarker>>()
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

/// ゴースト窓への OS の閉鎖要求（Alt＋F4・`taskkill`・「タスクの終了」）の受け手（裁定 3）。
///
/// 窓のキャラ／バルーンの印からスコープを読み、メニューの「終了」（`menu::request_close`）と
/// 同じ `CloseReason::User { scope }` の終了要求を kanade へ 1 件送る——終了系列は増やさない。
/// **窓は消さない**: 閉じるのは終了の握手の完了を受けた終了系列の完了通知 → [`quit_app`]。
/// 送り口（[`MouseWiring`]）が無い（窓は出たが kanade との結線に失敗した起動）なら、
/// 別れの台詞を流す相手がいないので `warn!` を残して [`quit_app`] で直ちに閉じる（要件 3.10・
/// 裁定 3「標準の道具で閉じられないアプリにしない」）。
pub(crate) fn on_ghost_os_close(world: &mut World, entity: Entity) {
    let scope = if let Some(m) = world.get::<CharWindowMarker>(entity) {
        m.scope
    } else if let Some(m) = world.get::<BalloonWindowMarker>(entity) {
        m.scope
    } else {
        tracing::warn!(
            event = "os_close_unknown_window",
            entity = ?entity,
            "[os_close] キャラ／バルーンの印が無い窓への閉鎖要求: 終了要求は送らない"
        );
        return;
    };
    let Some(mut wiring) = world.get_non_send_mut::<MouseWiring>() else {
        tracing::warn!(
            event = "os_close_no_mouse_wiring",
            scope,
            "[os_close] MouseWiring が無い（kanade 未結線）: 別れの台詞なしで全窓を閉じて終了を指示する"
        );
        quit_app(world, ExitOrigin::OsClose);
        return;
    };
    tracing::info!(
        event = "os_close_request",
        scope,
        kind = "ghost",
        "[os_close] OS の閉鎖要求をメニューの「終了」と同じ終了要求として kanade へ送る"
    );
    wiring.send_close_request(CloseReason::User {
        scope: scope as u32,
    });
}

/// HWND が付いた瞬間のゴースト窓へ [`on_ghost_os_close`] を差す system。
///
/// `register_ghost_windows_click_through` と同じ `Added<WindowHandle>` の捉え方で、同じ
/// `FrameFinalize` 段に登録する。`placement` は `crate::` パスを持てない（example の `#[path]`
/// include）ため、差し込みは `placement` の外のここで行う。
pub(crate) fn attach_os_close_request(
    mut commands: Commands,
    new_windows: Query<Entity, (With<GhostWindowMarker>, Added<WindowHandle>)>,
) {
    for e in &new_windows {
        commands.entity(e).insert(OnCloseRequest(on_ghost_os_close));
    }
}

#[cfg(test)]
#[path = "app_exit_tests.rs"]
mod tests;
