//! シェルの窓のハンドラの箱の前段（areka-P0-shell-balloon・design「箱のポインタの前段」）。
//!
//! `mod.rs` の `on_char_pointer_moved`・`on_char_pointer_pressed` の先頭（強制退避の判定の後）から
//! 呼ばれ、「処理した」（`true`）なら呼び手は既存の道へ落とさない。判断は `shell_box.rs` の純関数に
//! あり、ここは借用と送り出しだけを行う（借用の順は `balloon_pressed.rs` と同じ: `Emo2Wiring` から
//! 文字の層の `Rc` を複製して world の借用を解く → 文字の層の不変借用で写しを取って解く →
//! 持ち物へ書く・送る）。
//!
//! - 移動: 選択肢の行の上は強調だけでシェルへ送らない。箱の中で行の上でない移動は滞在を記録して
//!   から既存の道へ落とす（要件 8.2・9.1・9.7）。
//! - 押下: 箱の中の押下はすべて中断の入口（`user_break.rs` の `on_box_press`）を通す。左押下が
//!   選択の確定なら既存の選択の送り口で送り、結論がシェルへの操作なら既存の道へ（要件 8.3・8.5・9.6）。
//! - シェルの窓から出たら滞在を無しにし、強調も外す（`balloon_exit.rs` と同じ後始末・要件 6.11）。
//!
//! 箱の外（`shown_boxes` の四角の外）は何もしない＝本 spec の前と同じ通知（要件 9.2・9.8）。

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_compose::BoxName;
use areka_emo_text::actor::{ChoiceHitRow, ShownBox, TextLayerRuntime};
use areka_emo_text::place::{PlaceKey, TextPlace};
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::pointer::{PointerLeave, PointerState, dispatch_pointer_events};
use wintf::ecs::{Input, find_owner_window};

use super::balloon::{BalloonWiring, ChoiceSelection, HoverAction, hover_action};
use super::shell_box::{
    BoxMove, BoxPressVerdict, ShellBoxHover, box_under_point, judge_box_click, judge_box_move,
    next_box_hover,
};
use super::user_break::on_box_press;
use crate::emo2_boot::frame::Emo2Wiring;
use crate::placement::spawn::CharWindowMarker;

/// 届いた座標の写し（文字の層の借用を解いた後に判断と送り出しへ渡す）。
#[derive(Debug, Clone, Default)]
pub(super) struct BoxPoint {
    /// 座標の下の、文字の出ている箱（手前の 1 つ・無ければ箱の外）。
    pub(super) hit: Option<ShownBox>,
    /// 当たった箱の当たり行（選択肢とアンカー・シェルの窓の物理 px）。
    pub(super) rows: Vec<ChoiceHitRow>,
    /// スコープに押せる範囲（選択肢かアンカー）が出ているか（`hit_active`）。
    pub(super) active: bool,
}

/// 箱の場所の鍵。
fn box_place(scope: usize, name: &BoxName) -> PlaceKey {
    PlaceKey {
        actor: ActorKey::from(scope.to_string()),
        place: TextPlace::Box(name.clone()),
    }
}

/// 文字の層の `Rc` を複製する（`Emo2Wiring` 不在＝結線前は `None`・箱は無い）。
fn runtime_of(world: &World) -> Option<Rc<RefCell<TextLayerRuntime>>> {
    world
        .get_non_send::<Emo2Wiring>()
        .map(|w| Rc::clone(w.runtime()))
}

/// 文字の層の不変借用で写しを取る。借りられなければ記録して `None`（呼び手は既存の道へ）。
fn read_point(
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    scope: usize,
    x: f32,
    y: f32,
) -> Option<BoxPoint> {
    let Ok(rt) = runtime.try_borrow() else {
        tracing::error!(
            event = "balloon_runtime_borrow_failed",
            scope,
            "runtime try_borrow 失敗（箱の写し）: 箱の前段を素通り"
        );
        return None;
    };
    let actor = ActorKey::from(scope.to_string());
    let hit = box_under_point(rt.shown_boxes(&actor), x, y).cloned();
    let rows = hit.as_ref().map_or_else(Vec::new, |b| {
        rt.choice_hit_rows_at(&box_place(scope, &b.name)).to_vec()
    });
    Some(BoxPoint {
        hit,
        rows,
        active: rt.hit_active(&actor),
    })
}

/// 窓のスコープと、座標の写しを取る（シェルの窓・Emo2Wiring・文字の層が揃うときだけ）。
fn locate(
    world: &World,
    entity: Entity,
    state: &PointerState,
) -> Option<(usize, Rc<RefCell<TextLayerRuntime>>, BoxPoint)> {
    let scope = super::char_scope(world, entity)? as usize;
    let runtime = runtime_of(world)?;
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;
    let point = read_point(&runtime, scope, x, y)?;
    Some((scope, runtime, point))
}

