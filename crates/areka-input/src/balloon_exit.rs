//! balloon の子: 窓の外へ出たときのホバー解除（`clear_balloon_hover_on_leave`・離脱の排他システム）。

use std::rc::Rc;

use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::world::World;
use wintf::ecs::find_owner_window;
use wintf::ecs::pointer::PointerLeave;

use areka_placement::spawn::BalloonWindowMarker;
use areka_runtime::balloon_hover::BalloonHover;
use areka_runtime::frame::RuntimeWiring;

use super::{BalloonWiring, HoverAction, hover_action};

// ---------------------------------------------------------------------------
// clear_balloon_hover_on_leave 排他システム
//
// 窓外離脱（WM_MOUSELEAVE）時の hover 解除——`dispatch_pointer_events` は OnPointerExited/Entered を
// 配送しないため、離脱の hover-clear は `PointerLeave` マーカーを読む薄い排他システムが担う。判断分岐そのものは純関数
// `hover_action` へ集約済みで、本システムは leave 対象選別（balloon 所有チェック）→snapshot→純関数→
// 適用のみを行う（自前描画なし）。
// ---------------------------------------------------------------------------

/// バルーン窓外離脱時の hover 解除（排他システム）。
///
/// Input スケジュールの `dispatch_pointer_events` 後・FrameFinalize（`PointerLeave` クリア）前に実行
/// される排他システム。`PointerLeave` マーカー保持 entity のうち、所有窓（wintf `find_owner_window` の
/// 親チェーン走査）が `BalloonWindowMarker` を持つものだけを対象に scope を解決し、既存純関数
/// `hover_action(active, None, last_injected)` を再利用して hover 状態を解除する（hit は `None`＝ポインタが
/// 窓外へ出た高速離脱ゆえエッジ非採取）。
///
/// 併せて当該 scope の**バルーン滞在**（`BalloonHover::clear_balloon_hover`）を解除する。滞在は選択肢行の追跡とは独立の軸であり、解除は
/// `RuntimeWiring` の可否より前・選択肢機構の縮退に依らず行う（記録側との非対称は本文中コメント参照）。
///
/// **借用規律・縮退はハンドラと同一**:
/// - `RuntimeWiring` 不在（boot 前／失敗）は**正常縮退**＝`debug!(event = "choice_leave_no_runtime")`＋no-op
///   （donor presenter=None 同型）。runtime `Rc` は 1 度 clone し全 scope で共有する（同一資源）。
/// - `BalloonWiring` 不在は結線漏れ＝**構成異常** `error!(event = "balloon_wiring_missing")`＋skip。
/// - runtime `try_borrow`（不変）で `hit_active` を控え、借用解放後に `try_borrow_mut` で
///   `inject_choice_hover`（`Inject` アームのみ・値は必ず `None`）。`RefCell` 借用失敗は
///   `error!(event = "balloon_runtime_borrow_failed")`＋skip（panic しない）。
/// - `Inject(None)`（表示中・注入済→解除）: `inject_choice_hover(actor, None)`＋自前状態 `None`＋
///   `debug!(event = "choice_hover_inject")`。`ResetOwnState`（非表示・注入済）: 自前状態のみ `None`・
///   inject はしない（上流原子性が正本）。`Keep`／`NoopInactive`: 何もしない。
///
/// `PointerLeave` の除去は行わない（除去は既存 FrameFinalize `clear_transient_pointer_state` の責務——
/// 機構不変）。マーカー不在フレーム・バルーン所有 leave 皆無フレームは**完全 no-op**。
///
/// 登録は `register_balloon_leave_system`（`ghost_session::register_systems` からプロセスに 1 回）が
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
        if let Some(win) = find_owner_window(world, e) {
            if let Some(scope) = world.get::<BalloonWindowMarker>(win).map(|m| m.scope) {
                if !scopes.contains(&scope) {
                    scopes.push(scope);
                }
            }
        }
    }
    if scopes.is_empty() {
        return;
    }

    // ── バルーン滞在の解除——選択肢機構より前 ────────────────
    // 記録側（`on_balloon_pointer_moved` の借用規律 ②）と縮退方向を意図的に非対称にする: 記録は
    // 上流 `RuntimeWiring` が揃うときだけ行い（抑止を増やす側は保守的）、解除はその可否に依らず行う
    // （抑止を解く側は積極的）。滞在が真のまま残ると恒久抑止へ固着するため。
    // BalloonWiring 不在は結線漏れ＝構成異常 error!＋no-op（silent failure 禁止）。
    if world.get_non_send::<BalloonWiring>().is_some() {
        // 滞在の集合は `BalloonWiring` と同じ時に置かれる runtime の型（不在は可視性の相が error! を 1 回出す）。
        if let Some(mut balloon_hover) = world.get_non_send_mut::<BalloonHover>() {
            for &scope in &scopes {
                balloon_hover.clear_balloon_hover(scope);
            }
        }
    } else {
        tracing::error!(
            event = "balloon_wiring_missing",
            scopes = ?scopes,
            "BalloonWiring 不在（結線漏れ）: バルーン滞在の解除を no-op 縮退"
        );
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

        // BalloonWiring 不在は結線漏れ＝構成異常 error!＋skip。
        let Some(last_injected) = world
            .get_non_send::<BalloonWiring>()
            .map(|bw| bw.hover(scope))
        else {
            tracing::error!(
                event = "balloon_wiring_missing",
                scope,
                "BalloonWiring 不在（結線漏れ）: バルーン離脱 hover 解除を no-op 縮退"
            );
            continue;
        };

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

        match action {
            HoverAction::NoopInactive | HoverAction::Keep => {}
            // 消滅時整合: 自前状態のみ None 整合・inject はしない（上流原子性が正本）。
            HoverAction::ResetOwnState => {
                let mut bw = world
                    .get_non_send_mut::<BalloonWiring>()
                    .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
                bw.set_hover(scope, None);
            }
            HoverAction::Inject(value) => {
                match runtime.try_borrow_mut() {
                    Ok(mut rt) => rt.inject_choice_hover(&actor, value),
                    Err(_) => {
                        tracing::error!(
                            event = "balloon_runtime_borrow_failed",
                            scope,
                            "runtime try_borrow_mut 失敗（離脱 inject）: no-op 縮退"
                        );
                        continue;
                    }
                }
                let mut bw = world
                    .get_non_send_mut::<BalloonWiring>()
                    .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
                bw.set_hover(scope, value);
                // hover 遷移注入の marker（トラブルシュート用・info ではない）。
                tracing::debug!(
                    event = "choice_hover_inject",
                    scope,
                    ordinal = ?value,
                    "窓外離脱で hover 解除を上流 runtime へ注入"
                );
            }
        }
    }
}
