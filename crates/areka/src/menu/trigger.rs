//! メニューを出す引き金（areka-P0-popup-menu-minimal）。
//!
//! キャラクター窓で右ボタンを離したときのハンドラ（[`on_char_pointer_released`]＝登記の写しを
//! 取り、照会を送って返事待ちを預ける）と、照会の返事を毎 tick 覗いて表示するかどうかを決め、
//! 表示する計画まで作る system（[`poll_menu_query`]）を置く。
//!
//! このうち判断そのもの（[`poll_step`]・[`decide`]）は OS も World も触らない純粋関数で、
//! 時刻は引数で受け取る。待ち時間を実際に過ごさずに 4 通りを確かめられるようにするためで、
//! 兄弟テストはどれも寝ずに判定を全部踏む。World を触る 2 つの入口も、時刻を引数で受ける
//! 内側の関数（[`handle_release`]・[`poll_once`]）へ仕事を渡すだけにしてある。

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use areka_actor::ReplyReceiver;
use bevy_ecs::prelude::{Entity, World};
use windows::Win32::Foundation::HWND;
use wintf::ecs::WindowHandle;
use wintf::ecs::drag::{DragStateSnapshot, snapshot_drag_state};
use wintf::ecs::pointer::{Phase, PointerState};

use crate::input_events::{MouseWiring, PendingDoubleClick, char_scope};

use super::captions::{self, QUERY_TIMEOUT, QueryFailure, QueryReply, Visibility};
use super::plan::{self, MenuPlan};
use super::{Frame, MenuContext, MenuItem, MenuWiring};

/// メニューを 1 枚出すための要求（右ボタンを離した時点の材料）。
pub(crate) struct MenuRequest {
    /// 操作した窓のスコープ番号（本体側 0・相方側 1）。
    pub scope: u32,
    /// 操作した窓の entity。表示の直前に窓がまだ在るか確かめるのに使う。
    pub entity: Entity,
    /// メニューの持ち主にする窓ハンドル（メニュー用の窓は作らない・要件 7.5）。
    pub hwnd: HWND,
    /// メニューを出す画面座標。
    pub screen_pos: (i32, i32),
}

/// 表示が 1 枚だけであることを保つ旗の持ち主。
///
/// 作ると旗が立ち、落ちると必ず旗が降りる。早期に戻る経路でも降ろし忘れが起きないよう、
/// 降ろすのは [`Drop`] の 1 か所だけにしてある。
pub(crate) struct InFlightGuard(Rc<Cell<bool>>);

