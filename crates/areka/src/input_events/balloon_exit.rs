//! balloon の子: 窓の外へ出たときのホバー解除（`clear_balloon_hover_on_leave`・離脱の排他システム）。
//! 足す予定の spec: なし。

use std::rc::Rc;

use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::world::World;
use wintf::ecs::find_owner_window;
use wintf::ecs::pointer::PointerLeave;

use crate::emo2_boot::frame::Emo2Wiring;
use crate::placement::spawn::BalloonWindowMarker;

use super::{BalloonWiring, HoverAction, hover_action};

// ---------------------------------------------------------------------------
// clear_balloon_hover_on_leave 排他システム（tasks.md task 5・design「clear_balloon_hover_on_leave」）
//
// 窓外離脱（WM_MOUSELEAVE）時の hover 解除——`dispatch_pointer_events` は OnPointerExited/Entered を
// 配送しないため、離脱の hover-clear は `PointerLeave` マーカーを読む薄い排他システムが担う（design
// Existing Architecture Analysis point 1・WM_MOUSELEAVE bullet）。判断分岐そのものは純関数
// `hover_action` へ集約済みで、本システムは leave 対象選別（balloon 所有チェック）→snapshot→純関数→
// 適用のみを行う（自前描画なし・R1.6）。
// ---------------------------------------------------------------------------

