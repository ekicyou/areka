//! 倍率の変わりの相（shell-scale-linkage 要件 5.1〜5.9・design「ScaleNoticePhase」）。
//!
//! 表示層が表示成立点で積んだ [`ScaleChange`] を取り出し、スコープの倍率の表を書き、変わった窓ごとに
//! `OnShellScaling`／`OnBalloonScaling` を送る。判断は純粋な計画 [`plan_scale_notices`] にだけ置き、
//! 書き込みと送出の殻は計画をそのまま実行する。

use std::collections::BTreeMap;
use std::sync::PoisonError;
use std::sync::mpsc::Sender;

use areka_emo_present::{AppliedScale, ScaleChange};
use areka_kanade::{KanadeMsg, ShioriMethod};
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;
use tracing::{debug, info, warn};

use super::Emo2Wiring;
use crate::emo2_boot::target_map::{TargetWindow, target_scope};
use crate::ghost_session::GhostSlot;
use crate::ghost_session::ghost_session_scope_scale::{percent_text, scaling_text};

/// テストだけが入れる送り先（本番は入れない＝置き場のゴーストの kanade へ送る）。
#[derive(Resource, Clone)]
pub(crate) struct ScaleEventSeam(pub(crate) Sender<KanadeMsg>);

/// 倍率の変わりの相（毎フレームの並びの最後）。記録が無いフレームはすぐ戻る。
///
/// 表を全部書いてから、イベントを計画の順に汎用の入口（`RaiseEvent`・GET）で送る。応答の台本の
/// `\![get,property]` が変わった後の値を読むためである（要件 5.5）。送れないときも表は書いたまま。
pub(super) fn run_scale_notice_phase(wiring: &mut Emo2Wiring, world: &World) {
    let changes = wiring.presenter.take_scale_changes();
    if changes.is_empty() {
        return;
    }
    let plan = plan_scale_notices(&changes);
    {
        // 毒は前の持ち主の panic で、表の中身は壊れていない（読み手と同じ扱い）。
        let mut table = wiring
            .scope_scale
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        for &(scope, window, value) in &plan.writes {
            table.set(scope, window, value);
            debug!(
                event = "scope_scale_table_updated",
                scope,
                window = ?window,
                scaling = %scaling_text(value.ratio),
                dpi = ?value.dpi_x,
                "スコープの倍率の表を書いた"
            );
        }
    }
    if plan.events.is_empty() {
        return;
    }
    let kanade = match world.get_resource::<ScaleEventSeam>() {
        Some(seam) => Some(seam.0.clone()),
        None => world
            .get_non_send::<GhostSlot>()
            .and_then(|slot| slot.0.as_ref())
            .and_then(|session| session.kanade())
            .cloned(),
    };
    for event in plan.events {
        let (id, scope) = (event.id, event.scope);
        let (after, before) = (event.references[0].clone(), event.references[1].clone());
        let msg = KanadeMsg::RaiseEvent {
            id: id.to_string(),
            references: event.references,
            method: ShioriMethod::Get,
            reply: None,
        };
        if kanade.as_ref().is_some_and(|tx| tx.send(msg).is_ok()) {
            info!(
                event = "scale_event_raised",
                id,
                scope,
                before = %before,
                after = %after,
                "倍率の変わりのイベントを送った"
            );
        } else {
            warn!(
                event = "scale_event_not_sent",
                id, scope, "kanade が居ないので倍率の変わりのイベントを送れない（表は書いた）"
            );
        }
    }
}

/// 送る 1 件のイベント（名前・スコープ・Reference0〜4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScaleEvent {
    /// `"OnShellScaling"` か `"OnBalloonScaling"`。
    pub id: &'static str,
    pub scope: u32,
    /// 後・前・後・前・スコープ番号の 5 つ（横と縦は同じ数・要件 5.3・5.4）。
    pub references: Vec<String>,
}

/// 1 フレームの計画（表への書き込みと送るイベント・どちらも `TargetId` の昇順）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ScaleNoticePlan {
    pub writes: Vec<(u32, TargetWindow, AppliedScale)>,
    pub events: Vec<ScaleEvent>,
}

/// 記録の列から計画を作る（純粋）。
///
/// target ごとに最初の `before`・最後の `after` へ畳み、`TargetId` の昇順（スコープの若い順・同じ
/// スコープはシェル → バルーン）に並べる。書き込みは target ごとに 1 つ、イベントは `before` があり
/// 前後の百分率の文字が違うときだけ 1 つ（初めて決まった・DPI だけ違う・元に戻ったは送らない）。
pub(super) fn plan_scale_notices(changes: &[ScaleChange]) -> ScaleNoticePlan {
    let mut folded: BTreeMap<u32, (Option<AppliedScale>, AppliedScale)> = BTreeMap::new();
    for change in changes {
        folded
            .entry(change.target.0)
            .and_modify(|(_, after)| *after = change.after)
            .or_insert((change.before, change.after));
    }
    let mut plan = ScaleNoticePlan::default();
    for (target, (before, after)) in folded {
        let (scope, window) = target_scope(areka_emo_present::TargetId(target));
        plan.writes.push((scope, window, after));
        let Some(before) = before else { continue };
        let (before, after) = (percent_text(before.ratio), percent_text(after.ratio));
        if before == after {
            continue;
        }
        let id = match window {
            TargetWindow::Shell => "OnShellScaling",
            TargetWindow::Balloon => "OnBalloonScaling",
        };
        plan.events.push(ScaleEvent {
            id,
            scope,
            references: vec![
                after.clone(),
                before.clone(),
                after,
                before,
                scope.to_string(),
            ],
        });
    }
    plan
}

#[cfg(test)]
#[path = "scale_notice_tests.rs"]
mod tests;
