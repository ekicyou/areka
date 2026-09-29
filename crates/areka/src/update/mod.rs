//! ゴースト・シェル・バルーンのネットワーク更新（spec: areka-P0-network-update）。
//!
//! メニューと台本の受け口は、対象を解く前の要求 [`RawUpdateRequest`] を窓口へ運び、窓口が解いた
//! 依頼 [`UpdateOrder`] を手続きへ渡す。取得・照合・確定は `areka-update` のエンジンに任せる。
//! 登録の口と受付の口、子のモジュール（写し・手続き・背景のスレッド・UI 側の窓口）は、それを作る
//! タスクが宣言を 1 行ずつ足す。

use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::exit_wait::{self, WorkGate};
use crate::ghost_session::GhostSlot;

pub(crate) mod desk;
mod procedure;
mod refs;
mod worker;

/// 更新の対象の種別（Reference の綴り: ghost／shell／balloon）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetKind {
    Ghost,
    Shell,
    Balloon,
}

impl TargetKind {
    /// Reference に載せる綴り。
    pub(crate) fn as_ref_str(self) -> &'static str {
        match self {
            Self::Ghost => "ghost",
            Self::Shell => "shell",
            Self::Balloon => "balloon",
        }
    }
}

/// 更新の理由（Reference4 の綴り: manual／script）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdateReason {
    /// メニューから。
    Manual,
    /// 台本から。
    Script,
}

impl UpdateReason {
    /// Reference4 に載せる綴り。
    pub(crate) fn as_ref_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Script => "script",
        }
    }
}

/// 総括の形（`OnUpdateResult` か、先頭に名前の付く `OnUpdateResultEx` か）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SummaryKind {
    Result,
    ResultEx,
}

/// 解いた対象 1 つ（更新先はまだ決まっていない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetSpec {
    pub kind: TargetKind,
    /// 対象のフォルダ（エンジンの `target`・Reference1 の材料）。
    pub dir: PathBuf,
    /// 対象の名前（Reference0・総括 Ex の先頭）。
    pub name: String,
    /// `descript.txt` の `homeurl`（ゴーストは SHIORI の答えが無いときの倒れ先）。
    pub descript_homeurl: Option<String>,
}

/// 更新の依頼（窓口が解いた対象の列・並んだ順に扱う）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateOrder {
    pub targets: Vec<TargetSpec>,
    pub reason: UpdateReason,
    pub summary: SummaryKind,
    /// 依頼を受けた時点のゴーストのフォルダ（読み直しの照合と記録）。
    pub ghost_dir: PathBuf,
    /// 同じくフォルダ名（argv の起動は None＝読み直さない）。
    pub ghost_folder: Option<String>,
}

/// 入口が窓口へ運ぶ、対象を解く前の要求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawUpdateRequest {
    /// 今のゴースト・シェル・バルーンのうち並べた物（メニューは 3 つ・`all` も 3 つ）。
    Current(Vec<TargetKind>),
    /// 名前で引くシェル・バルーン（`\![updateother,…]`・並んだ順）。
    Other(Vec<(TargetKind, String)>),
}

/// 受付の判定（[`submit`] の戻り値）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmitVerdict {
    /// 受けて背景スレッドへ渡した。
    Started,
    /// 答え待ち（`OnUpdateProcessExec`）の間に届き、預かった（答えが出てから始めるか断る）。
    Held,
    NoDesk,
    Closing,
    /// 走っている（答え待ちで預かりが埋まっているときも）＝`OnUpdateFailure(executing)` を送った。
    Executing,
    /// 切替の予約が在る。
    NotSteady,
    /// 置き場のゴーストに送出端が無い・起動の文脈が無い。
    NoContext,
    NoTargets,
    /// 背景スレッドが居ない（仕事を渡せなかった・`error!`）。次の依頼で起こし直す。
    WorkerGone,
}

