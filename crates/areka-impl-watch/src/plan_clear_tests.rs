//! リポジトリの分だけ記録を消す判断（`clear --repo`・要件 11）の決定論テスト。時刻と生死は引数で渡し、
//! 眠らない。

use super::test_support::{
    REPO, T0, hold_load, hold_merge, queue_merge, queue_request, run, state_with, stop_for,
    test_freed, test_freed_record, wait_record,
};
use super::{Command, Event, Verdict};
use crate::state::{DeskSet, LastMerge, Recent, RecentKind, State, WaitKind};

/// ほかのリポジトリの名前。
const OTHER: &str = "other";

fn clear_repo(repo: &str) -> Command {
    Command::ClearRepo {
        repo: repo.to_owned(),
    }
}

fn left(id: &str) -> Event {
    Event::Left {
        id: id.to_owned(),
        why: "leave",
    }
}

fn cancelled(id: &str) -> Event {
    Event::Cancelled { id: id.to_owned() }
}

fn cleared(id: &str, at: u64) -> Recent {
    Recent {
        at,
        kind: RecentKind::Cleared,
        id: Some(id.to_owned()),
        detail: format!("repo={REPO}"),
    }
}

fn last_merge() -> LastMerge {
    LastMerge {
        pr: "281".to_owned(),
        sha: "414d43eb".to_owned(),
        spec: "spec-done".to_owned(),
        at: T0,
    }
}

/// 参加者のリポジトリを替える。
fn move_to(state: &mut State, id: &str, repo: &str) {
    state.participants.get_mut(id).expect("参加者が居る").repo = repo.to_owned();
}

/// 2 つのリポジトリの参加者が混ざった状態:
/// - `a`・`b`・`x` は消すリポジトリ（[`REPO`]）、`y`・`z` は [`OTHER`]。
/// - `x` が負荷テスト（排他テストの机とひとり占め）を持ち、`b` と `y` を止めている（`x` の申し込みの
///   待ちの記録もある）。止まっている `b` は排他テストの机を申し込んで並んでいる。
/// - `a` と `z` は [`OTHER`] のマージの机に並んでいる（並んでいる者は止まらない）。
/// - [`REPO`] のマージの机には、参加者の記録の無い持ち主 `ghost` と待ち行列の `ghost2`、直前の
///   マージの記録が残っている。
fn mixed() -> State {
    let mut state = state_with(&["a", "b", "x", "y", "z"]);
    move_to(&mut state, "y", OTHER);
    move_to(&mut state, "z", OTHER);
    hold_load(&mut state, "x", &["b", "y"]);
    stop_for(&mut state, "b", "x");
    stop_for(&mut state, "y", "x");
    queue_request(&mut state, "b", DeskSet::Test, T0);
    state.waits.push(wait_record("x", WaitKind::Request));
    queue_merge(&mut state, OTHER, "a", T0);
    queue_merge(&mut state, OTHER, "z", T0 + 1);
    state.waits.push(wait_record("z", WaitKind::Merge));
    hold_merge(&mut state, REPO, "ghost");
    queue_merge(&mut state, REPO, "ghost2", T0);
    state.merge.get_mut(REPO).expect("いま置いた").last = Some(last_merge());
    state
}

#[test]
fn clear_repo_leaves_the_repo_and_empties_its_desk_and_resumes_the_stopped() {
    let mut state = mixed();
    let now = T0 + 10;
    let got = run(&mut state, &clear_repo(REPO), None, now);

    assert!(got.changed);
    assert_eq!(got.verdict, Verdict::Applied);
    // 外れ方は `leave` と同じで昇順に 1 人 1 件（排他テストの机は訳 `left` で空く）→ 机の残りの
    // 取り消し（持ち主 → 待ち行列）→ ひとり占めが空いたので止められていた者の再開 → ほかの
    // リポジトリのマージの番（要件 6.4）。
    assert_eq!(
        got.events,
        vec![
            left("a"),
            left("b"),
            test_freed("x", "left"),
            left("x"),
            cancelled("ghost"),
            cancelled("ghost2"),
            Event::Resumed {
                id: "y".to_owned(),
                why: "no-sole",
            },
            Event::MergeGranted {
                repo: OTHER.to_owned(),
                id: "z".to_owned(),
            },
        ]
    );

    // 消したリポジトリの参加者・申し込み・机・待ちの記録は残らない。
    let ids: Vec<&str> = state.participants.keys().map(String::as_str).collect();
    assert_eq!(ids, ["y", "z"]);
    assert!(state.test.is_none());
    assert!(state.sole.is_none());
    assert!(state.requests.is_empty());
    assert!(
        state
            .waits
            .iter()
            .all(|wait| !["a", "b", "x"].contains(&wait.id.as_str())),
        "{:?}",
        state.waits
    );

    // 最近の出来事は今ある綴り `cleared`＋識別＋`repo=<名前>` と、排他テストの机の空き（訳 `left`）
    // （新しいものが前）。
    assert_eq!(
        state.recent,
        vec![
            cleared("x", now),
            test_freed_record("x", "left", now),
            cleared("b", now),
            cleared("a", now),
        ]
    );

    // 消したリポジトリの机は空になり、直前のマージの記録は残る。
    let desk = &state.merge[REPO];
    assert!(desk.holder.is_none());
    assert!(desk.queue.is_empty());
    assert_eq!(desk.last, Some(last_merge()));

    // 止められていた `y` は作業中へ戻り、ほかのリポジトリの机・参加者・待ちの記録はそのまま。
    let y = &state.participants["y"];
    assert_eq!(y.status, crate::state::ParticipantStatus::Working);
    assert!(y.stop_reason.is_none());
    let other = &state.merge[OTHER];
    assert_eq!(other.holder.as_ref().map(|h| h.id.as_str()), Some("z"));
    assert!(other.queue.is_empty());
    assert!(state.waits.contains(&wait_record("y", WaitKind::Watch)));
    assert!(state.waits.contains(&wait_record("z", WaitKind::Watch)));
    assert!(state.waits.contains(&wait_record("z", WaitKind::Merge)));
}

#[test]
fn clear_repo_keeps_the_load_test_desks_of_another_repo() {
    // `y`（ほかのリポジトリ）の負荷テストが `x`（消すリポジトリ）を止めている。
    let mut state = state_with(&["x", "y"]);
    move_to(&mut state, "y", OTHER);
    hold_load(&mut state, "y", &["x"]);
    stop_for(&mut state, "x", "y");
    state.waits.push(wait_record("y", WaitKind::Request));
    let before = (state.test.clone(), state.sole.clone());

    let got = run(&mut state, &clear_repo(REPO), None, T0 + 10);

    assert_eq!(got.events, vec![left("x")]);
    assert_eq!((state.test.clone(), state.sole.clone()), before);
    assert!(state.waits.contains(&wait_record("y", WaitKind::Request)));
    assert!(state.participants.contains_key("y"));
    assert!(!state.participants.contains_key("x"));
}

#[test]
fn clear_repo_with_nothing_to_clear_applies_and_creates_no_desk() {
    let mut state = state_with(&["y"]);
    move_to(&mut state, "y", OTHER);
    let before = state.clone();

    let got = run(&mut state, &clear_repo(REPO), None, T0 + 10);

    assert!(!got.changed);
    assert_eq!(got.verdict, Verdict::Applied);
    assert!(got.events.is_empty(), "{:?}", got.events);
    assert_eq!(state, before);
    assert!(!state.merge.contains_key(REPO));
}
