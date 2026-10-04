//! キャラクター窓のドラッグの開始・終了を kanade へ知らせる配線（areka-P0-mouse-drag-events）。
//!
//! 3 つの部品を置く:
//!
//! - [`on_char_drag_start`]: `OnDragStart` の受け手。Bubble の相でだけ開始を 1 回知らせる。
//! - [`on_char_drag_end_and_notify`]: `OnDragEnd` の包み。`placement::spawn` が付けた位置の保存
//!   （[`on_char_drag_end`]）を**先に**同じ引数で呼び、その後 Bubble の相でだけ終了を 1 回知らせる。
//! - [`notify_drag`]: 画面の物理 px から、スコープ・SHIORI へ渡す座標・当たり判定を引いて
//!   `KanadeMsg::Mouse` を送る。送らないと決めたときは理由を 1 件記録する。
//!
//! 受け手の付け方は親モジュールの [`super::attach_char_pointer_handlers`]（キャラクター窓だけ）。
//! 「開始を送った」印は持たない（設計 D2）。2 相の呼び出しは Bubble だけを扱えば 1 回になる。

use areka_kanade::{KanadeMsg, MouseEventKind, MouseInput};
use bevy_ecs::prelude::*;
use wintf::ecs::WindowPos;
use wintf::ecs::drag::{DragEndEvent, DragStartEvent};
use wintf::ecs::pointer::{Phase, PhysicalPoint};

use super::{MouseWiring, char_scope, resolve_hit_owned};
use crate::placement::follow::on_char_drag_end;

/// キャラクター窓のドラッグの開始を kanade へ 1 回知らせる（要件 1.1〜1.4）。
///
/// Tunnel の相は何もしない（同じ知らせの前半であり、後半の Bubble で 1 回だけ送る）。
/// wintf が開始を配る時点で窓はまだ動いていないので、`ev.position − WindowPos.position` が
/// 押した位置の窓の中の物理 px になる。伝播は止めない（常に false）。
pub(super) fn on_char_drag_start(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<DragStartEvent>,
) -> bool {
    if let Phase::Bubble(ev) = ev {
        notify_drag(
            world,
            entity,
            ev.target,
            ev.position,
            MouseEventKind::DragStart,
        );
    }
    false
}

/// 位置の保存を今までどおり行い、その後でドラッグの終了を kanade へ 1 回知らせる
/// （要件 2.1〜2.5・6.1・6.2）。
///
/// どの相でも先に [`on_char_drag_end`] を受けた引数のまま呼び、戻り値を控えて返す。保存は
/// 知らせの結果に左右されない（送り先が無い・送出に失敗した・送らないと決めた、のどれでも
/// 保存は済んでいる）。
///
/// `ev.cancelled` は見ない（取り消しでも同じに送る）。座標は保存を呼んだ**後**の
/// `WindowPos.position` から引く（設計 D1）: 取り消しでは保存が窓を開始の位置へ戻すので、
/// 押した位置（開始と同じ値）になる。
pub(super) fn on_char_drag_end_and_notify(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<DragEndEvent>,
) -> bool {
    let handled = on_char_drag_end(world, sender, entity, ev);
    if let Phase::Bubble(ev) = ev {
        notify_drag(
            world,
            entity,
            ev.target,
            ev.position,
            MouseEventKind::DragEnd,
        );
    }
    handled
}

/// 画面の物理 px から、スコープ・SHIORI へ渡す座標・当たり判定を引いて `KanadeMsg::Mouse` を送る
/// （要件 1.3・1.4・2.5・4.2・4.4・4.5・8.2）。
///
/// 判断の順（上から。送らないときは 1 件記録して戻る）:
///
/// 1. `target != entity` → warn `mouse_drag_dropped`（`reason = "target_mismatch"`）。
/// 2. [`MouseWiring`] 不在（起動に失敗したゴースト・結線の前）→ debug（`reason = "no_wiring"`）。
/// 3. [`char_scope`] が `None` → warn（`reason = "no_scope"`）。
/// 4. `WindowPos.position` 不在 → warn（`reason = "no_window_pos"`）。
/// 5. 窓の中の物理 px ＝ `screen − WindowPos.position` を [`resolve_hit_owned`] へ渡し、返った
///    `surface_point` と当たり判定で送る。送出失敗は warn `mouse_send_failed`。
///
/// 当たり判定の有無では分けない（要件 1.3）。移動の間引き（`plan_and_send_move`）は通らず、
/// その状態にも触れない（要件 1.4・2.5）。
fn notify_drag(
    world: &mut World,
    entity: Entity,
    target: Entity,
    screen: PhysicalPoint,
    kind: MouseEventKind,
) {
    let kind_name = match kind {
        MouseEventKind::DragStart => "drag_start",
        MouseEventKind::DragEnd => "drag_end",
        MouseEventKind::Move => "move",
        MouseEventKind::DoubleClick { .. } => "double_click",
    };

    if target != entity {
        tracing::warn!(
            event = "mouse_drag_dropped",
            reason = "target_mismatch",
            kind = kind_name,
            ?entity,
            ?target,
            "ドラッグの対象が受け手の窓と違う: 送らない"
        );
        return;
    }
    if world.get_non_send::<MouseWiring>().is_none() {
        tracing::debug!(
            event = "mouse_drag_dropped",
            reason = "no_wiring",
            kind = kind_name,
            "MouseWiring 不在（起動失敗・結線前）: 送らない"
        );
        return;
    }
    let Some(scope) = char_scope(world, entity) else {
        tracing::warn!(
            event = "mouse_drag_dropped",
            reason = "no_scope",
            kind = kind_name,
            ?entity,
            "CharWindowMarker 不在: 送らない"
        );
        return;
    };
    let Some(origin) = world.get::<WindowPos>(entity).and_then(|wp| wp.position) else {
        tracing::warn!(
            event = "mouse_drag_dropped",
            reason = "no_window_pos",
            kind = kind_name,
            scope,
            "WindowPos.position 不在: 送らない"
        );
        return;
    };
    let x = i64::from(screen.x) - i64::from(origin.x);
    let y = i64::from(screen.y) - i64::from(origin.y);

    // presenter 借用を解いて解決結果を owned で取り出してから &mut MouseWiring を取る（DD-IE-9）。
    let hit = resolve_hit_owned(world, scope, x, y);
    let wiring = world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は直上で存在確認済み");
    let (x, y) = hit.surface_point;
    let msg = KanadeMsg::Mouse(MouseInput {
        scope,
        x,
        y,
        region: hit.region,
        kind,
    });
    if wiring.sender.send(msg).is_err() {
        tracing::warn!(
            event = "mouse_send_failed",
            scope,
            kind = kind_name,
            "kanade Sender 送出失敗（actor 停止後）: no-op で継続"
        );
    }
}

#[cfg(test)]
#[path = "drag_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "drag_tests.rs"]
mod tests;
