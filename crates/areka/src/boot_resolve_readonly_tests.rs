//! 読み取りだけの走りの、起動中の印の 3 つの書き手と後始末の記録（readonly-mode 要件 2.6・3.1・3.2・6.1・6.3・11.1・11.4）。
//!
//! プロセスに 1 つの状態は `withhold_everything` を呼ばず、試験ごとの根に立てた
//! `WithholdGuard` だけで控える。根はワークツリーの `target\` の下だけ。

use std::path::Path;

use areka_sylphya::persist::withhold::WithholdGuard;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;

use super::*;

/// `event` の記録の件数（`event` 欄で引く）。
fn count(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// 置き場の木（相対パスと中身）。
fn tree(dir: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries {
            let p = e.expect("置き場の項目").path();
            let content = std::fs::read(&p).unwrap_or_default();
            out.push((p.strip_prefix(dir).unwrap().to_path_buf(), content));
        }
    }
    out.sort();
    out
}

/// 根の取っ手の下で、印の 3 関数は成功を返し、書いたときの記録を出さず、控えたときの記録を出し、
/// 置き場に何も作らない（要件 2.6・3.1・6.1・11.4）。
#[test]
fn three_mark_writers_withhold_under_the_guard() {
    let tmp = TempPath::under_target("mark-readonly-writers");
    let app = tmp.child("app");
    let _guard = WithholdGuard::under(&app);

    let (results, events) = capture(|| {
        (
            write_session_mark(&app, "動いている子"),
            write_switch_drop(&app, Some("切替先")),
            clear_session_mark(&app),
        )
    });
    assert!(
        results.0.is_ok() && results.1.is_ok() && results.2.is_ok(),
        "{results:?}"
    );
    let counts: Vec<(&str, usize)> = [
        "session_mark_written",
        "switch_drop_recorded",
        "session_mark_cleared",
        "session_mark_withheld",
        "switch_drop_withheld",
        "session_mark_clear_withheld",
    ]
    .into_iter()
    .map(|e| (e, count(&events, e)))
    .collect();
    assert_eq!(
        counts,
        vec![
            ("session_mark_written", 0),
            ("switch_drop_recorded", 0),
            ("session_mark_cleared", 0),
            ("session_mark_withheld", 1),
            ("switch_drop_withheld", 1),
            ("session_mark_clear_withheld", 1),
        ]
    );
    assert!(!app.exists(), "置き場は作られない");
}

/// 前のふつうの走りの印を植えると、読み手がその印を読み、後始末の後もファイルの印が同じ
/// （要件 3.1・3.2・11.1）。
#[test]
fn planted_mark_is_read_and_file_stays_after_cleanup() {
    let tmp = TempPath::under_target("mark-readonly-planted");
    let app = tmp.child("app");
    write_session_mark(&app, "前の子").expect("ふつうの走りで印を書く");
    let before = tree(&app);

    let _guard = WithholdGuard::under(&app);
    assert_eq!(read_session_mark(&app), Some("前の子".to_owned()));
    write_session_mark(&app, "今の子").expect("控える");
    write_switch_drop(&app, Some("切替先")).expect("控える");
    clear_session_mark(&app).expect("控える");
    keep_session_mark(&app, "run_failed", None);
    drop(_guard);

    assert_eq!(tree(&app), before, "置き場の木と中身は前のまま");
    assert_eq!(read_session_mark(&app), Some("前の子".to_owned()));
}

/// きれいに終わらなかった後始末は、読み取りだけの文を `readonly = true` で 1 件だけ出し、
/// 控えの印で次の起動を予告しない（要件 6.3）。
#[test]
fn keep_mark_says_readonly_without_foretelling() {
    let tmp = TempPath::under_target("mark-readonly-keep");
    let app = tmp.child("app");
    let _guard = WithholdGuard::under(&app);
    write_session_mark(&app, "動いている子").expect("控える");

    let ((), events) = capture(|| keep_session_mark(&app, "run_failed", None));
    let kept: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("session_mark_kept"))
        .collect();
    assert_eq!(kept.len(), 1, "{events:?}");
    assert_eq!(kept[0].field("readonly"), Some("true"));
    let text = kept[0].message();
    assert!(text.contains("読み取りだけ"), "{text}");
    assert!(!text.contains("Ref6/7 付き"), "{text}");
}
