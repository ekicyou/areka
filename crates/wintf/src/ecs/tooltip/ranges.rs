//! 窓ごとの範囲の表と当たり（純粋）。
//!
//! 範囲は窓の中の論理の単位（96 DPI の 1 ピクセル）で持つ。論理への換算は呼ぶ側がする。
//! 矩形は左と上の端を含み、右と下の端を含まない。

use crate::ecs::{PointF, Rect};
use bevy_ecs::prelude::*;

/// 範囲の場所。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipArea {
    /// 窓の全体。
    WholeWindow,
    /// 窓の中の矩形（論理の単位）。
    Rect(Rect),
}

/// 範囲の登録の中身。
#[derive(Debug, Clone, PartialEq)]
pub struct TooltipRange {
    pub area: TooltipArea,
    /// 預ける文字。None と空は「預けない」。
    pub text: Option<String>,
}

/// 登録した範囲の持ち手（中身は非公開）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TooltipRangeId {
    window: Entity,
    // u64 なので、増やし続けても一回りして使い回すことはない。
    serial: u64,
}

impl TooltipRangeId {
    /// 範囲を登録した窓。
    pub fn window(&self) -> Entity {
        self.window
    }
}

/// 窓のエンティティに付ける範囲の表（中身は非公開）。
///
/// 並びは登録の順。窓のエンティティが消えれば表も一緒に消える。
#[derive(Component, Debug, Default)]
pub struct TooltipRanges {
    // 持ち手には窓も入っているので、別の窓の持ち手では当たらない。
    entries: Vec<(TooltipRangeId, TooltipRange)>,
    next_serial: u64,
}

impl TooltipRanges {
    /// 範囲を並びの最後に足し、持ち手を返す。通し番号は増えるだけ。
    pub(crate) fn add(&mut self, window: Entity, range: TooltipRange) -> TooltipRangeId {
        let id = TooltipRangeId {
            window,
            serial: self.next_serial,
        };
        self.next_serial += 1;
        self.entries.push((id, normalize(range)));
        id
    }

    /// 中身を差し替える。並びの位置は変えない。持ち手が無ければ偽。
    pub(crate) fn replace(&mut self, id: TooltipRangeId, range: TooltipRange) -> bool {
        match self.entries.iter_mut().find(|(k, _)| *k == id) {
            Some((_, r)) => {
                *r = normalize(range);
                true
            }
            None => false,
        }
    }

    /// 登録を取り消す。持ち手が無ければ偽。
    pub(crate) fn remove(&mut self, id: TooltipRangeId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|(k, _)| *k != id);
        self.entries.len() != before
    }

    /// 持ち手の指す登録の中身。
    pub(crate) fn get(&self, id: TooltipRangeId) -> Option<&TooltipRange> {
        self.entries.iter().find(|(k, _)| *k == id).map(|(_, r)| r)
    }

    /// 論理の点に当たる範囲（後から登録したものが勝つ）。
    pub(crate) fn hit(&self, logical: PointF) -> Option<(TooltipRangeId, &TooltipRange)> {
        self.entries
            .iter()
            .rev()
            .find(|(_, r)| contains(r.area, logical))
            .map(|(k, r)| (*k, r))
    }
}

/// 空の文字は預けていないものとして持つ。
fn normalize(mut range: TooltipRange) -> TooltipRange {
    if range.text.as_deref() == Some("") {
        range.text = None;
    }
    range
}

fn contains(area: TooltipArea, p: PointF) -> bool {
    match area {
        TooltipArea::WholeWindow => true,
        TooltipArea::Rect(r) => r.left <= p.x && p.x < r.right && r.top <= p.y && p.y < r.bottom,
    }
}

#[cfg(test)]
#[path = "ranges_tests.rs"]
mod ranges_tests;
