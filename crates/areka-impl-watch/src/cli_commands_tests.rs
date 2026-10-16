//! 直ちに終わるコマンドの手順（状態を変える 1 回・`status`・`clear`）のテスト。
//!
//! 本物の口（状態ファイル・ロックファイル・本物の時計）を、ワークツリーの `target\` の下の
//! 一時の置き場所に向けて通す。置き場所の道筋には ASCII の外の字を入れ、名前と内容には
//! 日本語を渡す（それでも端末へ出る文が ASCII だけであることを、どの呼び出しでも判定する）。
//!
//! 2 つ以上の識別を使うテストは、呼ぶ前に全員の居る印（見張りのロックファイル）をテストの
//! プロセス内で握る。握らないと、呼んだ識別以外の「作業中」の参加者は次の呼び出しで回収される。

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use temp_path_kit::TempPath;

use super::test_support::{
    args, call, done, hold_watches, home_in, names, put_state, read_state, state_bytes,
};
use super::{exit_code, run_with};
use crate::error::{WatchError, escape_path};
use crate::state::RecentKind;

const NAME: &str = "見張り役";
const PURPOSE: &str = "負荷の計測";

fn merge(home: &Path, id: &str) -> (u8, String, String) {
    let words = [
        "merge",
        "--id",
        id,
        "--repo",
        "areka",
        "--spec",
        "impl-watch",
    ];
    call(home, &[&words[..], &["--name", NAME]].concat())
}

fn loadtest(home: &Path, id: &str) -> (u8, String, String) {
    let words = ["loadtest", "--id", id, "--repo", "areka"];
    call(
        home,
        &[&words[..], &["--purpose", PURPOSE, "--name", NAME]].concat(),
    )
}

fn merged(home: &Path, id: &str) -> (u8, String, String) {
    call(
        home,
        &["merged", "--id", id, "--pr", "281", "--sha", "414d43eb"],
    )
}

/// 当てはまらなかった: 終了コード 3・標準出力は空・標準エラーにその 1 行。
fn not_applied(why: &str) -> (u8, String, String) {
    (3, String::new(), format!("not applied: {why}\n"))
}

// ---- マージの机 ----

#[test]
fn a_merge_round_trip_prints_the_design_lines_and_exit_codes() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b"]);

    // 机が空いていれば、待たない申し込みも直ちに番を受ける。
    assert_eq!(
        merge(&home, "a"),
        done("granted merge repo=areka; last: none")
    );
    assert_eq!(merge(&home, "b"), done("queued merge repo=areka pos=1"));
    assert_eq!(merged(&home, "a"), done("merged repo=areka"));

    // 番は「済んだ」と同じ呼び出しで b へ移っている。直前のマージが 1 行に載る。
    let (code, out, err) = merge(&home, "b");
    assert_eq!((code, err.as_str()), (0, ""), "{out:?}");
    let at = out
        .strip_prefix("granted merge repo=areka; last: PR#281 414d43eb impl-watch ")
        .unwrap_or_else(|| panic!("{out:?}"));
    // 残りは UTC の時刻（`2026-10-10T01:00:00Z`）と改行。
    assert!(at.len() == 21 && at.ends_with("Z\n"), "{out:?}");

    // 済んだ参加者の記録は消えている。
    let ids: Vec<String> = read_state(&home).participants.into_keys().collect();
    assert_eq!(ids, ["b"]);
}

#[test]
fn merged_by_a_non_holder_is_3_and_leaves_the_state_file_untouched() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b"]);
    merge(&home, "a");
    merge(&home, "b");
    let before = state_bytes(&home);

    assert_eq!(merged(&home, "b"), not_applied("not the merge holder"));
    assert_eq!(merged(&home, "nobody"), not_applied("not the merge holder"));

    assert_eq!(state_bytes(&home), before);
}

