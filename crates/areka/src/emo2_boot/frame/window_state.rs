//! 常に手前の状態と、それを動かす取り出しの相（design「WindowStatePhase」・要件 1.1〜1.3／
//! 2.1／2.7〜2.9／3.1／3.2／3.4／6.1／6.2／6.7／7.2）。
//!
//! 台本のスレッドの受け口（`WindowStateCueSink`）が送った指令を、画面を持つ側で到着順に
//! 取り出し、ゴーストごとの状態（既定オフ）へ当てる。状態が変わったときはゴーストの窓の
//! 全部の entity に印 `StayOnTop` を付け外しし、wintf の帯の指令の列 `WindowBandQueue` へ
//! 入る・出るを積む。`\v` は状態を変えずに手前へ出すを積む。
//!
//! # 窓の正本が無い間は取り出さない
//!
//! 重なりの取り出しの相（`zorder_drain.rs`）と同じ作法である。`GhostWindows` が World に
//! 居ない間は `try_iter` を呼ばず、チャネルが保留を兼ねる（起動の台本の指令も落ちない）。
//!
//! # 正本の外の entity には触れない
//!
//! 印と列は `GhostWindows` の entity だけを指す（別のゴースト・メニュー・ツールチップに
//! 触れない・要件 2.7）。

use std::sync::mpsc::Receiver;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::{debug, info};
use wintf::ecs::window::{BandOp, StayOnTop, WindowBandQueue};

use crate::boot_config::BootContext;
use crate::emo2_boot::window_state_cue::WindowStateDirective;
use crate::placement::spawn::GhostWindows;

/// ゴーストごとの常に手前の状態（`Emo2Wiring` の欄・既定＝受信端なし・オフ）。
///
/// `Emo2Wiring` と一緒に作り直される（ゴーストの切り替え・読み直し・終了）ので、新しい
/// ゴーストはオフから始まる（要件 3.4）。
#[derive(Default)]
pub(in crate::emo2_boot) struct StayOnTopState {
    /// 受け口と対の受信端（起動の結線が後から渡す・無ければ相は何もしない）。
    pub(super) rx: Option<Receiver<WindowStateDirective>>,
    /// 常に手前の状態（正本）。
    pub(super) on: bool,
}

/// 遷移の判断の結果。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Transition {
    /// オフ→オン
    TurnOn,
    /// オン→オフ
    TurnOff,
    /// 状態と同じ指令（窓を動かさない・要件 2.8・3.2）
    Unchanged { on: bool },
    /// `\v`（状態は変えない・要件 2.9）
    Raise { on: bool },
}

/// 純関数：今の状態と指令から遷移を決める。
pub(crate) fn decide_transition(on: bool, directive: WindowStateDirective) -> Transition {
    match directive {
        WindowStateDirective::Raise => Transition::Raise { on },
        WindowStateDirective::StayOnTop { on: want } if want == on => Transition::Unchanged { on },
        WindowStateDirective::StayOnTop { on: true } => Transition::TurnOn,
        WindowStateDirective::StayOnTop { on: false } => Transition::TurnOff,
    }
}

/// Update の相：受信端を到着順に取り出して状態へ当てる。
///
/// 呼び手は `frame.rs` の重なりの取り出しの相の直後（5.1 で結ぶ）。
// 5.1 で外す（フレームの相から呼ぶまで本番では使わない）。
#[cfg_attr(not(test), allow(dead_code))]
pub(in crate::emo2_boot) fn run_window_state_phase(state: &mut StayOnTopState, world: &mut World) {
    let Some(rx) = state.rx.as_ref() else {
        return;
    };
    // 窓の正本が無い間はチャネルが保留を兼ねる（取り出さない）。
    let Some(windows) = world.get_resource::<GhostWindows>() else {
        return;
    };
    let directives: Vec<WindowStateDirective> = rx.try_iter().collect();
    if directives.is_empty() {
        return;
    }
    // 全スコープのキャラクターとバルーン（World にまだ居るものだけ）。
    let registered: Vec<Entity> = windows
        .scopes()
        .flat_map(|scope| [windows.char_window(scope), windows.balloon_window(scope)])
        .flatten()
        .collect();
    let targets: Vec<Entity> = registered
        .iter()
        .copied()
        .filter(|entity| world.get_entity(*entity).is_ok())
        .collect();
    let ghost = ghost_name(world);
    let dropped = registered.len() - targets.len();
    if dropped > 0 {
        debug!(
            ghost,
            dropped, "[window_state] 正本に載っているが World に居ない窓を飛ばす"
        );
    }
    for directive in directives {
        apply_directive(state, world, &targets, &ghost, directive);
    }
}

/// 指令 1 件を状態へ当て、印と帯の指令の列を動かし、記録する。
fn apply_directive(
    state: &mut StayOnTopState,
    world: &mut World,
    targets: &[Entity],
    ghost: &str,
    directive: WindowStateDirective,
) {
    match decide_transition(state.on, directive) {
        Transition::TurnOn | Transition::TurnOff => {
            let after = !state.on;
            for entity in targets {
                let mut entity = world.entity_mut(*entity);
                if after {
                    entity.insert(StayOnTop);
                } else {
                    entity.remove::<StayOnTop>();
                }
            }
            push_band_op(
                world,
                if after { BandOp::Enter } else { BandOp::Leave },
                targets,
            );
            info!(
                ghost,
                directive = directive_text(after),
                before = state_text(state.on),
                after = state_text(after),
                windows = targets.len(),
                "[window_state] 常に手前の状態が変わった"
            );
            state.on = after;
        }
        Transition::Unchanged { on } => {
            debug!(
                ghost,
                directive = directive_text(on),
                state = state_text(on),
                "[window_state] 常に手前の状態と同じ指令なので何もしない"
            );
        }
        Transition::Raise { on } => {
            push_band_op(world, BandOp::Raise, targets);
            debug!(
                ghost,
                band = if on { "topmost" } else { "normal" },
                windows = targets.len(),
                "[window_state] ゴーストの窓を今いる帯の最上へ出す"
            );
        }
    }
}

/// 帯の指令の列へ積む（資源が無ければ入れてから・本番で入れるのは確定段の結線）。
fn push_band_op(world: &mut World, op: BandOp, targets: &[Entity]) {
    world
        .get_resource_or_insert_with(WindowBandQueue::default)
        .push(op, targets.to_vec());
}

/// 記録のゴーストの名（今のゴーストのフォルダの名・無ければ `-`）。
fn ghost_name(world: &World) -> String {
    world
        .get_resource::<BootContext>()
        .and_then(|ctx| ctx.current.ghost.folder.clone())
        .unwrap_or_else(|| "-".to_owned())
}

fn directive_text(on: bool) -> &'static str {
    if on { "stayontop" } else { "!stayontop" }
}

fn state_text(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

#[cfg(test)]
#[path = "window_state_tests.rs"]
mod tests;
