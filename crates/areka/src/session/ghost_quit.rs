//! 終わる相: kanade の運行の通知の受け口・終わる相の本体と system・その登録（runtime-module-untangle
//! で `runtime/frame.rs`・`runtime/mod.rs` から相ごと移した。中身と記録の文は変えていない）。

use std::sync::mpsc::Receiver;

use areka_kanade::{KanadeNotice, KanadeStopped};
use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Update;

// 終了相（R15.4）の出口＝終了の統合操作（全窓を閉じてから終了を指示する）。
use crate::app_exit::{ExitOrigin, quit_app};
use crate::runtime::frame::frame_system;
// 終了相が運行の通知を切替の段で振り分ける先（areka-P0-ghost-shell-balloon-switch 要件 8.4）。
use crate::session::ghost_switch::{self, SwitchInFlight};

/// kanade の運行の通知（停止・定常到達・切替の中止）の受け口（World の NonSend 資源・
/// `Receiver` は `Sync` でない）。
///
/// 実 sink 結線（`wire_runtime`）と LogSink 側の起動とで同じ 1 つを共有する（要件 6.4）。
/// 取り出すのは [`run_ghost_quit_phase`] の 1 か所だけ。
pub(crate) struct KanadeNoticeRx(pub(crate) Receiver<KanadeNotice>);

/// 終了相: kanade の運行の通知を取り出して 1 件ずつ捌く（R15.4・design D15 の 4・
/// areka-P0-ghost-shell-balloon-switch design「NoticePhase」）。
///
/// [`ghost_quit_system`] として毎フレームの相（[`frame_system`]）より前に走る。ここで終了が
/// 指示されると全ゴースト窓はもう無く、同じ巡で続けて走る [`frame_system`] は入口の判定で
/// 相を 1 つも回さずに戻る。他の相を止める判定は本関数ではなく [`frame_system`] の入口に在る。
/// 返り値は「停止の通知を消化したか」。
///
/// # 判断
///
/// - 受け口 [`KanadeNoticeRx`] が World に無い（結線していない構成・既存の試験）→ 何もせず `false`。
/// - 届いていない → 何もせず `false`（定常フレームは無操作）。
/// - 届いた分を `try_iter` で**全件**取り出してから（受け口の借用を切ってから）届いた順に捌く:
///   - 停止: 切替の予約（[`SwitchInFlight`]）が在れば切替の振り分け
///     [`ghost_switch::on_ghost_stopped`] へ委ねる（要件 3.3・8.4）。無ければ今日どおり、最初の
///     1 件で `info!(event = "ghost_quit")` の上で統合操作 [`quit_app`]（出所 `KanadeStopped`）を
///     呼ぶ。今日どおりの終了の後の停止と、既に窓が無い場合は `debug!` で打ち切る（終了処理の
///     正常系であって失敗ではない——統合操作が破棄済み標的に対して敷いているのと同じ区別）。
///   - 定常到達・切替の中止: [`ghost_switch::on_notice`] へ（予約を下ろす／予約の外なら記録だけ）。
/// - 送出端が全て落ちた（`Disconnected`）→ 溜まっていた通知は上と同じに扱い、無ければ何もしない。
///   ghost boot に失敗した構成でも受信端だけは生き得るので、切断そのものは異常ではない。
///
/// 取り違えない（要件 3.4）: 受け口はプロセスに 1 つで FIFO。止まった kanade は停止の後に通知を
/// 出さず、切替先は取り出した後に起こすので、取り出した列の中で停止より後ろに切替先の通知が
/// 混ざることは無い。切替先の通知は次の呼び出しで、進んだ段（迎え入れ）の下で捌かれる。
///
/// 統合操作が全窓を閉じて終了を指示すると wintf の `run()` が戻り、`main.rs` の終了統括が走る。
/// そこは既に冪等である（kanade は停止済みゆえ `ForceQuit` の送出が失敗し `debug!` で流れる）。
///
/// 予約の無い停止で、原因（Fault を含む）によって終わらない分岐は置かない（要件 6.5）。
pub(crate) fn run_ghost_quit_phase(world: &mut World) -> bool {
    let Some(rx) = world.get_non_send::<KanadeNoticeRx>() else {
        return false;
    };
    // `Disconnected` も「もう来ない」だけで異常ではない＝溜まっていた分は消化する。
    let notices: Vec<KanadeNotice> = rx.0.try_iter().collect();
    let mut consumed = false;
    let mut quit = false;
    let mut extra = 0usize;
    for notice in notices {
        let KanadeNotice::Stopped(stopped) = notice else {
            ghost_switch::on_notice(world, notice);
            continue;
        };
        consumed = true;
        if world.get_non_send::<SwitchInFlight>().is_some() {
            ghost_switch::on_ghost_stopped(world, stopped);
        } else if quit {
            extra += 1;
        } else {
            quit = true;
            quit_as_today(world, stopped);
        }
    }
    if extra > 0 {
        tracing::debug!(
            event = "ghost_quit_extra",
            extra,
            "停止通知が 2 件以上届いた——窓を閉じるのは 1 度きりゆえ残りは打ち切る"
        );
    }
    consumed
}

