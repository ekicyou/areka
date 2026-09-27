//! OS のセッションの終了（シャットダウン・再起動・ログオフ）の受け手
//! （areka-P0-ghost-shell-balloon-switch 要件 12.9〜12.11・design「SessionEnd」・Flow 7）。
//!
//! `WM_ENDSESSION`（wParam が真）から戻った直後にプロセスは終了させられうるので、`run()` の後の
//! 後始末に頼らず、窓の手続きの中（wintf の `OnSessionEnd` に差した [`on_os_session_end`]）で
//! 同期に済ませる: 予約を下ろす → 未処理の運行の通知を今日の規則で捌く → 終了を指示する →
//! ゴーストを「システム」の理由で降ろす → きれいな終わりなら起動中の印を消す。告知（メッセージ
//! ボックス）は出さない（OS の終了を塞がない・失敗は記録に残る）。

use std::time::Instant;

use areka_kanade::CloseReason;
use bevy_ecs::prelude::*;

use crate::app_exit::{ExitOrigin, FirstExit, quit_app};
use crate::boot_config::BootContext;
use crate::emo2_boot::frame::run_ghost_quit_phase;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::ghost_session::GhostSlot;
use crate::{MarkInputs, settle_session_mark};

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
/// 指示する ④ 置き場のゴーストを `CloseReason::System` で同期に降ろす（kanade の強制終了が
/// `OnClose` を Ref0＝`system` の NOTIFY で送ってから SHIORI を降ろし、記憶を書き出す）
/// ⑤ きれいな終わりの判定で印を消すか残すか ⑥ 所要 ms を記録する。
///
/// 降ろす処理の join は UI スレッドを塞ぐ（送られてきたメッセージは配らない）。所要は
/// `os_session_end_done` の `ms` で実機サインオフで測る。
pub(crate) fn on_os_session_end(world: &mut World, _entity: Entity) {
    if world.contains_resource::<SessionEnded>() {
        tracing::debug!(
            event = "os_session_end_again",
            "[session_end] セッションの終了は処理済み——読み捨てる"
        );
        return;
    }
    let started = Instant::now();
    world.insert_resource(SessionEnded);
    tracing::info!(
        event = "os_session_end_begin",
        "[session_end] OS のセッションの終了: 窓の手続きの中で後始末を済ませる"
    );

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
    let down_ok = match session {
        Some(session) => match session.shutdown(CloseReason::System) {
            Ok(()) => true,
            Err(err) => {
                tracing::error!(
                    event = "os_session_end_down_failed",
                    error = %err,
                    "[session_end] ゴーストを降ろせなかった——起動中の印を残す（次の起動は Ref6/7 付き）"
                );
                false
            }
        },
        None => true,
    };

    // 降ろした後（実行系が 0）なので UI スレッドが App スコープへ直接書いてよい。
    let first = world.get_resource::<FirstExit>().map(|f| f.0.clone());
    match world.get_resource::<BootContext>() {
        Some(ctx) => {
            let mark = MarkInputs {
                app_profile_dir: ctx.app_profile_dir.clone(),
                argv_session: ctx.argv_session,
                first,
            };
            settle_session_mark(&mark, true, down_ok);
        }
        None => tracing::warn!(
            event = "boot_context_missing",
            "[session_end] 起動の文脈が無いので起動中の印を消せません（次の起動は前回落ちたとして Ref6/7 付きになります）"
        ),
    }

    tracing::info!(
        event = "os_session_end_done",
        ms = started.elapsed().as_millis() as u64,
        down_ok,
        "[session_end] OS のセッションの終了の後始末を済ませた"
    );
}

#[cfg(test)]
#[path = "session_end_tests.rs"]
mod tests;