#[test]
fn the_calling_id_is_not_reclaimed_by_its_own_call() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);

    // 見張りを立てていない a。自分の呼び出しでは回収されないので、机を持ったまま「済んだ」が通る。
    assert_eq!(
        merge(&home, "a"),
        done("granted merge repo=areka; last: none")
    );
    assert_eq!(merged(&home, "a"), done("merged repo=areka"));
}

#[test]
fn the_merge_position_counts_in_serving_order_bugs_first() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b", "c"]);
    merge(&home, "a");

    assert_eq!(merge(&home, "b"), done("queued merge repo=areka pos=1"));
    // 後から来たバグの申し込みが先頭。
    let bug = [
        "merge", "--id", "c", "--repo", "areka", "--spec", "fix", "--bug",
    ];
    assert_eq!(call(&home, &bug), done("queued merge repo=areka pos=1"));
    // 申し込み直しても二重に並ばず、番は 2 番目になっている。
    assert_eq!(merge(&home, "b"), done("queued merge repo=areka pos=2"));
}

#[test]
fn ids_that_differ_only_in_case_are_the_same_participant() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);

    assert_eq!(
        merge(&home, "A"),
        done("granted merge repo=areka; last: none")
    );
    // 記録に入るのは小文字の形。
    let state = read_state(&home);
    let ids: Vec<&String> = state.participants.keys().collect();
    assert_eq!(ids, ["a"]);
    let holder = state.merge["areka"].holder.as_ref();
    assert_eq!(holder.map(|holder| holder.id.as_str()), Some("a"));

    // 小文字で呼んでも同じ参加者（机の持ち主）。
    assert_eq!(merged(&home, "a"), done("merged repo=areka"));
}

// ---- 負荷テストの机・停止要請 ----

#[test]
fn a_load_round_trip_prints_the_design_lines_and_exit_codes() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b"]);

    // b を、申し込みの無いただの参加者にする（止まる相手が居なければ番は直ちに来る）。
    assert_eq!(loadtest(&home, "b"), done("granted load; stopped: none"));
    assert_eq!(call(&home, &["cancel", "--id", "b"]), done("cancelled"));

    // a の申し込みで b に停止要請が出る。b が止まるまで番は来ない。
    assert_eq!(loadtest(&home, "a"), done("queued load pos=1"));
    let stopped = |id| call(&home, &["stopped", "--id", id]);
    assert_eq!(stopped("a"), not_applied("not asked to stop"));
    assert_eq!(stopped("b"), done("stopped"));
    assert_eq!(stopped("b"), not_applied("not asked to stop"));
    assert_eq!(loadtest(&home, "a"), done("granted load; stopped: b"));

    let unstop_a = ["unstop", "--id", "a"];
    assert_eq!(call(&home, &unstop_a), not_applied("nobody to unstop"));
    assert_eq!(call(&home, &["unstop"]), done("unstopped n=1"));

    let loaddone = |id| call(&home, &["loaddone", "--id", id]);
    assert_eq!(loaddone("b"), not_applied("not the load-test holder"));
    assert_eq!(loaddone("a"), done("load done"));
    assert_eq!(loaddone("a"), not_applied("not the load-test holder"));
}

#[test]
fn the_load_position_counts_in_serving_order_earliest_request_first() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    // 待ち行列の並び（到着順）と申し込みの時刻の順が食い違う状態。マージの持ち主 m が居るので、
    // 負荷テストの番は来ない。
    put_state(
        &home,
        r#"{
          "version": 1,
          "participants": {
            "a": { "id": "a", "repo": "areka" },
            "b": { "id": "b", "repo": "areka" },
            "m": { "id": "m", "repo": "areka" }
          },
          "merge": { "areka": { "holder": { "id": "m", "spec": "s" } } },
          "load": { "queue": [ { "id": "a", "requested": 20 }, { "id": "b", "requested": 10 } ] }
        }"#,
    );
    let _held = hold_watches(&home, &["a", "b", "m"]);

    assert_eq!(loadtest(&home, "a"), done("queued load pos=2"));
    assert_eq!(loadtest(&home, "b"), done("queued load pos=1"));
}

