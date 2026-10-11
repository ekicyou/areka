//! 表示の決定論テスト。いまの時刻と生死の答えは引数で渡す（ファイルも時計も触らない）。

use super::{render_markdown, render_terminal, utc, utc_compact};
use crate::plan::Presence;
use crate::state::{
    LastMerge, LoadHolder, LoadRequest, MergeDesk, MergeHolder, MergeRequest, Participant,
    ParticipantStatus, Recent, RecentKind, State, StopReason, WaitKind, WaitRecord, WatchInfo,
};

/// テストの時刻の起点（2026-10-03T04:00:00Z）。
const T0: u64 = 1_791_000_000;
const T0_UTC: &str = "2026-10-03T04:00:00Z";

/// 偽の生死: 名指しした識別の「見張り」だけが居ない。ほかの種類を尋ねられたら「居る」と答えるので、
/// 見張り以外の印で `absent` を決める実装はここで外れる。
struct WatchAbsent(&'static [&'static str]);

impl Presence for WatchAbsent {
    fn is_present(&self, id: &str, kind: WaitKind) -> bool {
        !(kind == WaitKind::Watch && self.0.contains(&id))
    }
}

const ALL_PRESENT: WatchAbsent = WatchAbsent(&[]);

/// 見張りの記録（人が読む `WatchInfo`）つきの参加者。
fn participant(id: &str, name: &str, repo: &str, status: ParticipantStatus) -> Participant {
    Participant {
        id: id.to_owned(),
        name: name.to_owned(),
        repo: repo.to_owned(),
        status,
        since: T0,
        watch: Some(WatchInfo {
            pid: 1234,
            since: T0,
        }),
        ..Participant::default()
    }
}

fn put(state: &mut State, participant: Participant) {
    state
        .participants
        .insert(participant.id.clone(), participant);
}

fn wait(id: &str, kind: WaitKind, repo: Option<&str>) -> WaitRecord {
    WaitRecord {
        id: id.to_owned(),
        kind,
        repo: repo.map(str::to_owned),
        pid: 1234,
        since: T0,
    }
}

fn merge_request(id: &str, bug: bool, requested: u64) -> MergeRequest {
    MergeRequest {
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug,
        requested,
    }
}

fn load_request(id: &str, requested: u64) -> LoadRequest {
    LoadRequest {
        id: id.to_owned(),
        purpose: "負荷の計測 5 回".to_owned(),
        requested,
    }
}

/// 設計の「状態の確認」の見本の状態。名前と内容は日本語（端末へ出てはならない値）。
/// 生死は [`SAMPLE_PRESENCE`]（C の見張りだけが居ない）。
fn sample() -> State {
    let mut state = State::empty();
    put(
        &mut state,
        participant("A", "あれか実装", "areka", ParticipantStatus::Working),
    );
    let mut b = participant(
        "B",
        "B の見張り役",
        "areka",
        ParticipantStatus::StopRequested,
    );
    b.stop_reason = Some(StopReason {
        by: "C".to_owned(),
        purpose: "負荷の計測 5 回".to_owned(),
    });
    put(&mut state, b);
    put(
        &mut state,
        participant("C", "計測係", "pasta", ParticipantStatus::Working),
    );
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: Some(MergeHolder {
                id: "A".to_owned(),
                spec: "impl-watch-2".to_owned(),
                bug: false,
                requested: T0 + 10,
                granted: T0 + 11,
            }),
            queue: Vec::new(),
            last: Some(LastMerge {
                pr: "281".to_owned(),
                sha: "414d43eb".to_owned(),
                spec: "impl-watch".to_owned(),
                at: 1_791_594_000,
            }),
        },
    );
    state.load.queue.push(load_request("C", T0 + 20));
    state.waits = vec![
        wait("A", WaitKind::Watch, Some("areka")),
        wait("B", WaitKind::Watch, Some("areka")),
        wait("C", WaitKind::Load, None),
    ];
    state.push_recent(Recent {
        at: T0 + 1,
        kind: RecentKind::Recovered,
        id: None,
        detail: "created".to_owned(),
    });
    state.push_recent(Recent {
        at: T0 + 5,
        kind: RecentKind::Reclaimed,
        id: Some("Z".to_owned()),
        detail: "watch absent".to_owned(),
    });
    state
}

const SAMPLE_PRESENCE: WatchAbsent = WatchAbsent(&["C"]);

/// 端末向けの要約のうち、その識別の参加者の行。
fn terminal_line(text: &str, id: &str) -> String {
    let head = format!("{id} ");
    text.lines()
        .find(|line| line.starts_with(&head))
        .unwrap_or_else(|| panic!("{id} の行が在る: {text}"))
        .to_owned()
}