/// 移動の前段。`true` なら選択肢の行の上で、シェルへは送らない。
pub(super) fn on_box_pointer_moved(
    world: &mut World,
    entity: Entity,
    state: &PointerState,
) -> bool {
    let Some((scope, runtime, point)) = locate(world, entity, state) else {
        return false;
    };
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;
    move_with_point(world, &runtime, scope, point, x, y)
}

/// 移動 1 回の本体（写しを引数に取るのは、箱の四角と当たり行が提示（GPU）でしか埋まらないため）。
pub(super) fn move_with_point(
    world: &mut World,
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    scope: usize,
    point: BoxPoint,
    x: f32,
    y: f32,
) -> bool {
    let mv = judge_box_move(point.hit.as_ref(), &point.rows, x, y);
    let Some(hover) = world.get_non_send::<ShellBoxHover>() else {
        if mv != BoxMove::Outside {
            tracing::error!(
                event = "shell_box_hover_missing",
                scope,
                "箱の上の滞在の記録が無い（結線漏れ）: 箱の前段を素通り"
            );
        }
        return false;
    };
    let prev = hover.get(scope).cloned();
    let last = hover.injected(scope);
    let (next, clear) = next_box_hover(prev.as_ref(), &mv);
    if next != prev {
        tracing::debug!(
            event = "box_hover_changed",
            scope,
            from = prev.as_ref().map(BoxName::as_str),
            to = next.as_ref().map(BoxName::as_str),
            x,
            y,
            "ポインタの居る箱が替わった"
        );
    }
    // 前の箱から出た・行から外れた: 強調していれば外す（していなければ Keep で何もしない）。
    if let Some(old) = clear {
        apply_highlight(
            world,
            runtime,
            scope,
            &old,
            hover_action(point.active, None, last),
        );
    }
    let mut hover = world
        .get_non_send_mut::<ShellBoxHover>()
        .expect("ShellBoxHover は直上で存在確認済み");
    hover.set(scope, next);
    let last = hover.injected(scope);
    match mv {
        BoxMove::OverChoice { name, ordinal } => {
            apply_highlight(
                world,
                runtime,
                scope,
                &name,
                hover_action(point.active, Some(ordinal), last),
            );
            true
        }
        BoxMove::OverBody { .. } | BoxMove::Outside => false,
    }
}

/// 強調の決定を箱の場所へ当てる（`balloon_moved.rs` の適用と同じ腕）。
fn apply_highlight(
    world: &mut World,
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    scope: usize,
    name: &BoxName,
    action: HoverAction,
) {
    let value = match action {
        HoverAction::NoopInactive | HoverAction::Keep => return,
        // 選択肢が消えた: 自前の記録だけ落とす（注入はしない・上流が正本）。
        HoverAction::ResetOwnState => None,
        HoverAction::Inject(value) => {
            let Ok(mut rt) = runtime.try_borrow_mut() else {
                tracing::error!(
                    event = "balloon_runtime_borrow_failed",
                    scope,
                    "runtime try_borrow_mut 失敗（箱の強調）: no-op 縮退"
                );
                return;
            };
            rt.inject_choice_hover_at(&box_place(scope, name), value);
            tracing::debug!(
                event = "box_choice_hover_inject",
                scope,
                r#box = name.as_str(),
                ordinal = ?value,
                "箱の選択肢の強調を注入"
            );
            value
        }
    };
    if let Some(mut hover) = world.get_non_send_mut::<ShellBoxHover>() {
        hover.set_injected(scope, value);
    }
}

/// 押下の前段。`true` なら箱の選択肢の確定か箱での中断（またはその抑止）で、シェルへは送らない。
pub(super) fn on_box_pointer_pressed(
    world: &mut World,
    entity: Entity,
    state: &PointerState,
) -> bool {
    let Some((scope, _, point)) = locate(world, entity, state) else {
        return false;
    };
    press_with_point(world, scope, state, point)
}

