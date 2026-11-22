//! 後始末と OS のセッションの終了が共有する起動中の印の判定。

use crate::app_exit;

/// 起動中の印の判定の材料（`run()` の後に組み、降ろした後に [`settle_session_mark`] へ渡す）。
pub struct MarkInputs {
    /// 記憶の置き場（App スコープ）。
    pub app_profile_dir: std::path::PathBuf,
    /// 起動の文脈の argv の旗。
    pub argv_session: bool,
    /// 最初に終了を指示した出所（無ければ `None`）。
    pub first: Option<app_exit::ExitOrigin>,
    /// 最初の起動が LogSink へ倒れたか（倒れた先の成否を問わない）。
    pub logsink_fallback: bool,
}

/// 降ろした結果（`fn main` の後始末は `shiori_cut=false`・OS のセッションの終了は見張りの結果）。
#[derive(Debug, Clone, Copy)]
pub struct Teardown {
    /// メッセージループ（`run()`）が成功したか。
    pub run_ok: bool,
    /// 降ろす処理が成功したか。
    pub down_ok: bool,
    /// OS のセッションの終了で SHIORI の待ちを上限で打ち切ったか。
    pub shiori_cut: bool,
}

/// きれいな終わりの判定の結論。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkVerdict {
    /// きれいに終わった＝印を消す。
    Clear,
    /// きれいに終わらなかった＝印を残す（理由は記録の `reason`）。
    Keep(&'static str),
    /// argv で始まったプロセス＝印に触れない。
    Untouched,
}

/// きれいな終わりの判定（後始末と OS のセッションの終了が共有する唯一の判定）。
///
/// 時系列で最初に起きた理由を採る。上から順に: argv で始まったプロセスは触らない → 最初の起動が
/// LogSink へ倒れた → 最初の出所が無い → 出所が失敗（SHIORI の失敗・既定へ戻せない致命）→ `run()` の
/// 失敗 → OS のセッションの終了で SHIORI の待ちを上限で打ち切った → 降ろす処理の失敗 → それ以外は
/// 消す。出所は網羅の match で見るので、出所を足すとこの判定の漏れがコンパイルで止まる。
/// 名前は見ないので、倒れたのが既定ゴースト自身でも同じく残す。
pub fn session_mark_verdict(
    first: Option<&app_exit::ExitOrigin>,
    argv_session: bool,
    logsink_fallback: bool,
    end: Teardown,
) -> MarkVerdict {
    use app_exit::ExitOrigin;
    use areka_kanade::KanadeStopCause;
    if argv_session {
        return MarkVerdict::Untouched;
    }
    if logsink_fallback {
        return MarkVerdict::Keep("logsink_fallback");
    }
    let Some(first) = first else {
        return MarkVerdict::Keep("no_exit_origin");
    };
    match first {
        ExitOrigin::KanadeStopped(KanadeStopCause::Fault(_)) => return MarkVerdict::Keep("fault"),
        ExitOrigin::GhostFallbackFailed(_) => return MarkVerdict::Keep("switch_fatal"),
        ExitOrigin::KanadeStopped(
            KanadeStopCause::Quit
            | KanadeStopCause::Forced
            | KanadeStopCause::CloseSilent
            | KanadeStopCause::DeadlineExceeded,
        )
        | ExitOrigin::Escape
        | ExitOrigin::Smoke
        | ExitOrigin::OsClose
        | ExitOrigin::SessionEnd
        | ExitOrigin::MemoryWriteFailed(_) => {}
    }
    if !end.run_ok {
        return MarkVerdict::Keep("run_failed");
    }
    if end.shiori_cut {
        return MarkVerdict::Keep("session_end_deadline");
    }
    if !end.down_ok {
        return MarkVerdict::Keep("down_failed");
    }
    MarkVerdict::Clear
}

/// 降ろした後の印の始末: 判定して、消すなら消し（`info!(session_mark_cleared)`）、残すなら理由つきの
/// `info!(session_mark_kept)`（`boot_resolve::keep_session_mark`）、argv なら `debug!` だけ。
/// 呼ぶのはゴーストの実行系が 0 の時点だけ。印を消せなかった失敗は組の 2 つ目で返す（記録済み）。
pub fn settle_session_mark(
    mark: &MarkInputs,
    end: Teardown,
) -> (MarkVerdict, Option<areka_sylphya::PersistFailure>) {
    use crate::boot_resolve;
    let verdict = session_mark_verdict(
        mark.first.as_ref(),
        mark.argv_session,
        mark.logsink_fallback,
        end,
    );
    let failed = match verdict {
        MarkVerdict::Clear => boot_resolve::clear_session_mark(&mark.app_profile_dir).err(),
        MarkVerdict::Keep(reason) => {
            boot_resolve::keep_session_mark(&mark.app_profile_dir, reason, mark.first.as_ref());
            None
        }
        MarkVerdict::Untouched => {
            tracing::debug!(
                event = "session_mark_untouched_argv",
                "[main] argv で始まったプロセスなので起動中の印に触れません"
            );
            None
        }
    };
    (verdict, failed)
}
