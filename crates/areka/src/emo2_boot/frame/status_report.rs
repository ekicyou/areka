//! バルーンの表示の届け（areka-P0-status-execution-states 要件 4.1〜4.7・7.1・design「balloon の届け」）。
//!
//! フレームの終わり（提示の後）の届けの相で、表示層の照会 2 本（`target_visible`・`current_surface_id`）から
//! 「見えているバルーンの組」を作り、最後に kanade へ送った組と違うときだけ置き場のゴーストの
//! kanade へ `ExecutionState(Balloons(組))` を 1 件送る。表示の真実源は `EmoPresenter` のままで、
//! 台帳が覚えるのは「最後に何を送ったか」「番号が取れない旨を警告済みの scope」「scope ごとに最後に
//! 取れた番号（表示層の値の写し）」だけである。
//! 台帳は `Emo2Wiring::balloon_status`（ゴーストごとに新品）にあり、`emo2_frame_system` が提示の相の
//! 直後に [`run_status_report_phase`] を呼ぶ。窓の可視（可視性の相が発行済み）と箱の写し（同じ
//! フレームの提示で作り直し済み）がどちらもそのフレームの最終の姿になってから組を作るので、
//! 届けは画面から遅れない（areka-P0-shell-balloon-frame-align 要件 2.3）。
//!
//! 箱に文字が出ているスコープ（文字の層の `shown_boxes` が空でない）も、普通のバルーンの窓が
//! 見えているときと同じ形で組に載せる（areka-P0-shell-balloon 要件 5.5・5.6）。
//!
//! 同じ相の、文字の層を借りる手前で、窓の知らせ（開いているか・面の番号・出番の世代）も前と違う
//! scope だけ置き場のゴーストの seriko へ送る（areka-P0-animated-image-playback 要件 2.3・6.1）。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_kanade::{BalloonBinding, ExecutionStateUpdate, KanadeMsg};
use areka_sakura::ActorKey;
use areka_seriko::StageNote;
use bevy_ecs::world::World;
use tracing::{debug, error, warn};

use super::Emo2Wiring;
use crate::emo2_boot::target_map::balloon_target;
use crate::ghost_session::GhostSlot;

/// 最後に kanade へ送った組と、番号が取れない scope の警告済み集合と、最後に取れた番号
/// （ゴーストごと・`Emo2Wiring` が持つ）。
#[derive(Default)]
pub(in crate::emo2_boot) struct BalloonStatusLedger {
    last_sent: Vec<BalloonBinding>,
    surface_unknown_warned: BTreeSet<u32>,
    /// スコープ → 普通のバルーンの面の番号で、最後に取れたもの（表示層の値の写し）。
    last_surface: BTreeMap<u32, u32>,
    /// 文字の層を借りられない旨を記録済みか。
    runtime_busy_logged: bool,
    /// スコープ → 前に seriko へ知らせた窓の組（animated-image-playback 要件 2.3・6.1）。
    last_stage: BTreeMap<u32, StageTuple>,
}

/// 窓の知らせの組（開いているか, 面の番号, 出番の世代）。
type StageTuple = (bool, u32, u64);

/// 1 scope の窓の観測（表示層の照会 3 本・表示層に装着済みの scope だけ）。
#[derive(Clone)]
pub(in crate::emo2_boot) struct StageObservation {
    scope: u32,
    open: bool,
    surface_id: Option<u32>,
    generation: u64,
}

/// 1 scope の観測（表示層の照会 2 本と、箱に文字が出ているか）。
pub(in crate::emo2_boot) struct BalloonObservation {
    scope: u32,
    visible: Option<bool>,
    surface_id: Option<u32>,
    box_showing: bool,
}

