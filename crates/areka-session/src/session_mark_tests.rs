//! 起動中の印の判定の表・判定の始末・印を消せないときの告知のテスト。

use temp_path_kit::TempPath;

use super::*;
use crate::app_exit::ExitOrigin;
use crate::boot_resolve::{read_session_mark, write_session_mark};
use crate::session_end::test_support::fault;
use areka_kanade::{KanadeStopCause, ShioriFaultKind};
use areka_sylphya::{PersistKey, PersistScope};
use log_capture_kit::capture;

fn kanade_fault() -> ExitOrigin {
    ExitOrigin::KanadeStopped(KanadeStopCause::Fault(fault(
        ShioriFaultKind::ConnectFailed,
    )))
}

/// 降ろした結果の組（メッセージループの成否・降ろす処理の成否・上限の打ち切りの有無）。
fn end(run_ok: bool, down_ok: bool, shiori_cut: bool) -> Teardown {
    Teardown {
        run_ok,
        down_ok,
        shiori_cut,
    }
}

/// 時系列で最初の理由を採る。上から順に: argv のセッションは触らない → LogSink へ倒れた →
/// 出所なし → 失敗の出所 → `run()` の失敗 → 上限の打ち切り → 降ろす処理の失敗 → それ以外は消す。
/// 判定は名前を見ないので、倒れたのが既定ゴースト自身でも同じ行で印が残る。
#[test]
fn session_mark_verdict_table() {
    use KanadeStopCause::{CloseSilent, DeadlineExceeded, Forced, Quit};
    let fatal = ExitOrigin::GhostFallbackFailed(fault(ShioriFaultKind::Internal));
    let clean = [
        ExitOrigin::KanadeStopped(Quit),
        ExitOrigin::KanadeStopped(Forced),
        ExitOrigin::KanadeStopped(CloseSilent),
        ExitOrigin::KanadeStopped(DeadlineExceeded),
        ExitOrigin::Escape,
        ExitOrigin::Smoke,
        ExitOrigin::OsClose,
        ExitOrigin::SessionEnd,
        // 記憶を書けなかった終わりも降ろす手順は正規の道を通る＝消す側。
        ExitOrigin::MemoryWriteFailed(areka_sylphya::PersistFailure {
            scope: PersistScope::App,
            dir: None,
            keys: vec![PersistKey::LastRunning],
            cause: areka_sylphya::PersistFailureCause::NoRoot,
        }),
    ];
    let quit = ExitOrigin::KanadeStopped(Quit);
    let session_end = ExitOrigin::SessionEnd;
    let ok = end(true, true, false);
    let v = session_mark_verdict;

    // 倒れていない・打ち切りなし。
    let mut got: Vec<MarkVerdict> = clean.iter().map(|o| v(Some(o), false, false, ok)).collect();
    let mut want = vec![MarkVerdict::Clear; clean.len()];
    let others = [
        (
            v(Some(&kanade_fault()), false, false, ok),
            MarkVerdict::Keep("fault"),
        ),
        (
            v(Some(&fatal), false, false, ok),
            MarkVerdict::Keep("switch_fatal"),
        ),
        (
            v(None, false, false, ok),
            MarkVerdict::Keep("no_exit_origin"),
        ),
        (
            v(Some(&quit), false, false, end(false, true, false)),
            MarkVerdict::Keep("run_failed"),
        ),
        (
            v(Some(&quit), false, false, end(true, false, false)),
            MarkVerdict::Keep("down_failed"),
        ),
        // 失敗の出所は run() と降ろす処理の失敗より先に理由になる。
        (
            v(
                Some(&kanade_fault()),
                false,
                false,
                end(false, false, false),
            ),
            MarkVerdict::Keep("fault"),
        ),
        // argv で始まったプロセスは出所・成否によらず触らない（切替の後の致命も）。
        (v(Some(&quit), true, false, ok), MarkVerdict::Untouched),
        (v(Some(&fatal), true, false, ok), MarkVerdict::Untouched),
        (
            v(None, true, false, end(false, false, false)),
            MarkVerdict::Untouched,
        ),
    ];

    // LogSink へ倒れた × きれいな出所 × 成否（打ち切りの有無も）: 常に倒れた語で残す。
    let outcomes: Vec<Teardown> = [false, true]
        .into_iter()
        .flat_map(|r| [false, true].into_iter().map(move |d| (r, d)))
        .flat_map(|(r, d)| [false, true].into_iter().map(move |c| end(r, d, c)))
        .collect();
    let mut fallback_clean = Vec::new();
    for o in &clean {
        for e in &outcomes {
            fallback_clean.push((
                v(Some(o), false, true, *e),
                MarkVerdict::Keep("logsink_fallback"),
            ));
        }
    }
    // LogSink へ倒れた × 他の理由: 倒れたのが時系列で最初なので倒れた語が勝つ。
    let fallback_other = [
        (
            v(Some(&kanade_fault()), false, true, ok),
            MarkVerdict::Keep("logsink_fallback"),
        ),
        (
            v(Some(&fatal), false, true, ok),
            MarkVerdict::Keep("logsink_fallback"),
        ),
        (
            v(None, false, true, ok),
            MarkVerdict::Keep("logsink_fallback"),
        ),
        (
            v(Some(&quit), false, true, end(false, true, false)),
            MarkVerdict::Keep("logsink_fallback"),
        ),
        (
            v(Some(&quit), false, true, end(true, false, false)),
            MarkVerdict::Keep("logsink_fallback"),
        ),
        (
            v(Some(&session_end), false, true, end(true, true, true)),
            MarkVerdict::Keep("logsink_fallback"),
        ),
        // LogSink へ倒れた × argv: argv の上書きが先に効き、触らない。
        (v(Some(&quit), true, true, ok), MarkVerdict::Untouched),
        (
            v(Some(&kanade_fault()), true, true, ok),
            MarkVerdict::Untouched,
        ),
        (
            v(None, true, true, end(false, false, true)),
            MarkVerdict::Untouched,
        ),
    ];
    // 上限の打ち切り × 各組み合わせ。
    let cut = [
        (
            v(Some(&session_end), false, false, end(true, true, true)),
            MarkVerdict::Keep("session_end_deadline"),
        ),
        // 打ち切りは降ろす処理の失敗より先（打ち切りが原因で降ろす処理が失敗しても語は上限）。
        (
            v(Some(&session_end), false, false, end(true, false, true)),
            MarkVerdict::Keep("session_end_deadline"),
        ),
        // 失敗の出所・出所なし・run() の失敗は打ち切りより先に起きている。
        (
            v(Some(&kanade_fault()), false, false, end(true, true, true)),
            MarkVerdict::Keep("fault"),
        ),
        (
            v(Some(&fatal), false, false, end(true, true, true)),
            MarkVerdict::Keep("switch_fatal"),
        ),
        (
            v(None, false, false, end(true, true, true)),
            MarkVerdict::Keep("no_exit_origin"),
        ),
        (
            v(Some(&session_end), false, false, end(false, true, true)),
            MarkVerdict::Keep("run_failed"),
        ),
        // 打ち切り × argv: 触らない。
        (
            v(Some(&session_end), true, false, end(true, true, true)),
            MarkVerdict::Untouched,
        ),
        // 打ち切りが無ければ OS のセッションの終了は消す／降ろす処理の失敗で残す。
        (v(Some(&session_end), false, false, ok), MarkVerdict::Clear),
        (
            v(Some(&session_end), false, false, end(true, false, false)),
            MarkVerdict::Keep("down_failed"),
        ),
    ];
    for (g, w) in others
        .into_iter()
        .chain(fallback_clean)
        .chain(fallback_other)
        .chain(cut)
    {
        got.push(g);
        want.push(w);
    }
    assert_eq!(got, want);
}

