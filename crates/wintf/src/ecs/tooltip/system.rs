//! 画面更新ごとの判定（集める → 状態機械）。
//!
//! World と OS の見本から足元と追っている範囲の様子を集め、状態機械を 1 回進めて
//! 「すること」を返す。OS の読み取り（見本・可視・待ち時間の設定）は引数の関数で受けるので、
//! ここまでは窓なしでテストできる。「すること」を表示と利用側へ当てるのは適用の段。

use super::geometry::{PointPx, RectPx};
use super::os::{OsSample, TipWindow};
use super::ranges::{TooltipArea, TooltipRangeId, TooltipRanges};
use super::turn::{Effect, Over, StepInput, Tracked, TurnMachine};
use super::{OnTooltip, TooltipNotice, TooltipTurn, TooltipTurnToken};
use crate::ecs::layout::hit_test_in_window;
use crate::ecs::{DPI, PointF, PointerState, Window, WindowPos, find_owner_window};
use bevy_ecs::prelude::*;
use std::cell::Cell;
use std::time::{Duration, Instant};

thread_local! {
    /// 前回の判定の後にボタンが押されたか（UI スレッドの印）。
    static PRESSED: Cell<bool> = const { Cell::new(false) };
}

/// 「前回の判定の後に押された」を立てる。窓のメッセージの配送点が、ボタンの押下と
/// ダブルクリックのメッセージで呼ぶ（World を借りない）。押してすぐ離した場合を、
/// 判定の時点のボタンの読み取りだけでは取りこぼすため。
pub(crate) fn note_button_press() {
    PRESSED.set(true);
}

/// 窓が見えているか。本物は窓のハンドルを OS に問い合わせる。
pub(crate) type Visible<'a> = &'a dyn Fn(&World, Entity) -> bool;

/// 判定の持ちもの（スレッドに縛られる資源。表示の窓が UI スレッドのものなので）。
#[derive(Default)]
pub(crate) struct TooltipSession {
    pub(crate) machine: TurnMachine,
    /// 標準のツールチップの窓（最初に出すときに作る）。
    // テストの外では資源ごと未使用（モジュールの宣言の expect が受ける）。
    #[cfg_attr(
        test,
        expect(dead_code, reason = "適用の段（出す・消す）から使い始める")
    )]
    pub(crate) tip: Option<TipWindow>,
    /// 今の出す番の控え。
    pub(crate) note: Option<TurnNote>,
}

/// 今の出す番の控え。
#[derive(Clone, Copy)]
pub(crate) struct TurnNote {
    pub(crate) token: TooltipTurnToken,
    /// 出す番の始まりに窓から写した知らせの関数（窓が壊れても終わりを知らせられる）。
    pub(crate) notify: Option<OnTooltip>,
}

/// World と OS の見本から「すること」を出す。
///
/// 範囲の表がどの窓にも無く、状態が「何もない」なら、OS を 1 回も呼ばずに空を返す
/// （範囲を登録していない利用側の画面更新には何も足さない）。押下の印はその前に読んで倒す
/// （範囲が無い間の古い押下を、後で登録した最初の判定が拾って休みにならないため）。
pub(crate) fn decide(
    world: &mut World,
    session: &mut TooltipSession,
    now: Instant,
    sample: &mut dyn FnMut() -> OsSample,
    visible: Visible<'_>,
    hover_time: &mut dyn FnMut() -> Duration,
) -> Vec<Effect> {
    let pressed = PRESSED.take();
    let candidates: Vec<Entity> = world
        .query_filtered::<Entity, With<TooltipRanges>>()
        .iter(world)
        .collect();
    if candidates.is_empty() && session.machine.is_idle() {
        return Vec::new();
    }
    // ポインタの状態を配下に持つ窓（OS がマウスをその窓へ届けている）。
    let pointed: Vec<Entity> = world
        .query_filtered::<Entity, With<PointerState>>()
        .iter(world)
        .collect();
    let world = &*world;
    let pointed: Vec<Entity> = pointed
        .into_iter()
        .filter_map(|e| find_owner_window(world, e))
        .collect();
    let sees = Sees {
        world,
        pointed: &pointed,
        visible,
    };

    let os = sample();
    let over = os
        .cursor
        .and_then(|p| candidates.iter().find_map(|&w| sees.over(w, p)));
    let tracked = session
        .machine
        .tracked_range()
        .map(|(w, id)| sees.tracked(w, id, session.machine.active_area(), os.cursor));
    let input = StepInput {
        now,
        cursor: os.cursor,
        button_down: os.button_down || pressed,
        over,
        tracked,
    };
    let fx = session.machine.step(&input, hover_time);
    for e in &fx {
        if let Effect::TurnStarted { token, window, .. } = *e {
            session.note = Some(TurnNote {
                token,
                notify: world.get::<OnTooltip>(window).copied(),
            });
        }
    }
    fx
}

