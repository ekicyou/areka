//! ゴースト／シェル／バルーン／追加ファイルの `.nar` を入れる手続き（spec: areka-P0-ghost-install）。
//!
//! メニュー・台本・窓への投げ込み（`input_events/file_drop.rs`）は受付 [`submit`] へ依頼を渡す（別のスレッドの入口は
//! [`RawInstallRequest`] を窓口 [`desk`] へ送る）。系の登録は [`register`]（プロセスに 1 回）。
//! 書庫の読み取り・検査・展開は `areka-nar` に任せる。子のモジュール（判断・利用条件・手続き・
//! 背景のスレッド・UI 側の窓口・ファイルを選ぶ画面・置換語の値）は、それを作るタスクが
//! 宣言を 1 行ずつ足す。

use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use crate::exit_wait::{self, WorkGate};

pub(crate) mod desk;
pub(crate) mod fetch_url;
#[cfg(test)]
pub(crate) mod fetch_url_test_support;
pub(crate) mod judge;
pub(crate) mod names;
mod pick;
mod procedure;
mod terms;
mod worker;

/// インストールの依頼（書庫のパスを 1 本以上・並んだ順に扱う＝要件 9.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallOrder {
    pub archives: Vec<PathBuf>,
    pub origin: InstallOrigin,
}

/// 依頼の出どころ（記録の語彙・手続きは分岐しない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallOrigin {
    Menu,
    Script,
    /// 窓への投げ込み（areka-P0-file-drop）。
    WindowDrop,
}

/// 受付の判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmitVerdict {
    Queued,
    /// 書庫が 0 本。
    Empty,
    /// 窓口が無い（系の登録の前）。
    NoDesk,
    /// 終了が始まっている。
    Closing,
}

/// 別のスレッド（台本の受け口・選ぶ画面）から窓口へ届く、依頼になる前の要求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawInstallRequest {
    pub path: PathBuf,
    pub origin: InstallOrigin,
}

/// 依頼を手続きへ渡す唯一の口（UI スレッド・要件 1.1・1.9）。受けたら窓口の待ち行列の末尾に
/// 積んで `info!(install_order_queued)`、断ったら `warn!(install_order_refused)` を 1 件残す。
pub(crate) fn submit(world: &mut World, order: InstallOrder) -> SubmitVerdict {
    let count = order.archives.len();
    let origin = order.origin;
    let verdict = match world.get_non_send_mut::<desk::InstallDesk>() {
        _ if count == 0 => SubmitVerdict::Empty,
        None => SubmitVerdict::NoDesk,
        Some(desk) if desk.gate.is_closing() => SubmitVerdict::Closing,
        Some(mut desk) => {
            desk.queue.push_back(order);
            SubmitVerdict::Queued
        }
    };
    if verdict == SubmitVerdict::Queued {
        tracing::info!(
            event = "install_order_queued",
            origin = ?origin,
            count,
            "[install] 依頼を受けました"
        );
    } else {
        tracing::warn!(
            event = "install_order_refused",
            verdict = ?verdict,
            origin = ?origin,
            count,
            "[install] 依頼を受けられません（書庫が無い・窓口が無い・終了が始まっている）"
        );
    }
    verdict
}

/// 窓口の待ち行列の中身（届いた順の写し・テストが依頼の中身と順を読む口・本番には無い）。
#[cfg(test)]
pub(crate) fn queued_orders(world: &World) -> Vec<InstallOrder> {
    world
        .get_non_send::<desk::InstallDesk>()
        .map_or_else(Vec::new, |desk| desk.queue.iter().cloned().collect())
}

/// 窓口を据え、取り出しの系を Input の段（`dispatch_pointer_events` の後）へ登録し、門を終了の
/// 待ちへ登記する（プロセスに 1 回・呼び手は `ghost_session::register_systems`）。窓口はゴーストを
/// 起こし直しても作り直さない。
pub(crate) fn register(world: &mut World) {
    let gate = Arc::new(WorkGate::default());
    world.insert_non_send(desk::InstallDesk::new(gate.clone()));
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, desk::drain.after(dispatch_pointer_events));
    exit_wait::register_gate(world, "install", gate, desk::discard_for_exit);
}
