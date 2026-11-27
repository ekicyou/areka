//! balloon_visibility の子: 寿命の知らせを届いた順のまま列に持ち、フレームの時刻が来た物を頭から判断へ渡す。
//!
//! 知らせが効くのは、フレームの時刻がその知らせの時刻（[`TalkLifecycleSignal::at`]）に届いた最初の
//! フレームで、頭の物が待つ間は後ろの物も待つ。寿命の線はフレームの中で取り出すので、フレームの
//! 時刻を読んだ後に刻まれた合図の知らせが同じフレームで届く事が在り、その知らせは次のフレームで効く。
//! 列を空にする口は置かない（列は結線が持ち、結線ごと作り直す時にだけ無くなる）。

use areka_sakura::held::HeldQueue;
use dola::{TimePoint, TimeSpan};

use super::TalkLifecycleSignal;

/// 線から取り出した知らせ `arrived` を届いた順のまま列の後ろへ足し、`frame_now` までに時刻の来た物を
/// 頭から順に返す。フレームの時刻が無ければ何も返さない（列に残る）。
pub(super) fn take_due(
    held: &mut HeldQueue<TalkLifecycleSignal>,
    arrived: Vec<TalkLifecycleSignal>,
    frame_now: Option<TimePoint>,
) -> Vec<TalkLifecycleSignal> {
    for signal in arrived {
        held.push(Some(signal.at()), signal);
    }
    let Some(now) = frame_now else {
        return Vec::new();
    };
    std::iter::from_fn(|| held.pop_due(now)).collect()
}

/// 列の頭の物が待っている時刻までの残り（起こしの予約に入れる長さ）。待つ物が無いか、フレームの時刻が
/// 無ければ `None`。
pub(super) fn head_wait(
    held: &HeldQueue<TalkLifecycleSignal>,
    frame_now: Option<TimePoint>,
) -> Option<TimeSpan> {
    held.waiting_until()?
        .span_since(frame_now?)
        .filter(|left| *left > TimeSpan::ZERO)
}

#[cfg(test)]
#[path = "balloon_visibility_held_tests.rs"]
mod tests;
