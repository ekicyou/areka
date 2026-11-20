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
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        double_click,
        ctrl_down,
        ..Default::default()
    })
}

/// Bubble 相の合成 `PointerState`（Ctrl／Shift を別々に指定する版・R15.1／15.2）。
///
/// [`bubble_pointer`] は Ctrl だけを取る（既存の呼び手 20 箇所以上がそのまま動く）。結線済みでは
/// 強制退避と通常の左ダブルクリックが Shift の有無で分かれるため、両方を指定できる口をここへ足す。
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

/// 単調増加する注入 clock を作る（毎呼出で +step ms）。
pub fn stepping_clock(start: u64, step: u64) -> Box<dyn FnMut() -> u64> {
    let mut t = start;
    Box::new(move || {
        let now = t;
        t += step;
        now
    })
}