/// 観測列と「最後に取れた番号」から組を作る純関数。`visible == Some(true)` または `box_showing` の scope だけを
/// `character_id = scope` で並べ（入力の順＝昇順）、番号が取れなかった警告の対象を第 2 の返り値に添える。
///
/// 番号は今の `surface_id`。取れないとき、窓が見えていれば 0 で警告の対象、箱だけなら覚えた番号（無ければ 0）で
/// 警告の対象にしない（areka-P0-shell-balloon-frame-align 要件 3.1・3.2）。
pub(in crate::emo2_boot) fn collect_bindings(
    observed: &[BalloonObservation],
    last_surface: &BTreeMap<u32, u32>,
) -> (Vec<BalloonBinding>, Vec<u32>) {
    let mut bindings = Vec::new();
    let mut surface_unknown = Vec::new();
    for o in observed
        .iter()
        .filter(|o| o.visible == Some(true) || o.box_showing)
    {
        let balloon_id = match o.surface_id {
            Some(id) => id,
            None if o.visible == Some(true) => {
                surface_unknown.push(o.scope);
                0
            }
            None => last_surface.get(&o.scope).copied().unwrap_or(0),
        };
        bindings.push(BalloonBinding {
            character_id: o.scope,
            balloon_id,
        });
    }
    (bindings, surface_unknown)
}

/// 届けの相。装着済みバルーンのスコープ（昇順）について照会 → 組 → 差分 → 送出 → 記録。
///
/// 文字の層を借りる手前で、窓の知らせ（[`report_stages`]）を seriko へ送る。
/// スコープの一覧は相の中で自分で作る（可視性の相の手元の一覧は使わない）。文字の層を借りられない
/// フレームは誤りの段で 1 度だけ記録して届けを次のフレームへ回し、借りられたら記録の印を戻す。
pub(in crate::emo2_boot) fn run_status_report_phase(wiring: &mut Emo2Wiring, world: &World) {
    // `HashMap` の列挙順は不定なので、組の並びを決定論にするため昇順へ固定する。
    let mut scopes: Vec<u32> = wiring.balloon_models.keys().copied().collect();
    scopes.sort_unstable();
    let ledger = &mut wiring.balloon_status;
    // 窓の知らせは文字の層を借りる手前（借りられないフレームでも届ける・animated-image-playback 要件 2.3）。
    report_stages(&wiring.presenter, world, ledger, &scopes);
    let Ok(runtime) = wiring.runtime.try_borrow() else {
        if !ledger.runtime_busy_logged {
            ledger.runtime_busy_logged = true;
            error!(
                event = "balloon_status_runtime_busy",
                "文字の層を借りられないので、バルーンの組を届けるのを次のフレームへ回す"
            );
        }
        return;
    };
    ledger.runtime_busy_logged = false;
    report_balloons(&wiring.presenter, &runtime, world, ledger, &scopes);
}

/// 窓の知らせ（animated-image-playback 要件 2.3・6.1・design「窓の知らせ」）。装着済みバルーンの
/// スコープ（昇順）ごとに表示層の照会 3 本（`target_visible`・`current_surface_id`・
/// `stage_generation`）を読み、前に知らせた組と違うときだけ置き場のゴーストの seriko へ送る。
///
/// 表示層に未装着の scope は知らせない（装着されたフレームで知らせる）。
pub(in crate::emo2_boot) fn report_stages(
    presenter: &EmoPresenter,
    world: &World,
    ledger: &mut BalloonStatusLedger,
    scopes: &[u32],
) {
    let observed: Vec<StageObservation> = scopes
        .iter()
        .filter_map(|&scope| {
            let target = balloon_target(scope);
            Some(StageObservation {
                scope,
                open: presenter.target_visible(target)?,
                surface_id: presenter.current_surface_id(target),
                generation: presenter.stage_generation(target)?,
            })
        })
        .collect();
    let sink = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.seriko_sink());
    report_stage_observed(
        ledger,
        &observed,
        sink.map(|sink| |note| sink.send_stage(note)),
    );
}