/// 依頼を手続きへ渡す唯一の口（UI スレッド）。窓口が無い → 終了が始まった → 走っている（答え待ちで
/// 預かりが埋まっているときも）→ 切替の予約が在る → 送出端・起動の文脈が無い → 解いた対象が 0 の順に
/// 断り、判定ごとに `warn!(update_refused)` を 1 件残す。答え待ちで預かりが空なら対象を解かずに預かる。
/// 受けたら対象を解き、kanade の送出端の写しと一緒に背景スレッドへ渡す。
pub(crate) fn submit(
    world: &mut World,
    raw: RawUpdateRequest,
    reason: UpdateReason,
) -> SubmitVerdict {
    let verdict = judge(world, raw, reason);
    match verdict {
        // 記録は判定した場所が残す（`Executing` は送れたかを添えるため）。
        SubmitVerdict::Started
        | SubmitVerdict::Held
        | SubmitVerdict::Executing
        | SubmitVerdict::WorkerGone => {}
        _ => tracing::warn!(
            event = "update_refused",
            verdict = ?verdict,
            reason = reason.as_ref_str(),
            "[update] 更新の要求を受けられません（窓口が無い・終了が始まった・切替中・文脈が無い・対象が無い）"
        ),
    }
    verdict
}

fn judge(world: &mut World, raw: RawUpdateRequest, reason: UpdateReason) -> SubmitVerdict {
    let Some(desk) = world.get_non_send::<desk::UpdateDesk>() else {
        return SubmitVerdict::NoDesk;
    };
    if desk.gate.is_closing() {
        return SubmitVerdict::Closing;
    }
    match (desk.stage, desk.held.is_some()) {
        (desk::Stage::Running, _) | (desk::Stage::AwaitingExec, true) => {
            desk::refuse_executing(world, &raw, reason);
            return SubmitVerdict::Executing;
        }
        (desk::Stage::AwaitingExec, false) => {
            tracing::info!(
                event = "update_request_held",
                raw = ?raw,
                reason = reason.as_ref_str(),
                "[update] ゴーストの答えを待っているので、要求を預かります"
            );
            world.non_send_mut::<desk::UpdateDesk>().held = Some((raw, reason));
            return SubmitVerdict::Held;
        }
        (desk::Stage::Idle, _) => {}
    }
    if world.get_non_send::<SwitchInFlight>().is_some() {
        return SubmitVerdict::NotSteady;
    }
    let session = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref());
    let (Some(ctx), Some(session)) = (world.get_resource::<BootContext>(), session) else {
        return SubmitVerdict::NoContext;
    };
    let Some(kanade) = session.kanade().cloned() else {
        return SubmitVerdict::NoContext;
    };
    let ghost_dir = session.ghost_dir().to_path_buf();
    let ghost_folder = ctx.current.ghost.folder.clone();
    let targets = desk::resolve_targets(world, &raw);
    if targets.is_empty() {
        return SubmitVerdict::NoTargets;
    }
    let summary = match raw {
        RawUpdateRequest::Current(_) => SummaryKind::Result,
        RawUpdateRequest::Other(_) => SummaryKind::ResultEx,
    };
    let order = UpdateOrder {
        targets,
        reason,
        summary,
        ghost_dir,
        ghost_folder,
    };
    desk::hand_over(world, worker::UpdateJob { order, kanade })
}

/// 窓口を据え、取り出しの系を Input の段（`dispatch_pointer_events` の後＝投げ込みの捌きの後）へ
/// 登録し、門 `update` を片付けの関数と一緒に終了の待ちへ登記する（プロセスに 1 回・呼び手は
/// `ghost_session::register_systems`）。窓口はゴーストを起こし直しても作り直さない。
pub(crate) fn register(world: &mut World) {
    let gate = Arc::new(WorkGate::default());
    world.insert_non_send(desk::UpdateDesk::new(gate.clone()));
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, desk::drain.after(dispatch_pointer_events));
    exit_wait::register_gate(world, "update", gate, desk::discard_for_exit);
}

#[cfg(test)]
mod update_tests;
