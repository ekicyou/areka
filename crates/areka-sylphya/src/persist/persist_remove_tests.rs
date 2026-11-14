//! 記憶の鍵を消す口（[`remove_scope_keys`]）の檻（audit-placement-core 指摘 17・要件 6.6・9.2）。
//!
//! - 消すのは名指しの鍵だけ。知らない表・知らない値・ほかの鍵は残り、空になった表は消える。
//! - 錠が取れない・commit の失敗は `save_scope` と同じ道で ERROR＋`Degraded` になり、書かない。

use super::lock_tests::LockRefused;
use super::*;
use crate::test_log_capture::capture;
use std::sync::atomic::Ordering;

fn ghost_roots() -> ScopeRoots {
    ScopeRoots {
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    }
}

fn ghost_file() -> PathBuf {
    PathBuf::from("/g").join(PERSIST_FILE_NAME)
}

fn offset(scope: u32, axis: Axis) -> PersistKey {
    PersistKey::BalloonOffset { scope, axis }
}

fn both_offsets(scopes: &[u32]) -> Vec<PersistKey> {
    scopes
        .iter()
        .flat_map(|&s| [offset(s, Axis::X), offset(s, Axis::Y)])
        .collect()
}

const SEEDED: &str = r#"format-version = 1

[window."0"]
x = "10"
y = "20"

[balloon-offset."0"]
x = "5"
y = "6"

[balloon-offset."1"]
x = "7"
y = "8"
note = "kept"

[future-table]
a = "b"
"#;

fn seeded_io() -> FakePersistIo {
    let io = FakePersistIo::new();
    io.commit(&ghost_file(), SEEDED).expect("種を置く");
    io
}

fn stored_table(io: &FakePersistIo) -> toml::Table {
    io.read(&ghost_file())
        .unwrap()
        .expect("記憶のファイルがある")
        .parse()
        .expect("TOML として読める")
}

#[test]
fn removing_keys_keeps_unknown_tables_and_values_and_drops_emptied_tables() {
    let io = seeded_io();

    let outcome = remove_scope_keys(
        PersistScope::Ghost,
        &ghost_roots(),
        &io,
        both_offsets(&[0, 1]),
    );

    assert_eq!(outcome, PersistOutcome::Saved);
    let table = stored_table(&io);
    let offsets = table["balloon-offset"]
        .as_table()
        .expect("知らない値が残る表");
    assert!(
        !offsets.contains_key("0"),
        "空になった表は消える: {table:?}"
    );
    assert_eq!(
        offsets["1"].as_table().unwrap().clone(),
        toml::toml! { note = "kept" },
        "知らない値は残り、名指しの鍵だけ消える"
    );
    assert_eq!(
        table["future-table"].as_table().unwrap().clone(),
        toml::toml! { a = "b" }
    );
    assert_eq!(
        load_scope(PersistScope::Ghost, &ghost_roots(), &io),
        vec![
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X
                },
                "10".to_string()
            ),
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::Y
                },
                "20".to_string()
            ),
        ],
        "ほかの鍵は残る"
    );
}

#[test]
fn removing_one_axis_keeps_the_other_and_the_last_key_drops_the_whole_table() {
    let io = seeded_io();

    let one_axis = remove_scope_keys(
        PersistScope::Ghost,
        &ghost_roots(),
        &io,
        vec![PersistKey::WindowPos {
            scope: 0,
            axis: Axis::X,
        }],
    );
    let all_offsets = {
        // 知らない値の無いファイルで、表の最後の鍵を消すと表ごと消える。
        let io2 = FakePersistIo::new();
        io2.commit(
            &ghost_file(),
            "format-version = 1\n[balloon-offset.\"0\"]\nx = \"5\"\ny = \"6\"\n[boot]\ncount = \"3\"\n",
        )
        .unwrap();
        let outcome = remove_scope_keys(
            PersistScope::Ghost,
            &ghost_roots(),
            &io2,
            both_offsets(&[0]),
        );
        (outcome, stored_table(&io2))
    };

    assert_eq!(one_axis, PersistOutcome::Saved);
    assert_eq!(
        stored_table(&io)["window"]["0"].as_table().unwrap().clone(),
        toml::toml! { y = "20" }
    );
    assert_eq!(all_offsets.0, PersistOutcome::Saved);
    assert!(
        !all_offsets.1.contains_key("balloon-offset"),
        "空になった [balloon-offset] は書かない: {:?}",
        all_offsets.1
    );
    assert_eq!(all_offsets.1["boot"]["count"].as_str(), Some("3"));
}

#[test]
fn a_lock_that_cannot_be_taken_degrades_the_removal_and_writes_nothing() {
    let io = LockRefused::default();
    let mut outcome = None;
    let events = capture(|| {
        outcome = Some(remove_scope_keys(
            PersistScope::Ghost,
            &ghost_roots(),
            &io,
            both_offsets(&[0]),
        ))
    });

    let outcome = outcome.unwrap();
    assert!(
        matches!(&outcome, PersistOutcome::Degraded(f)
            if f.keys == both_offsets(&[0])
                && matches!(&f.cause, PersistFailureCause::Io(r) if r.contains("injected lock failure"))),
        "錠が取れない消しは Degraded になる: {outcome:?}"
    );
    assert_eq!(io.commits.load(Ordering::SeqCst), 0, "書き込みは呼ばれない");
    assert_eq!(
        events
            .iter()
            .filter(|e| e.level == tracing::Level::ERROR
                && e.message().contains("persist update lock not acquired"))
            .count(),
        1,
        "ERROR persist_lock_failed が 1 件: {events:?}"
    );
}

#[test]
fn a_failed_commit_degrades_the_removal_and_leaves_the_file_intact() {
    let io = seeded_io();
    io.fail_next_commit();
    let mut outcome = None;
    let events = capture(|| {
        outcome = Some(remove_scope_keys(
            PersistScope::Ghost,
            &ghost_roots(),
            &io,
            both_offsets(&[0]),
        ))
    });

    let outcome = outcome.unwrap();
    assert!(
        matches!(&outcome, PersistOutcome::Degraded(f)
            if f.keys == both_offsets(&[0]) && matches!(f.cause, PersistFailureCause::Io(_))),
        "commit の失敗は Degraded になる: {outcome:?}"
    );
    assert_eq!(
        io.read(&ghost_file()).unwrap().as_deref(),
        Some(SEEDED),
        "既存は無傷"
    );
    assert!(
        events
            .iter()
            .any(|e| e.level == tracing::Level::ERROR
                && e.message().contains("persist commit failed")),
        "ERROR persist commit failed が出る: {events:?}"
    );
}
