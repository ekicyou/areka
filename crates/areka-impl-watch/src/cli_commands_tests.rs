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
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::test_support::{args, entries, run_captured};
use super::{exit_code, run_with};
use crate::error::{WatchError, escape_path};
use crate::home::Home;
use crate::presence::{Held, hold};
use crate::state::{RecentKind, State, WaitKind};

const NAME: &str = "見張り役";
const PURPOSE: &str = "負荷の計測";

/// 一時の置き場所。道筋に ASCII の外の字が入る。
fn home_in(root: &TempPath) -> PathBuf {
    root.child("置き場")
}

/// 走らせて、終了コード・標準出力・標準エラーを返す。端末へ出る文は ASCII だけ。
fn call(home: &Path, words: &[&str]) -> (u8, String, String) {
    let (result, out, err) = run_captured(&args(words), Some(home.into()));
    assert!(
        out.is_ascii(),
        "{words:?}: 標準出力に ASCII の外の字: {out:?}"
    );
    assert!(
        err.is_ascii(),
        "{words:?}: 標準エラーに ASCII の外の字: {err:?}"
    );
    (exit_code(&result), out, err)
}

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

/// できた: 終了コード 0・標準出力にその 1 行・標準エラーは空。
fn done(line: &str) -> (u8, String, String) {
    (0, format!("{line}\n"), String::new())
}

/// 当てはまらなかった: 終了コード 3・標準出力は空・標準エラーにその 1 行。
fn not_applied(why: &str) -> (u8, String, String) {
    (3, String::new(), format!("not applied: {why}\n"))
}

/// その識別たちの居る印（見張りのロックファイル）を握る。返した値を持っている間だけ「居る」。
fn hold_watches(home: &Path, ids: &[&str]) -> Vec<Held> {
    let home = Home {
        dir: home.to_path_buf(),
    };
    ids.iter()
        .map(|id| {
            hold(&home.alive_path(id, WaitKind::Watch))
                .expect("ロックファイルを開ける")
                .expect("まだ誰も握っていない")
        })
        .collect()
}

/// 置き場所の直下に在るものの名前（並べ替え済み）。
fn names(home: &Path) -> Vec<String> {
    let name = |path: PathBuf| {
        path.file_name()
            .expect("名前が在る")
            .to_string_lossy()
            .into_owned()
    };
    entries(home).into_iter().map(name).collect()
}

fn state_bytes(home: &Path) -> Vec<u8> {
    fs::read(home.join("state.json")).expect("状態ファイルが在る")
}

fn read_state(home: &Path) -> State {
    serde_json::from_slice(&state_bytes(home)).expect("状態ファイルが状態として読める")
}

/// 状態ファイルを直に置く（コマンドの列では作れない状態・壊れたファイル）。
fn put_state(home: &Path, text: &str) {
    fs::create_dir_all(home).expect("置き場所を作れる");
    fs::write(home.join("state.json"), text).expect("書ける");
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
