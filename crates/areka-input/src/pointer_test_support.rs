use super::*;
use areka_placement::spawn::GhostWindowMarker;
use std::sync::mpsc;
use wintf::ecs::Point;
use wintf::ecs::pointer::{DoubleClick, Phase, PointerState};

/// Bubble 相の合成 `PointerState`（client 物理 px・DoubleClick・Ctrl 押下）を組む。
pub fn bubble_pointer(
    x: i32,
    y: i32,
    double_click: DoubleClick,
    ctrl_down: bool,
) -> Phase<PointerState> {
    bubble_pointer_mods(x, y, double_click, ctrl_down, false)
}

/// Bubble 相の合成 `PointerState`（Ctrl／Shift を別々に指定する版）。
pub fn bubble_pointer_mods(
    x: i32,
    y: i32,
    double_click: DoubleClick,
    ctrl_down: bool,
    shift_down: bool,
) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        double_click,
        ctrl_down,
        shift_down,
        ..Default::default()
    })
}

/// `GhostWindowMarker` 窓の現数を数える。
pub fn ghost_count(world: &mut World) -> usize {
    world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .count()
}

/// 強制退避が終了を指示したか（`world_with_wiring` が挿す `AppExit` を見る）。
#[cfg(test)]
pub(crate) fn exit_requested(world: &World) -> bool {
    world
        .get_non_send::<wintf::AppExit>()
        .expect("AppExit は挿入済み")
        .is_requested()
}

/// Mock resolver＋注入 clock の `MouseWiring` を NonSend 挿入した World を組む。
pub fn world_with_wiring(
    region: fn(u32, i64, i64) -> HitRegion,
    clock: Box<dyn FnMut() -> u64>,
) -> (World, mpsc::Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let wiring = MouseWiring::with_clock(tx, RegionSource::Mock(region), clock);
    let mut world = World::new();
    world.insert_non_send(wiring);
    // 終了の受け口（本番では `WinApp` が必ず挿す）。強制退避は統合操作で終了を指示する。
    world.insert_non_send(wintf::AppExit::new());
    (world, rx)
}

/// 窓の中の点 `(x, y)` から区別できる座標 `(2x, 3y)` を返し、`x < 100` なら `Head`・それ以外は
/// 当たり判定なしにする偽の当たり判定。受け口の値から渡した位置を読み戻せる。
#[cfg(test)]
pub(crate) fn fake_hit(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: (x < 100).then(|| "Head".to_string()),
        surface_point: (x * 2, y * 3),
    }
}

/// kanade 宛ての受信端に溜まったマウスの知らせを全部取り出す（マウス以外が来たら落とす）。
#[cfg(test)]
pub(crate) fn drain_mouse(rx: &mpsc::Receiver<KanadeMsg>) -> Vec<MouseInput> {
    rx.try_iter()
        .map(|msg| match msg {
            KanadeMsg::Mouse(m) => m,
            _ => panic!("マウス以外の知らせが届いた"),
        })
        .collect()
}

/// 預かっている右ダブルクリックを取り出す。
#[cfg(test)]
pub(crate) fn take_pending(world: &mut World) -> Option<PendingDoubleClick> {
    world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .take_pending_right_double_click()
}

/// 取り出した預かりを送る（メニュー側が「送る」と決めたときの操作）。
#[cfg(test)]
pub(crate) fn send_pending(world: &mut World, pending: PendingDoubleClick) {
    world
        .get_non_send_mut::<MouseWiring>()
        .expect("MouseWiring は挿入済み")
        .send_pending_right_double_click(pending);
}

/// 単調増加する注入 clock を作る（毎呼出で +step ms）。
pub fn stepping_clock(start: u64, step: u64) -> Box<dyn FnMut() -> u64> {
    let mut t = start;
    Box::new(move || {
        let now = t;
        t += step;
        now
    })
}