/// 観測から先（組 → 差分 → 送出 → 記録）。送り手が無い（置き場にゴーストが居ない）フレームは
/// 台帳を変えずに見送り、ゴーストが据わった最初のフレームで送る。
fn report_stage_observed(
    ledger: &mut BalloonStatusLedger,
    observed: &[StageObservation],
    send: Option<impl FnMut(StageNote)>,
) {
    let changed: Vec<(u32, StageTuple)> = observed
        .iter()
        .filter_map(|o| {
            let last = ledger.last_stage.get(&o.scope);
            // 隠すと表示層は今の面の番号を消す（`Hide`）。閉じた知らせは前に知らせた番号を運ぶ。
            let face = o.surface_id.or(last.map(|t| t.1)).unwrap_or(0);
            let tuple = (o.open, face, o.generation);
            (last != Some(&tuple)).then_some((o.scope, tuple))
        })
        .collect();
    if changed.is_empty() {
        return;
    }
    let Some(mut send) = send else {
        debug!(
            event = "balloon_stage_no_ghost",
            "置き場にゴーストが無いので、窓の知らせを seriko へ送るのを見送る"
        );
        return;
    };
    for (scope, tuple @ (open, face, generation)) in changed {
        send(StageNote::Balloon {
            scope: ActorKey::from(scope.to_string()),
            open,
            face,
            generation,
        });
        ledger.last_stage.insert(scope, tuple);
    }
}

/// 照会の配線。照会（表示層 2 本＋文字の層の `shown_boxes`）→ 純関数 → 差分 →
/// `GhostSlot` の kanade へ送出 → 記録。
///
/// `scopes` は装着済みバルーンの scope 昇順（[`run_status_report_phase`] が作る）。
pub(in crate::emo2_boot) fn report_balloons(
    presenter: &EmoPresenter,
    runtime: &TextLayerRuntime,
    world: &World,
    ledger: &mut BalloonStatusLedger,
    scopes: &[u32],
) {
    let observed: Vec<BalloonObservation> = scopes
        .iter()
        .map(|&scope| {
            let target = balloon_target(scope);
            BalloonObservation {
                scope,
                visible: presenter.target_visible(target),
                surface_id: presenter.current_surface_id(target),
                box_showing: !runtime
                    .shown_boxes(&ActorKey::from(scope.to_string()))
                    .is_empty(),
            }
        })
        .collect();
    report_observed(world, ledger, &observed);
}

/// 観測から先（組 → 警告 → 差分 → 送出 → 記録）。照会を持たない檻が直接踏む。
fn report_observed(
    world: &World,
    ledger: &mut BalloonStatusLedger,
    observed: &[BalloonObservation],
) {
    // 番号が取れた scope は、組を作る前に覚え直し、警告を再武装する（窓が見えていなくても覚える）。
    for o in observed {
        if let Some(id) = o.surface_id {
            ledger.last_surface.insert(o.scope, id);
            ledger.surface_unknown_warned.remove(&o.scope);
        }
    }
    let (bindings, surface_unknown) = collect_bindings(observed, &ledger.last_surface);

    // 取れない scope は scope ごとに 1 回だけ警告する。
    for scope in surface_unknown {
        if ledger.surface_unknown_warned.insert(scope) {
            warn!(
                event = "balloon_status_surface_unknown",
                scope, "見えているバルーンの番号が取れないので 0 として届ける"
            );
        }
    }

    if bindings == ledger.last_sent {
        return;
    }
    let Some(kanade) = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.kanade())
    else {
        // 切替の最中など。台帳を保ち、ゴーストが据わった最初のフレームで送る。
        debug!(
            event = "balloon_status_no_ghost",
            "置き場にゴーストが無いので、バルーンの組を届けるのを見送る"
        );
        return;
    };
    let msg = KanadeMsg::ExecutionState(ExecutionStateUpdate::Balloons(bindings.clone()));
    if kanade.send(msg).is_ok() {
        debug!(
            event = "balloon_status_reported",
            bindings = ?bindings,
            "見えているバルーンの組を運行の側へ届けた"
        );
    } else {
        // 受け手が消えた kanade へ毎フレーム鳴らさないよう、台帳は更新する。
        error!(
            event = "balloon_status_send_failed",
            bindings = ?bindings,
            "運行の側へバルーンの組を渡せない（受け手が消えている）"
        );
    }
    ledger.last_sent = bindings;
}

#[cfg(test)]
#[path = "status_report_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "status_report_stage_tests.rs"]
mod stage_tests;
