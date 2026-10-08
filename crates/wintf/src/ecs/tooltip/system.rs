//! 画面更新ごとの判定（集める → 状態機械 → 適用）と、文字を渡す・消す・取り消す口の中身。
//!
//! World と OS の見本から足元と追っている範囲の様子を集め、状態機械を 1 回進めて
//! 「すること」を返す。OS の読み取り（見本・可視・待ち時間の設定）は引数の関数で受けるので、
//! ここまでは窓なしでテストできる。「すること」を表示と利用側へ当てる適用の段と公開の口の中身も、
//! 表示の出入口（`Tip`）を引数で受けるので窓なしでテストできる。本物の表示を渡すのは薄い包みだけ。

use super::geometry::{PointPx, RectPx};
use super::os::{self, OsSample, TipWindow, TooltipOsError};
use super::ranges::{TooltipArea, TooltipRangeId, TooltipRanges};
use super::turn::{Effect, HideReason, Over, StepInput, SupplyDecision, Tracked, TurnMachine};
use super::{OnTooltip, TooltipNotice, TooltipSupply, TooltipTurn, TooltipTurnToken};
use crate::ecs::layout::hit_test_in_window;
use crate::ecs::world::tick_wake;
use crate::ecs::{DPI, PointF, PointerState, Window, WindowHandle, WindowPos, find_owner_window};
use bevy_ecs::prelude::*;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

thread_local! {
    /// 前回の判定の後にボタンが押されたか（UI スレッドの印）。
    static PRESSED: Cell<bool> = const { Cell::new(false) };
    /// 標準のツールチップの窓（UI スレッドに 1 枚・最初に出すときに作る）。資源の外に置くのは、
    /// 適用の途中で呼ぶ知らせの中から文字を渡されても、同じ 1 枚を借りられるようにするため。
    static TIP_WINDOW: RefCell<TipWindow> = RefCell::new(TipWindow::default());
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
    /// 今の出す番の控え。
    pub(crate) note: Option<TurnNote>,
}

/// 今の出す番の控え。
#[derive(Clone, Copy)]
pub(crate) struct TurnNote {
    pub(crate) token: TooltipTurnToken,
    pub(crate) window: Entity,
    /// 出す番の始まりに窓から写した知らせの関数（窓が壊れても終わりを知らせられる）。
    pub(crate) notify: Option<OnTooltip>,
    /// 基準の位置（画面の物理ピクセル）。その出す番で初めて出すまでは判定のたびにマウスの位置へ
    /// 合わせ、出したら固定する（出ている間・出し直しでツールチップを動かさない）。
    pub(crate) anchor: PointPx,
    /// 基準の位置を固定したか（その出す番で一度出したか）。
    pub(crate) anchored: bool,
    /// 今ツールチップが出ているか。
    pub(crate) shown: bool,
    /// 手前へ当て直せなかったことを記録したか（見回りのたびに出さず、出す番に 1 回だけ）。
    pub(crate) topmost_warned: bool,
}

