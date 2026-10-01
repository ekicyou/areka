//! 差し替えの相（spec: areka-P0-shell-balloon-switch 要件 1.12・1.14・1.16・5.1・5.4・5.5・5.7・
//! 5.8・12.2・12.7・12.9・design「SwitchPhase」）。
//!
//! drain の直後に毎フレーム 1 回、進行中の印 [`SkinSwitchInFlight`] の段を見る。
//!
//! - 待ちの段: 台詞の切れ目の返事と背景の資産づくりの結果を `try_recv` だけで覗く（UI スレッドを
//!   塞がない）。中止・取りやめ・無視・失敗なら記録を 1 件残して印を消す（差し替え・イベント・
//!   記憶は 0）。資産がそろった後に切れ目を受けたら seriko へ差し替えを頼み、頼んだ段へ進む。
//! - 頼んだ段: 置き換えの返信を待って後始末する（完了の段・9.4）。
//!
//! 切替の経路は終了の指示を出さない（要件 5.8）。

use std::sync::PoisonError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::TryRecvError;
use std::time::Instant;

use areka_actor::{ReplyReceiver, reply_channel};
use areka_emo_present::PresentOutcome;
use areka_kanade::{GapLeft, KanadeMsg, MarkedEnd, TalkGap};
use areka_seriko::SerikoReplace;
use bevy_ecs::world::World;
use tracing::{debug, error, info, warn};

use super::Emo2Wiring;
use crate::emo2_boot::assets::actor_keyed_balloon_tables;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinSwitchInFlight, SkinSwitchStage, SwapFinish,
};
use crate::emo2_boot::switch_assets::{SwapBuilt, SwapPayload};
use crate::ghost_session::{GhostSession, GhostSlot};

/// 差し替えの世代（プロセスで単調に進む＝ゴーストをまたいでも置き場の荷物と合図を取り違えない）。
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);

/// 差し替えの相（`emo2_frame_system` の drain の直後に毎フレーム 1 回）。印が無ければ無操作。
pub(super) fn run_switch_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    let Some(mut flight) = world.remove_non_send::<SkinSwitchInFlight>() else {
        return;
    };
    let keep = match flight.stage {
        SkinSwitchStage::Waiting { .. } => step_waiting(&mut flight, wiring, world),
        // 置き換えの返信を待って後始末するのは完了の段（9.4）。
        SkinSwitchStage::Committed { .. } => true,
    };
    if keep {
        world.insert_non_send(flight);
    }
}

/// 待ちの段の 1 フレーム。戻り値は印を残すか。
fn step_waiting(flight: &mut SkinSwitchInFlight, wiring: &mut Emo2Wiring, world: &World) -> bool {
    let kind = flight.kind;
    let to = flight.target.folder.clone();
    let SkinSwitchStage::Waiting {
        gap,
        gap_result,
        build,
        built,
    } = &mut flight.stage
    else {
        return true;
    };

    // 資産を先に見る（同じフレームに切れ目も届いていれば、資産がそろった後に受けた切れ目になる）。
    if built.is_none() {
        match build.try_recv() {
            Ok(Ok(assets)) => {
                *built = Some(assets);
                // 切れ目が資産より先に届いていた: kanade はもう見張っていないので、印なしの待ちを
                // 送り直してその返事で決める（1 度目の返事は印の台詞の終わり方の記録に残す）。
                if gap_result.is_some() {
                    let Some(rx) = await_gap_again(world, kind, &to) else {
                        return false;
                    };
                    *gap = Some(rx);
                }
            }
            Ok(Err(err)) => {
                error!(
                    event = "skin_switch_failed",
                    stage = "build",
                    reason = err.to_string().as_str(),
                    ?kind,
                    to = %to,
                    "新しい資産を作れなかった——元のまま続ける（差し替え・イベント・記憶なし）"
                );
                return false;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                error!(
                    event = "skin_switch_failed",
                    stage = "build",
                    reason = "worker_gone",
                    ?kind,
                    to = %to,
                    "資産づくりのスレッドが結果を返さずに倒れた——元のまま続ける"
                );
                return false;
            }
        }
    }

    let Some(rx) = gap.as_ref() else {
        return true;
    };
    let reply = match rx.try_recv() {
        Ok(None) => return true,
        Ok(Some(reply)) => reply,
        Err(_) => {
            info!(
                event = "skin_switch_dropped",
                reason = "kanade_stopped",
                ?kind,
                to = %to,
                "台詞の切れ目の返事の前に kanade が止まった——切替を取りやめた"
            );
            return false;
        }
    };
    *gap = None;
    match reply {
        TalkGap::Reached { marked } => {
            // 印の台詞の終わり方は 1 度目の返事のもの（送り直しの返事は印なし）。
            let first = match gap_result.take() {
                Some(TalkGap::Reached { marked: first }) => first,
                _ => marked,
            };
            match built.take() {
                Some(assets) => commit(flight, assets, first, wiring, world),
                None => {
                    *gap_result = Some(TalkGap::Reached { marked: first });
                    true
                }
            }
        }
        TalkGap::CancelledByUser => {
            info!(
                event = "skin_switch_cancelled",
                ?kind,
                to = %to,
                "切り替えの台詞を利用者が中断した——切替を中止した（差し替え・記憶なし）"
            );
            false
        }
        TalkGap::Left { reason } => {
            // 腕は理由ごとに書き切る（理由が増えたらここでコンパイルが止まる）。
            let reason = match reason {
                GapLeft::NotSteady => {
                    warn!(
                        event = "skin_switch_not_steady",
                        ?kind,
                        to = %to,
                        "切替要求が届いたとき kanade が定常になかった——無視する"
                    );
                    return false;
                }
                GapLeft::Closing => "closing",
                GapLeft::GhostChange => "ghost_change",
            };
            info!(
                event = "skin_switch_dropped",
                reason,
                ?kind,
                to = %to,
                "台詞の切れ目を待つ間に終了かゴースト切替が始まった——切替を取りやめた"
            );
            false
        }
        TalkGap::NotSent { outcome } => {
            warn!(
                event = "skin_switch_not_sent",
                ?outcome,
                ?kind,
                to = %to,
                "切り替え前のイベントを送れなかった——切替を取りやめた"
            );
            false
        }
    }
}

