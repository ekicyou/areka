//! 届いた物を、届いた順のまま、絶対の時刻が来るまで持つ列

use std::collections::VecDeque;

use dola::TimePoint;

/// フレームで評価する受け手が、届いた物を、届いた順のまま、時刻が来るまで持つ列。
///
/// 起点も台本の時刻も持たない。時計も窓も見ない。
#[derive(Debug)]
pub struct HeldQueue<T> {
    items: VecDeque<(Option<TimePoint>, T)>,
}

impl<T> HeldQueue<T> {
    /// 空の列を作る。
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
        }
    }

    /// 列の後ろへ足す。`at` は、その物が効く絶対の時刻。`None` は「自分の時刻を持たない」（前の物に続く）。
    pub fn push(&mut self, at: Option<TimePoint>, item: T) {
        self.items.push_back((at, item));
    }

    /// 列の頭の物の時刻が来ていれば（`at` が `frame_now` 以前、または `at` が `None`）、取り出す。
    ///
    /// 来ていなければ `None`（後ろの物を先に出さない）。
    pub fn pop_due(&mut self, frame_now: TimePoint) -> Option<T> {
        match self.items.front()? {
            (Some(at), _) if *at > frame_now => None,
            _ => self.items.pop_front().map(|(_, item)| item),
        }
    }

    /// 列の頭の物が待っている時刻（列が空か、頭の物が時刻を持たなければ `None`）。
    pub fn waiting_until(&self) -> Option<TimePoint> {
        self.items.front().and_then(|(at, _)| *at)
    }

    /// 列が空か。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T> Default for HeldQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "held_tests.rs"]
mod tests;
