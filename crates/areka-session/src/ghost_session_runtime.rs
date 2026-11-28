//! 実行系に結びつく窓側の結線。
//!
//! `boot_wired` の boot 後の塊を置く場所。実行系（[`GhostRuntime`]）の送出端を窓の側の状態へ
//! 差し直す。窓の側だけの結線（`wire_balloon_choice`）は含めない。
//!
//! shiori だけの読み直しの本体 [`rebuild_runtime_in_place`] も置く。

use std::time::Instant;

use areka_ghost::GhostRuntime;
use areka_kanade::{BootOrigin, CloseReason, ReloadScope};
use areka_sylphya::PersistFailure;
use bevy_ecs::world::World;

use super::{GhostBootInputsSource, GhostSlot};
use crate::body_systems::BodySystems;
use crate::boot_config::BootContext;
use crate::boot_resolve::{BalloonDecision, GhostDecision, write_switch_drop};
use crate::runtime_boot;
use areka_runtime::frame::RuntimeWiring;

/// 実行系に結びつく窓側の結線をやり直す。`wire_balloon_choice`（窓の側）は含めない。
pub(crate) fn rewire_runtime_consumers(
    world: &mut World,
    runtime: &GhostRuntime,
    ghost: &GhostDecision,
    balloon: &BalloonDecision,
) {
    // kanade Sender クローンで MouseWiring（NonSend・Presenter）を挿入する。挿入は wire_runtime 成功後＝
    // RuntimeWiring 挿入済みなので、presenter 経由の region 解決が成立する。
    let sender = runtime.kanade().clone();
    areka_input::wire_mouse_input(world, sender);
    // 右クリックメニューの結線・枠の登記 8 本・置換語の載せ直し（本体の `wire_body_after_runtime`）。
    (BodySystems::of(world).after_runtime_wired)(world, runtime);
    // 位置永続の World 結線: ゴースト窓を保持する同一 World へ sylphya publisher clone を持つ PersistWiring
    // （NonSend）を差し、DragEnd→persist_entries の write-through 導管を確立する。続けて起動成功時の記憶を書く。
    crate::boot_seam::on_boot_ok(world, runtime, ghost, balloon);
    // 選択確定通知の受信結線: 呼び手が続けて `wire_balloon_choice` で挿入する `ChoiceSelectionInbox` を毎フレーム
    // drain し、dispatcher の inbox 経由で kanade へ全件転送する系（登録は `register_systems`）の送り口を置く。
    // 送り先が dispatcher なのは、押下の選択を再生の `ChoiceWaiting` と同じ inbox に並べて追い越させないため。
    areka_input::choice_drain::wire_choice_drain(world, runtime.dispatcher().clone());
    // 窓の矩形の読み手を新しい実行系へ登録する。
    super::ghost_session_scope_rect::wire(world, runtime);
    // 倍率と DPI の読み手を毎フレームの配線の表で登録する。
    let table = world
        .get_non_send::<RuntimeWiring>()
        .map(RuntimeWiring::scope_scale_table);
    super::ghost_session_scope_scale::wire(runtime.sylphya_reader(), table);
}

/// shiori だけの読み直しの答え（呼び手 `switch_to` が段・既定への戻し・致命を選ぶ）。
#[derive(Debug)]
pub enum RuntimeRebuild {
    /// 新しい実行系を同じ単位に付け直し、窓側の結線をやり直した。
    Rebuilt,
    /// 起こせなかった（理由は記録済み）。置き場には実行系の無い単位か、手を付けていない単位が在る。
    BootFailed,
    /// 降ろした直後の記憶を書けなかった（実行系の無い単位は置き場に在る）。
    WriteFailed(PersistFailure),
}

