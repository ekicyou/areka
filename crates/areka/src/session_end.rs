//! OS のセッションの終了（シャットダウン・再起動・ログオフ）の受け手
//! （areka-P0-ghost-shell-balloon-switch 要件 12.9〜12.11・design「SessionEnd」・Flow 7）。
//!
//! `WM_ENDSESSION`（wParam が真）から戻った直後にプロセスは終了させられうるので、`run()` の後の
//! 後始末に頼らず、窓の手続きの中（wintf の `OnSessionEnd` に差した [`on_os_session_end`]）で
//! 同期に済ませる: 予約を下ろす → 未処理の運行の通知を今日の規則で捌く → 終了を指示する →
//! ゴーストを「システム」の理由で降ろす → きれいな終わりなら起動中の印を消す → 書いている最中の
//! 背景の仕事を同じ予算の残りで待つ。告知（メッセージボックス）は出さない（OS の終了を塞がない・
//! 失敗は記録に残る）。

use std::time::{Duration, Instant};

use areka_kanade::{CloseReason, WaitBudget};
use bevy_ecs::prelude::*;

use crate::app_exit::{ExitOrigin, FirstExit, quit_app};
use crate::boot_config::BootContext;
use crate::emo2_boot::frame::run_ghost_quit_phase;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::exit_wait;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::{MarkInputs, Teardown, settle_session_mark};

/// OS のセッションの終了を 1 回処理した印（プロセスに高々 1 つ）。`fn main` の後始末はこれを見て
/// 告知も印の判定も行わない。
///
/// 2 通目以降の読み捨ては守りの判定: 本番では最初の受け手の `quit_app` が全ゴースト窓の entity を
/// 破棄するので、残りの窓への `WM_ENDSESSION` は wintf の破棄済みの打ち切りで止まり、ここまで来ない。
#[derive(Resource)]
pub(crate) struct SessionEnded;

/// wintf の `OnSessionEnd` に差す関数（`app_exit::attach_os_close_request` が差す）。
///
/// 手順（要件 12.9）: ① 済みの印が在れば `debug!(os_session_end_again)` で戻る ② 切替の予約を
/// 下ろす（後から届く停止通知が切替として切替先を起こさない）②' 受け口の未処理の運行の通知を
/// 終了の相で今日の規則どおり捌く（予約は外した後なので停止は今日どおり終了へ＝先に届いていた
/// SHIORI の失敗が最初の出所として勝つ）③ 出所「OS のセッションの終了」で全窓を閉じて終了を
/// 指示する ④ 置き場のゴーストの倒れた旗を読んでから `CloseReason::System` で同期に、SHIORI を
/// 待つ合計を [`SESSION_END_SHIORI_LIMIT`] に収めて降ろす（kanade の強制終了が `OnClose` を
/// Ref0＝`system` の NOTIFY で送ってから SHIORI を降ろし、記憶を書き出す・上限に達したら見張りが
/// 補助プロセスを終わらせて待ちを解く）⑤ きれいな終わりの判定で印を消すか残すか（倒れた旗と
/// 打ち切りの有無も材料）⑥ 所要 ms と打ち切りの有無を記録する。
///
/// # join と同期の送信（要件 5.4・`research.md` §6）
/// 降ろす処理の join は UI スレッド（この関数を呼ぶ窓の手続きのスレッド）を塞ぎ、その間は
/// 送られてきたメッセージを配らない。それでも止まらない理由:
/// - areka のどのスレッドも UI スレッドの窓へ同期に送らない。ゴーストの実行系（kanade・shiori・
///   dispatcher ほか）から UI への知らせはすべて `mpsc` の送信。shiori のスレッドの
///   `SendMessageTimeoutW` の宛先は 32bit の補助プロセスの message-only 窓（別プロセス）で、
///   補助プロセスからの応答の宛先は shiori のスレッドが持つ親窓。wintf の窓の操作は UI スレッド
///   自身が行い、VSync のスレッドは窓に触らない。COM は MTA なので、別スレッドからの呼び出しが
///   メッセージで取り次がれない。したがって join が待つ相手は、UI スレッドを待たない。
/// - 外から UI の窓へ同期に送る相手（IME・シェル・他のアプリ）は送り手の側が待つだけで、join は
///   それに依らないので輪にならない。その送信は後始末から戻った後に返る
///   （`session_end_sync_send_tests.rs` が固定する）。
/// - 理屈の上で残る唯一の輪は、補助プロセスの中の SHIORI・SAORI が `OnClose` の処理中などに
///   areka の窓へ同期に送る形（UI スレッドの join → kanade → shiori のスレッドの往復 → 補助
///   プロセスの中の送信 → UI スレッド）。この輪は [`SESSION_END_SHIORI_LIMIT`] に達した時点で
///   見張りが補助プロセスを終わらせて切る。
///
/// 所要は `os_session_end_done` の `ms` で実機サインオフで測る。
pub(crate) fn on_os_session_end(world: &mut World, _entity: Entity) {
    end_session_within(world, SESSION_END_SHIORI_LIMIT)
}