#[test]
fn loadrunning_records_the_holder_and_is_3_while_the_desk_is_held() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b"]);
    let running = |id| {
        let words = ["loadrunning", "--id", id, "--repo", "areka"];
        call(&home, &[&words[..], &["--purpose", PURPOSE]].concat())
    };

    assert_eq!(running("a"), done("recorded running load"));
    let before = state_bytes(&home);
    assert_eq!(
        running("b"),
        not_applied("the load-test desk already has a holder")
    );
    // 断った b は参加もしていない。
    assert_eq!(state_bytes(&home), before);
}

// ---- 離脱・取り下げ・周期の一回り ----

#[test]
fn leave_cancel_and_tick_print_one_fixed_line_each() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a"]);
    merge(&home, "a");

    assert_eq!(call(&home, &["tick"]), done("tick"));
    assert_eq!(read_state(&home).participants.len(), 1);
    assert_eq!(call(&home, &["leave", "--id", "a"]), done("left"));
    assert_eq!(read_state(&home).participants.len(), 0);
    // 外す相手が居なくても「できた」（代わりに行う手の操作は何度呼んでもよい）。
    assert_eq!(call(&home, &["leave", "--id", "a"]), done("left"));
    assert_eq!(call(&home, &["cancel", "--id", "a"]), done("cancelled"));
}

// ---- 状態の確認 ----

#[test]
fn status_changes_nothing_and_keeps_japanese_values_off_the_terminal() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    // a だけが居る印を持つ。b は見張りの無い「作業中」（次の状態を変える呼び出しで回収される）。
    let _held = hold_watches(&home, &["a"]);
    merge(&home, "a");
    merge(&home, "b");
    loadtest(&home, "b");
    let before = state_bytes(&home);
    fs::remove_file(home.join("status.md")).expect("消せる");

    let (code, out, err) = call(&home, &["status"]);

    let path = escape_path(&home.join("status.md").to_string_lossy());
    assert!(path.contains("\\u{"), "道筋に ASCII の外の字が無い: {path}");
    let expected = format!(
        "participants=2 load-holder=none load-queued=1 merging=1 merge-queued=1 not-working=0 waits=0\n\
         a areka working\n\
         b areka working absent\n\
         merge areka: holder=a queue=1 last=none\n\
         load: holder=none queue=b\n\
         status: {path}\n"
    );
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (0, expected.as_str(), "")
    );
    // 回収もしない: 状態ファイルは 1 バイトも変わらない。
    assert_eq!(state_bytes(&home), before);
    // 日本語の値は読み物で読める。
    let reading = fs::read_to_string(home.join("status.md")).expect("読み物が書かれている");
    assert!(
        reading.contains(NAME) && reading.contains(PURPOSE),
        "{reading}"
    );
}

#[test]
fn status_without_a_state_file_says_so_and_creates_neither_state_nor_reading() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);

    assert_eq!(call(&home, &["status"]), done("no state file"));

    // ログのファイルだけ（状態ファイルも、ロックも、読み物も、居る印のフォルダも無い）。
    assert_eq!(names(&home), ["impl-watch.log"]);
}

#[test]
fn status_over_a_broken_state_file_is_1_and_does_not_move_or_change_it() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    put_state(&home, "{ not json");

    let got = call(&home, &["status"]);

    let text = "state file is broken; see impl-watch.log\n";
    assert_eq!(got, (1, String::new(), text.to_owned()));
    assert_eq!(names(&home), ["impl-watch.log", "state.json"]);
    assert_eq!(state_bytes(&home), b"{ not json");
}

