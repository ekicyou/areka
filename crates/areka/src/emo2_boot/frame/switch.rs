//! 差し替えの相（spec: areka-P0-shell-balloon-switch 要件 1.12・1.14・1.16・5.1・5.4・5.5・5.7・
//! 5.8・12.2・12.7・12.9・design「SwitchPhase」）。
//!
//! drain の直後に毎フレーム 1 回、進行中の印 [`SkinSwitchInFlight`] の段を見る。
//!
//! - 待ちの段: 台詞の切れ目の返事と背景の資産づくりの結果を `try_recv` だけで覗く（UI スレッドを
//!   塞がない）。中止・取りやめ・無視・失敗なら記録を 1 件残して印を消す（差し替え・イベント・
//!   記憶は 0）。資産がそろった後に切れ目を受けたら seriko へ差し替えを頼み、頼んだ段へ進む。
//! - 頼んだ段（完了の段）: 置き換えの返信を `try_recv` だけで覗き、どれかが失敗・脱落なら
//!   `error!` を 1 件残して印を消す（記憶・通知 0）。全部そろえば後始末（シェル: 配置の値の
//!   入れ直し → 重なりの基底 → 実行系の今のシェル → `LastShell` → `OnShellChanged`／バルーン:
//!   文字の模型と背景色 → 可視の記憶 → 今のバルーン → `LastBalloon` → `OnBalloonChange`）を
//!   して `info!(skin_switch_done)` を残し、印を消す（要件 2.4・2.7・3.2〜3.4・6.1・6.5・6.7）。
//!
//! 切替の経路は終了の指示を出さない（要件 5.8）。

use std::sync::PoisonError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::TryRecvError;
use std::time::Instant;

use areka_actor::{ReplyReceiver, reply_channel};
use areka_emo_present::PresentOutcome;
use areka_kanade::{GapLeft, KanadeMsg, MarkedEnd, ShioriMethod, TalkGap};
use areka_parsers::balloon::BalloonModel;
use areka_sakura::ActorKey;
use areka_seriko::SerikoReplace;
use areka_sylphya::{PersistKey, SylphyaPublisher};
use bevy_ecs::world::World;
use tracing::{debug, error, info, warn};

use super::{Emo2Wiring, reconcile_reported_sizes};
use crate::boot_config::BootContext;
use crate::boot_resolve::{BalloonDecision, BalloonRoute, record_last_balloon, record_last_shell};
use crate::emo2_boot::assets::actor_keyed_balloon_tables;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::shell_balloon_resolve::SkinCandidate;
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinSwitchInFlight, SkinSwitchStage, SwapFinish, skin_ref_name, skin_ref_path,
};
use crate::emo2_boot::shell_box_assets::ShellBoxAssets;
use crate::emo2_boot::switch_assets::{SwapBuilt, SwapPayload};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::placement::config::build_placement_config;
use crate::placement::reseed::{
    BalloonPlacementInputs, apply_shell_descript, reanchor_char_windows,
};
use crate::placement::source::DescriptSource;
use crate::placement::spawn::GhostWindows;

/// 差し替えの世代（プロセスで単調に進む＝ゴーストをまたいでも置き場の荷物と合図を取り違えない）。
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);

