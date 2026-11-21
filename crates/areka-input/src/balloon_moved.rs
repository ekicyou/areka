//! balloon の子: ホバーの追従（`on_balloon_pointer_moved`・選択肢の行とアンカーの範囲のハイライトとバルーン滞在の記録）。

use std::rc::Rc;

use areka_emo_text::place::PlaceKey;
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use wintf::ecs::pointer::{Phase, PointerState};

use areka_placement::spawn::BalloonWindowMarker;
use areka_runtime::balloon_hover::BalloonHover;
use areka_runtime::frame::RuntimeWiring;

use super::{BalloonWiring, apply_hover, hit_choice_row, hover_action};

// ---------------------------------------------------------------------------
// 配線層
//
// wintf `PointerEventHandler` 署名の移動ハンドラ。判断分岐は純関数核（hit_choice_row／hover_action）に
// あり、本ハンドラは借用→スナップショット→純関数→適用だけを行う。
// ---------------------------------------------------------------------------

/// バルーン窓のポインタ移動ハンドラ（Bubble のみ処理・hover 追従駆動）。
///
/// Tunnel は伝播続行のため即 `false`。moved は非侵襲ゆえ常に `false`。`client_point`（窓 client 物理 px）は
/// `as f32` で**無変換のまま**純関数核へ渡す（行矩形が `to_window_physical` で既に実適用 k ×済みの窓物理 px
/// で、点と同一空間ゆえ。k=1.0 だからではなく、÷k の追加は二重縮約）。
///
/// 借用の順: `RuntimeWiring` 共有借用で runtime の `Rc` を clone→world 側の借用を解放→`BalloonWiring` から
/// `last_injected` を copy し `BalloonHover` へバルーン滞在を記録→runtime を不変借用して `hit_active` と
/// 現行 `choice_hit_rows` をスナップショット→借用を解いてから `apply_hover` で強調を当て、自前 `hover` を更新する。
/// バルーン滞在の記録は選択肢の有無・行ヒットの有無に依存しない独立の軸。
/// `RuntimeWiring` 不在は正常縮退（`debug!`＋no-op）、`BalloonWiring` 不在は結線漏れの `error!`＋no-op、
/// `RefCell` の借用失敗も `error!`＋no-op（panic しない）。クリック確定・送出は押下ハンドラの仕事。
///
/// [`attach_balloon_pointer_handlers`] が `BalloonWindowMarker` 窓へ `OnPointerMoved` として挿入する。
pub(crate) fn on_balloon_pointer_moved(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let state = match ev {
        Phase::Tunnel(_) => return false,
        Phase::Bubble(s) => s,
    };

    // attach は marker 窓のみを標的とするため、scope の不在は理論上不到達の構成異常＝error!＋no-op（panic しない）。
    let Some(scope) = world.get::<BalloonWindowMarker>(entity).map(|m| m.scope) else {
        tracing::error!(
            event = "balloon_marker_missing",
            "BalloonWindowMarker 不在の entity へ移動ハンドラが着火（理論上不到達）: no-op 縮退"
        );
        return false;
    };
    let actor = ActorKey::from(scope.to_string());

    // client 物理 px（i32）を f32 へ——無変換のまま渡す。行矩形が to_window_physical で既に実適用
    // k ×済みの窓物理 px ゆえ同一空間で一致する。÷k 追加は二重縮約。
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;

    // RuntimeWiring 不在（boot 前／失敗）は正常縮退＝debug!＋no-op。
    let Some(runtime) = world
        .get_non_send::<RuntimeWiring>()
        .map(|w| Rc::clone(w.runtime()))
    else {
        tracing::debug!(
            event = "choice_moved_no_runtime",
            scope,
            "RuntimeWiring 不在（boot 前／失敗）: hover 追従を no-op 縮退"
        );
        return false;
    };

    // ── バルーン滞在の記録 ──
    // 滞在の記録は選択肢機構と**独立の軸**——選択肢の表示有無・
    // 行ヒットの有無に依らず、バルーン窓上の移動そのもので当該 scope を真にする（ゆえに選択肢
    // スナップショットより前で行い、その縮退に巻き込まれない）。解除は窓外離脱
    // （`clear_balloon_hover_on_leave`）と非表示遷移時の掃除（`BalloonHover::clear_balloon_hover`）。
    // 読み口 `is_balloon_hovered` はバルーン可視性の相が毎フレーム読む（抑止条件の観測）。
    // BalloonWiring 不在は結線漏れ＝構成異常 error!＋no-op。
    let Some(last_injected) = world
        .get_non_send::<BalloonWiring>()
        .map(|bw| bw.hover(scope))
    else {
        tracing::error!(
            event = "balloon_wiring_missing",
            scope,
            "BalloonWiring 不在（結線漏れ）: hover 追従を no-op 縮退"
        );
        return false;
    };
    // 滞在の集合は `BalloonWiring` と同じ時に置かれる runtime の型（不在は可視性の相が error! を 1 回出す）。
    if let Some(mut balloon_hover) = world.get_non_send_mut::<BalloonHover>() {
        balloon_hover.set_balloon_hover(scope);
    }

    // ── 現行 rows のスナップショット ──
    // 毎イベント現行 choice_hit_rows に対して hit 判定する（新選択肢集合へ持ち越さない）。
    // try_borrow 失敗（理論上不到達の構成異常）は error!＋no-op（panic しない）。
    let action = match runtime.try_borrow() {
        Ok(rt) => {
            // 押せる範囲（選択肢かアンカー）があるか。アンカーの行も同じ通し番号で強調する。
            let active = rt.hit_active(&actor);
            let rows = rt.choice_hit_rows(&actor);
            let hit_ordinal = hit_choice_row(rows, x, y).map(|i| rows[i].ordinal);
            hover_action(active, hit_ordinal, last_injected)
        }
        Err(_) => {
            tracing::error!(
                event = "balloon_runtime_borrow_failed",
                scope,
                "runtime try_borrow 失敗（不変・スナップショット）: no-op 縮退"
            );
            return false;
        }
    };

    if let Some(value) = apply_hover(&runtime, &PlaceKey::balloon(&actor), scope, action) {
        world
            .get_non_send_mut::<BalloonWiring>()
            .expect("BalloonWiring は直上で存在確認済み")
            .set_hover(scope, value);
    }

    false
}
