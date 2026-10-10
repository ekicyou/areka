//! looper の子: 一番上の面の引き金の配線（spec: areka-P0-seriko-trigger-intervals 要件 2〜7）。
//!
//! [`LoopRuntime`](super::LoopRuntime) の刻みと面の切り替えが呼ぶ自由関数を置く: 引き金の状態を構えて
//! 窓の開け閉めを写す（[`arm_slot`]）・一番上の `runonce`／`periodic`／`talk` を判定して再生を始める
//! （[`fire_top_triggers`]）・文字の列と文字の数えの世話（[`revealed_at`]・[`glyph_wall_ms`]・
//! [`restart_talk`]・[`settle_talk`]）。「いつ始めるか」の判定は `trigger.rs`、文字が現れる時刻の式は
//! `talk.rs`、始まった再生のコマの進み方は親（抽選の再生と同じ経路）が持ち、ここは配線だけ。
//!
//! # 文字の数えと文字の列の間の決まり（要件 4.3・4.4）
//!
//! 文字の列（スコープごと）の序数は消去で振り直される（現れなかった文字を末尾から捨て、次の塊が
//! その序数を使う）。そこで、生きている数え（見えている面ごと）はいつも
//! 「列の頭の序数 ≤ 数え済みの序数 ≤ 列の文字の総数」を保つ:
//!
//! - 構える・現すときは、その時刻までに現れた文字の数から数え始める（[`revealed_at`]）。
//! - 刻みの最後に数えを今現れている数まで進め、列はそこまでしか刈り込まない（[`settle_talk`]）。
//! - 消去の知らせでは、列を切り詰めた直後に数えを列の総数に揃える（[`restart_talk`]・数えた文字の数は
//!   保つ）。
//!
//! だから文字の窓が尋ねる序数（前の判定までの数え済み以後・今現れている数より前）は必ず列に在り、
//! 消去の後に届く文字は必ず「まだ数えていない文字」として数えられる（新しい台詞の頭で口が止まらない）。

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use areka_emo_compose::PatternState;
use areka_sakura::ActorKey;

use super::{Playback, SlotPlayback, put_top_play};
use crate::state::Slot;
use crate::table::{LoopAnimation, LoopTrigger};
use crate::talk::{TalkEpoch, TalkFeed};
use crate::timeline::{FrameStatus, frame_at};
use crate::trigger::{Armed, TalkWindow};

/// `key` の面の引き金の状態を返す。無ければ `at_ms` を見え始めの時刻として構え（面に入った）、在れば
/// 窓の開け閉めだけを写す: 閉じたら隠し、開いたら `at_ms` を新しい起点にして現す（`runonce` の印は
/// 残るので開き直しでは鳴らない・spec: areka-P0-seriko-trigger-intervals 要件 2.1・3.2・5.5）。
///
/// `revealed` は `at_ms` までにそのスコープで現れた文字の数（[`revealed_at`]）で、`talk` はその次の
/// 文字から数える（面に入るたび・窓が開くたびに数え直す・要件 4.4）。文字を数えない面（表に `talk` が
/// 無い・窓が閉じている）では `None` を渡す。
///
/// 呼ぶのは構える条件（一番上に 3 語が在る、または表に `talk` が在る）を満たす面でだけ（要件 8.1）。
pub(super) fn arm_slot<'a>(
    armed: &'a mut HashMap<(ActorKey, Slot), Armed>,
    key: &(ActorKey, Slot),
    surface_id: u32,
    open: bool,
    at_ms: u64,
    revealed: Option<u64>,
) -> &'a mut Armed {
    match armed.entry(key.clone()) {
        Entry::Occupied(entry) => {
            let state = entry.into_mut();
            match (open, state.is_visible()) {
                (true, false) => state.show(at_ms, revealed),
                (false, true) => state.hide(),
                _ => {}
            }
            state
        }
        Entry::Vacant(entry) => {
            tracing::debug!(
                scope = key.0.as_str(),
                slot = ?key.1,
                surface_id,
                at_ms,
                "seriko: trigger 面に入った（引き金を構えた）"
            );
            entry.insert(Armed::arm(Some(at_ms), open, revealed))
        }
    }
}

