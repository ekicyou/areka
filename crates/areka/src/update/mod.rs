//! ゴースト・シェル・バルーンのネットワーク更新（spec: areka-P0-network-update）。
//!
//! メニューと台本の受け口は、対象を解く前の要求 [`RawUpdateRequest`] を窓口へ運び、窓口が解いた
//! 依頼 [`UpdateOrder`] を手続きへ渡す。取得・照合・確定は `areka-update` のエンジンに任せる。
//! 登録の口と受付の口、子のモジュール（写し・手続き・背景のスレッド・UI 側の窓口）は、それを作る
//! タスクが宣言を 1 行ずつ足す。
// 呼び手（受付の口・窓口・手続き）がまだ無い。受付の口を足すタスク 6.3 で外す。
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use crate::exit_wait::{self, WorkGate};

mod desk;
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