#[test]
fn a_version_mismatched_state_file_is_1_and_stays_byte_identical() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let file = r#"{ "version": 2, "participants": { "x": {} } }"#;
    put_state(&home, file);
    let text = "state file version mismatch: file has 2, this exe knows 1. \
                Do not mix old and new exes; see doc/impl-watch.md\n";

    assert_eq!(
        call(&home, &["status"]),
        (1, String::new(), text.to_owned())
    );
    assert_eq!(names(&home), ["impl-watch.log", "state.json"]);

    assert_eq!(merge(&home, "a"), (1, String::new(), text.to_owned()));
    // 状態を変える呼び出しはロックを取るが、読まず、退避もせず、上書きもしない。
    assert_eq!(names(&home), ["impl-watch.log", "state.json", "state.lock"]);
    assert_eq!(state_bytes(&home), file.as_bytes());
}

// ---- 全部消す ----

#[test]
fn clear_sets_the_state_aside_at_once_and_prints_the_backup_path() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    merge(&home, "a");
    let before = state_bytes(&home);

    let (code, out, err) = call(&home, &["clear"]);

    assert_eq!((code, err.as_str()), (0, ""), "{out:?}");
    let backups: Vec<String> = names(&home)
        .into_iter()
        .filter(|name| name.starts_with("state.json.cleared-"))
        .collect();
    let [backup] = backups.as_slice() else {
        panic!("退避が 1 つでない: {backups:?}");
    };
    let backup = home.join(backup);
    // 道筋は ASCII の外の字を逃がして出る。
    let path = escape_path(&backup.to_string_lossy());
    assert!(path.contains("\\u{"), "道筋に ASCII の外の字が無い: {path}");
    assert_eq!(out, format!("cleared; backup: {path}\n"));
    // 消す前の状態は退避に残り、状態は「消した記録」だけになる。
    assert_eq!(fs::read(&backup).expect("退避を読める"), before);
    let state = read_state(&home);
    assert!(state.participants.is_empty() && state.merge.is_empty());
    let [recent] = state.recent.as_slice() else {
        panic!("記録が 1 件でない: {:?}", state.recent);
    };
    assert_eq!(recent.kind, RecentKind::Cleared);
    assert_eq!(Path::new(&recent.detail), backup);
}

#[test]
fn clear_without_a_state_file_says_there_is_no_backup() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);

    assert_eq!(call(&home, &["clear"]), done("cleared; backup: none"));

    assert!(read_state(&home).participants.is_empty());
}

// ---- リポジトリの分だけ消す ----

#[test]
fn clear_repo_leaves_everyone_of_the_repo_and_prints_the_ids_without_a_backup() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b", "c"]);
    merge(&home, "a");
    merge(&home, "b");
    let other = ["merge", "--id", "c", "--repo", "other", "--spec", "s"];
    call(&home, &other);

    assert_eq!(
        call(&home, &["clear", "--repo", "areka"]),
        done("cleared repo=areka; left: a, b")
    );

    // 退避は作らず、ほかのリポジトリの参加者はそのまま。
    assert!(
        !names(&home).iter().any(|name| name.contains("cleared")),
        "{:?}",
        names(&home)
    );
    let ids: Vec<String> = read_state(&home).participants.into_keys().collect();
    assert_eq!(ids, ["c"]);
    // 消すものが無くても 0。
    assert_eq!(
        call(&home, &["clear", "--repo", "areka"]),
        done("cleared repo=areka; left: none")
    );
}

#[test]
fn a_malformed_clear_repo_is_2_and_creates_or_changes_nothing() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let malformed: [&[&str]; 8] = [
        &["clear", "--repo"],
        &["clear", "--repo", "a/b"],
        &["clear", "--repo", r"C:\x"],
        &["clear", "--repo", "areka", "--repo", "areka"],
        &["clear", "--repo", "areka", "--id", "a"],
        &["clear", "--repo", "areka", "extra"],
        &["clear", "--repo", "areka", "--wait"],
        &["clear", "--wait"],
    ];
    // 置き場所の無いときは作らない。
    for words in malformed {
        let (code, out, _) = call(&home, words);
        assert_eq!((code, out.as_str()), (2, ""), "{words:?}");
        assert!(!home.exists(), "{words:?}: 置き場所を作った");
    }
    // 状態の在るときは変えず、何も足さない。
    merge(&home, "a");
    let (before, listed) = (state_bytes(&home), names(&home));
    for words in malformed {
        assert_eq!(call(&home, words).0, 2, "{words:?}");
        assert_eq!(state_bytes(&home), before, "{words:?}");
        assert_eq!(names(&home), listed, "{words:?}");
    }
}