/// OS のセッションの終了で SHIORI を待つ合計の上限（2026-09-27 開発者の確定・要件 1.2・8.1）。
/// 後始末に入った時点から数える。
pub(crate) const SESSION_END_SHIORI_LIMIT: Duration = Duration::from_secs(3);

/// [`on_os_session_end`] の本体。上限を引数で受ける（本番は定数だけを渡す・テストは任意の上限）。
/// 出発点は後始末に入った今。
pub(crate) fn end_session_within(world: &mut World, limit: Duration) {
    end_session_from(
        world,
        WaitBudget {
            started: Instant::now(),
            limit,
        },
    )
}

/// [`end_session_within`] の中身。予算（出発点と上限）を外から受ける（テストは出発点を過去に置ける）。
/// ゴーストを降ろす待ちと背景の仕事の待ちは、同じ予算から数える（足し算にしない）。
pub(crate) fn end_session_from(world: &mut World, budget: WaitBudget) {
    if world.contains_resource::<SessionEnded>() {
        tracing::debug!(
            event = "os_session_end_again",
            "[session_end] セッションの終了は処理済み——読み捨てる"
        );
        return;
    }
    let WaitBudget { started, limit } = budget;
    world.insert_resource(SessionEnded);
    tracing::info!(
        event = "os_session_end_begin",
        "[session_end] OS のセッションの終了: 窓の手続きの中で後始末を済ませる"
    );
    // 背景の仕事の門を閉じる（待っている依頼を捨て、以後は書き始めない・待つのは印の始末の後）。
    let closing = exit_wait::begin_close(world);

    if world.remove_non_send::<SwitchInFlight>().is_some() {
        tracing::info!(
            event = "ghost_switch_cancelled",
            reason = "session_end",
            "[session_end] 切替の予約を下ろした（後から届く停止通知で切替先を起こさない）"
        );
    }
    run_ghost_quit_phase(world);
    quit_app(world, ExitOrigin::SessionEnd);

    let session = world
        .get_non_send_mut::<GhostSlot>()
        .and_then(|mut slot| slot.0.take());
    // 倒れた旗は単位を消費する前に読む。
    let logsink_fallback = session.as_ref().is_some_and(GhostSession::logsink_fallback);
    let (down_ok, shiori_cut) = match session {
        Some(session) => {
            let (result, cut) =
                session.shutdown_within(CloseReason::System, WaitBudget { started, limit });
            let down_ok = match result {
                Ok(()) => true,
                Err(err) => {
                    tracing::error!(
                        event = "os_session_end_down_failed",
                        error = %err,
                        "[session_end] ゴーストを降ろせなかった——起動中の印を残す（次の起動は Ref6/7 付き）"
                    );
                    false
                }
            };
            (down_ok, cut.is_some())
        }
        None => (true, false),
    };

    // 降ろした後（実行系が 0）なので UI スレッドが App スコープへ直接書いてよい。
    let first = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    match world.get_resource::<BootContext>() {
        Some(ctx) => {
            let mark = MarkInputs {
                app_profile_dir: ctx.app_profile_dir.clone(),
                argv_session: ctx.argv_session,
                first,
                logsink_fallback,
            };
            settle_session_mark(
                &mark,
                Teardown {
                    run_ok: true,
                    down_ok,
                    shiori_cut,
                },
            );
        }
        None => tracing::warn!(
            event = "boot_context_missing",
            "[session_end] 起動の文脈が無いので起動中の印を消せません（次の起動は前回落ちたとして Ref6/7 付きになります）"
        ),
    }

    // 書いている最中の背景の仕事を、降ろす待ちと同じ出発点・同じ上限で待つ（足し算にしない・
    // ghost-install 要件 8.2）。印の始末の後なので、待つ間に終わらされても印は済んでいる。
    closing.wait(WaitBudget { started, limit });

    tracing::info!(
        event = "os_session_end_done",
        ms = started.elapsed().as_millis() as u64,
        down_ok,
        shiori_cut,
        "[session_end] OS のセッションの終了の後始末を済ませた"
    );
}

#[cfg(test)]
#[path = "session_end_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "session_end_deadline_tests.rs"]
mod deadline_tests;

#[cfg(test)]
#[path = "session_end_sync_send_tests.rs"]
mod sync_send_tests;