/// 印なしの待ちを送り直す。送れなければ `error!(skin_switch_send_failed)` で `None`。
fn await_gap_again(world: &World, kind: SkinKind, to: &str) -> Option<ReplyReceiver<TalkGap>> {
    let (reply, rx) = reply_channel();
    let sent = session(world)
        .and_then(GhostSession::kanade)
        .is_some_and(|k| {
            k.send(KanadeMsg::AwaitTalkGap { raise: None, reply })
                .is_ok()
        });
    if !sent {
        error!(
            event = "skin_switch_send_failed",
            ?kind,
            to,
            "台詞の切れ目の待ちを送り直せなかった（kanade は止まっている）——切替を取りやめた"
        );
        return None;
    }
    debug!(
        event = "skin_switch_gap_recheck",
        ?kind,
        to,
        "切れ目の返事が資産より先に届いた——資産がそろったので待ちを送り直す"
    );
    Some(rx)
}

/// 資産がそろった後に切れ目を受けた: ゴースト切替が進んでいれば取りやめ、無ければ世代を進め、
/// 荷物を置き場へ置き、返信の受け手を控え、seriko へ差し替えを頼んで頼んだ段へ進む。
fn commit(
    flight: &mut SkinSwitchInFlight,
    built: SwapBuilt,
    marked: Option<MarkedEnd>,
    wiring: &mut Emo2Wiring,
    world: &World,
) -> bool {
    let kind = flight.kind;
    let to = flight.target.folder.as_str();
    if world.get_non_send::<SwitchInFlight>().is_some() {
        info!(
            event = "skin_switch_dropped",
            reason = "ghost_switch",
            ?kind,
            to,
            "差し替えの前にゴースト切替が進んでいた——切替を取りやめた（ゴースト切替が勝つ）"
        );
        return false;
    }
    let epoch = NEXT_EPOCH.fetch_add(1, Ordering::Relaxed);
    let (targets, replace, finish) = split(built, epoch);
    let mut replies = Vec::new();
    let mut receivers = Vec::new();
    for &(scope, ..) in &targets {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        replies.push((scope, tx));
        receivers.push((scope, rx));
    }
    // 鍵を持つのは置く一瞬だけ。毒は前の持ち主の panic で、置き場そのものは壊れていない。
    *wiring
        .swap_slot
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(SwapPayload {
        epoch,
        targets,
        replies,
    });
    let sent = session(world)
        .and_then(GhostSession::seriko_sink)
        .is_some_and(|sink| sink.send_replace(replace));
    if !sent {
        error!(
            event = "skin_switch_failed",
            stage = "seriko",
            epoch,
            ?kind,
            to,
            "seriko へ差し替えを頼めなかった——元のまま続ける"
        );
        wiring
            .swap_slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        return false;
    }
    flight.stage = SkinSwitchStage::Committed {
        epoch,
        replies: receivers,
        finish,
        marked,
        committed_at: Instant::now(),
    };
    true
}

/// 荷物の中身（scope ごとの `EmoWorld`・アトラス・作者の DPI）。
type Targets = Vec<(
    u32,
    areka_emo_compose::EmoWorld,
    areka_emo_atlas::AtlasTable,
    u16,
)>;

/// 資産を荷物の中身・seriko の定義・後始末の残りへ分ける。
fn split(built: SwapBuilt, epoch: u64) -> (Targets, SerikoReplace, SwapFinish) {
    match built {
        SwapBuilt::Shell {
            assets,
            source,
            balloon,
            restored,
        } => (
            assets
                .shells
                .into_iter()
                .map(|s| (s.scope, s.emo_world, s.atlas, assets.author_dpi))
                .collect(),
            SerikoReplace::Shell {
                epoch,
                resolver: assets.resolver,
                static_binds: assets.static_binds,
                bind_resolver: assets.bind_resolver,
                shell_table: assets.loop_table,
            },
            SwapFinish::Shell {
                source,
                balloon,
                restored,
            },
        ),
        SwapBuilt::Balloon { assets } => {
            let mut targets = Vec::new();
            let mut scopes = Vec::new();
            for b in assets.balloons {
                targets.push((b.scope, b.emo_world, b.atlas, assets.author_dpi));
                scopes.push((b.scope, b.model, b.background_color));
            }
            (
                targets,
                SerikoReplace::Balloon {
                    epoch,
                    balloon_tables: actor_keyed_balloon_tables(assets.loop_tables),
                },
                SwapFinish::Balloon { scopes },
            )
        }
    }
}

/// 置き場のゴースト（無ければ `None`）。
fn session(world: &World) -> Option<&GhostSession> {
    world.get_non_send::<GhostSlot>()?.0.as_ref()
}

#[cfg(test)]
#[path = "switch_tests.rs"]
mod tests;