/// 一番上の 3 語の animation `anims` を番号の昇順に判定し、始まるものを `playback` に入れる（開始の
/// 時刻は判定が返した時刻＝`runonce` は見え始め・`periodic` は周の境目・`talk` は区切りの文字が
/// 現れた時刻で、`now_ms` ではない・spec: areka-P0-seriko-trigger-intervals 要件 2.1・3.1・3.5・4.5・
/// 6.1）。始まった（animation, 開始の時刻）を返す。`talk` は文字の窓 `talk` が在るときだけ判定され、
/// 文字の数えはここでは進めない（進めるのは刻みの最後の [`settle_talk`]）。
///
/// 「再生中か」は刻みの時刻でなく開始の時刻で測る: `playback` に在って、その時刻のコマがまだ終わり
/// （負の番号で停止・末尾）でないこと（要件 3.3・3.5・5.3）。開始の時刻には終えていて、まだ片付けて
/// いない再生は、その時刻まで進めて片付けてから（終わりの記録は刻みの進行と同じ 1 件）入れ替える。
///
/// 乱数は引かない。始めない経路（隠れている・再生中・境目の前）は記録しない（要件 5.7・7.5）。
/// 始まった後の進み方は抽選の再生と同じ（`playback` に入るだけ・要件 5.1〜5.3）。`talk` の開始は
/// 文字の到着ごとに起きうるので `debug!` に留める（要件 7.3）。
#[allow(clippy::too_many_arguments)]
pub(super) fn fire_top_triggers<'a>(
    state: &mut Armed,
    playback: &mut HashMap<(ActorKey, Slot), SlotPlayback>,
    pattern: &mut PatternState,
    warned_negative: &mut HashSet<(ActorKey, Slot, u32)>,
    key: &(ActorKey, Slot),
    anims: impl Iterator<Item = &'a LoopAnimation>,
    now_ms: u64,
    talk: Option<&TalkWindow<'_>>,
) -> Vec<(&'a LoopAnimation, u64)> {
    let mut anims: Vec<&LoopAnimation> = anims.collect();
    anims.sort_by_key(|a| a.id);
    let mut fired = Vec::new();
    for anim in anims {
        // まだ片付けていない前の再生の、時刻 `at` での経過（前の再生が無ければ `None`）。
        let prev = playback.get(key).and_then(|pb| pb.get(&anim.id)).copied();
        let since_prev = |at: u64| prev.map(|p| at.saturating_sub(p.started_at_ms));
        let playing_at = |at| {
            let status = since_prev(at).map(|elapsed| frame_at(&anim.frames, elapsed));
            matches!(status, Some(FrameStatus::Pending | FrameStatus::Active(_)))
        };
        let Some(started_at_ms) = state.poll(anim, now_ms, playing_at, talk) else {
            continue;
        };
        if let Some(elapsed) = since_prev(started_at_ms) {
            put_top_play(pattern, playback, warned_negative, key, anim, elapsed);
        }
        playback
            .entry(key.clone())
            .or_default()
            .insert(anim.id, Playback { started_at_ms });
        match anim.trigger {
            LoopTrigger::Runonce => tracing::info!(
                scope = key.0.as_str(),
                slot = ?key.1,
                animation_id = anim.id,
                started_at_ms,
                "seriko: trigger runonce を鳴らした（再生開始・先頭コマから・要件 2.1）"
            ),
            LoopTrigger::Periodic { .. } => tracing::info!(
                scope = key.0.as_str(),
                slot = ?key.1,
                animation_id = anim.id,
                started_at_ms,
                "seriko: trigger periodic を鳴らした（再生開始・先頭コマから・要件 3.1）"
            ),
            LoopTrigger::Talk { .. } => tracing::debug!(
                scope = key.0.as_str(),
                slot = ?key.1,
                animation_id = anim.id,
                started_at_ms,
                "seriko: trigger talk を鳴らした（再生開始・先頭コマから・要件 4.1）"
            ),
            _ => {}
        }
        fired.push((anim, started_at_ms));
    }
    fired
}