/// 判定の始末: 消すなら印が消え、残すなら理由つきの `info!(session_mark_kept)`、触らないなら
/// `debug!(session_mark_untouched_argv)`。どれも記憶の置き場の印をそれ以外に変えない。
#[test]
fn settle_session_mark_applies_the_verdict() {
    let tmp = TempPath::new("mark-settle");
    let mut outcomes = Vec::new();
    for (argv_session, first, level, event) in [
        (
            false,
            Some(ExitOrigin::Smoke),
            tracing::Level::INFO,
            "session_mark_cleared",
        ),
        (
            false,
            Some(kanade_fault()),
            tracing::Level::INFO,
            "session_mark_kept",
        ),
        (
            true,
            Some(ExitOrigin::Smoke),
            tracing::Level::DEBUG,
            "session_mark_untouched_argv",
        ),
    ] {
        let app_profile_dir = tmp.child(event);
        let _ = write_session_mark(&app_profile_dir, "A");
        let mark = MarkInputs {
            app_profile_dir: app_profile_dir.clone(),
            argv_session,
            first,
            logsink_fallback: false,
        };
        let (verdict, events) = capture(|| settle_session_mark(&mark, end(true, true, false)).0);
        let logged = events
            .iter()
            .filter(|e| e.level == level && e.field_str("event") == Some(event))
            .count();
        let reason = events
            .iter()
            .find(|e| e.field_str("event") == Some("session_mark_kept"))
            .and_then(|e| e.field_str("reason").map(str::to_owned));
        outcomes.push((verdict, logged, reason, read_session_mark(&app_profile_dir)));
    }
    assert_eq!(
        outcomes,
        vec![
            (MarkVerdict::Clear, 1, None, None),
            (
                MarkVerdict::Keep("fault"),
                1,
                Some("fault".to_owned()),
                Some("A".to_owned())
            ),
            (MarkVerdict::Untouched, 1, None, Some("A".to_owned())),
        ],
        "判定の始末が崩れた（判定・記録の件数・残す理由・印）"
    );
}

/// 終わるときに印を消せない → 後始末の 2 つ目が失敗・途中の告知が 1 件で `Err`。
#[test]
fn clear_mark_unwritable_tells_running_once() {
    let tmp = TempPath::under_target("mark-write-clear");
    let app = tmp.child("app-profile");
    std::fs::write(&app, "x").expect("置き場の位置に普通のファイルを置く");
    let (verdict, failed) = settle_session_mark(
        &MarkInputs {
            app_profile_dir: app,
            argv_session: false,
            first: Some(ExitOrigin::Escape),
            logsink_fallback: false,
        },
        Teardown {
            run_ok: true,
            down_ok: true,
            shiori_cut: false,
        },
    );
    assert_eq!(verdict, MarkVerdict::Clear);
    let failed = failed.expect("印を消せない失敗が返る");
    let (told, events) = log_capture_kit::capture(|| {
        crate::write_failure::tell_at_close(None, Some(failed), None, true)
    });
    let alerts: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("alert"))
        .collect();
    assert_eq!(
        (told.is_err(), alerts.len()),
        (true, 1),
        "(終了コード 1・告知の数): {events:?}"
    );
    let scene = alerts[0].field("scene").unwrap_or_default();
    assert!(
        scene.contains("Running") && scene.contains("LastRunning"),
        "途中の告知で、起動中の印: {scene}"
    );
}