/// 予約の無い停止: 今日どおり全ゴースト窓を閉じて終了を指示する。
fn quit_as_today(world: &mut World, stopped: KanadeStopped) {
    tracing::info!(
        event = "ghost_quit",
        cause = ?stopped.cause,
        "kanade の終了系列が完了した: 全ゴースト窓を閉じる"
    );
    let closed = quit_app(world, ExitOrigin::KanadeStopped(stopped.cause));
    if closed == 0 {
        tracing::debug!(
            event = "ghost_quit_no_windows",
            "閉じる窓が既に無い（強制退避が先に走った等）——正常系として打ち切る"
        );
    }
}

/// 終了相 [`run_ghost_quit_phase`] を呼ぶだけの排他 system（起動の 2 経路で共有する）。
///
/// `Update` への登録は `wire_kanade_stop` の 1 か所（[`frame_system`] より前）。
pub(crate) fn ghost_quit_system(world: &mut World) {
    run_ghost_quit_phase(world);
}

/// 運行の通知（停止ほか）の受け口を World に据え、終了相を `Update` に登録する（受け口はプロセスに 1 つ・
/// 要件 6.4）。
///
/// 呼び手は `ghost_session::register_systems` で、プロセスに 1 回、[`register_frame_system`](crate::runtime::register_frame_system) の
/// 直後に呼ぶ。終了相は毎フレームの相より前に走る。順序の相手（毎フレームの相）が登録されていない
/// World でも登録は通る（`frame_schedule_tests` の 1 本がこの形を固定する）。
pub fn wire_kanade_stop(world: &mut World, rx: Receiver<KanadeNotice>) {
    world.insert_non_send(KanadeNoticeRx(rx));
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, ghost_quit_system.before(frame_system));
}

// 終了相の判断（areka-P0-emo2-conformance-e2e タスク 6.9・R15.4）。GPU も実窓も要らない。
#[cfg(test)]
#[path = "ghost_quit_tests.rs"]
mod ghost_quit_tests;

// LogSink 側の起動（実 sink 結線なし）でも停止通知が終了の指示まで届くこと
// （areka-P0-shiori-fault-notice タスク 3.3）。
#[cfg(test)]
#[path = "ghost_quit_logsink_tests.rs"]
mod ghost_quit_logsink_tests;

// 切替の予約が在る間の停止通知と運行の通知の振り分け（areka-P0-ghost-shell-balloon-switch
// タスク 8.4・要件 10.7）。
#[cfg(test)]
#[path = "ghost_quit_switch_tests.rs"]
mod ghost_quit_switch_tests;