/// スコープ `scope` で時刻 `at_ms` までに現れた文字の数（序数で数える）。文字がまだ 1 つも届いて
/// いないスコープは 0（届いた最初の文字が序数 0 になる）。
pub(super) fn revealed_at(
    epoch: &TalkEpoch,
    feeds: &HashMap<ActorKey, TalkFeed>,
    scope: &ActorKey,
    at_ms: u64,
) -> u64 {
    // 列が在れば起点の見積もりも在る（文字を積む前に必ず起点を見積もる）。
    feeds
        .get(scope)
        .zip(epoch.talk_time(at_ms))
        .map_or(0, |(feed, t_s)| feed.revealed_until(t_s))
}

/// 序数 `glyph` の文字が現れた壁時刻（ms）。文字の窓の「序数 → 壁時刻」の写しの中身。
///
/// 窓が尋ねる序数は必ず列に在る（モジュールの doc の決まり）。万一無ければ決まりの破れなので、
/// `warn!` を残して刻みの時刻 `now_ms` で代える（落とさない・黙って時刻を作らない）。
pub(super) fn glyph_wall_ms(
    epoch: &TalkEpoch,
    feed: Option<&TalkFeed>,
    glyph: u64,
    now_ms: u64,
) -> u64 {
    let at_ms = feed
        .and_then(|feed| feed.time_of(glyph))
        .and_then(|r_s| epoch.wall_ms(r_s));
    debug_assert!(at_ms.is_some(), "区切りの文字 {glyph} が文字の列に無い");
    at_ms.unwrap_or_else(|| {
        tracing::warn!(
            glyph,
            now_ms,
            "seriko: trigger talk の区切りの文字が文字の列に無い（刻みの時刻で代える）"
        );
        now_ms
    })
}

/// 消去の知らせ（台本の秒 `at_s`）を `scope` の文字の列へ写す: 現れなかった文字を捨てて継ぎ目を
/// 初期化し、そのスコープの数えを列の文字の総数に揃える（捨てた序数は次の塊の文字に振り直される）。
pub(super) fn restart_talk(
    feed: &mut TalkFeed,
    armed: &mut HashMap<(ActorKey, Slot), Armed>,
    scope: &ActorKey,
    at_s: f64,
) {
    feed.restart_chain(at_s);
    // 残ったのは消去の時刻までに現れた文字だけなので、これが列の文字の総数。
    let total = feed.revealed_until(at_s);
    for ((_, _), state) in armed.iter_mut().filter(|((s, _), _)| s == scope) {
        state.realign_talk(total);
    }
}

/// 刻みの最後に呼ぶ: 一番上も部品も文字の窓を読み終えたので、各スコープの数えを今現れている数まで
/// 進め、そこまでの文字を列から刈り込む（序数は変わらない）。
///
/// 数えを進めるのを判定より前に置くと、後から読む側（部品）が同じ区切りを見落とす。隠れた面
/// （閉じたバルーンの窓）は数えを持たないので、刈り込みを止めない。数えが今現れている数より先に
/// 在る間（起点が前へ飛んだ直後）は、その分の文字を列に残す（刈り込みはどの数え済みも越えない）。
pub(super) fn settle_talk(
    epoch: &TalkEpoch,
    feeds: &mut HashMap<ActorKey, TalkFeed>,
    armed: &mut HashMap<(ActorKey, Slot), Armed>,
    now_ms: u64,
) {
    let Some(t_s) = epoch.talk_time(now_ms) else {
        return;
    };
    for (scope, feed) in feeds.iter_mut() {
        let now_seen = feed.revealed_until(t_s);
        for ((_, _), state) in armed.iter_mut().filter(|((s, _), _)| s == scope) {
            state.advance_talk(now_seen);
        }
        feed.prune_before(now_seen);
    }
}
