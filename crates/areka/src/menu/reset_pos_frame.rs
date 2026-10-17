//! 右クリックメニューの「シェル位置初期化」の葉（areka-P0-move-canon 要件 8.3〜8.5・design ResetPosFrame）。
//!
//! 選ぶと `OnResetWindowPos` を答えの受け手つきで kanade へ送り、受け手を [`PendingResetRaise`]
//! （高々 1 つ）に置く。毎 tick の [`poll_reset_raise`] が答えを覗き、[`decide_reset`] の分類で
//! 並べ直すか（[`reset_window_positions`]）を決める。台本が返ればゴーストに任せて並べ直さず、
//! それ以外の答えでは並べ直す。待つ時間に上限は設けない（kanade は答えを 1 回必ず返し、会話が
//! 終われば受け手が切れる）。

use std::rc::Rc;

use areka_actor::{ReplyError, ReplyReceiver, reply_channel};
use areka_kanade::{KanadeMsg, RaiseOutcome, ShioriMethod};
use bevy_ecs::world::World;
use tracing::{info, warn};

use super::{Frame, ItemBody, MenuContext, MenuItem, captions};
use crate::ghost_session::GhostSlot;
use crate::placement::reset_pos::reset_window_positions;
use crate::placement::spawn::GhostWindows;

/// 「シェル位置初期化」枠の供給関数を登記する。呼び手はゴーストを起こすたびの結線（`wire_menu` の後）。
pub(crate) fn register(world: &mut World) {
    super::register(world, Frame::ResetPosition, Rc::new(reset_pos_item));
}

/// 「シェル位置初期化」の葉（要件 8.1・8.2）。選べるのは窓の組が在り、答えを待っていないとき
/// （動作は再入を拒まないので、待っている間はここで灰色にする）。
fn reset_pos_item(world: &World, _ctx: &MenuContext) -> MenuItem {
    MenuItem {
        label: captions::default_label(Frame::ResetPosition).to_string(),
        caption_resource: captions::resource_for(Frame::ResetPosition),
        enabled: world.contains_resource::<GhostWindows>()
            && !world.contains_non_send::<PendingResetRaise>(),
        checked: None,
        body: ItemBody::Action(Rc::new(|world: &mut World, _: &MenuContext| {
            raise_reset_window_pos(world)
        })),
    }
}

/// `OnResetWindowPos` の答えの受け手（NonSend・高々 1 つ）。在る間は答えを待っている。
pub(crate) struct PendingResetRaise(pub(crate) ReplyReceiver<RaiseOutcome>);

/// 答えを覗いた結果の決め。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetDecision {
    /// まだ答えが無い。
    Wait,
    /// 並べ直す。
    Reset,
    /// 並べ直さない。
    Leave,
}

/// 答えの分類（純粋）。まだ → 待つ、台本 → 並べ直さない、受け手が切れた → 並べ直さない、
/// それ以外（返事なし・`NotSteady`・`NotAllowed`・`Failed`）→ 並べ直す（要件 8.4・8.5・D8）。
pub(crate) fn decide_reset(outcome: Result<Option<RaiseOutcome>, ReplyError>) -> ResetDecision {
    match outcome {
        Ok(None) => ResetDecision::Wait,
        Ok(Some(RaiseOutcome::Script)) | Err(_) => ResetDecision::Leave,
        Ok(Some(
            RaiseOutcome::NoReply
            | RaiseOutcome::NotSteady
            | RaiseOutcome::NotAllowed
            | RaiseOutcome::Failed,
        )) => ResetDecision::Reset,
    }
}

/// 項目の動作: `OnResetWindowPos` を GET・答えの受け手つきで送り、受け手を置く（要件 8.3）。
/// 送り口が無い・送れないときは答えが無いのと同じく `warn!` のうえ並べ直す（要件 8.5）。
pub(crate) fn raise_reset_window_pos(world: &mut World) {
    let (tx, rx) = reply_channel();
    let sent = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.kanade())
        .is_some_and(|k| {
            k.send(KanadeMsg::RaiseEvent {
                id: "OnResetWindowPos".to_owned(),
                references: vec![],
                method: ShioriMethod::Get,
                reply: Some(tx),
            })
            .is_ok()
        });
    if sent {
        world.insert_non_send(PendingResetRaise(rx));
        return;
    }
    warn!(
        event = "reset_pos_menu_outcome",
        outcome = "not_sent",
        "OnResetWindowPos を送れなかった（kanade の送り口が無い）——既定の位置へ並べ直す"
    );
    reset_window_positions(world);
}

/// 毎 tick、答えの受け手を覗く（`Input` の段の排他の system）。答えが来ていれば受け手を外し、
/// 分類を `reset_pos_menu_outcome` に記録して、決めどおりに並べ直す。
pub(crate) fn poll_reset_raise(world: &mut World) {
    let Some(pending) = world.get_non_send::<PendingResetRaise>() else {
        return;
    };
    let outcome = pending.0.try_recv();
    match &outcome {
        Ok(None) => return,
        Ok(Some(RaiseOutcome::Script)) => info!(
            event = "reset_pos_menu_outcome",
            outcome = "script",
            "OnResetWindowPos に台本が返った——並べ直さない（台本は kanade が再生する）"
        ),
        Ok(Some(RaiseOutcome::NoReply)) => info!(
            event = "reset_pos_menu_outcome",
            outcome = "no_reply",
            "OnResetWindowPos に返事が無い——既定の位置へ並べ直す"
        ),
        Ok(Some(other)) => warn!(
            event = "reset_pos_menu_outcome",
            outcome = ?other,
            "OnResetWindowPos が送られなかった・往復が失敗した——既定の位置へ並べ直す"
        ),
        Err(err) => warn!(
            event = "reset_pos_menu_outcome",
            outcome = "dropped",
            %err,
            "OnResetWindowPos の答えの受け手が切れた（会話が終わった）——並べ直さない"
        ),
    }
    world.remove_non_send::<PendingResetRaise>();
    let decision = decide_reset(outcome);
    if decision == ResetDecision::Reset {
        reset_window_positions(world);
    }
}

#[cfg(test)]
#[path = "reset_pos_frame_tests.rs"]
mod tests;
