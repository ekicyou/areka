//! parts の子: 部品の引き金の配線（spec: areka-P0-seriko-trigger-intervals 要件 5.4・5.6・5.7・5.9）。
//!
//! [`PartClocks`](super::PartClocks) の評価の 2 段目が、見えると決まった部品 1 つごとに呼ぶ
//! [`fire_part_triggers`] を置く。「いつ始めるか」の判定は `trigger.rs`、始まった時計のコマの進み方は
//! 親（抽選の時計と同じ経路）が持ち、ここは引き金の状態を構える・判定する・時計を作る配線だけ。

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use areka_emo_compose::PartKey;
use areka_sakura::ActorKey;

use super::{ClockKey, Label, Look, PartAnim, is_trigger, look, sort_by_id};
use crate::state::Slot;
use crate::table::{LoopAnimation, LoopTrigger};
use crate::trigger::Armed;

/// 2 段目（見えると決まった部品 1 つ）: `runonce`・`periodic`・`talk` を持つ部品なら引き金の状態を
/// 持たせ（無ければ `at_ms` を見え始めの時刻として構える・在れば窓の開け閉めを写す）、3 語の
/// animation を番号の昇順に判定して、始まるものの時計を判定が返した開始の時刻（`runonce` は見え始め・
/// `periodic` は周の境目で、`at_ms` ではない）で作る。始まった animation を返す（コマを書くのは
/// 呼び手）。3 語を持たない部品には何もしない＝状態も記録も生まれない（spec:
/// areka-P0-seriko-trigger-intervals 要件 5.4・5.6・5.9・6.1・8.1）。
///
/// 「再生中か」は開始の時刻で測る（要件 3.3・5.3）。`was_playing` は、この評価の 1 段目が進める前に
/// 再生中だった 3 語の時計の（鍵, 開始の時刻）で、開始の時刻のコマがまだ終わり（負の番号で停止・
/// 末尾）でなければ再生中。1 段目がこの刻みで片付けた再生も、境目の時刻に再生中だったならその周を
/// 飛ばし、境目の時刻に終えていたなら鳴らす（長さが周期ちょうどの animation も毎周鳴る）。
///
/// `runonce_only` は出来事の直後の評価（`periodic`・`talk` は刻みが鳴らす）。乱数は引かない。始めない
/// 経路（隠れている・再生中・境目の前）は記録しない（要件 5.7・7.5）。部品は文字の数えを持たず、
/// `talk` は面の文字の窓を借りる（渡すのはタスク 5.2。窓が無い間は鳴らない）。
#[allow(clippy::too_many_arguments)]
pub(super) fn fire_part_triggers<'a>(
    clocks: &mut BTreeMap<ClockKey, PartAnim>,
    armed: &mut BTreeMap<PartKey, Armed>,
    order: &mut Vec<usize>,
    anims: &'a [LoopAnimation],
    scope: &ActorKey,
    slot: Slot,
    part: PartKey,
    at_ms: u64,
    open: bool,
    was_playing: &[(ClockKey, u64)],
    runonce_only: bool,
) -> Vec<&'a LoopAnimation> {
    let mut started = Vec::new();
    if !anims.iter().any(is_trigger) {
        return started;
    }
    let state = match armed.entry(part) {
        Entry::Occupied(entry) => {
            let state = entry.into_mut();
            match (open, state.is_visible()) {
                (true, false) => state.show(at_ms, None),
                (false, true) => state.hide(),
                _ => {}
            }
            state
        }
        Entry::Vacant(entry) => {
            tracing::debug!(
                scope = scope.as_str(),
                slot = ?slot,
                part = %Label(part),
                at_ms,
                "seriko: part trigger 部品が見えた（引き金を構えた）"
            );
            entry.insert(Armed::arm(Some(at_ms), open, None))
        }
    };
    sort_by_id(anims, order);
    for &i in order.iter() {
        let anim = &anims[i];
        if !is_trigger(anim) || (runonce_only && anim.trigger != LoopTrigger::Runonce) {
            continue;
        }
        let key = (part, anim.id);
        let before = was_playing
            .iter()
            .find(|(k, _)| *k == key)
            .map(|&(_, started_at_ms)| PartAnim::Playing { started_at_ms });
        let playing_at = |at| {
            before.is_some() && matches!(look(anim, before, at), Look::Nothing | Look::Frame(_))
        };
        let Some(started_at_ms) = state.poll(anim, at_ms, playing_at, None) else {
            continue;
        };
        clocks.insert(key, PartAnim::Playing { started_at_ms });
        match anim.trigger {
            LoopTrigger::Runonce => tracing::info!(
                scope = scope.as_str(),
                slot = ?slot,
                part = %Label(part),
                animation_id = anim.id,
                started_at_ms,
                "seriko: part trigger runonce を鳴らした（再生開始・先頭コマから・要件 2.1）"
            ),
            LoopTrigger::Periodic { .. } => tracing::info!(
                scope = scope.as_str(),
                slot = ?slot,
                part = %Label(part),
                animation_id = anim.id,
                started_at_ms,
                "seriko: part trigger periodic を鳴らした（再生開始・先頭コマから・要件 3.1）"
            ),
            LoopTrigger::Talk { .. } => tracing::debug!(
                scope = scope.as_str(),
                slot = ?slot,
                part = %Label(part),
                animation_id = anim.id,
                started_at_ms,
                "seriko: part trigger talk を鳴らした（再生開始・先頭コマから・要件 4.1）"
            ),
            _ => {}
        }
        started.push(anim);
    }
    started
}