// ---- 書けなかった読み物 ----

/// 読み物（`status.md`）が書けなくても、コマンドの結果と終了コードは変わらない。
#[test]
fn a_reading_matter_that_cannot_be_written_changes_neither_the_result_nor_the_exit_code() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    merge(&home, "a");
    let (_, summary, _) = call(&home, &["status"]);
    assert!(summary.contains("\nstatus: "), "{summary:?}");
    let reading = fs::read(home.join("status.md")).expect("読み物が在る");
    // 一時ファイルの名前（プロセス番号入り）をフォルダで塞ぐ: 読み物だけが書けなくなる。
    let temp = format!("status.md.{}.tmp", std::process::id());
    fs::create_dir(home.join(temp)).expect("作れる");

    // 状態の確認: 要約と読み物の道筋を、書けたときと同じに出す。
    assert_eq!(call(&home, &["status"]), (0, summary, String::new()));
    // 状態を変える 1 回: 結果の文と 0（変化は状態ファイルに入っている）。
    assert_eq!(call(&home, &["leave", "--id", "a"]), done("left"));
    assert!(read_state(&home).participants.is_empty());
    // 全部消す: 退避の道筋と 0。
    let (code, out, err) = call(&home, &["clear"]);
    assert_eq!((code, err.as_str()), (0, ""), "{out:?}");
    assert!(
        out.starts_with("cleared; backup: ") && out.contains("state.json.cleared-"),
        "{out:?}"
    );
    assert_eq!(read_state(&home).recent.len(), 1);

    // 較正: 読み物は、どの呼び出しでも書けていない（塞ぐ前のまま）。
    assert_eq!(fs::read(home.join("status.md")).expect("読める"), reading);
}

// ---- 出せなかった結果 ----

/// 何も書けない書き手。
struct Closed;

impl Write for Closed {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_result_that_cannot_be_printed_is_a_failure_not_a_silent_success() {
    let root = TempPath::under_target("impl-watch-cli");
    let home = home_in(&root);
    let mut err = Vec::new();

    let result = run_with(&args(&["tick"]), Some(home.into()), &mut Closed, &mut err);