/// World と OS の見本から「すること」を出す。
///
/// 中身のある範囲の表がどの窓にも無く、状態が「何もない」なら、OS を 1 回も呼ばずに空を返す
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
    // 全部取り消して空になった表は数えない（表は通し番号を守るために残す）。
    let candidates: Vec<Entity> = world
        .query::<(Entity, &TooltipRanges)>()
        .iter(world)
        .filter(|(_, t)| !t.is_empty())
        .map(|(e, _)| e)
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
    if let (Some(note), Some(p)) = (session.note.as_mut(), os.cursor)
        && !note.anchored
    {
        note.anchor = p;
    }
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
        // 来た回は位置が読めている（読めない回は待ちを進めない）。
        if let (Effect::TurnStarted { token, window, .. }, Some(p)) = (*e, os.cursor) {
            session.note = Some(TurnNote {
                token,
                window,
                notify: world.get::<OnTooltip>(window).copied(),
                anchor: p,
                anchored: false,
                shown: false,
                topmost_warned: false,
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

/// 表示の出入口。本物は標準のツールチップの窓を呼ぶ。テストは記録するだけの閉包を渡す。
pub(crate) struct Tip<'a> {
    pub show: &'a mut dyn FnMut(&str, PointPx) -> Result<RectPx, TooltipOsError>,
    pub hide: &'a mut dyn FnMut(),
}

/// 位置が読めないまま待ちの期限を過ぎた回に、期限を預け直す間隔。
const RETRY: Duration = Duration::from_millis(100);

/// 預ける期限。過ぎた期限（位置が読めないまま待ちの期限を過ぎると毎回返る）をそのまま預けると
/// すぐ起き直し続けるので、少し先へ送る。
pub(crate) fn wake_at(deadline: Instant, now: Instant) -> Instant {
    if deadline > now {
        deadline
    } else {
        now + RETRY
    }
}

/// 1 回の画面更新: 判定して、資源を World に戻してから適用する。
pub(crate) fn frame_with(
    world: &mut World,
    now: Instant,
    sample: &mut dyn FnMut() -> OsSample,
    visible: Visible<'_>,
    hover_time: &mut dyn FnMut() -> Duration,
    tip: &mut Tip<'_>,
) {
    let Some(mut session) = world.remove_non_send::<TooltipSession>() else {
        return;
    };
    let fx = decide(world, &mut session, now, sample, visible, hover_time);
    world.insert_non_send(session);
    apply_with(world, fx, now, tip);
}

/// 「すること」を順に適用する（期限を預ける・預けた文字を出す・消す）。知らせは溜めておき、
/// 資源を World に戻してから順に同期で呼ぶ（知らせの中から文字を渡せるように）。
pub(crate) fn apply_with(world: &mut World, effects: Vec<Effect>, now: Instant, tip: &mut Tip<'_>) {
    let Some(mut session) = world.remove_non_send::<TooltipSession>() else {
        return;
    };
    let mut notices = Vec::new();
    for e in &effects {
        match *e {
            Effect::ArmDeadline(at) => tick_wake::arm_deadline(wake_at(at, now)),
            Effect::TurnStarted {
                window,
                range,
                stored: true,
                ..
            } => {
                let text = world
                    .get::<TooltipRanges>(window)
                    .and_then(|t| t.get(range))
                    .and_then(|r| r.text.clone());
                if let Some(text) = text {
                    // 静的な使い方では失敗は記録だけ（知らせは続けて出す）。
                    let _ = show(&mut session, tip, &text, "stored", now);
                }
            }
            Effect::Hide { token, reason } => hide(&mut session, tip, token, reason),
            Effect::TurnStarted { .. } | Effect::TurnEnded { .. } => {}
        }
        // 知らせの関数は始まりに写したもの（窓が壊れていても終わりを知らせられる）。
        let Some(n) = notice(e) else { continue };
        let token = match n {
            TooltipNotice::TurnStarted(t) => t.token,
            TooltipNotice::TurnEnded { token, .. } => token,
        };
        let Some(note) = session.note.filter(|note| note.token == token) else {
            continue;
        };
        if matches!(n, TooltipNotice::TurnEnded { .. }) {
            session.note = None;
        }
        notices.extend(note.notify.map(|f| (f, n)));
    }
    world.insert_non_send(session);
    for (notify, n) in notices {
        (notify.0)(world, &n);
    }
}

/// 続いている出す番に文字を渡す（公開の口 `supply_text` の中身）。
pub(crate) fn supply_text_with(
    world: &mut World,
    token: TooltipTurnToken,
    text: &str,
    now: Instant,
    tip: &mut Tip<'_>,
) -> TooltipSupply {
    let mut session = world.get_non_send_mut::<TooltipSession>();
    let decision = session.as_mut().map_or(SupplyDecision::Stale, |s| {
        s.machine.supply(token, text.is_empty())
    });
    match (decision, session) {
        (SupplyDecision::Show, Some(mut s)) => match show(&mut s, tip, text, "supplied", now) {
            Ok(()) => TooltipSupply::Shown,
            Err(e) => TooltipSupply::Failed(e),
        },
        (SupplyDecision::HideEmpty, Some(mut s)) => {
            if s.note.is_some_and(|n| n.shown) {
                hide(&mut s, tip, token, HideReason::EmptyText);
            }
            // 矩形を外し、消えた時刻を出し直しの起点にする（状態機械は消したことを知らない）。
            s.machine.set_tip(None, now);
            TooltipSupply::Cleared
        }
        _ => {
            debug!(token = ?token, "[tooltip_supply_stale] 終わった出す番の印で文字を渡された");
            TooltipSupply::StaleTurn
        }
    }
}

/// 続いている出す番のツールチップを消す（公開の口 `dismiss` の中身）。出す番は続く。
pub(crate) fn dismiss_with(
    world: &mut World,
    token: TooltipTurnToken,
    now: Instant,
    tip: &mut Tip<'_>,
) -> bool {
    let Some(mut s) = world.get_non_send_mut::<TooltipSession>() else {
        return false;
    };
    let shown = s.note.is_some_and(|n| n.token == token && n.shown);
    if !s.machine.dismiss(token, now) {
        return false;
    }
    if shown {
        hide(&mut s, tip, token, HideReason::Dismissed);
    }
    true
}

/// 登録を取り消す（公開の口 `unregister` の中身）。追っている範囲なら、戻る前に消して終わりを知らせる。
///
/// 表が空になっても窓の部品は外さない。外すと次の登録で通し番号が数え直しになり、取り消した
/// 持ち手が新しい登録に当たる（判定は早戻りせず、OS の見本を毎回 1 回読むようになる）。
pub(crate) fn unregister_with(
    world: &mut World,
    id: TooltipRangeId,
    now: Instant,
    tip: &mut Tip<'_>,
) -> bool {
    let removed = world
        .get_mut::<TooltipRanges>(id.window())
        .is_some_and(|mut t| t.remove(id));
    if !removed {
        return false;
    }
    let fx = world
        .get_non_send_mut::<TooltipSession>()
        .map(|mut s| s.machine.on_unregistered(id, now))
        .unwrap_or_default();
    apply_with(world, fx, now, tip);
    true
}

/// 基準の位置に文字を出し、結果を状態機械と控えに覚える。失敗は `warn`（出ていたものも消えている）。
fn show(
    session: &mut TooltipSession,
    tip: &mut Tip<'_>,
    text: &str,
    source: &'static str,
    now: Instant,
) -> Result<(), TooltipOsError> {
    let TooltipSession { machine, note } = session;
    // 出す番が来た回に必ず控えを作るので、無いのは出す番が無いとき。
    let Some(note) = note else {
        warn!(
            source,
            "[tooltip_show_failed] 出す番の控えが無いので出さない"
        );
        return Err(TooltipOsError::Show { stage: "anchor" });
    };
    let chars = text.chars().count();
    match (tip.show)(text, note.anchor) {
        Ok(rect) => {
            machine.set_tip(Some(rect), now);
            note.anchored = true;
            note.shown = true;
            debug!(
                window = ?note.window,
                token = ?note.token,
                source,
                chars,
                "[tooltip_shown] ツールチップを出した"
            );
            Ok(())
        }
        Err(e) => {
            machine.set_tip(None, now);
            note.shown = false;
            warn!(
                window = ?note.window,
                token = ?note.token,
                source,
                chars,
                error = %e,
                "[tooltip_show_failed] ツールチップを出せなかった（出す番は続ける）"
            );
            Err(e)
        }
    }
}

/// ツールチップを消して記録する（状態機械の矩形は呼ぶ側が外す）。
fn hide(
    session: &mut TooltipSession,
    tip: &mut Tip<'_>,
    token: TooltipTurnToken,
    reason: HideReason,
) {
    (tip.hide)();
    match session.note.as_mut().filter(|n| n.token == token) {
        Some(n) => {
            n.shown = false;
            debug!(window = ?n.window, token = ?token, reason = ?reason, "[tooltip_hidden] ツールチップを消した");
        }
        None => debug!(token = ?token, reason = ?reason, "[tooltip_hidden] ツールチップを消した"),
    }
}

/// 画面更新の末尾に登録する系。本物の OS の読み取りと表示で判定と適用を 1 回行う。
pub(crate) fn tooltip_frame(world: &mut World) {
    let now = Instant::now();
    let visible = |w: &World, e: Entity| {
        w.get::<WindowHandle>(e)
            .is_some_and(|h| os::is_visible(h.hwnd))
    };
    with_os_tip(|tip| {
        frame_with(
            world,
            now,
            &mut os::sample,
            &visible,
            &mut os::read_hover_time,
            tip,
        )
    });
    // 出ている間は見回りのたびに、いつも手前の窓の中の最前へ当て直す（後から最前面へ
    // 当て直された別のいつも手前の窓に覆われるため）。
    let Some(mut session) = world.get_non_send_mut::<TooltipSession>() else {
        return;
    };
    let Some(note) = session.note.as_mut().filter(|n| n.shown) else {
        return;
    };
    if let Err(e) = TIP_WINDOW.with_borrow(TipWindow::keep_on_top)
        && !note.topmost_warned
    {
        note.topmost_warned = true;
        warn!(
            window = ?note.window,
            token = ?note.token,
            error = %e,
            "[tooltip_show_failed] ツールチップを手前へ当て直せなかった（この出す番では以後記録しない）"
        );
    }
}

/// 本物の表示（UI スレッドの標準のツールチップの窓）を表示の出入口にして f を呼ぶ。
pub(crate) fn with_os_tip<R>(f: impl FnOnce(&mut Tip<'_>) -> R) -> R {
    // 窓を借りるのは 1 回の表示・消去の間だけ（知らせの中から入れ子で呼ばれても借りられる）。
    let mut show = |text: &str, at: PointPx| TIP_WINDOW.with_borrow_mut(|w| w.show(text, at));
    let mut hide = || TIP_WINDOW.with_borrow_mut(TipWindow::hide);
    f(&mut Tip {
        show: &mut show,
        hide: &mut hide,
    })
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
