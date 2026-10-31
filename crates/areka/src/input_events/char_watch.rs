//! 毎 tick の窓の見張りの系（mouse-click-wheel-events 要件 3.3・6・7・design「CharWatch」）。
//!
//! キャラ窓ごとに出入り（`OnMouseEnter`／`Leave`／`EnterAll`／`LeaveAll`）とホイール
//! （`OnMouseWheel`）を見張って kanade へ送る排他の系。処理の順は design「System Flows」の
//! 「毎 tick の見張り」の 1〜7（出る窓を入る窓より先・出入りをホイールより先）。
//! 出入りは移動の間引きを通さない（要件 7.6）。横のホイールは読まない（要件 6.7）。

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::scroll_marker::WheelLines;
use areka_kanade::{ClickButton, MouseEventKind, MouseInput};
use areka_sakura::ActorKey;
use bevy_ecs::prelude::*;
use wintf::ecs::Input;
use wintf::ecs::find_owner_window;
use wintf::ecs::pointer::{PointerState, dispatch_pointer_events};
use wintf::ecs::world::tick_wake;

use super::balloon_wheel::os_wheel_lines;
use super::hover_region::{HoverSend, HoverSighting, plan_hover};
use super::shell_box::box_under_point;
use super::shell_box_handler::box_place;
use super::wheel::{WheelAccum, WheelPlan, WheelRotation, plan_wheel};
use super::{MouseWiring, char_scope, resolve_hit_owned};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::placement::spawn::CharWindowMarker;

/// 窓の外へ出たときに捨てる押下の控え（左は捕捉で持ち主が残るので含めない・要件 3.3）。
const UNCAPTURED_BUTTONS: [ClickButton; 4] = [
    ClickButton::Right,
    ClickButton::Middle,
    ClickButton::XButton1,
    ClickButton::XButton2,
];

/// 見張りの系を `Input` の段の `dispatch_pointer_events` の前へ 1 回だけ登録する
/// （呼び手は `ghost_session::register_systems`・プロセスに 1 回）。
pub(crate) fn register_char_watch(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, char_watch.before(dispatch_pointer_events));
    cursor_answer::register_char_cursor(world);
}

/// カーソルが上に居るキャラ窓 1 つの見え方。
struct Over {
    window: Entity,
    scope: u32,
    client: (i64, i64),
    /// 持ち主が窓の entity そのもの（絵の透けた所の上・要件 6.10）。
    transparent: bool,
    wheel: i16,
}

fn wiring_mut(world: &mut World) -> Mut<'_, MouseWiring> {
    world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は系の頭で存在確認済み")
}

fn send_hover(wiring: &MouseWiring, scope: u32, s: HoverSend) {
    let input = MouseInput {
        scope,
        x: s.surface.0,
        y: s.surface.1,
        region: s.region,
        kind: MouseEventKind::Hover(s.edge),
    };
    wiring.send_mouse(input, "hover");
}

/// 毎 tick の見張りの系（排他）。
pub(crate) fn char_watch(world: &mut World) {
    char_watch_with(world, os_wheel_lines);
}

