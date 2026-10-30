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

use std::collections::HashMap;

use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GW_HWNDNEXT, GWL_EXSTYLE, GetTopWindow, GetWindow, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
};

use super::WindowHandle;
use super::command::guarded_set_window_pos;
use crate::api::get_window_long_ptr;

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
pub(crate) fn band_insert_after(op: BandOp) -> HWND {
    match op {
        BandOp::Enter => HWND_TOPMOST,
        BandOp::Leave => HWND_NOTOPMOST,
        // HWND_TOP は帯を変えない（オフで手前へ出した窓は普通の窓のまま・要件 2.10）。
        BandOp::Raise => HWND_TOP,
    }
}

/// 純関数：当てた後に帯の印が期待どおりか（Raise は帯を変えないので確かめない＝`None`）。
pub(crate) fn expected_band(op: BandOp) -> Option<bool> {
    match op {
        BandOp::Enter => Some(true),
        BandOp::Leave => Some(false),
        BandOp::Raise => None,
    }
}

/// 順位を測るときに最上から辿る窓の上限（これを超えた先の窓は測れない＝奥の端）。
const RANK_WALK_LIMIT: usize = 4096;

/// 確定段の系：列の指令を積まれた順に当て、この巡に HWND が付いた印のある窓を帯へ入れ、
/// 列を空にする（要件 2.1・2.3・2.6・2.9・3.1・6.1・6.3・7.3）。
///
/// 排他の系なので UI スレッド（スケジュールを回すスレッド）で走る。
pub fn apply_window_band(world: &mut World) {
    // 取り出した時点で列は空になる（⑶）。
    let ops = world
        .get_resource_mut::<WindowBandQueue>()
        .map(|mut queue| std::mem::take(&mut queue.ops))
        .unwrap_or_default();
    for (op, entities) in ops {
        apply_op(world, op, &entities);
    }

    // ⑵ 作る時点で帯に入り損ねた窓の保険。帯に居る窓には書かない。
    let mut fresh =
        world.query_filtered::<(Entity, &WindowHandle), (Added<WindowHandle>, With<StayOnTop>)>();
    let outside: Vec<Entity> = fresh
        .iter(world)
        .filter(|(_, handle)| os_in_band(handle.hwnd) != Some(true))
        .map(|(entity, _)| entity)
        .collect();
    if !outside.is_empty() {
        apply_op(world, BandOp::Enter, &outside);
    }
}

/// 指令を 1 つ当てる（帯を当てる手順の 1〜3）。
fn apply_op(world: &World, op: BandOp, entities: &[Entity]) {
    let mut targets = Vec::with_capacity(entities.len());
    let mut skipped = 0usize;
    for &entity in entities {
        let Ok(entity_ref) = world.get_entity(entity) else {
            debug!(?entity, ?op, "[window_band] skipped: entity not found");
            skipped += 1;
            continue;
        };
        match entity_ref.get::<WindowHandle>() {
            Some(handle) => targets.push((entity, handle.hwnd)),
            None => {
                debug!(?entity, ?op, "[window_band] skipped: no HWND yet");
                skipped += 1;
            }
        }
    }

    let ranks = measure_ranks(&targets);
    for (_, hwnd) in &targets {
        if !ranks.contains_key(&key(*hwnd)) {
            warn!(
                hwnd = format!("0x{:X}", key(*hwnd)),
                "[window_band] z-order rank not measured, placed at the back"
            );
        }
    }
    let ordered = back_to_front(&targets, |hwnd| ranks.get(&key(hwnd)).copied());

    let insert_after = band_insert_after(op);
    let mut applied = 0usize;
    for (_, hwnd) in &ordered {
        match os_write(*hwnd, insert_after) {
            Ok(()) => applied += 1,
            // 1 枚の失敗で止めず、残りの窓を書く。
            Err(error) => {
                warn!(hwnd = format!("0x{:X}", key(*hwnd)), ?op, %error, "[window_band] SetWindowPos failed")
            }
        }
    }

    if let Some(expected) = expected_band(op) {
        for (_, hwnd) in &ordered {
            let actual = os_in_band(*hwnd);
            if actual != Some(expected) {
                warn!(
                    hwnd = format!("0x{:X}", key(*hwnd)),
                    ?op,
                    expected,
                    ?actual,
                    "[window_band] band mismatch after apply"
                );
            }
        }
    }
    debug!(?op, applied, skipped, "[window_band] applied");
}

/// 対象の窓の今の重なりの順位（0 が最上）を、最上の窓から辿って測る。
fn measure_ranks(targets: &[(Entity, HWND)]) -> HashMap<usize, usize> {
    let mut ranks = HashMap::new();
    if targets.is_empty() {
        return ranks;
    }
    let mut current = os_top();
    for rank in 0..RANK_WALK_LIMIT {
        let Some(hwnd) = current else { break };
        if targets.iter().any(|(_, t)| *t == hwnd) {
            ranks.insert(key(hwnd), rank);
            if ranks.len() == targets.len() {
                break;
            }
        }
        current = os_next(hwnd);
    }
    ranks
}

fn key(hwnd: HWND) -> usize {
    hwnd.0 as usize
}

// ============================================================================
// 実行環境への窓口（4 本・テストでは兄弟の偽の OS へ差し替える）
// ============================================================================

/// 最上のトップレベル窓。
fn os_top() -> Option<HWND> {
    #[cfg(test)]
    if let Some(scripted) = tests::double::top() {
        return scripted;
    }
    // SAFETY: Win32 境界。読み取りだけ。
    unsafe { GetTopWindow(None) }.ok()
}

/// 生の重なりで 1 つ奥の窓（不可視の窓も含む）。
fn os_next(hwnd: HWND) -> Option<HWND> {
    #[cfg(test)]
    if let Some(scripted) = tests::double::next(hwnd) {
        return scripted;
    }
    // SAFETY: Win32 境界。読み取りだけ。
    unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok()
}

/// 帯の印（`WS_EX_TOPMOST`）を読む。読めなければ `None`。
fn os_in_band(hwnd: HWND) -> Option<bool> {
    #[cfg(test)]
    if let Some(scripted) = tests::double::in_band(hwnd) {
        return scripted;
    }
    get_window_long_ptr(hwnd, GWL_EXSTYLE)
        .ok()
        .map(|style| style as u32 & WS_EX_TOPMOST.0 != 0)
}

/// 帯を直に書く（位置・大きさ・活性を変えない＝要件 2.6）。
fn os_write(hwnd: HWND, insert_after: HWND) -> windows::core::Result<()> {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    #[cfg(test)]
    if let Some(scripted) = tests::double::write(hwnd, insert_after, flags) {
        return scripted;
    }
    // SAFETY: Win32 境界。渡すのは HWND と整数だけ。
    unsafe { guarded_set_window_pos(hwnd, Some(insert_after), 0, 0, 0, 0, flags) }
}

#[cfg(test)]
#[path = "window_band_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "window_band_real_tests.rs"]
mod real_tests;

#[cfg(test)]
#[path = "window_band_pipeline_real_tests.rs"]
mod pipeline_real_tests;
