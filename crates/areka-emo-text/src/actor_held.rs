//! actor の子: 届いた物を列に持ち、フレームの頭で時刻の来た物を届いた順に当てる。
//!
//! UI のスレッドの受け取り（[`receive_msg`]）は当てずに列へ足し、[`TextLayerRuntime::advance`] が
//! フレームの時刻までに時刻の来た物を届いた順に当てる。時刻を持たない物（先渡し・トークの印）は、
//! 列が空なら受け取った場で当てる。

use std::cell::RefCell;
use std::ops::ControlFlow;

use areka_sakura::contract::{TalkCue, TimePoint};
use areka_sakura::strict::DetectionPoint;
use dola::cue::{CueProbe, OriginNotice};
use wintf::ecs::world::tick_wake;

use crate::sink::{TextMsg, handle_text_msg_with};

use super::TextLayerRuntime;

/// 列に持つ物 1 つ（届いた形のまま）。
#[derive(Debug)]
pub(super) enum Held {
    /// 合図（名前の形の `\b` には知らせの口が付く）。列に入れる時刻は `cue.fire_time`。
    Cue(TalkCue, Option<CueProbe>),
    /// 先渡し（時刻を持たない）。
    Upcoming(Vec<TalkCue>),
    /// 選択待ちの始まり（時刻を持たない）。
    ChoiceWaitBegan,
    /// トークの終わり（時刻を持たない）。
    TalkEnded,
    /// 置き直しの知らせ（前の起点が在る物だけ）。列に入れる時刻は効く時刻。
    Origin(OriginNotice),
}

impl TextLayerRuntime {
    /// フレームの頭で 1 回呼ぶ。`frame_now` までに時刻の来た物を届いた順に当て、列の頭の物が
    /// 待っている時刻を返す（待っている物が無ければ `None`）。
    pub fn advance(&mut self, frame_now: TimePoint) -> Option<TimePoint> {
        while let Some(item) = self.held.pop_due(frame_now) {
            self.apply_held(item);
        }
        self.held.waiting_until()
    }

    /// 届いた物を列の後ろへ足す。時刻を持たない物は、列が空なら待つ相手が無いのでその場で当てる。
    fn hold(&mut self, at: Option<TimePoint>, item: Held) {
        if at.is_none() && self.held.is_empty() {
            self.apply_held(item);
        } else {
            self.held.push(at, item);
        }
    }

    fn apply_held(&mut self, item: Held) {
        match item {
            Held::Cue(cue, probe) => {
                // 箱を探すのは当てる時の状態なので、知らせも当てる時に出す。
                if let (true, Some(probe)) = (self.apply_cue(&cue), probe) {
                    probe.report(DetectionPoint::BalloonBoxMissing.finding());
                }
            }
            Held::Upcoming(upcoming) => {
                // 次のトークの先渡し＝前のトークの選択待ちは終わっている。
                self.close_choice_wait();
                self.preview_talk(&upcoming);
            }
            Held::ChoiceWaitBegan => self.note_choice_wait_began(),
            Held::TalkEnded => {
                self.settle_all_choices();
                self.close_choice_wait();
            }
            Held::Origin(notice) => self.state.apply_origin_notice(&notice),
        }
    }
}

/// UI のスレッドの受け取りの口（[`super::spawn_emo_text`] の取り出しが呼ぶ）。終了の決まりは
/// [`handle_text_msg_with`] のまま。足した後（その場で当てた時は当てた後）に `PRESENT` の旗を立てる。
/// 文字の層が借用中なら `Err` を返し（基盤が `error!`＋継続）、その物は失われる。
pub(super) fn receive_msg(
    runtime: &RefCell<TextLayerRuntime>,
    msg: TextMsg,
) -> Result<ControlFlow<()>, String> {
    handle_text_msg_with(
        msg,
        |cue, probe| hold(runtime, Some(cue.fire_time), Held::Cue(cue, probe)),
        |upcoming| hold(runtime, None, Held::Upcoming(upcoming)),
        |mark| match mark {
            TextMsg::ChoiceWaitBegan => hold(runtime, None, Held::ChoiceWaitBegan),
            TextMsg::Origin(notice) => {
                hold(runtime, Some(notice.effective_at), Held::Origin(notice))
            }
            _ => hold(runtime, None, Held::TalkEnded),
        },
    )
}

fn hold(
    runtime: &RefCell<TextLayerRuntime>,
    at: Option<TimePoint>,
    item: Held,
) -> Result<(), String> {
    let Ok(mut rt) = runtime.try_borrow_mut() else {
        return Err(match item {
            Held::Cue(cue, _) => format!(
                "TextLayerRuntime が借用中のため cue を受け取れない（actor={}, fire_time={}）——当該 cue は失われる",
                cue.actor, cue.fire_time
            ),
            Held::Upcoming(upcoming) => format!(
                "TextLayerRuntime が借用中のため先渡しを受け取れない（cues={}）——そのトークは先渡し無し",
                upcoming.len()
            ),
            Held::Origin(notice) => format!(
                "TextLayerRuntime が借用中のため置き直しの知らせを受け取れない（effective_at={}）——字の時刻は動かない",
                notice.effective_at
            ),
            mark => format!("TextLayerRuntime が借用中のためトークの印を受け取れない（{mark:?}）"),
        });
    };
    rt.hold(at, item);
    drop(rt);
    // 足した後に立てる（先に立てると、足す前にフレームが回って旗だけが倒れる）。
    tick_wake::mark(tick_wake::PRESENT);
    Ok(())
}

#[cfg(test)]
#[path = "actor_held_tests.rs"]
mod tests;
