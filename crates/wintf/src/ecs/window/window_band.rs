//! 帯の働き——ゴーストの窓を、今の並びを保ったまま常に手前の窓の重なり（帯）へ入れる・
//! 帯から出す・今いる帯の最上へ 1 回出す（`stay-on-top-directives` 要件 2.4・2.5・2.10・7.3）。
//!
//! design.md「WindowBand（`window_band.rs`）」が正本である。状態（オン・オフ）は areka が
//! 持ち、ここは窓の印 [`StayOnTop`] と帯の指令の列 [`WindowBandQueue`] だけを受け取る。
//!
//! 本番で `HWND_TOPMOST`・`HWND_NOTOPMOST` を書くのはこのファイルだけ（ツールチップの既存の
//! 1 か所を除く）。`zorder_*` の本番ファイルに帯の語が無いことは
//! `zorder_pair_deferred_vocabulary_tests.rs`（守り G1）が見張る。
//!
//! # 帯を当てる手順
//!
//! 3 つの指令はどれも「その窓をその帯の最上へ置く」なので、今の重なりで奥の窓から順に
//! 当てると、最後の並びは当てた順＝元の並びになる。並べ方は [`back_to_front`]、挿入位置は
//! [`band_insert_after`]、当てた後の確かめは [`expected_band`] の純関数が決める。

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST};

/// この窓は常に手前の帯に居るべき（窓を作るときと HWND が付いた巡に読む）。
///
/// 状態の写しであって正本ではない（正本は areka の常に手前の状態）。
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct StayOnTop;

/// 帯の指令。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandOp {
    /// 帯へ入る（`\![set,windowstate,stayontop]`）
    Enter,
    /// 帯から出る（`\![set,windowstate,!stayontop]`）
    Leave,
    /// 今いる帯の最上へ 1 回出す（`\v`・帯は変えない）
    Raise,
}

/// 帯の指令の列。同じ巡のうちに積まれ、確定段の帯の系が積まれた順に当てて空にする。
#[derive(Resource, Debug, Default)]
pub struct WindowBandQueue {
    // 読むのは確定段の帯の系（同じファイル）だけ。
    #[cfg_attr(not(test), allow(dead_code))]
    ops: Vec<(BandOp, Vec<Entity>)>,
}

impl WindowBandQueue {
    /// 指令を 1 つ、対象の窓の entity の列と一緒に末尾へ積む。
    pub fn push(&mut self, op: BandOp, windows: Vec<Entity>) {
        self.ops.push((op, windows));
    }
}

/// 純関数：今の重なりの順位（0 が最上）で奥から手前へ並べる。
///
/// 順位が測れない窓は奥の端へ置く（手前の窓の並びを崩さない側へ倒す）。測れない窓どうし・
/// 同じ順位の窓どうしは元の順を保つ。
#[cfg_attr(not(test), allow(dead_code))] // 呼ぶのは確定段の帯の系（task 2.2）
pub(crate) fn back_to_front(
    windows: &[(Entity, HWND)],
    rank: impl Fn(HWND) -> Option<usize>,
) -> Vec<(Entity, HWND)> {
    let mut keyed: Vec<_> = windows.iter().map(|&w| (rank(w.1), w)).collect();
    // None は Some より前（奥）・Some どうしは順位の大きい（奥の）方が前。sort_by は安定。
    keyed.sort_by(|(a, _), (b, _)| match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(a), Some(b)) => b.cmp(a),
    });
    keyed.into_iter().map(|(_, w)| w).collect()
}

/// 純関数：指令 → 挿入位置（Enter = `HWND_TOPMOST`・Leave = `HWND_NOTOPMOST`・Raise = `HWND_TOP`）。
#[cfg_attr(not(test), allow(dead_code))] // 呼ぶのは確定段の帯の系（task 2.2）
pub(crate) fn band_insert_after(op: BandOp) -> HWND {
    match op {
        BandOp::Enter => HWND_TOPMOST,
        BandOp::Leave => HWND_NOTOPMOST,
        // HWND_TOP は帯を変えない（オフで手前へ出した窓は普通の窓のまま・要件 2.10）。
        BandOp::Raise => HWND_TOP,
    }
}

/// 純関数：当てた後に帯の印が期待どおりか（Raise は帯を変えないので確かめない＝`None`）。
#[cfg_attr(not(test), allow(dead_code))] // 呼ぶのは確定段の帯の系（task 2.2）
pub(crate) fn expected_band(op: BandOp) -> Option<bool> {
    match op {
        BandOp::Enter => Some(true),
        BandOp::Leave => Some(false),
        BandOp::Raise => None,
    }
}

#[cfg(test)]
#[path = "window_band_tests.rs"]
mod tests;