/// 差し替えの相（`emo2_frame_system` の drain の直後に毎フレーム 1 回）。印が無ければ無操作。
pub(super) fn run_switch_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    let Some(mut flight) = world.remove_non_send::<SkinSwitchInFlight>() else {
        return;
    };
    let keep = match flight.stage {
        SkinSwitchStage::Waiting { .. } => {
            step_waiting(&mut flight, wiring, world).then_some(flight)
        }
        SkinSwitchStage::Committed { .. } => step_committed(flight, wiring, world),
    };
    if let Some(flight) = keep {
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
/// 荷物を置き場へ置き、返信の受け手を控え、seriko へ差し替えを頼んで頼んだ段へ進む。置き場を
/// 引き上げるのはこの関数の間だけ（戻った後は表示の橋渡しだけが荷物を生かす＝seriko が倒れれば
/// 返信の送り手も消え、頼んだ段が脱落を見て印を消す）。
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
    // 置き場を強く持つのは表示の橋渡しだけ。引き上げられなければ橋渡し（seriko）はもう居ない。
    let Some(swap_slot) = wiring.swap_slot.upgrade() else {
        error!(
            event = "skin_switch_failed",
            stage = "seriko",
            reason = "bridge_gone",
            epoch,
            ?kind,
            to,
            "表示の橋渡しが消えていて差し替えの荷物を置けない（seriko は止まっている）——元のまま続ける"
        );
        return false;
    };
    let (targets, replace, finish) = split(built, epoch);
    let mut replies = Vec::new();
    let mut receivers = Vec::new();
    for &(scope, ..) in &targets {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        replies.push((scope, tx));
        receivers.push((scope, rx));
    }
    // 鍵を持つのは置く一瞬だけ。毒は前の持ち主の panic で、置き場そのものは壊れていない。
    *swap_slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(SwapPayload {
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
        swap_slot
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

/// 頼んだ段の 1 フレーム。印を残すなら `Some`（返信待ち）、消すなら `None`（完了・失敗）。
fn step_committed(
    flight: SkinSwitchInFlight,
    wiring: &mut Emo2Wiring,
    world: &mut World,
) -> Option<SkinSwitchInFlight> {
    let SkinSwitchInFlight {
        kind,
        target,
        stage,
    } = flight;
    let SkinSwitchStage::Committed {
        epoch,
        mut replies,
        finish,
        marked,
        committed_at,
    } = stage
    else {
        // 待ちの段は `step_waiting` の持ち場（呼び手が振り分け済み）。
        return Some(SkinSwitchInFlight {
            kind,
            target,
            stage,
        });
    };
    match poll_replies(&mut replies) {
        Ok(false) => {
            return Some(SkinSwitchInFlight {
                kind,
                target,
                stage: SkinSwitchStage::Committed {
                    epoch,
                    replies,
                    finish,
                    marked,
                    committed_at,
                },
            });
        }
        Ok(true) => {}
        Err((scope, reason)) => {
            error!(
                event = "skin_switch_failed",
                stage = "attach",
                epoch,
                scope,
                reason = reason.as_str(),
                ?kind,
                to = %target.folder,
                "新しい装着への置き換えが済まなかった——記憶も通知も書かない"
            );
            return None;
        }
    }

    // UI スレッドを止めた時間 `swap_ms`＝このフレームの drain（置き換えの適用はこの中で返信を
    // そろえた）＋後始末。drain は他の指令も捌くので上限の見積もり（要件 4.4・実機サインオフで記録）。
    let started = Instant::now();
    match finish {
        SwapFinish::Shell {
            source,
            balloon,
            restored,
            boxes,
        } => finish_shell(wiring, world, &target, &source, &balloon, &restored, boxes),
        SwapFinish::Balloon { scopes } => finish_balloon(wiring, world, &target, scopes),
    }
    info!(
        event = "skin_switch_done",
        ?kind,
        to = %target.folder,
        epoch,
        ?marked,
        swap_ms = (wiring.last_drain + started.elapsed()).as_secs_f64() * 1000.0,
        since_commit_ms = committed_at.elapsed().as_secs_f64() * 1000.0,
        "シェル・バルーンの切替を終えた"
    );
    None
}

/// 置き換えの返信を覗く（`try_recv` だけ）。届いた成功は外し、全部そろえば `Ok(true)`、まだ残れば
/// `Ok(false)`。失敗の返信か送り手の脱落があれば、最初のものの (scope, 理由) を `Err` で返す。
fn poll_replies(
    replies: &mut Vec<(u32, ReplyReceiver<PresentOutcome>)>,
) -> Result<bool, (u32, String)> {
    let mut failed = None;
    replies.retain(|(scope, rx)| match rx.try_recv() {
        Ok(None) => true,
        Ok(Some(Ok(()))) => false,
        Ok(Some(Err(err))) => {
            failed.get_or_insert((*scope, err.to_string()));
            false
        }
        Err(err) => {
            failed.get_or_insert((*scope, err.to_string()));
            false
        }
    });
    match failed {
        Some(failure) => Err(failure),
        None => Ok(replies.is_empty()),
    }
}

/// シェルの後始末（design「SwitchPhase」の完了の後始末・この順）: 箱の束を文字の層へ渡す
/// （areka-P0-shell-balloon）→ 配置の値を入れ直す → 重なりの
/// 基底を置き直す → 実行系の今のシェルを書き換える → `LastShell` だけを書く → `OnShellChanged`。
fn finish_shell(
    wiring: &mut Emo2Wiring,
    world: &mut World,
    target: &SkinCandidate,
    source: &DescriptSource,
    balloon: &BalloonPlacementInputs,
    restored: &[(PersistKey, String)],
    boxes: ShellBoxAssets,
) {
    // 新しいシェルの箱の束を文字の層へ渡す（areka-P0-shell-balloon 要件 6.8）: 前の箱の面と
    // 箱の文字を捨て、今のサーフェス番号を保ったまま行き先を新しい表で引き直す（`\s` を書かない
    // 次の台詞が新しいシェルの同じ番号のサーフェスの箱へ入る）。
    boxes.hand_to(&mut wiring.runtime.borrow_mut(), world);
    // 配置の解決は窓寸を入力に取るので、drain が置き換えで積んだ窓寸の報告をここで先に窓へ
    // 反映し、新しいシェルの寸法で解かせる（同じフレームの後段の照合は取り出し済みで何もしない）。
    reconcile_reported_sizes(&mut wiring.presenter, world);
    match world.get_resource::<GhostWindows>().cloned() {
        Some(windows) => {
            apply_shell_descript(world, &windows, source, balloon, restored);
            // 揃え方が変わっても窓寸が同じだと後段の再スナップは置き直さないので、ここで新しい
            // 揃え方へ置き直す（要件 2.7）。
            reanchor_char_windows(world, &windows);
        }
        None => warn!(
            event = "reseed_skipped",
            reason = "no_ghost_windows",
            to = %target.folder,
            "reseed: 窓の正本が無いので見た目の値を入れ直さない（古い値のまま）"
        ),
    }
    wiring.reseed_zorder_descript_base(
        build_placement_config(&source.ghost_kv, &source.shell_kv)
            .zorder_raw
            .as_deref(),
    );
    let replaced = world
        .get_non_send_mut::<GhostSlot>()
        .is_some_and(|mut slot| {
            slot.0
                .as_mut()
                .is_some_and(|s| s.set_shell_dir(target.dir.clone()))
        });
    if !replaced {
        warn!(
            event = "skin_shell_dir_not_set",
            to = %target.folder,
            "実行系が無いので今のシェルを書き換えられない（更新の対象とメニューの印は古いシェルのまま）"
        );
    }
    record_memory(world, SkinKind::Shell, |p| {
        record_last_shell(p, &target.folder)
    });
    let ghost = ghost_ref_name(world);
    raise_changed(
        world,
        "OnShellChanged",
        vec![skin_ref_name(target), ghost, skin_ref_path(&target.dir)],
    );
}

/// バルーンの後始末（design「SwitchPhase」の完了の後始末・この順）: scope ごとの文字の模型と
/// 背景色 → 可視の記憶 → 今のバルーン → `LastBalloon` だけを書く（argv で起きたプロセスでも）→
/// `OnBalloonChange`。
fn finish_balloon(
    wiring: &mut Emo2Wiring,
    world: &mut World,
    target: &SkinCandidate,
    scopes: Vec<(u32, BalloonModel, (u8, u8, u8))>,
) {
    for (scope, model, background) in scopes {
        // 文字の層は同じフレームの再追従が新しいスロットへ結び直す（そのとき背景色も焼き直り、
        // 警告の名前の欄も新しいバルーンのフォルダ名になる・areka-P0-shell-balloon 要件 3.12）。
        let actor = ActorKey::from(scope.to_string());
        let mut runtime = wiring.runtime.borrow_mut();
        runtime.set_balloon_label(&actor, target.folder.clone());
        runtime.set_balloon_background(actor, background);
        wiring.balloon_models.insert(scope, model);
        wiring.balloon_visibility.forget_scope(scope);
    }
    match world.get_resource_mut::<BootContext>() {
        // 記憶に書くので、これが次の起動の選び方になる（`BalloonRoute` に腕を足さない）。
        Some(mut ctx) => {
            ctx.current.balloon = BalloonDecision {
                route: BalloonRoute::Memory,
                dir: target.dir.clone(),
                folder: Some(target.folder.clone()),
            };
        }
        None => warn!(
            event = "skin_current_balloon_not_set",
            to = %target.folder,
            "起動の文脈が無いので今のバルーンを書き換えられない（更新の対象は古いバルーンのまま）"
        ),
    }
    record_memory(world, SkinKind::Balloon, |p| {
        record_last_balloon(p, &target.folder)
    });
    raise_changed(
        world,
        "OnBalloonChange",
        vec![skin_ref_name(target), skin_ref_path(&target.dir)],
    );
}

/// 実行系の記憶の書き手で書く。書き手が無ければ `warn!(skin_memory_not_recorded)` で、切替は
/// 成功として扱う（要件 6.1・記憶の縮退は今日と同じ）。
fn record_memory(world: &World, kind: SkinKind, write: impl FnOnce(&SylphyaPublisher)) {
    match session(world).and_then(GhostSession::memory_publisher) {
        Some(publisher) => write(publisher),
        None => warn!(
            event = "skin_memory_not_recorded",
            ?kind,
            "記憶の書き手が無いので切替の選択を記憶へ書けない（切替は成功として扱う）"
        ),
    }
}

/// 切り替えた後のイベントを汎用の入口で GET・返信なしで送る。送れなければ `warn!`（差し替えは
/// 済んでいるので切替は成功のまま）。
fn raise_changed(world: &World, id: &'static str, references: Vec<String>) {
    let sent = session(world)
        .and_then(GhostSession::kanade)
        .is_some_and(|k| {
            k.send(KanadeMsg::RaiseEvent {
                id: id.to_owned(),
                references,
                method: ShioriMethod::Get,
                reply: None,
            })
            .is_ok()
        });
    if !sent {
        warn!(
            event = "skin_switch_event_not_sent",
            id,
            "切り替えた後のイベントを送れなかった（kanade は止まっている）——差し替えは済んでいる"
        );
    }
}

/// Reference のゴーストの名前（`descript.txt` の `name`、無ければフォルダ名・要件 2.4）。
fn ghost_ref_name(world: &World) -> String {
    session(world).map_or_else(String::new, |s| {
        s.ghost_name().unwrap_or_else(|| {
            s.ghost_dir()
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    })
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
                boxes: assets.boxes,
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

#[cfg(test)]
#[path = "switch_finish_tests.rs"]
mod finish_tests;