/// バルーン窓外離脱時の hover 解除（排他システム・R1.3/3.4）。
///
/// Input スケジュールの `dispatch_pointer_events` 後・FrameFinalize（`PointerLeave` クリア）前に実行
/// される排他システム。`PointerLeave` マーカー保持 entity のうち、所有窓（wintf `find_owner_window` の
/// 親チェーン走査）が `BalloonWindowMarker` を持つものだけを対象に scope を解決し、既存純関数
/// `hover_action(active, None, last_injected)` を再利用して hover 状態を解除する（hit は `None`＝ポインタが
/// 窓外へ出た高速離脱ゆえエッジ非採取・R1.3）。
///
/// 併せて当該 scope の**バルーン滞在**（`BalloonWiring::clear_balloon_hover`・
/// areka-P0-balloon-visibility 5.2）を解除する。滞在は選択肢行の追跡とは独立の軸であり、解除は
/// `Emo2Wiring` の可否より前・選択肢機構の縮退に依らず行う（記録側との非対称は本文中コメント参照・5.5）。
///
/// **借用規律・縮退はハンドラ（task 4.1）と同一**:
/// - `Emo2Wiring` 不在（boot 前／失敗）は**正常縮退**＝`debug!(event = "choice_leave_no_emo2")`＋no-op
///   （donor presenter=None 同型・R4.1）。runtime `Rc` は 1 度 clone し全 scope で共有する（同一資源）。
/// - `BalloonWiring` 不在は結線漏れ＝**構成異常** `error!(event = "balloon_wiring_missing")`＋skip。
/// - runtime `try_borrow`（不変）で `choice_active` を控え、借用解放後に `try_borrow_mut` で
///   `inject_choice_hover`（`Inject` アームのみ・値は必ず `None`）。`RefCell` 借用失敗は
///   `error!(event = "balloon_runtime_borrow_failed")`＋skip（panic しない・log-first）。
/// - `Inject(None)`（表示中・注入済→解除）: `inject_choice_hover(actor, None)`＋自前状態 `None`＋
///   `debug!(event = "choice_hover_inject")`。`ResetOwnState`（非表示・注入済）: 自前状態のみ `None`・
///   inject はしない（上流原子性が正本・R3.4）。`Keep`／`NoopInactive`: 何もしない。
///
/// `PointerLeave` の除去は行わない（除去は既存 FrameFinalize `clear_transient_pointer_state` の責務——
/// 機構不変）。マーカー不在フレーム・バルーン所有 leave 皆無フレームは**完全 no-op**（design Risks）。
///
/// 本番到達済み——[`register_balloon_leave_system`]（`ghost_session::register_systems` から
/// プロセスに 1 回）が Input スケジュールへ `dispatch_pointer_events` の後として登録するため、
/// 以降は毎フレーム走る。
pub(crate) fn clear_balloon_hover_on_leave(world: &mut World) {
    // (1) PointerLeave マーカー保持 entity を収集（query→即 collect で World 可変借用と分離）。
    let leaving: Vec<Entity> = world
        .query_filtered::<Entity, With<PointerLeave>>()
        .iter(world)
        .collect();
    if leaving.is_empty() {
        // マーカー不在フレームは完全 no-op（design Risks・Emo2Wiring にも触れない）。
        return;
    }

    // (2) 所有窓（find_owner_window の親チェーン走査）が BalloonWindowMarker を持つ leave のみを対象に
    // scope を解決する。非バルーン窓（marker 不在）・所有窓不在の leave は無視（key assertion）。
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
        // バルーン所有 leave が皆無なら完全 no-op（非バルーン leave は無視・key assertion）。
        return;
    }

    // ── バルーン滞在の解除（areka-P0-balloon-visibility 5.2）——選択肢機構より前 ────────────────
    // 記録側（`on_balloon_pointer_moved` の借用規律 ②）と縮退方向を意図的に非対称にする: 記録は
    // 上流 `Emo2Wiring` が揃うときだけ行い（抑止を増やす側は保守的）、解除はその可否に依らず行う
    // （抑止を解く側は積極的）。滞在が真のまま残ると恒久抑止へ固着し Requirement 5.5 に反するため。
    // BalloonWiring 不在は結線漏れ＝構成異常 error!＋no-op（silent failure 禁止）。
    if let Some(mut bw) = world.get_non_send_mut::<BalloonWiring>() {
        for &scope in &scopes {
            bw.clear_balloon_hover(scope);
        }
    } else {
        tracing::error!(
            event = "balloon_wiring_missing",
            scopes = ?scopes,
            "BalloonWiring 不在（結線漏れ）: バルーン滞在の解除を no-op 縮退"
        );
    }

    // ── 借用規律 ① Emo2Wiring 共有借用→runtime() で Rc clone→world 側借用解放 ─────────────────
    // Emo2Wiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op（donor presenter=None 同型・R4.1）。
    // runtime は単一グローバル資源ゆえ 1 度 clone して全 scope で共有する（ハンドラの per-event clone と
    // 等価・per-scope 再取得は不要）。
    let Some(runtime) = world
        .get_non_send::<Emo2Wiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "choice_leave_no_emo2",
            "Emo2Wiring 不在（boot 前／失敗）: バルーン離脱 hover 解除を no-op 縮退"
        );
        return;
    };

    // (3) 各バルーン scope へハンドラ（task 4.1）と同一の借用規律・縮退で hover 解除を適用する。
    for scope in scopes {
        let actor = ActorKey::from(scope.to_string());

        // ── 借用規律 ② BalloonWiring から last_injected を copy（共有借用即解放）─────────────────
        // BalloonWiring 不在は結線漏れ＝構成異常 error!（配線存在檻が開発時に検出）＋skip。
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

        // ── 借用規律 ③ runtime 不変借用で choice_active スナップショット（hit は None＝窓外離脱・R1.3）──
        // try_borrow 失敗（理論上不到達の構成異常）は error!＋skip（panic しない・log-first）。
        let action = match runtime.try_borrow() {
            Ok(rt) => hover_action(rt.choice_active(&actor), None, last_injected),
            Err(_) => {
                tracing::error!(
                    event = "balloon_runtime_borrow_failed",
                    scope,
                    "runtime try_borrow 失敗（離脱・active スナップショット）: no-op 縮退"
                );
                continue;
            }
        };
        // ここで不変借用（Ref）は解放済み——④ の可変借用と同時に持たない。

        // 純関数の決定を適用する（hit=None ゆえ Inject は常に None＝ハイライト解除・R1.3）。
        match action {
            // 非表示・未注入／同値既注入（None==None）は何もしない。
            HoverAction::NoopInactive | HoverAction::Keep => {}
            // 消滅時整合（R3.4）: 自前状態のみ None 整合・inject はしない（上流原子性が正本）。
            HoverAction::ResetOwnState => {
                let mut bw = world
                    .get_non_send_mut::<BalloonWiring>()
                    .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
                bw.set_hover(scope, None);
            }
            // 離脱遷移（R1.3）: ④ runtime 可変借用で inject(None)→⑤ BalloonWiring 自前状態を None 更新。
            HoverAction::Inject(value) => {
                // hit=None ゆえ value は必ず None（ハイライト解除注入・描画 API は呼ばない）。
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
                // ⑤ 可変借用は上で解放済み。BalloonWiring の自前 last-injected を更新する。
                let mut bw = world
                    .get_non_send_mut::<BalloonWiring>()
                    .expect("BalloonWiring は直上（②）で存在確認済み（donor self-gating 同型）");
                bw.set_hover(scope, value);
                // hover 遷移注入の marker（DD-CI-7・トラブルシュート用・info ではない）。
                tracing::debug!(
                    event = "choice_hover_inject",
                    scope,
                    ordinal = ?value,
                    "窓外離脱で hover 解除を上流 runtime へ注入"
                );
            }
        }
    }
    // PointerLeave は除去しない（除去は FrameFinalize clear_transient_pointer_state の責務・機構不変）。
}