/// 端末向けの要約の 1 行目（数の行）。
fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// 読み物の参加者の表のうち、その識別の行の欄。
fn markdown_row(text: &str, id: &str) -> Vec<String> {
    let head = format!("| {id} |");
    text.lines()
        .find(|line| line.starts_with(&head))
        .unwrap_or_else(|| panic!("{id} の行が在る: {text}"))
        .trim_matches('|')
        .split(" | ")
        .map(|cell| cell.trim().to_owned())
        .collect()
}

#[track_caller]
fn assert_has_line(text: &str, line: &str) {
    assert!(
        text.lines().any(|l| l == line),
        "行が無い: {line}\n----\n{text}"
    );
}

#[test]
fn terminal_summary_renders_the_design_sample() {
    let got = render_terminal(&sample(), &SAMPLE_PRESENCE);
    let want = "\
participants=3 load-holder=none load-queued=1 merging=1 merge-queued=0 not-working=1 waits=3
A areka working
B areka stop-requested
C pasta working absent
merge areka: holder=A queue=0 last=PR#281 414d43eb impl-watch 2026-10-10T01:00:00Z
load: holder=none queue=C
";
    assert_eq!(got, want);
}

#[test]
fn terminal_summary_is_ascii_while_markdown_carries_the_japanese_values() {
    let mut state = sample();
    // 持ち主の内容も日本語にする（見本の机は持ち主が居ない）。
    state.load.holder = Some(LoadHolder {
        id: "D".to_owned(),
        purpose: "走っている計測".to_owned(),
        requested: T0,
        granted: T0 + 1,
        running: true,
        stopped: vec!["A".to_owned(), "B".to_owned()],
    });
    let terminal = render_terminal(&state, &SAMPLE_PRESENCE);
    assert!(terminal.is_ascii(), "{terminal}");
    assert_has_line(&terminal, "load: holder=D queue=C");

    let markdown = render_markdown(&state, &SAMPLE_PRESENCE, T0 + 100);
    for value in [
        "あれか実装",
        "B の見張り役",
        "計測係",
        "負荷の計測 5 回",
        "走っている計測",
    ] {
        assert!(markdown.contains(value), "{value} が読み物に在る");
    }
}

#[test]
fn terminal_summary_escapes_a_non_ascii_id_from_a_hand_edited_state() {
    let mut state = State::empty();
    put(
        &mut state,
        participant("あ", "あ", "倉庫", ParticipantStatus::Working),
    );
    let terminal = render_terminal(&state, &ALL_PRESENT);
    assert!(terminal.is_ascii(), "{terminal}");
    assert_has_line(&terminal, "\\u{3042} \\u{5009}\\u{5eab} working");
}

#[test]
fn marks_follow_the_presence_port_and_the_working_status() {
    use ParticipantStatus::{StopRequested, Stopped, Working};
    // (状態, 見張り待ちの印, 人が読む見張りの記録, 見張りが居るか) → 印
    let cases = [
        (Working, false, true, true, ""),
        // 殺された見張りの記録（WatchInfo）は残る。印は生死の口で決める。
        (Working, false, true, false, "absent"),
        // 記録が無くても、口が「居る」と答えるなら印は付かない。
        (Working, false, false, true, ""),
        (Working, true, false, false, "awaiting-watch"),
        (Working, true, true, true, "awaiting-watch"),
        // 停止要請の出し直しの後も見張り待ちの印は残る。示すのは「作業中」のときだけ。
        (StopRequested, true, true, true, ""),
        (StopRequested, true, false, false, "absent"),
        (StopRequested, false, true, false, "absent"),
        (Stopped, false, true, true, ""),
        (Stopped, false, false, false, "absent"),
        (Stopped, true, false, false, "absent"),
    ];
    for (status, awaiting, info, present, want) in cases {
        let label = format!("{status:?} awaiting={awaiting} info={info} present={present}");
        let mut p = participant("P", "名前", "areka", status);
        p.awaiting_watch_since = awaiting.then_some(T0 + 3);
        if !info {
            p.watch = None;
        }
        let mut state = State::empty();
        put(&mut state, p);
        let presence = if present {
            ALL_PRESENT
        } else {
            WatchAbsent(&["P"])
        };

        let line = terminal_line(&render_terminal(&state, &presence), "P");
        let tail = line.split(' ').nth(3).unwrap_or("");
        assert_eq!(tail, want, "端末: {label}: {line}");

        let row = markdown_row(&render_markdown(&state, &presence, T0), "P");
        let shown = if want.is_empty() { "-" } else { want };
        assert_eq!(row[5], shown, "読み物: {label}: {row:?}");
    }
}

