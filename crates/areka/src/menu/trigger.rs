//! メニューを出す引き金（areka-P0-popup-menu-minimal）。
//!
//! キャラクター窓で右ボタンを離したときのハンドラ（登記の写しを取り、照会を送る）、
//! 照会の返事を毎 tick 覗いて表示するかどうかを決める system、メニューを表示して
//! 選ばれた項目の動作を行うタスクを置く。
//!
//! このうち判断そのもの（[`poll_step`]・[`decide`]）は OS も World も触らない純粋関数で、
//! 時刻は引数で受け取る。待ち時間を実際に過ごさずに 4 通りを確かめられるようにするためで、
//! 兄弟テストはどれも寝ずに判定を全部踏む。

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use areka_actor::ReplyReceiver;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::HWND;

use crate::input_events::PendingDoubleClick;

use super::captions::{QueryFailure, QueryReply, Visibility};
use super::{Frame, MenuItem};

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
    pub(crate) fn engage(flag: &Rc<Cell<bool>>) -> Self {
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

#[cfg(test)]
#[path = "trigger_tests.rs"]
mod trigger_tests;