/// shiori だけの読み直しの本体: 置き場の単位から実行系だけを外して
/// 降ろし（窓・seriko・loop ticker・束は残る）、降ろした直後の記憶を `switch_to` と同じ引数で書き、
/// 起動時と同じ作り口（[`GhostBootInputsSource`]）で由来 `Reloaded{Shiori}` の実行系を今のシェルで
/// 起こし、付け直して [`rewire_runtime_consumers`] をやり直す。`target_name` は印に書く切替先の名前。
///
/// 外している間も単位は置き場に戻しておく＝失敗して呼び手が既定へ戻す `take_down` は seriko・ticker
/// を正しく降ろす。起動の失敗の記録は `boot_runtime` が 1 度だけ残す（ここでは記録し直さない）。
pub fn rebuild_runtime_in_place(world: &mut World, target_name: &str) -> RuntimeRebuild {
    let no_context = |reason: &str| {
        tracing::error!(
            event = "reload_runtime_no_context",
            reason,
            "実行系を組み直す材料が無い（置き場の単位には手を付けていない）"
        );
        RuntimeRebuild::BootFailed
    };
    let Some(ctx) = world.get_resource::<BootContext>() else {
        return no_context("boot_context");
    };
    let (cfg, ghost, balloon) = (
        ctx.current.cfg.clone(),
        ctx.current.ghost.clone(),
        ctx.current.balloon.clone(),
    );
    let (app_profile_dir, mark) = (
        ctx.app_profile_dir.clone(),
        (!ctx.argv_session).then_some(target_name),
    );
    if !world.contains_non_send::<GhostBootInputsSource>() {
        return no_context("inputs_source");
    }
    let Some(mut slot) = world.get_non_send_mut::<GhostSlot>() else {
        return no_context("slot");
    };
    // 束が無い単位（LogSink へ倒れた腕など）は組み直せない。単位はそのまま置き場に残す。
    let Some((session, sources)) = slot
        .0
        .as_mut()
        .and_then(|s| s.sink_sources().cloned().map(|sources| (s, sources)))
    else {
        drop(slot);
        return no_context("sink_sources");
    };
    let shell = session.current_shell_folder();
    let old = session.split_runtime();
    drop(slot);

    if let Some(old) = old {
        let started = Instant::now();
        let result = old.shutdown(CloseReason::System);
        let ms = started.elapsed().as_millis() as u64;
        if result.is_err() {
            tracing::error!(
                event = "ghost_switch_down_failed",
                ms,
                "実行系を降ろすのに失敗した——戻す先が無いので組み直しを続ける"
            );
        }
        tracing::info!(event = "ghost_switch_down_ms", ms, "実行系を降ろした");
    }
    // `switch_to` の降ろした直後の記憶と同じ意味・同じ引数で書く。
    if let Err(failure) = write_switch_drop(&app_profile_dir, mark) {
        return RuntimeRebuild::WriteFailed(failure);
    }
    let inputs = (world.non_send::<GhostBootInputsSource>().0)(
        &cfg,
        BootOrigin::Reloaded {
            scope: ReloadScope::Shiori,
        },
    );
    let Ok(runtime) = runtime_boot::boot_runtime(
        world,
        inputs.wiring,
        &sources,
        inputs.kanade_stop,
        shell.as_deref(),
    ) else {
        return RuntimeRebuild::BootFailed;
    };
    drop(sources);
    rewire_runtime_consumers(world, &runtime, &ghost, &balloon);
    let mut slot = world.get_non_send_mut::<GhostSlot>();
    if let Some(session) = slot.as_mut().and_then(|slot| slot.0.as_mut()) {
        session.attach_runtime(runtime);
        return RuntimeRebuild::Rebuilt;
    }
    drop(slot);
    tracing::error!(
        event = "reload_runtime_slot_lost",
        "組み直している間に置き場の単位が消えた——起こした実行系を降ろす"
    );
    if runtime.shutdown(CloseReason::System).is_err() {
        tracing::error!(
            event = "ghost_switch_down_failed",
            "起こした実行系を降ろすのに失敗した"
        );
    }
    RuntimeRebuild::BootFailed
}

#[cfg(test)]
#[path = "ghost_session_runtime_tests.rs"]
mod tests;