/// 押下 1 回の本体。箱の中の押下はすべて中断の入口を通す（直前の押下の記憶を普通のバルーンの
/// 窓と共有しているため・左ダブルクリックに限らない）。
pub(super) fn press_with_point(
    world: &mut World,
    scope: usize,
    state: &PointerState,
    point: BoxPoint,
) -> bool {
    let Some(hit) = point.hit.as_ref() else {
        return false;
    };
    let x = state.client_point.x as f32;
    let y = state.client_point.y as f32;
    let selection = if state.left_down {
        judge_box_click(Some(hit), point.active, &point.rows, x, y, scope)
    } else {
        None
    };
    let selected_now = selection.is_some_and(|sel| send_selection(world, hit, sel));
    let verdict = on_box_press(world, scope, state.double_click, selected_now);
    tracing::debug!(
        event = "box_press",
        scope,
        r#box = hit.name.as_str(),
        x,
        y,
        double_click = ?state.double_click,
        selected_now,
        verdict = ?verdict,
        "箱の中の押下の結論"
    );
    verdict != BoxPressVerdict::ShellOp
}

/// 選択の確定を既存の送り口で送る（`balloon_pressed.rs` と同じ記録）。送れたら `true`。
fn send_selection(world: &World, hit: &ShownBox, sel: ChoiceSelection) -> bool {
    let (scope, id, label, references_len) = (
        sel.scope,
        sel.id.clone(),
        sel.label.clone(),
        sel.references.len(),
    );
    let Some(wiring) = world.get_non_send::<BalloonWiring>() else {
        tracing::error!(
            event = "balloon_wiring_missing",
            scope,
            "BalloonWiring 不在（結線漏れ）: 箱の選択の確定を no-op 縮退"
        );
        return false;
    };
    if wiring.send_selection(sel) {
        tracing::info!(
            event = "choice_selected",
            scope,
            id = %id,
            label = %label,
            references_len,
            r#box = hit.name.as_str(),
            "選択確定: ChoiceSelection を発行（箱）"
        );
        true
    } else {
        tracing::error!(
            event = "choice_selection_send_failed",
            scope,
            id = %id,
            "ChoiceSelection 発行シンク送出失敗（受け口消滅後）: no-op 縮退"
        );
        false
    }
}

/// シェルの窓から出たときの後始末（排他システム・`clear_balloon_hover_on_leave` の鏡写し）。
///
/// `PointerLeave` を持つ entity のうち、所有窓がシェルの窓（`CharWindowMarker`）のスコープの
/// 滞在を無しにし、強調していれば外す。滞在の記録が無い（結線前・LogSink の起動）は何もしない。
pub(crate) fn clear_box_hover_on_leave(world: &mut World) {
    let leaving: Vec<Entity> = world
        .query_filtered::<Entity, With<PointerLeave>>()
        .iter(world)
        .collect();
    let mut scopes: Vec<usize> = Vec::new();
    for e in leaving {
        let scope = find_owner_window(world, e)
            .and_then(|win| world.get::<CharWindowMarker>(win))
            .map(|m| m.scope);
        if let Some(scope) = scope.filter(|s| !scopes.contains(s)) {
            scopes.push(scope);
        }
    }
    if scopes.is_empty() {
        return;
    }
    let Some(hover) = world.get_non_send::<ShellBoxHover>() else {
        tracing::trace!(
            event = "shell_box_leave_no_wiring",
            "箱の上の滞在の記録が無い（結線前）: 何もしない"
        );
        return;
    };
    let entries: Vec<(usize, BoxName, Option<usize>)> = scopes
        .into_iter()
        .filter_map(|s| hover.get(s).map(|n| (s, n.clone(), hover.injected(s))))
        .collect();
    let runtime = runtime_of(world);
    for (scope, name, last) in entries {
        if let Some(runtime) = &runtime {
            let actor = ActorKey::from(scope.to_string());
            // 押せる範囲（選択肢かアンカー）があるか——箱のアンカーの強調もここで外す。
            match runtime.try_borrow().map(|rt| rt.hit_active(&actor)) {
                Ok(active) => apply_highlight(
                    world,
                    runtime,
                    scope,
                    &name,
                    hover_action(active, None, last),
                ),
                Err(_) => tracing::error!(
                    event = "balloon_runtime_borrow_failed",
                    scope,
                    "runtime try_borrow 失敗（シェルの窓の離脱）: 強調は外さず滞在だけ消す"
                ),
            }
        }
        world
            .get_non_send_mut::<ShellBoxHover>()
            .expect("ShellBoxHover は直上で存在確認済み")
            .set(scope, None);
    }
}

/// [`clear_box_hover_on_leave`] を入力の段（`dispatch_pointer_events` の後）へ登録する。
/// 呼び手は `ghost_session::register_systems`（プロセスに 1 回）。
pub(crate) fn register_box_leave_system(world: &mut World) {
    world.resource_mut::<Schedules>().add_systems(
        Input,
        clear_box_hover_on_leave.after(dispatch_pointer_events),
    );
}

#[cfg(test)]
#[path = "shell_box_handler_tests.rs"]
mod tests;
