//! 記憶の鍵を消す口（[`remove_scope_keys`]）のテスト。
//!
//! - 消すのは名指しの鍵だけ。知らない表・知らない値・ほかの鍵は残り、空になった表は消える。
//! - 錠が取れない・commit の失敗は `save_scope` と同じ道で ERROR＋`Degraded` になり、書かない。

use super::lock_tests::LockRefused;
use super::unreadable_tests::{ghost_file, ghost_roots};
use super::*;
use crate::test_log_capture::capture;
use std::sync::atomic::Ordering;

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

/// 鍵の種類の番号。`_` の腕を置かないので、種類が増えるとここが組めなくなる
/// （下のテストの並びへもその種類を足す合図）。
fn kind(key: PersistKey) -> usize {
    match key {
        PersistKey::WindowPos { .. } => 0,
        PersistKey::BalloonOffset { .. } => 1,
        PersistKey::WindowAdjacent { .. } => 2,
        PersistKey::BootCount => 3,
        PersistKey::VanishCount => 4,
        PersistKey::LastGhost => 5,
        PersistKey::LastBalloon => 6,
        PersistKey::LastShell => 7,
        PersistKey::LastRunning => 8,
        PersistKey::BasewareRoots => 9,
    }
}
/// [`kind`] の腕の数。
const KINDS: usize = 10;

/// どの鍵も、保存してから消すとファイルに何も残らない（消しの置き場が保存の置き場と同じ）。
#[test]
fn every_key_saved_then_removed_leaves_nothing_in_the_file() {
    let keys = [
        PersistKey::WindowPos {
            scope: 2,
            axis: Axis::X,
        },
        PersistKey::WindowPos {
            scope: 2,
            axis: Axis::Y,
        },
        offset(2, Axis::X),
        offset(2, Axis::Y),
        PersistKey::WindowAdjacent { scope: 2 },
        PersistKey::BootCount,
        PersistKey::VanishCount,
        PersistKey::LastGhost,
        PersistKey::LastBalloon,
        PersistKey::LastShell,
        PersistKey::LastRunning,
        PersistKey::BasewareRoots,
    ];
    let mut listed: Vec<usize> = keys.iter().map(|&key| kind(key)).collect();
    listed.dedup();
    assert_eq!(
        listed,
        (0..KINDS).collect::<Vec<_>>(),
        "並びが鍵のどの種類も、番号の順に持つ"
    );
    for key in keys {
        let io = FakePersistIo::new();
        let saved = save_scope(
            PersistScope::Ghost,
            &ghost_roots(),
            &io,
            vec![(key, "v".to_string())],
        );
        assert_eq!(saved, PersistOutcome::Saved, "{key:?}");
        assert_eq!(
            load_scope(PersistScope::Ghost, &ghost_roots(), &io),
            vec![(key, "v".to_string())],
            "{key:?}: 保存した値が読める（消しの前提）"
        );

        let removed = remove_scope_keys(PersistScope::Ghost, &ghost_roots(), &io, vec![key]);

        assert_eq!(removed, PersistOutcome::Saved, "{key:?}");
        assert_eq!(
            load_scope(PersistScope::Ghost, &ghost_roots(), &io),
            vec![],
            "{key:?}: 消した鍵が読める"
        );
        let table = stored_table(&io);
        assert_eq!(
            table.keys().collect::<Vec<_>>(),
            vec!["format-version"],
            "{key:?}: ファイルに残りがある: {table:?}"
        );
    }
}

/// 手で書いた `"00"` のような ID も読みは同じスコープ（`parse::<u32>()`）なので、消しはそれも消す。
#[test]
fn removal_also_drops_hand_written_spellings_of_the_same_scope() {
    let io = FakePersistIo::new();
    io.commit(
        &ghost_file(),
        "format-version = 1\n[balloon-offset.\"00\"]\nx = \"5\"\ny = \"6\"\n[window-adjacent]\n\"00\" = \"left\"\n[window.\"1\"]\nx = \"1\"\ny = \"2\"\n",
    )
    .unwrap();

    let mut keys = both_offsets(&[0]);
    keys.push(PersistKey::WindowAdjacent { scope: 0 });
    let outcome = remove_scope_keys(PersistScope::Ghost, &ghost_roots(), &io, keys);

    assert_eq!(outcome, PersistOutcome::Saved);
    assert_eq!(
        load_scope(PersistScope::Ghost, &ghost_roots(), &io),
        vec![
            (
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::X
                },
                "1".to_string()
            ),
            (
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::Y
                },
                "2".to_string()
            ),
        ],
        "\"00\" の綴りの鍵が残っている: {:?}",
        stored_table(&io)
    );
}

/// 保存も同じ。手で書いた `"00"` の綴りを残すと、正準の `"0"` と並んで古い値が読み戻される。
#[test]
fn saving_also_drops_hand_written_spellings_of_the_same_key() {
    let io = FakePersistIo::new();
    io.commit(
        &ghost_file(),
        "format-version = 1\n[balloon-offset.\"00\"]\nx = \"5\"\ny = \"6\"\n[window-adjacent]\n\"00\" = \"left\"\n",
    )
    .unwrap();

    let adjacent = PersistKey::WindowAdjacent { scope: 0 };
    let outcome = save_scope(
        PersistScope::Ghost,
        &ghost_roots(),
        &io,
        vec![
            (offset(0, Axis::X), "9".to_string()),
            (adjacent, "right".to_string()),
        ],
    );

    assert_eq!(outcome, PersistOutcome::Saved);
    let mut loaded = load_scope(PersistScope::Ghost, &ghost_roots(), &io);
    loaded.sort_by_key(|(key, _)| key.to_canonical_key());
    let mut expected = vec![
        (offset(0, Axis::X), "9".to_string()),
        // 保存しなかった軸は、手で書いた綴りの値がそのまま残る。
        (offset(0, Axis::Y), "6".to_string()),
        (adjacent, "right".to_string()),
    ];
    expected.sort_by_key(|(key, _)| key.to_canonical_key());
    assert_eq!(
        loaded,
        expected,
        "古い綴りの値が残っている: {:?}",
        stored_table(&io)
    );
}