#[test]
fn utc_text_is_right_at_the_boundaries() {
    let cases = [
        (0, "1970-01-01T00:00:00Z", "19700101T000000Z"),
        // 400 で割れる年はうるう年。
        (951_782_400, "2000-02-29T00:00:00Z", "20000229T000000Z"),
        (951_868_800, "2000-03-01T00:00:00Z", "20000301T000000Z"),
        (1_709_164_800, "2024-02-29T00:00:00Z", "20240229T000000Z"),
        (1_709_251_199, "2024-02-29T23:59:59Z", "20240229T235959Z"),
        (1_709_251_200, "2024-03-01T00:00:00Z", "20240301T000000Z"),
        // 年の変わり目。
        (1_798_761_599, "2026-12-31T23:59:59Z", "20261231T235959Z"),
        (1_798_761_600, "2027-01-01T00:00:00Z", "20270101T000000Z"),
        (T0, T0_UTC, "20261003T040000Z"),
        // 符号つき 32 ビットの秒が尽きた先。
        (2_147_483_648, "2038-01-19T03:14:08Z", "20380119T031408Z"),
        // 100 で割れて 400 で割れない年はうるう年でない。
        (4_107_542_399, "2100-02-28T23:59:59Z", "21000228T235959Z"),
        (4_107_542_400, "2100-03-01T00:00:00Z", "21000301T000000Z"),
        // 符号なし 32 ビットの秒が尽きた先。
        (4_294_967_296, "2106-02-07T06:28:16Z", "21060207T062816Z"),
    ];
    for (secs, readable, compact) in cases {
        assert_eq!(utc(secs), readable, "{secs}");
        assert_eq!(utc_compact(secs), compact, "{secs}");
    }
}

#[test]
fn empty_state_renders_zero_counts_and_empty_sections() {
    let state = State::empty();
    assert_eq!(
        render_terminal(&state, &ALL_PRESENT),
        "participants=0 load-holder=none load-queued=0 merging=0 merge-queued=0 not-working=0 waits=0\n\
         load: holder=none queue=none\n"
    );
    let markdown = render_markdown(&state, &ALL_PRESENT, 0);
    assert_eq!(
        markdown,
        "\
# areka-impl-watch status

generated: 1970-01-01T00:00:00Z

## Participants

none

## Load desk

- holder: none
- queue: none

## Merge desks

none

## Waits and watches

none

## Recent reclaims, recoveries and clears

none
"
    );
}

#[test]
fn markdown_carries_every_item_of_the_status_report() {
    let mut state = sample();
    state.load.holder = Some(LoadHolder {
        id: "D".to_owned(),
        purpose: "走っている計測".to_owned(),
        requested: T0,
        granted: T0 + 1,
        running: true,
        stopped: vec!["A".to_owned(), "B".to_owned()],
    });
    state.push_recent(Recent {
        at: T0 + 9,
        kind: RecentKind::Cleared,
        id: None,
        detail: r"C:\置き場\state.json.cleared-20261003T040009Z".to_owned(),
    });
    let got = render_markdown(&state, &SAMPLE_PRESENCE, T0 + 100);
    let watch = format!("pid 1234 since {T0_UTC}");
    for line in [
        "generated: 2026-10-03T04:01:40Z".to_owned(),
        // 参加者: 識別・名前・リポジトリ・状態・時刻・印・停止要請の理由・見張り。
        "| id | name | repo | status | since | mark | stop reason | watch |".to_owned(),
        format!("| A | あれか実装 | areka | working | {T0_UTC} | - | - | {watch} |"),
        format!(
            "| B | B の見張り役 | areka | stop-requested | {T0_UTC} | - | by C: 負荷の計測 5 回 | {watch} |"
        ),
        format!("| C | 計測係 | pasta | working | {T0_UTC} | absent | - | {watch} |"),
        // 負荷テストの机: 持ち主（内容・走っている印・止まった一覧）と待ち行列。
        format!(
            "- holder: D (requested {T0_UTC}, granted 2026-10-03T04:00:01Z, running: yes, stopped: A, B): 走っている計測"
        ),
        "- queue:".to_owned(),
        "  1. C (requested 2026-10-03T04:00:20Z): 負荷の計測 5 回".to_owned(),
        // リポジトリごとのマージの机: 持ち主・待ち行列・直前のマージ。
        "### areka".to_owned(),
        "- holder: A (spec impl-watch-2, bug: no, requested 2026-10-03T04:00:10Z, granted 2026-10-03T04:00:11Z)"
            .to_owned(),
        "- queue: none".to_owned(),
        "- last: PR#281 414d43eb impl-watch 2026-10-10T01:00:00Z".to_owned(),
        // 走っている待ち・見張り: 種類・識別・リポジトリ・プロセス番号・時刻。
        "| kind | id | repo | pid | since |".to_owned(),
        format!("| watch | A | areka | 1234 | {T0_UTC} |"),
        format!("| load | C | - | 1234 | {T0_UTC} |"),
        // 最近の回収・復旧・消した記録（新しい順）。
        "| at | kind | id | detail |".to_owned(),
        r"| 2026-10-03T04:00:09Z | cleared | - | C:\置き場\state.json.cleared-20261003T040009Z |"
            .to_owned(),
        "| 2026-10-03T04:00:05Z | reclaimed | Z | watch absent |".to_owned(),
        "| 2026-10-03T04:00:01Z | recovered | - | created |".to_owned(),
    ] {
        assert_has_line(&got, &line);
    }
}