    assert!(
        matches!(result, Err(WatchError::Io { op: "stdout", .. })),
        "{result:?}"
    );
    assert_eq!(exit_code(&result), 1);
    assert_eq!(
        String::from_utf8(err).expect("UTF-8 で出る"),
        "io stdout: BrokenPipe (os error 0)\n"
    );
}

// ---- 番の順・`pos=`・読み物の並びの一致 ----

/// 誰のどの待ちも「居る」と答える生死（回収で誰も消えない）。
struct AllPresent;

impl crate::plan::Presence for AllPresent {
    fn is_present(&self, _: &str, _: crate::state::WaitKind) -> bool {
        true
    }
}

/// 待ち行列の識別すべてを参加者にする（`pos=` の判定は参加者の記録を引く）。
fn join_all(state: &mut crate::state::State, ids: &[&str]) {
    for id in ids {
        let participant = crate::state::Participant {
            id: (*id).to_owned(),
            ..Default::default()
        };
        state.participants.insert((*id).to_owned(), participant);
    }
}

/// 読み物の待ち行列（`  1. <id> (...`）の識別を上から順に。
fn reading_queue(state: &crate::state::State) -> Vec<String> {
    let reading = crate::status::render_markdown(state, &AllPresent, 0);
    let ids = reading.lines().filter_map(|line| {
        let (nth, rest) = line.strip_prefix("  ")?.split_once(". ")?;
        nth.parse::<usize>().ok()?;
        Some(rest.split_once(" (")?.0.to_owned())
    });
    ids.collect()
}

#[test]
fn the_serving_order_the_position_and_the_reading_agree_with_ties_and_bugs_mixed() {
    use crate::plan::{Command, apply};
    use crate::state::{LoadHolder, LoadRequest, MergeDesk, MergeHolder, MergeRequest, State};

    // マージ: バグと通常が混ざり、時刻の同点が両方の区分に在る。行列の並びは番の順と食い違う。
    let request = |id: &str, bug, requested| MergeRequest {
        id: id.to_owned(),
        spec: "s".to_owned(),
        bug,
        requested,
    };
    let mut merge = State::empty();
    merge.merge.insert(
        "areka".to_owned(),
        MergeDesk {
            queue: vec![
                request("n1", false, 10),
                request("b1", true, 20),
                request("n2", false, 10),
                request("b2", true, 20),
                request("n3", false, 5),
                request("b3", true, 30),
            ],
            ..MergeDesk::default()
        },
    );
    let expected = ["b1", "b2", "b3", "n3", "n1", "n2"];

    // (1) 持ち主を空けて周期の一回りを繰り返したときの番の順。
    let mut served = merge.clone();
    let mut granted = Vec::new();
    while let Some(holder) = {
        apply(&mut served, &Command::Tick, None, 0, &AllPresent);
        served
            .merge
            .get_mut("areka")
            .and_then(|desk| desk.holder.take())
    } {
        granted.push(holder.id);
    }
    assert_eq!(granted, expected);

    // (2) 別の持ち主が居る間の `pos=`、(3) 読み物の待ち行列。
    let mut held = merge;
    join_all(&mut held, &expected);
    held.merge.get_mut("areka").expect("机が在る").holder = Some(MergeHolder {
        id: "m".to_owned(),
        ..MergeHolder::default()
    });
    for (nth, id) in expected.iter().enumerate() {
        let command = Command::Merge {
            id: (*id).to_owned(),
            name: None,
            repo: "areka".to_owned(),
            spec: "s".to_owned(),
            bug: false,
        };
        assert_eq!(
            super::result_line("merge", &command, &held, &[]),
            format!("queued merge repo=areka pos={}", nth + 1)
        );
    }
    assert_eq!(reading_queue(&held), expected);

    // 負荷テスト: 同点入り（負荷テストの待ちが在るとマージの番が出ないので別の状態）。
    let request = |id: &str, requested| LoadRequest {
        id: id.to_owned(),
        purpose: PURPOSE.to_owned(),
        requested,
    };
    let mut load = State::empty();
    load.load.queue = vec![
        request("l1", 10),
        request("l2", 5),
        request("l3", 10),
        request("l4", 5),
    ];
    let expected = ["l2", "l4", "l1", "l3"];

    let mut served = load.clone();
    let mut granted = Vec::new();
    while let Some(holder) = {
        apply(&mut served, &Command::Tick, None, 0, &AllPresent);
        served.load.holder.take()
    } {
        granted.push(holder.id);
    }
    assert_eq!(granted, expected);

    let mut held = load;
    join_all(&mut held, &expected);
    held.load.holder = Some(LoadHolder {
        id: "h".to_owned(),
        ..LoadHolder::default()
    });
    for (nth, id) in expected.iter().enumerate() {
        let command = Command::LoadTest {
            id: (*id).to_owned(),
            name: None,
            repo: "areka".to_owned(),
            purpose: PURPOSE.to_owned(),
        };
        assert_eq!(
            super::result_line("loadtest", &command, &held, &[]),
            format!("queued load pos={}", nth + 1)
        );
    }
    assert_eq!(reading_queue(&held), expected);
}
