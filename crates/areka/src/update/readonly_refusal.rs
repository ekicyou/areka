//! 読み取りだけの走りの更新の断り（readonly-mode・design「UpdateRefusal」）。
//!
//! 段・預かり・背景のスレッドには触れず、ネットワークへも行かない。readonly の SSP の測り
//! （2026-10-23）どおり、成否の欄が空の `OnUpdateResultEx` → `OnUpdateResult` だけを送る。

use areka_kanade::{KanadeMsg, ShioriMethod};
use bevy_ecs::world::World;

use super::refs::{ON_UPDATE_RESULT, ON_UPDATE_RESULT_EX, readonly_summary_refs};
use super::{RawUpdateRequest, SummaryKind, UpdateReason, desk};
use crate::ghost_session::GhostSlot;

/// 対象を解き、置き場のゴーストへ Ex → Result を返事なしで 1 件ずつ送り（対象が 0 なら送らない）、
/// `info!(readonly_refused)` を 1 件残す（送れたかを添える・要件 7.3〜7.5・11.3）。
pub(super) fn refuse(world: &World, raw: &RawUpdateRequest, reason: UpdateReason) {
    let targets = desk::resolve_targets(world, raw);
    let kanade = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|s| s.kanade());
    let notified = !targets.is_empty()
        && kanade.is_some_and(|kanade| {
            [
                (ON_UPDATE_RESULT_EX, SummaryKind::ResultEx),
                (ON_UPDATE_RESULT, SummaryKind::Result),
            ]
            .into_iter()
            .all(|(id, kind)| {
                kanade
                    .send(KanadeMsg::RaiseEvent {
                        id: id.to_owned(),
                        references: readonly_summary_refs(kind, &targets),
                        method: ShioriMethod::Get,
                        reply: None,
                    })
                    .is_ok()
            })
        });
    tracing::info!(
        event = "readonly_refused",
        what = "update",
        reason = reason.as_ref_str(),
        targets = targets.len(),
        notified,
        "[update] 読み取りだけなので、ネットワーク更新を断ります（notified＝総括を送れたか）"
    );
}

#[cfg(test)]
#[path = "readonly_refusal_tests.rs"]
mod tests;