/// 本体。`read_lines` は箱の送りへ回すホイールを受けた時だけ呼ぶ（テストは差し替える）。
pub(crate) fn char_watch_with(world: &mut World, mut read_lines: impl FnMut() -> WheelLines) {
    // 1. 結線の前は何もしない。
    let Some(wiring) = world.get_non_send::<MouseWiring>() else {
        return;
    };

    // 2. 消えた窓（entity が無い・CharWindowMarker が無い）の覚えを黙って捨てる（要件 7.10・7.14）。
    let known: HashSet<Entity> = wiring
        .hover
        .keys()
        .chain(wiring.wheel.keys())
        .chain(wiring.presses.keys().map(|(e, _)| e))
        .copied()
        .collect();
    let gone: Vec<Entity> = known
        .into_iter()
        .filter(|e| world.get::<CharWindowMarker>(*e).is_none())
        .collect();

    // 3. PointerState の持ち主からキャラ窓を引く（持ち主は 1 窓に高々 1 つ）。
    let owners: Vec<(Entity, (i64, i64), i16)> = world
        .query::<(Entity, &PointerState)>()
        .iter(world)
        .map(|(e, s)| {
            let client = (s.client_point.x as i64, s.client_point.y as i64);
            (e, client, s.wheel.vertical)
        })
        .collect();
    let overs: Vec<Over> = owners
        .into_iter()
        .filter_map(|(owner, client, wheel)| {
            let window = find_owner_window(world, owner)?;
            let scope = char_scope(world, window)?;
            Some(Over {
                window,
                scope,
                client,
                transparent: owner == window,
                wheel,
            })
        })
        .collect();

    let mut wiring = wiring_mut(world);
    let now_ms = (wiring.now_ms)();
    for e in gone {
        wiring.hover.remove(&e);
        wiring.wheel.remove(&e);
        wiring.presses.retain(|(w, _), _| *w != e);
        tracing::trace!(event = "mouse_watch_forget", ?e, "消えた窓の覚えを捨てる");
    }

    // 4. 持ち主が居なくなった窓の出入りを先に送り、右・中・拡張の押下の控えを捨てる。
    let over_set: HashSet<Entity> = overs.iter().map(|o| o.window).collect();
    let leaving: Vec<Entity> = wiring
        .hover
        .keys()
        .filter(|e| !over_set.contains(e))
        .copied()
        .collect();
    for e in leaving {
        let prev = wiring.hover.remove(&e);
        let scope = prev.as_ref().map_or(0, |p| p.scope);
        let (_, sends) = plan_hover(prev.as_ref(), HoverSighting::Away);
        for s in sends {
            send_hover(&wiring, scope, s);
        }
        for b in UNCAPTURED_BUTTONS {
            wiring.presses.remove(&(e, b));
        }
    }

    // 5. 上に居る窓: 動いた・初めて見た・ホイールが回ったときだけ当たり判定を解く。
    let runtime = world
        .get_non_send::<Emo2Wiring>()
        .map(|w| Rc::clone(w.runtime()));
    let mut rotations: HashMap<Entity, (u32, WheelRotation)> = HashMap::new();
    for o in overs {
        let prev = world
            .get_non_send::<MouseWiring>()
            .expect("MouseWiring は系の頭で存在確認済み")
            .hover
            .get(&o.window)
            .cloned();
        let moved = prev.as_ref().is_none_or(|p| p.client != o.client);
        if !moved && o.wheel == 0 {
            continue;
        }
        let hit = resolve_hit_owned(world, o.scope, o.client.0, o.client.1);
        // 透けた所は同じ持ち主で当たり判定「無し」として扱う。
        let region = if o.transparent { None } else { hit.region };
        let mut wiring = wiring_mut(world);
        if moved {
            let sighting = HoverSighting::Over {
                scope: o.scope,
                client: o.client,
                surface: hit.surface_point,
                region: region.clone(),
            };
            let (next, sends) = plan_hover(prev.as_ref(), sighting);
            for s in sends {
                send_hover(&wiring, o.scope, s);
            }
            match next {
                Some(n) => wiring.hover.insert(o.window, n),
                None => wiring.hover.remove(&o.window),
            };
        }
        if o.wheel == 0 {
            continue;
        }
        // 隠れた行のある箱の上の回転は箱の送りに使い、`OnMouseWheel` へ積まない（balloon-scroll-arrows
        // 要件 7.5・7.6）。透けた所の判断より前（箱の文字の上は絵が透けていても箱の上）。
        if runtime
            .as_ref()
            .is_some_and(|rt| wheel_box(rt, o.scope, o.client, o.wheel, &mut read_lines))
        {
            continue;
        }
        if o.transparent {
            tracing::trace!(
                event = "mouse_wheel_not_sent",
                reason = "transparent",
                scope = o.scope
            );
            continue;
        }
        let rotation = WheelRotation {
            delta: i32::from(o.wheel),
            surface: hit.surface_point,
            region,
        };
        rotations.insert(o.window, (o.scope, rotation));
    }

    // 6. 溜めの残る窓と回った窓の期限を確かめて吐き出す。
    let mut wiring = wiring_mut(world);
    let mut windows: Vec<Entity> = wiring.wheel.keys().copied().collect();
    windows.extend(rotations.keys().filter(|e| !wiring.wheel.contains_key(e)));
    let mut earliest: Option<u64> = None;
    for e in windows {
        let rotation = rotations.remove(&e);
        let acc = wiring.wheel.entry(e).or_insert_with(|| WheelAccum {
            scope: rotation.as_ref().map_or(0, |(s, _)| *s),
            last_sent_ms: None,
            pending: None,
        });
        let scope = acc.scope;
        match plan_wheel(acc, now_ms, rotation.map(|(_, r)| r)) {
            WheelPlan::Send(p) => {
                let input = MouseInput {
                    scope,
                    x: p.surface.0,
                    y: p.surface.1,
                    region: p.region,
                    kind: MouseEventKind::Wheel { delta: p.sum },
                };
                wiring.send_mouse(input, "wheel");
            }
            WheelPlan::DropZero => {
                tracing::debug!(event = "mouse_wheel_not_sent", reason = "zero_sum", scope)
            }
            WheelPlan::Wait { until_ms } => {
                earliest = Some(earliest.map_or(until_ms, |t| t.min(until_ms)));
            }
            WheelPlan::Idle => {}
        }
    }

    // 7. 溜めが残っていれば最も早い期限で起こしを預け直す（預かりは到来すると倒れる）。
    // `Wait` は `now_ms < until_ms` のときだけ返るので、期限は常に未来。
    if let Some(until_ms) = earliest {
        tick_wake::arm_deadline(Instant::now() + Duration::from_millis(until_ms - now_ms));
    }
}

/// 点（シェルの窓のクライアント物理 px）の下の箱に隠れた行があれば、その箱を回転 `delta` で送って
/// `true`。箱が無い・隠れた行が無い・借りられないなら `false`（呼び手は今までどおり積む）。
fn wheel_box(
    runtime: &RefCell<TextLayerRuntime>,
    scope: u32,
    client: (i64, i64),
    delta: i16,
    read_lines: &mut impl FnMut() -> WheelLines,
) -> bool {
    let Ok(mut rt) = runtime.try_borrow_mut() else {
        tracing::error!(
            event = "balloon_runtime_borrow_failed",
            scope,
            "runtime try_borrow_mut 失敗（箱の上のホイール）: 今までどおり OnMouseWheel へ"
        );
        return false;
    };
    let actor = ActorKey::from(scope.to_string());
    let Some(name) = box_under_point(rt.shown_boxes(&actor), client.0 as f32, client.1 as f32)
        .map(|b| b.name.clone())
    else {
        return false;
    };
    let place = box_place(scope as usize, &name);
    if !rt.has_hidden_lines_at(&place) {
        return false;
    }
    let change = rt.wheel_at(&place, i32::from(delta), read_lines());
    tracing::trace!(
        event = "box_wheel_scrolled",
        scope,
        r#box = name.as_str(),
        delta,
        ?change,
        "隠れた行のある箱の上のホイール: 箱を送り OnMouseWheel にしない"
    );
    true
}

#[path = "cursor_answer.rs"]
pub(crate) mod cursor_answer;

#[cfg(test)]
#[path = "char_watch_tests.rs"]
mod tests;