/// 「すること」のうち、利用側へ知らせるもの（来た・終わった）を知らせの形にする。
pub(crate) fn notice(effect: &Effect) -> Option<TooltipNotice> {
    match *effect {
        Effect::TurnStarted {
            token,
            window,
            range,
            pos_logical,
            stored,
        } => Some(TooltipNotice::TurnStarted(TooltipTurn {
            window,
            range,
            position: pos_logical,
            token,
            has_text: stored,
        })),
        Effect::TurnEnded {
            token,
            window,
            range,
            reason,
        } => Some(TooltipNotice::TurnEnded {
            window,
            range,
            token,
            reason,
        }),
        Effect::ArmDeadline(_) | Effect::Hide { .. } => None,
    }
}

/// 集めるときに World から読むもの。
struct Sees<'a> {
    world: &'a World,
    /// 配下にポインタの状態がある窓。
    pointed: &'a [Entity],
    visible: Visible<'a>,
}

impl Sees<'_> {
    /// 窓がマウスを受けていれば、窓の中の位置（物理ピクセル）を返す。受けている＝配下に
    /// ポインタの状態がある かつ 当たり判定が当たる かつ 窓が見えている。
    fn receives(&self, window: Entity, p: PointPx) -> Option<PointPx> {
        if !self.pointed.contains(&window) {
            return None;
        }
        let origin = self.world.get::<WindowPos>(window)?.position?;
        let client = PointPx {
            x: p.x - origin.x,
            y: p.y - origin.y,
        };
        hit_test_in_window(
            self.world,
            window,
            PointF::new(client.x as f32, client.y as f32),
        )?;
        (self.visible)(self.world, window).then_some(client)
    }

    /// 足元の候補（窓がマウスを受けていて、窓の DPI で論理に直した位置が範囲に当たる）。
    fn over(&self, window: Entity, p: PointPx) -> Option<Over> {
        let client = self.receives(window, p)?;
        let dpi = self.dpi(window);
        let pos = PointF::new(dpi.to_logical_x(client.x), dpi.to_logical_y(client.y));
        let (range, r) = self.world.get::<TooltipRanges>(window)?.hit(pos)?;
        Some(Over {
            window,
            range,
            area_logical: r.area,
            has_text: r.text.is_some(),
            pos_logical: pos,
        })
    }

    /// 追っている範囲の様子。出す番の間は始まりの場所（start）で矩形を作る。
    fn tracked(
        &self,
        window: Entity,
        id: TooltipRangeId,
        start: Option<TooltipArea>,
        cursor: Option<PointPx>,
    ) -> Tracked {
        if self.world.get::<Window>(window).is_none() {
            return Tracked::WindowDestroyed;
        }
        let Some(reg) = self
            .world
            .get::<TooltipRanges>(window)
            .and_then(|t| t.get(id))
        else {
            return Tracked::RangeUnregistered;
        };
        if !(self.visible)(self.world, window) {
            return Tracked::WindowHidden;
        }
        // 位置の分からない窓は、まだ画面に出ていないものとして隠れた扱いにする。
        let Some(range_px) = self.range_px(window, start.unwrap_or(reg.area)) else {
            return Tracked::WindowHidden;
        };
        Tracked::Present {
            range_px,
            receives: cursor.is_some_and(|p| self.receives(window, p).is_some()),
        }
    }

    /// 範囲の画面の矩形（論理の矩形 × DPI ＋ 窓の位置。窓の全体なら窓の大きさ）。
    fn range_px(&self, window: Entity, area: TooltipArea) -> Option<RectPx> {
        let pos = self.world.get::<WindowPos>(window)?;
        let o = pos.position?;
        Some(match area {
            TooltipArea::WholeWindow => {
                let s = pos.size?;
                RectPx {
                    left: o.x,
                    top: o.y,
                    right: o.x + s.width,
                    bottom: o.y + s.height,
                }
            }
            TooltipArea::Rect(r) => {
                let dpi = self.dpi(window);
                let x = |v: f32| o.x + dpi.to_physical_x(v);
                let y = |v: f32| o.y + dpi.to_physical_y(v);
                RectPx {
                    left: x(r.left),
                    top: y(r.top),
                    right: x(r.right),
                    bottom: y(r.bottom),
                }
            }
        })
    }

    fn dpi(&self, window: Entity) -> DPI {
        self.world.get::<DPI>(window).copied().unwrap_or_default()
    }
}