#[test]
fn queues_are_shown_in_the_order_they_will_be_served() {
    let mut state = State::empty();
    // 行列は到着順のまま持たれている。番は「バグ優先 → 申し込みの早い順・同点は行列の前」。
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: None,
            queue: vec![
                merge_request("late", false, T0 + 30),
                merge_request("tie1", false, T0 + 10),
                merge_request("bug", true, T0 + 40),
                merge_request("tie2", false, T0 + 10),
            ],
            last: None,
        },
    );
    // 負荷テストは申し込みの早い順・同点は行列の前。
    state.load.queue = vec![
        load_request("L3", T0 + 9),
        load_request("L1", T0 + 2),
        load_request("L2", T0 + 2),
    ];

    let terminal = render_terminal(&state, &ALL_PRESENT);
    assert_has_line(&terminal, "merge areka: holder=none queue=4 last=none");
    assert_has_line(&terminal, "load: holder=none queue=L1,L2,L3");

    let markdown = render_markdown(&state, &ALL_PRESENT, T0);
    let numbered: Vec<&str> = markdown
        .lines()
        .filter(|line| line.starts_with("  "))
        .map(|line| line.split(' ').nth(3).unwrap_or(""))
        .collect();
    assert_eq!(
        numbered,
        ["L1", "L2", "L3", "bug", "tie1", "tie2", "late"],
        "{markdown}"
    );
    assert_has_line(
        &markdown,
        "  1. bug (spec spec-bug, bug: yes, requested 2026-10-03T04:00:40Z)",
    );
    assert_has_line(&markdown, "- last: none");

    // 1 行目の数。机をもう 1 つ足す: 机は 2 つ在るが持ち主は居ない（merging=0）。
    // マージの待ち行列は全リポジトリの合計（4 + 1）。
    state.merge.insert(
        "pasta".to_owned(),
        MergeDesk {
            holder: None,
            queue: vec![merge_request("p", false, T0)],
            last: None,
        },
    );
    assert_eq!(
        first_line(&render_terminal(&state, &ALL_PRESENT)),
        "participants=0 load-holder=none load-queued=3 merging=0 merge-queued=5 not-working=0 waits=0"
    );
}

#[test]
fn first_line_counts_desks_with_a_holder_and_participants_not_working() {
    use ParticipantStatus::{StopRequested, Stopped, Working};
    let mut state = State::empty();
    for (id, status) in [("A", Working), ("B", StopRequested), ("C", Stopped)] {
        put(&mut state, participant(id, "名前", "areka", status));
    }
    // 持ち主の居る机。
    state.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            holder: Some(MergeHolder {
                id: "A".to_owned(),
                spec: "spec-A".to_owned(),
                bug: false,
                requested: T0,
                granted: T0 + 1,
            }),
            queue: Vec::new(),
            last: None,
        },
    );
    // マージの済んだ机: 持ち主は居ないが、直前のマージが残るので机は在る。
    state.merge.insert(
        "pasta".to_owned(),
        MergeDesk {
            holder: None,
            queue: Vec::new(),
            last: Some(LastMerge {
                pr: "7".to_owned(),
                sha: "0123abcd".to_owned(),
                spec: "spec-old".to_owned(),
                at: T0,
            }),
        },
    );

    let terminal = render_terminal(&state, &ALL_PRESENT);
    // 「停止要請中」と「止まった」の両方が not-working。merging は持ち主の居る机だけ。
    assert_eq!(
        first_line(&terminal),
        "participants=3 load-holder=none load-queued=0 merging=1 merge-queued=0 not-working=2 waits=0"
    );
    assert_has_line(&terminal, "merge areka: holder=A queue=0 last=none");
    assert_has_line(
        &terminal,
        &format!("merge pasta: holder=none queue=0 last=PR#7 0123abcd spec-old {T0_UTC}"),
    );
}

#[test]
fn markdown_keeps_free_text_inside_its_table_cell() {
    let mut state = State::empty();
    put(
        &mut state,
        participant("A", "a|b\r\nc", "areka", ParticipantStatus::Working),
    );
    let row = markdown_row(&render_markdown(&state, &ALL_PRESENT, T0), "A");
    assert_eq!(row.len(), 8, "{row:?}");
    assert_eq!(row[1], r"a\|b  c");
}