impl InFlightGuard {
    /// 旗を立てて持ち主を作る。
    ///
    /// 旗が既に立っているときに呼んではならない。解放ハンドラは旗が立っていれば要求を
    /// 受けずに戻るので、ここへ来るのは旗が降りているときだけである。
    pub(crate) fn engage(flag: &Rc<Cell<bool>>) -> Self {
        debug_assert!(!flag.get(), "表示 1 枚の旗が立ったまま次の要求を受けている");
        flag.set(true);
        InFlightGuard(Rc::clone(flag))
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// 照会の返事待ち 1 件分。返事が来るか期限が切れるまで、メニューの結線状態が抱える。
pub(crate) struct PendingQuery {
    /// 右ボタンを離した時点の要求。
    pub request: MenuRequest,
    /// 右クリックの時点で取った登記の写し（要件 6.2）。
    pub snapshot: Vec<(Frame, MenuItem)>,
    /// 返事の受け口。`None` は照会を送れなかったことを表し、次の tick で全件が既定名になる。
    pub rx: Option<ReplyReceiver<QueryReply>>,
    /// 返事を待つ期限（[`super::captions::QUERY_TIMEOUT`] を足した時刻）。
    pub deadline: Instant,
    /// 表示 1 枚の旗。この待ちが捨てられた時点で自動的に降りる。
    guard: InFlightGuard,
}

impl PendingQuery {
    /// 返事待ちを組み立てる。
    pub(crate) fn new(
        request: MenuRequest,
        snapshot: Vec<(Frame, MenuItem)>,
        rx: Option<ReplyReceiver<QueryReply>>,
        deadline: Instant,
        guard: InFlightGuard,
    ) -> Self {
        PendingQuery {
            request,
            snapshot,
            rx,
            deadline,
            guard,
        }
    }
}

/// 1 tick 分の返事の覗き見の結果。
#[derive(Debug)]
pub(crate) enum PollOutcome {
    /// まだ返事が来ておらず期限も残っている。次の tick でもう一度覗く。
    Wait,
    /// 待ちは終わった。返事そのものか、返事が得られなかった理由を持つ。
    Decided(Result<QueryReply, QueryFailure>),
}

/// 返事を 1 回覗いて、待ちを続けるか終えるかを決める（純粋・要件 3.4）。
///
/// 覗く順は ⑴ 返事の有無 → ⑵ 期限。既に届いている返事は期限が切れていても使う——待った理由は
/// 返事を得ることであり、手元にある返事を捨てて既定名に落とす理由が無いからである。
///
/// 送れなかった照会（`rx` が `None`）は覗くものが無いので、その場で「送れなかった」と決める。
///
/// 届いていた返事はこの呼び出しで受け口から取り出される。同じ返事待ちをもう一度覗いても
/// 同じ返事は出てこないので、呼び手は 1 tick に 1 回だけ呼び、`Decided` の中身を必ず使い切る。
pub(crate) fn poll_step(pending: &PendingQuery, now: Instant) -> PollOutcome {
    let Some(rx) = pending.rx.as_ref() else {
        return PollOutcome::Decided(Err(QueryFailure::SendFailed));
    };
    match rx.try_recv() {
        Ok(Some(reply)) => PollOutcome::Decided(Ok(reply)),
        Ok(None) if now < pending.deadline => PollOutcome::Wait,
        Ok(None) => PollOutcome::Decided(Err(QueryFailure::Timeout)),
        // `try_recv` が返す失敗は返信端が捨てられたときだけ（上限超過はこちらで測る）。
        Err(_) => PollOutcome::Decided(Err(QueryFailure::Dropped)),
    }
}

/// 表示可否と、預かった右ダブルクリックの行き先（要件 1.10・3.7・11.3）。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// メニューを出す。預かりがあれば捨てる（送らない）。
    Show,
    /// メニューを出さない。
    Suppress {
        /// 預かっていた右ダブルクリックを `OnMouseDoubleClick`（Ref5＝1）として送るか。
        send_double_click: bool,
    },
}

/// メニューを出すかどうかと、預かった右ダブルクリックを送るかどうかを 1 か所で決める（純粋）。
///
/// 正典の Ref5＝1 は「メニューを抑止したゴースト」のためのものなので、送るのは抑止したときだけで、
/// メニューを出すなら預かりは捨てる（要件 1.10）。1 度目の解放と 2 度目の押下が同じ処理単位に
/// 入っても、送る／送らないを決める場所はここ 1 か所に限る（要件 11.3）。
pub(crate) fn decide(visibility: Visibility, pending: Option<&PendingDoubleClick>) -> Decision {
    match visibility {
        Visibility::Show => Decision::Show,
        Visibility::Suppress => Decision::Suppress {
            send_double_click: pending.is_some(),
        },
    }
}

/// 表示の準備が整ったメニュー 1 枚分。
pub(crate) struct ReadyMenu {
    /// 右ボタンを離した時点の要求（持ち主の窓と、出す画面座標）。
    pub request: MenuRequest,
    /// 照会で決まった項目名を写した計画。
    pub plan: MenuPlan,
    /// 表示 1 枚の旗。この値が捨てられた時点で降りる。
    guard: InFlightGuard,
}

/// キャラクター窓のポインタ解放ハンドラ（Bubble 相のみ処理）。
///
/// 右ボタンの解放だけを見る。結線前・ドラッグ中なら、預かっていた右ダブルクリックを捨てて
/// `trace!` で記録し、何もしない（要件 1.8・1.9）。返事待ちまたは表示中（表示 1 枚の旗が立って
/// いる間）も `trace!` で記録して何もしないが、**預かりには触れない**（理由は下）。それ以外は
/// 右クリックの時点の登記の写しを取り（要件 6.2）、運行へ照会を送って返事待ちを預け、表示
/// 1 枚の旗を立てる。**返事は待たない**——覗くのは毎 tick の [`poll_menu_query`] である。
///
/// 旗が立っている間の解放が預かりに触れないのは、右ダブルクリックが「押下 1 →解放 1 →押下 2
/// （ここで預かる）→解放 2」の順に届くからである。SHIORI の返事がダブルクリックの間隔より
/// 遅いと、解放 2 は解放 1 の照会がまだ返事を待っている間に届く。そこで預かりを捨てると、
/// 後から `popupmenu.visible`＝0 が返ってきても送る材料が残っておらず、メニューを抑止する
/// ゴーストへ右ダブルクリックが届かなくなる（要件 1.10）。預かりは残しておき、送るか捨てるかは
/// 返事が決着した tick の [`decide`] が決める。
///
/// Tunnel 相は伝播を続けるため常に `false`。要求を預かったときだけ `true`。
pub(crate) fn on_char_pointer_released(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let state = match ev {
        Phase::Tunnel(_) => return false,
        Phase::Bubble(state) => state,
    };
    handle_release(world, entity, state, Instant::now())
}

/// 解放 1 件分の仕事。返事を待つ期限は `now` から数える。
fn handle_release(world: &mut World, entity: Entity, state: &PointerState, now: Instant) -> bool {
    if !state.released.right {
        return false;
    }
    if world.get_non_send::<MouseWiring>().is_none() || world.get_non_send::<MenuWiring>().is_none()
    {
        return ignore_release(world, "not wired");
    }
    if !matches!(snapshot_drag_state(), DragStateSnapshot::Idle) {
        return ignore_release(world, "dragging");
    }

    // ここから先の読み取りはどれも共有借用で足りる。供給関数は `&World` を受け取り、自分でも
    // World の資源（説明書の結線など）を読むので、`MenuWiring` を可変で借りたままでは写しを
    // 取れない。共有借用どうしなら両立するので、可変で借りるのは最後に預けるときだけにする。
    let wiring = world
        .get_non_send::<MenuWiring>()
        .expect("MenuWiring は直上で存在確認済み");
    if wiring.in_flight.get() {
        // 預かりには触れない（この関数を呼ぶ入口の doc に理由がある）。
        return trace_ignored_release("menu in flight", false);
    }
    let Some(scope) = char_scope(world, entity) else {
        return ignore_release(world, "not a character window");
    };
    let Some(hwnd) = world.get::<WindowHandle>(entity).map(|handle| handle.hwnd) else {
        return ignore_release(world, "no window handle");
    };
    let Some(screen_pos) = (wiring.to_screen)(hwnd, state.client_point.x, state.client_point.y)
    else {
        return ignore_release(world, "client_to_screen failed");
    };

    let snapshot = wiring.registry.snapshot(world, &MenuContext { scope });
    // 送れなかった理由はここでは記録しない。受け口の無い返事待ちとして預け、次の tick の
    // `interpret` が「全件既定名」の警告 1 行にまとめる（要件 3.4）。
    let rx = captions::send_query(&wiring.kanade, captions::query_ids(&snapshot, scope)).ok();
    let guard = InFlightGuard::engage(&wiring.in_flight);

    let request = MenuRequest {
        scope,
        entity,
        hwnd,
        screen_pos,
    };
    world
        .get_non_send_mut::<MenuWiring>()
        .expect("MenuWiring は直上で存在確認済み")
        .pending = Some(PendingQuery::new(
        request,
        snapshot,
        rx,
        now + QUERY_TIMEOUT,
        guard,
    ));
    true
}

/// 右ボタンの解放を受けずに終える。預かっていた右ダブルクリックは捨てる——この解放からは
/// 照会が出ないので、残しておくと後の無関係な要求の抑止で送られてしまうからである。常に `false`。
///
/// 表示 1 枚の旗が立っている間の解放だけはここを通らない（先の照会の決着が預かりを使う）。
fn ignore_release(world: &mut World, reason: &'static str) -> bool {
    let dropped_double_click = take_deferred_double_click(world).is_some();
    trace_ignored_release(reason, dropped_double_click)
}

/// 解放を受けなかったことを記録する。常に `false`。
fn trace_ignored_release(reason: &'static str, dropped_double_click: bool) -> bool {
    tracing::trace!(
        event = "menu_release_ignored",
        reason = reason,
        dropped_double_click = dropped_double_click,
        "[menu] ignored release"
    );
    false
}

/// 預かっている右ダブルクリックを取り出す（入力の結線が無ければ預かりも無い）。
fn take_deferred_double_click(world: &mut World) -> Option<PendingDoubleClick> {
    world
        .get_non_send_mut::<MouseWiring>()?
        .take_pending_right_double_click()
}

/// 毎 tick 動く system（`Input` スケジュール）。返事待ちを覗き、決着していれば計画まで作る。
///
/// できた計画はここで手放す。手放すと表示 1 枚の旗が降り、次の右クリックを受けられる。
pub(crate) fn poll_menu_query(world: &mut World) {
    drop(poll_once(world, Instant::now()));
}

/// 返事待ちを 1 回覗く。表示する計画ができたときだけ `Some`。
///
/// 返事待ちが無い tick は最初の判定で戻る（通常の tick の費用はこれだけ）。返事も期限もまだなら
/// 返事待ちを残して戻る。決着したら返事待ちを取り出し、⑴ 窓が既に無ければ記録して捨てる、
/// ⑵ 表示が抑止されていれば預かっていた右ダブルクリックを送って終える、⑶ それ以外は預かりを
/// 捨てて計画を作る。⑴⑵ では旗の持ち主がここで落ちるので、表示 1 枚の旗はその場で降りる。
fn poll_once(world: &mut World, now: Instant) -> Option<ReadyMenu> {
    let result = match poll_step(world.get_non_send::<MenuWiring>()?.pending.as_ref()?, now) {
        PollOutcome::Wait => return None,
        PollOutcome::Decided(result) => result,
    };
    let PendingQuery {
        request,
        snapshot,
        guard,
        ..
    } = world.get_non_send_mut::<MenuWiring>()?.pending.take()?;
    // 預かりはどの決着でもここで取り出す。窓が消えていた場合も含めて、後の要求へ持ち越さない。
    // 返事を待っている間に届いた右ダブルクリック（2 度目の押下）もここで拾われる。
    let deferred = take_deferred_double_click(world);

    if world.get::<WindowHandle>(request.entity).is_none() {
        tracing::debug!(
            event = "menu_window_gone_while_waiting",
            scope = request.scope,
            entity = ?request.entity,
            "[menu] window gone while waiting for the reply: the request is dropped"
        );
        return None;
    }

    let scope = request.scope;
    let interpreted = captions::interpret(result, captions::visible_resource_for(scope), scope);
    match decide(interpreted.visibility, deferred.as_ref()) {
        // 抑止の記録（`info!`）は `interpret` が出している。
        Decision::Suppress { send_double_click } => {
            if send_double_click && let Some(deferred) = deferred {
                world
                    .get_non_send_mut::<MouseWiring>()
                    .expect("預かりを取り出せたので MouseWiring は在る")
                    .send_pending_right_double_click(deferred);
            }
            None
        }
        Decision::Show => {
            if deferred.is_some() {
                tracing::trace!(
                    event = "menu_deferred_double_click_dropped",
                    scope = scope,
                    "[menu] dropped the deferred right double-click: the menu is shown"
                );
            }
            Some(ReadyMenu {
                request,
                plan: plan::build(snapshot, &interpreted.captions),
                guard,
            })
        }
    }
}

#[cfg(test)]
#[path = "trigger_tests.rs"]
mod trigger_tests;

#[cfg(test)]
#[path = "trigger_flow_tests.rs"]
mod trigger_flow_tests;
