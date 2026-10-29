//! 窓 1 つの形の状態の遷移と、出す形とその出どころを決める純関数（shell-mouse-cursor・design.md「CursorPick」）。
//!
//! 状態の置き場所と結線は CursorAnswer（`cursor_answer.rs`）。当たり判定はここでは引かず、
//! 引くときだけ渡された `resolve` を呼ぶ。
// 消費側（CursorAnswer）は tasks.md 4.2 で来る。
#![cfg_attr(not(test), allow(dead_code))]

use wintf::ecs::cursor::CursorTrigger;

use crate::emo2_boot::shell_cursor_table::{
    CursorRows, CursorScene, GhostCursorKeys, Shape, SystemCursor,
};

/// その位置の当たり: 当たり判定の上・当たり判定の無い所・絵の透けた所。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::input_events) enum Spot {
    Region(String),
    Plain,
    SeeThrough,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::input_events) enum Hold {
    Free,
    /// scene は Down か RightDown。
    Pressed {
        scene: CursorScene,
        region: String,
    },
    /// Hover か Wheel。
    Resting(CursorScene),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::input_events) struct WindowCursor {
    /// 当たり判定を引いた位置（窓 client 物理 px・動いたかの比べに使う）。
    pub client: (i32, i32),
    pub spot: Spot,
    pub hold: Hold,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::input_events) enum Shown {
    Region { scene: CursorScene, region: String },
    Arrow,
    Pass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::input_events) enum Origin {
    Brace,
    GhostDescript,
    Default,
}

/// その位置の場面（Region → 乗る・Plain → 矢印・SeeThrough → 通す）。
fn at(spot: &Spot) -> Shown {
    match spot {
        Spot::Region(r) => Shown::Region {
            scene: CursorScene::Up,
            region: r.clone(),
        },
        Spot::Plain => Shown::Arrow,
        Spot::SeeThrough => Shown::Pass,
    }
}

/// hold から決まる場面。
fn shown_of(c: &WindowCursor) -> Shown {
    match (&c.hold, &c.spot) {
        (Hold::Pressed { scene, region }, _) => Shown::Region {
            scene: *scene,
            region: region.clone(),
        },
        (Hold::Resting(scene), Spot::Region(r)) => Shown::Region {
            scene: *scene,
            region: r.clone(),
        },
        _ => at(&c.spot),
    }
}

/// 前の状態と引き金から、次の状態と出す場面を決める（design の 13 行の表）。resolve は当たり判定を引くとき
/// （位置が変わった・初めて・押した・放した・ホイール・前の状態が無い）だけ呼ぶ。
pub(in crate::input_events) fn plan_cursor(
    prev: Option<&WindowCursor>,
    trigger: CursorTrigger,
    client: (i32, i32),
    buttons_down: bool,
    resolve: impl FnOnce() -> Spot,
) -> (WindowCursor, Shown) {
    let mut resolve = Some(resolve);
    let first;
    let prev = match prev {
        Some(p) => p,
        None => {
            // 初めて: その位置で引き、前の hold を Free として同じ表で決める。
            first = WindowCursor {
                client,
                spot: resolve.take().map_or(Spot::Plain, |f| f()),
                hold: Hold::Free,
            };
            &first
        }
    };
    // 初めてのときは引いたばかりの当たりを返す（引くのは 1 回だけ）。
    let mut resolve = || resolve.take().map_or_else(|| prev.spot.clone(), |f| f());
    let moved = prev.client != client;
    let looked = |spot: Spot, hold: Hold| {
        let next = WindowCursor { client, spot, hold };
        let shown = shown_of(&next);
        (next, shown)
    };
    match trigger {
        CursorTrigger::Move if !moved => (prev.clone(), shown_of(prev)),
        CursorTrigger::Move => match &prev.hold {
            Hold::Pressed { .. } if buttons_down => looked(resolve(), prev.hold.clone()),
            _ => looked(resolve(), Hold::Free),
        },
        CursorTrigger::Press { right } => {
            let spot = resolve();
            let hold = match (&prev.hold, &spot) {
                (Hold::Pressed { .. }, _) => prev.hold.clone(),
                (_, Spot::Region(r)) => Hold::Pressed {
                    scene: if right {
                        CursorScene::RightDown
                    } else {
                        CursorScene::Down
                    },
                    region: r.clone(),
                },
                _ => Hold::Free,
            };
            looked(spot, hold)
        }
        CursorTrigger::Release => looked(resolve(), Hold::Free),
        CursorTrigger::Hover => {
            let mut next = prev.clone();
            if next.hold == Hold::Free && matches!(next.spot, Spot::Region(_)) {
                next.hold = Hold::Resting(CursorScene::Hover);
            }
            let shown = shown_of(&next);
            (next, shown)
        }
        CursorTrigger::Wheel => {
            let spot = resolve();
            let hold = match (&prev.hold, &spot) {
                (Hold::Pressed { .. }, _) => prev.hold.clone(),
                (_, Spot::Region(_)) => Hold::Resting(CursorScene::Wheel),
                _ => prev.hold.clone(),
            };
            looked(spot, hold)
        }
        CursorTrigger::Refresh => (prev.clone(), shown_of(prev)),
    }
}

/// 出す場面から形を選ぶ（cursorブレスの行 ＞ ゴーストの差し替え ＞ 既定）。Pass は None。
pub(in crate::input_events) fn pick_shape<F: Clone>(
    scope: u32,
    shown: &Shown,
    rows: &CursorRows<F>,
    ghost: &GhostCursorKeys<F>,
) -> Option<(Shape<F>, Origin)> {
    let fallback = |g: &Option<Shape<F>>, sys| match g {
        Some(s) => (s.clone(), Origin::GhostDescript),
        None => (Shape::System(sys), Origin::Default),
    };
    match shown {
        Shown::Pass => None,
        Shown::Arrow => Some(fallback(&ghost.arrow, SystemCursor::Arrow)),
        Shown::Region { scene, region } => {
            // 止まった・ホイールに行が無ければ、同じ当たり判定の乗る形のまま（SSP の測り・要件 4.1・4.3）。
            let base = match scene {
                CursorScene::Hover | CursorScene::Wheel => CursorScene::Up,
                s => *s,
            };
            let row = |s| rows.rows.get(&(scope, region.clone(), s)).cloned();
            if let Some(s) = row(*scene).or_else(|| row(base)) {
                return Some((s, Origin::Brace));
            }
            let bust = region.eq_ignore_ascii_case("bust");
            Some(match (base, bust) {
                (CursorScene::Up, true) => fallback(&ghost.hand, SystemCursor::Hand),
                (CursorScene::Down, true) => fallback(&ghost.grip, SystemCursor::Grip),
                _ => fallback(&ghost.finger, SystemCursor::Finger),
            })
        }
    }
}

#[cfg(test)]
#[path = "cursor_pick_tests.rs"]
mod tests;
