//! ゴースト／シェル／バルーン／追加ファイルの `.nar` を入れる手続き。
//!
//! メニュー・台本・窓への投げ込み（`file_drop.rs`）は受付 [`submit`] へ依頼を渡す（別のスレッドの入口は
//! [`RawInstallRequest`] を窓口 [`desk`] へ送る）。系の登録は [`register`]（プロセスに 1 回）。
//! 書庫の読み取り・検査・展開は `areka-nar` に任せる。

use std::path::PathBuf;
use std::sync::Arc;

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

use areka_session::exit_wait::{self, WorkGate};

mod before_boot_ports;
pub(crate) mod desk;
mod destination;
pub(crate) mod fetch_url;
#[cfg(test)]
pub(crate) mod fetch_url_test_support;
/// `\![execute,install,…]` の受け口。
pub(crate) mod install_cue;
pub(crate) mod judge;
pub(crate) mod names;
pub(crate) mod pick;
mod procedure;
pub(crate) mod startup;
mod terms;
mod worker;

/// インストールの依頼（書庫のパスを 1 本以上・並んだ順に扱う）。
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
    /// 窓への投げ込み。
    WindowDrop,
    /// 起動の引数 `--install`。
    StartupArgs,
    /// exe の隣の `initial_install.nar`。
    InitialInstall,
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
    /// 読み取りだけの走り（積まない・ゴーストへ何も送らない）。
    ReadOnly,
}

/// 別のスレッド（台本の受け口・選ぶ画面・取得のスレッド）から窓口へ届く、依頼になる前の要求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RawInstallRequest {
    Path {
        path: PathBuf,
        origin: InstallOrigin,
    },
    /// 台本の `\![execute,install,url,…]` の URL（窓口が取得の待ち行列へ積む）。
    Url(String),
}

/// 依頼を手続きへ渡す唯一の口（UI スレッド）。受けたら窓口の待ち行列の末尾に
/// 積んで `info!(install_order_queued)`、断ったら `warn!(install_order_refused)` を 1 件残す。
/// 読み取りだけの走りなら積む前に断り、`info!(readonly_refused)` だけを 1 件残す。
pub(crate) fn submit(world: &mut World, order: InstallOrder) -> SubmitVerdict {
    let count = order.archives.len();
    let origin = order.origin;
    if areka_session::readonly::of(world) {
        record_readonly_refused(origin, count);
        return SubmitVerdict::ReadOnly;
    }
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

/// 読み取りだけの断りの記録（窓口の `submit` と起動の前の道の共通）。
pub(crate) fn record_readonly_refused(origin: InstallOrigin, count: usize) {
    tracing::info!(
        event = "readonly_refused",
        what = "install",
        origin = ?origin,
        count,
        "[install] 読み取りだけなので、インストールの依頼を断ります"
    );
}

/// 窓口の待ち行列の中身（届いた順の写し・テストが依頼の中身と順を読む口・本番には無い）。
#[cfg(test)]
pub(crate) fn queued_orders(world: &World) -> Vec<InstallOrder> {
    world
        .get_non_send::<desk::InstallDesk>()
        .map_or_else(Vec::new, |desk| desk.queue.iter().cloned().collect())
}

/// 窓口を据え、取り出しの系を Input の段（`dispatch_pointer_events` の後）へ登録し、門 2 つ（依頼と
/// 取得）を終了の待ちへ登記する（プロセスに 1 回・呼び手は `BODY_SYSTEMS` の `after_input`）。窓口はゴーストを
/// 起こし直しても作り直さない。
pub(crate) fn register(world: &mut World) {
    let gate = Arc::new(WorkGate::default());
    let fetch_gate = Arc::new(WorkGate::default());
    world.insert_non_send(desk::InstallDesk::new(gate.clone(), fetch_gate.clone()));
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, desk::drain.after(dispatch_pointer_events));
    exit_wait::register_gate(world, "install", gate, desk::discard_for_exit);
    exit_wait::register_gate(world, "install-fetch", fetch_gate, |_| {});
}

#[cfg(test)]
#[path = "submit_readonly_tests.rs"]
mod submit_readonly_tests;
