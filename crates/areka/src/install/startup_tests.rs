//! 起動の依頼の持ち物と消す書庫の判断のテスト（design「Testing Strategy / 単体」・要件 3.1・3.2・4.3・4.4・9.6）。

use std::path::PathBuf;

use super::*;
use crate::install::judge::{FailureWord, InstalledItem};

fn item(kind: &'static str, place: &str) -> InstalledItem {
    InstalledItem {
        kind,
        name: format!("{kind} name"),
        place: place.to_owned(),
    }
}

fn installed(kind: &'static str, place: &str) -> ArchiveEnd {
    ArchiveEnd::Installed(vec![item(kind, place)])
}

fn ghost(folder: &str) -> ArchiveEnd {
    installed("ghost", &format!(r"C:\areka\ghost\{folder}"))
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

// ---- sources_to_delete ----

#[test]
fn only_installed_of_the_five_ends_is_deleted() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar", r"C:\c.nar", r"C:\d.nar"]);
    let ends = [
        ghost("a"),
        ArchiveEnd::Refused,
        ArchiveEnd::Declined,
        ArchiveEnd::Failed(FailureWord::Extraction),
    ];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

#[test]
fn abandoned_and_unhandled_rest_are_not_deleted() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar", r"C:\c.nar"]);
    let ends = [
        installed("shell", r"C:\areka\ghost\x\shell\s"),
        ArchiveEnd::Abandoned,
    ];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

#[test]
fn nothing_installed_deletes_nothing() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar"]);
    let ends = [ArchiveEnd::Refused, ArchiveEnd::Failed(FailureWord::FileIo)];
    assert!(sources_to_delete(&archives, &ends).is_empty());
}

#[test]
fn same_path_in_other_case_is_deleted_once_with_first_spelling() {
    // 片方だけが入った（1 回目は失敗・2 回目が入った）でも、最初の綴りで 1 回だけ。
    let archives = paths(&[r"C:\Dir\A.nar", r"C:\b.nar", r"c:\dir\a.NAR"]);
    let ends = [
        ArchiveEnd::Failed(FailureWord::Unsupported),
        ghost("b"),
        ghost("a"),
    ];
    assert_eq!(
        sources_to_delete(&archives, &ends),
        paths(&[r"C:\Dir\A.nar", r"C:\b.nar"])
    );
}

#[test]
fn same_path_installed_twice_is_deleted_once() {
    let archives = paths(&[r"C:\a.nar", r"C:\A.NAR"]);
    let ends = [ghost("a"), ghost("a")];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

// ---- 持ち物を作る・出さずに終わる（タスク 3.2） ----

fn args(archives: &[&str]) -> InstallArgs {
    InstallArgs {
        archives: paths(archives),
        call_ghost: true,
        delete_source: true,
    }
}

fn not_submitted(events: &[log_capture_kit::CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("startup_install_not_submitted"))
        .map(|e| {
            assert_eq!(e.level, tracing::Level::WARN);
            e.field_str("reason").unwrap_or_default().to_owned()
        })
        .collect()
}

#[test]
fn new_makes_relative_paths_absolute_from_the_current_folder() {
    let absolute = std::env::current_dir()
        .expect("今のフォルダ")
        .join("abs.nar");
    let (startup, events) = log_capture_kit::capture(|| {
        StartupInstall::new_with(
            InstallArgs {
                archives: vec![PathBuf::from("rel.nar"), absolute.clone(), PathBuf::new()],
                ..Default::default()
            },
            None,
        )
    });
    let pending = startup.pending.as_ref().expect("据えた");
    let cwd = std::env::current_dir().expect("今のフォルダ");
    assert_eq!(
        pending.archives,
        vec![cwd.join("rel.nar"), absolute, PathBuf::new()],
        "相対は今のフォルダから・絶対はそのまま・直せない空は綴りのまま"
    );
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("install_path_not_absolute"))
        .collect();
    assert_eq!(warns.len(), 1, "直せない 1 本だけ warn");
    assert_eq!(warns[0].level, tracing::Level::WARN);
    assert_eq!(warns[0].field("archive"), Some(""), "直せない綴りを載せる");
    assert!(warns[0].field("error").is_some(), "理由を載せる");
    std::mem::forget(startup); // 落としたときの記録はこのテストの外
}

#[test]
fn dropping_unsubmitted_startup_warns_boot_stopped_once() {
    let ((), events) =
        log_capture_kit::capture(|| drop(StartupInstall::new_with(args(&[r"C:\a.nar"]), None)));
    assert_eq!(not_submitted(&events), ["boot_stopped"]);
    let warn = startup_events(&events, "startup_install_not_submitted");
    assert_eq!(
        warn[0].field("archives"),
        Some(format!("{:?}", paths(&[r"C:\a.nar"])).as_str())
    );
    assert!(warn[0].field("initial_install").is_some());
}

#[test]
fn dropping_with_only_initial_install_warns_and_nothing_to_install_is_silent() {
    let tmp = temp_path_kit::TempPath::under_target("startup-install-drop-initial");
    let dir = tmp.child("exe");
    std::fs::create_dir_all(&dir).expect("exe の代わり");
    let ((), silent) = log_capture_kit::capture(|| {
        drop(StartupInstall::new_with(
            InstallArgs::default(),
            Some(dir.clone()),
        ))
    });
    assert!(
        not_submitted(&silent).is_empty(),
        "入れる物が無ければ出さない"
    );
    std::fs::write(dir.join("Initial_Install.NAR"), b"nar").expect("書庫の代わり");
    let ((), events) = log_capture_kit::capture(|| {
        drop(StartupInstall::new_with(
            InstallArgs::default(),
            Some(dir.clone()),
        ))
    });
    assert_eq!(
        not_submitted(&events),
        ["boot_stopped"],
        "大小を問わず exe の隣を見る"
    );
}

fn startup_events<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    name: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}
