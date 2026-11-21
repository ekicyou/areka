//! balloon の子: 窓の外へ出たときのホバー解除（`clear_balloon_hover_on_leave`・離脱の排他システム）。

use std::rc::Rc;

use areka_emo_text::place::PlaceKey;
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::world::World;
use wintf::ecs::find_owner_window;
use wintf::ecs::pointer::PointerLeave;

use areka_placement::spawn::BalloonWindowMarker;
use areka_runtime::balloon_hover::BalloonHover;
use areka_runtime::frame::RuntimeWiring;

use super::{BalloonWiring, apply_hover, hover_action};

// ---------------------------------------------------------------------------
// clear_balloon_hover_on_leave 排他システム
//
// 窓外離脱（WM_MOUSELEAVE）では `dispatch_pointer_events` が OnPointerExited/Entered を配送しないため、
// `PointerLeave` マーカーを読む排他システムが hover を解除する。
// ---------------------------------------------------------------------------

/// バルーン窓外離脱時の hover 解除（排他システム）。
///
/// `PointerLeave` を持つ entity のうちバルーン窓の所有のものだけを対象に scope を解決し、
/// `hover_action(active, None, last_injected)` で選択肢・アンカーの強調を解除する（ポインタが窓外へ出た
/// 高速離脱ゆえ hit は `None`）。併せて当該 scope の**バルーン滞在**（`BalloonHover::clear_balloon_hover`）を
/// 解除する。滞在は選択肢行の追跡とは独立の軸で、解除は `RuntimeWiring` の可否より前・選択肢機構の縮退に
/// 依らず行う（本文のコメント参照）。
///
/// `RuntimeWiring` 不在は正常縮退（`debug!`＋no-op）、`BalloonWiring` 不在は結線漏れとして `error!` を 1 回出して
/// 戻る。`RefCell` の借用失敗は `error!`＋skip（panic しない）。`PointerLeave` の除去は FrameFinalize の
/// 責務でここでは行わない。対象の無いフレームは完全 no-op。
///
/// 登録は `register_balloon_leave_system`（本体の系の登録の口からプロセスに 1 回）が
/// Input スケジュールの `dispatch_pointer_events` の後へ行う。
pub(crate) fn clear_balloon_hover_on_leave(world: &mut World) {
    let leaving: Vec<Entity> = world
        .query_filtered::<Entity, With<PointerLeave>>()
        .iter(world)
        .collect();
    if leaving.is_empty() {
        // マーカー不在フレームは完全 no-op（RuntimeWiring にも触れない）。
        return;
    }

    // 非バルーン窓（marker 不在）・所有窓不在の leave は無視する。
    // 複数 entity が同一バルーン窓へ写る場合は scope で dedup（1 scope につき高々 1 回処理）。
    let mut scopes: Vec<usize> = Vec::new();
    for e in leaving {
        if let Some(win) = find_owner_window(world, e)
            && let Some(scope) = world.get::<BalloonWindowMarker>(win).map(|m| m.scope)
            && !scopes.contains(&scope)
        {
            scopes.push(scope);
        }
    }
    if scopes.is_empty() {
        return;
    }

    if world.get_non_send::<BalloonWiring>().is_none() {
        tracing::error!(
            event = "balloon_wiring_missing",
            scopes = ?scopes,
            "BalloonWiring 不在（結線漏れ）: バルーン離脱の後始末を no-op 縮退"
        );
        return;
    }

    // 解除は RuntimeWiring の可否より前に行う（記録は RuntimeWiring が揃うときだけ）。滞在が真のまま
    // 残ると抑止が恒久に固着するため、解く側は縮退に依らない。
    // 滞在の集合は `BalloonWiring` と同じ時に置かれる runtime の型（不在は可視性の相が error! を 1 回出す）。
    if let Some(mut balloon_hover) = world.get_non_send_mut::<BalloonHover>() {
        for &scope in &scopes {
            balloon_hover.clear_balloon_hover(scope);
        }
    }

    // runtime は単一グローバル資源ゆえ 1 度 clone して全 scope で共有する。
    // RuntimeWiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op。
    let Some(runtime) = world
        .get_non_send::<RuntimeWiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "choice_leave_no_runtime",
            "RuntimeWiring 不在（boot 前／失敗）: バルーン離脱 hover 解除を no-op 縮退"
        );
        return;
    };

    for scope in scopes {
        let actor = ActorKey::from(scope.to_string());

        let last_injected = world
            .get_non_send::<BalloonWiring>()
            .expect("BalloonWiring は頭で存在確認済み")
            .hover(scope);

        // 押せる範囲（選択肢かアンカー）があるかを見る——アンカーの強調もここで外す。
        // try_borrow 失敗（理論上不到達の構成異常）は error!＋skip（panic しない）。
        let action = match runtime.try_borrow() {
            Ok(rt) => hover_action(rt.hit_active(&actor), None, last_injected),
            Err(_) => {
                tracing::error!(
                    event = "balloon_runtime_borrow_failed",
                    scope,
                    "runtime try_borrow 失敗（離脱・active スナップショット）: no-op 縮退"
                );
                continue;
            }
        };

        if let Some(value) = apply_hover(&runtime, &PlaceKey::balloon(&actor), scope, action) {
            world
                .get_non_send_mut::<BalloonWiring>()
                .expect("BalloonWiring は頭で存在確認済み")
                .set_hover(scope, value);
        }
    }
}
