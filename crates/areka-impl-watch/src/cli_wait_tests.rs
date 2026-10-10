//! 待つコマンド（`watch`・`merge --wait`・`loadtest --wait`・`stopped --wait`・`resume`）の
//! 手順のテスト。
//!
//! 本物の口（状態ファイル・ロックファイル・本物の時計・1 秒の眠り）を、ワークツリーの `target\`
//! の下の一時の置き場所に向けて通す。どのテストも、あらかじめ条件が満ちた状態に対して呼ぶので
//! 直ちに終わる（眠りに入らない）。待ってからほかの呼び出しで終わる長い流れは、本物の実行
//! ファイルの実機テストが通す。
//!
//! コマンドの列だけでは作れない状態（すでに停止要請中・止まった参加者）は状態ファイルを直に
//! 置く。2 つ以上の識別を使うテストは、呼ぶ前にほかの識別の見張りの居る印を握る。

use std::fs;
use std::path::Path;

use temp_path_kit::TempPath;

use super::test_support::{
    call, done, hold_sign, hold_watches, home_in, names, put_state, read_state, state_bytes,
};
use crate::error::escape_path;
use crate::state::{ParticipantStatus, WaitKind};

const MERGE_A: [&str; 7] = ["merge", "--id", "a", "--repo", "areka", "--spec", "x"];
const LOAD_A: [&str; 7] = [
    "loadtest",
    "--id",
    "a",
    "--repo",
    "areka",
    "--purpose",
    "負荷の計測",
];
const WATCH_A: [&str; 5] = ["watch", "--id", "a", "--repo", "areka"];

/// 負荷テストの机: c が待っている（a が止まるまで番は来ない）。
const C_QUEUED: &str =
    r#"{ "queue": [ { "id": "c", "purpose": "負荷の計測", "requested": 10 } ] }"#;

/// `--wait` を付けた呼び出し。
fn waiting(home: &Path, words: &[&str]) -> (u8, String, String) {
    call(home, &[words, &["--wait"]].concat())
}

/// 消えた: 終了コード 3・標準出力は空・標準エラーにその 1 行。
fn gone(why: &str) -> (u8, String, String) {
    (3, String::new(), format!("gone: {why}\n"))
}

/// 同じ待ちがすでに走っている: 終了コード 1・標準出力は空・標準エラーにその 1 行。
fn already_running(kind: &str) -> (u8, String, String) {
    let text = format!("a {kind} wait for a is already running\n");
    (1, String::new(), text)
}

/// 見張りの終わりの 2 行目（読み物の道筋。ASCII の外の字は逃がして出る）。
fn details(home: &Path) -> String {
    let path = escape_path(&home.join("status.md").to_string_lossy());
    assert!(path.contains("\\u{"), "道筋に ASCII の外の字が無い: {path}");
    format!("details: {path}\n")
}

/// a が c の負荷テストのために `status`（停止要請中・止まった）になっている状態を置く。
/// `load` は負荷テストの机（c が待っている・持っている）。
fn put_not_working(home: &Path, status: &str, load: &str) {
    put_state(
        home,
        &format!(
            r#"{{
              "version": 1,
              "participants": {{
                "a": {{ "id": "a", "repo": "areka", "status": "{status}",
                        "stop_reason": {{ "by": "c", "purpose": "負荷の計測" }} }},
                "c": {{ "id": "c", "repo": "areka" }}
              }},
              "load": {load}
            }}"#
        ),
    );
}

// ---- マージの番・負荷テストの番 ----

#[test]
fn merge_wait_ends_at_once_with_the_last_merge_when_the_turn_has_already_come() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b"]);
    let merge_b = ["merge", "--id", "b", "--repo", "areka", "--spec", "y"];
    call(&home, &MERGE_A);
    call(&home, &merge_b);
    let merged = ["merged", "--id", "a", "--pr", "281", "--sha", "414d43eb"];
    assert_eq!(call(&home, &merged), done("merged repo=areka"));
    let before = state_bytes(&home);
    let written = || {
        let file = fs::metadata(home.join("state.json")).expect("状態ファイルが在る");
        file.modified().expect("更新時刻が読める")
    };
    let written_before = written();

    let (code, out, err) = waiting(&home, &merge_b);

    assert_eq!((code, err.as_str()), (0, ""), "{out:?}");
    let at = out
        .strip_prefix("granted merge repo=areka; last: PR#281 414d43eb x ")
        .unwrap_or_else(|| panic!("{out:?}"));
    // 残りは UTC の時刻（`2026-10-10T01:00:00Z`）と改行。
    assert!(at.len() == 21 && at.ends_with("Z\n"), "{out:?}");
    // 元の申し込みを引き継ぐ: 二重に並ばず、待ちの記録も残らない。
    assert_eq!(state_bytes(&home), before);
    // 待ちの記録を置いて消すこともしない（状態ファイルを書き直さず、ほかの待ちを読み直させない）。
    assert_eq!(written(), written_before);
}

#[test]
fn merge_wait_makes_the_request_before_waiting() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);

    // 参加もしていない a。申し込みが先に通り、机が空いているので直ちに番を受ける。
    let granted = done("granted merge repo=areka; last: none");
    assert_eq!(waiting(&home, &MERGE_A), granted);

    let state = read_state(&home);
    let holder = state.merge["areka"].holder.as_ref().expect("持ち主が居る");
    assert_eq!((holder.id.as_str(), holder.spec.as_str()), ("a", "x"));
    assert_eq!(state.waits, []);
    // 終わった待ちは居る印を握ったままにしない: 始め直しても 1 にならず、同じ答えで終わる。
    assert_eq!(waiting(&home, &MERGE_A), granted);
}

#[test]
fn loadtest_wait_is_granted_at_once_when_nobody_has_to_stop() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);

    let granted = done("granted load; stopped: none");
    assert_eq!(waiting(&home, &LOAD_A), granted);

    let state = read_state(&home);
    assert_eq!(state.load.holder.expect("持ち主が居る").id, "a");
    assert_eq!(state.waits, []);
    assert_eq!(waiting(&home, &LOAD_A), granted);
}

#[test]
fn a_second_merge_wait_for_the_same_id_is_1_and_changes_nothing() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    // 番はもう来ている: 印を確かめずに進むように壊れても、このテストは待ち続けずに赤になる。
    call(&home, &MERGE_A);
    // 1 つ目の待ちが走っている（その居る印をテストが握る）。
    let _first = hold_sign(&home, "a", WaitKind::Merge);
    let before = state_bytes(&home);

    // 通れば名前が書き替わる申し込み。印が取れないので、申し込みも通らない。
    let second = [&MERGE_A[..], &["--name", "other"]].concat();
    assert_eq!(waiting(&home, &second), already_running("merge"));

    assert_eq!(state_bytes(&home), before);
}

// ---- 見張り ----

#[test]
fn watch_is_1_while_its_presence_sign_is_held_and_touches_no_state() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    // 停止要請はもう出ている: 印を確かめずに進むように壊れても、このテストは待ち続けずに赤になる。
    put_not_working(&home, "stop-requested", C_QUEUED);
    let before = state_bytes(&home);
    let _held = hold_watches(&home, &["c"]);
    let _first = hold_sign(&home, "a", WaitKind::Watch);

    assert_eq!(call(&home, &WATCH_A), already_running("watch"));

    // 見張りの開始（参加）もしない: 状態ファイルは元のまま、ロックも読み物も作られない。
    assert_eq!(state_bytes(&home), before);
    assert_eq!(names(&home), ["alive", "impl-watch.log", "state.json"]);
}

#[test]
fn watch_for_a_stop_requested_participant_ends_at_once_with_who_and_where_to_read() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    put_not_working(&home, "stop-requested", C_QUEUED);
    let _held = hold_watches(&home, &["c"]);
    let told = format!("stop requested by c\n{}", details(&home));

    assert_eq!(call(&home, &WATCH_A), (0, told.clone(), String::new()));

    // 見張りは参加者の状態を変えない。終わった見張りの記録は残らず、2 行目の読み物は在る。
    let state = read_state(&home);
    let status = |id: &str| state.participants[id].status;
    assert_eq!(status("a"), ParticipantStatus::StopRequested);
    assert_eq!(status("c"), ParticipantStatus::Working);
    assert!(
        state.participants["a"].watch.is_some(),
        "見張りの開始が通る"
    );
    assert_eq!(state.waits, []);
    assert!(home.join("status.md").is_file());
    // 居る印も握ったままにしない: 立て直しても 1 にならない。
    assert_eq!(call(&home, &WATCH_A), (0, told, String::new()));
}

#[test]
fn watch_for_a_stopped_participant_ends_at_once_saying_what_to_run() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    put_not_working(
        &home,
        "stopped",
        r#"{ "holder": { "id": "c", "purpose": "負荷の計測", "stopped": ["a"] } }"#,
    );
    let _held = hold_watches(&home, &["c"]);
    let told = format!(
        "already stopped; run stopped --wait or resume\n{}",
        details(&home)
    );

    assert_eq!(call(&home, &WATCH_A), (0, told, String::new()));

    assert_eq!(
        read_state(&home).participants["a"].status,
        ParticipantStatus::Stopped
    );
}

// ---- 再開の待ち ----

#[test]
fn resume_without_a_record_is_3_and_joins_nobody() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    let resume = ["resume", "--id", "a"];

    assert_eq!(call(&home, &resume), gone("removed"));

    let state = read_state(&home);
    assert!(state.participants.is_empty(), "{state:?}");
    assert_eq!(state.waits, []);
    assert_eq!(call(&home, &resume), gone("removed"));
}

#[test]
fn resume_for_a_working_participant_ends_at_once_and_changes_nothing() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    // 見張りを立てていない a。待ちの呼び手は自分なので、自分の待ちには回収されない。
    call(&home, &MERGE_A);
    let before = state_bytes(&home);

    assert_eq!(call(&home, &["resume", "--id", "a"]), done("resumed"));

    // 待つだけ: 机も参加者も元のまま、待ちの記録も残らない。
    assert_eq!(state_bytes(&home), before);
}

#[test]
fn stopped_wait_for_a_working_participant_is_resumed_at_once_not_a_refusal() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);
    // 見張りを立てていない a（呼んだ識別は、同じ呼び出しでは回収されない）。
    call(&home, &MERGE_A);
    let stopped = ["stopped", "--id", "a"];
    // `--wait` 無しは「停止要請中でない」の断り。
    let refusal = "not applied: not asked to stop\n".to_owned();
    assert_eq!(call(&home, &stopped), (3, String::new(), refusal));
    let before = state_bytes(&home);

    assert_eq!(waiting(&home, &stopped), done("resumed"));

    assert_eq!(state_bytes(&home), before);
}

#[test]
fn stopped_wait_for_an_unknown_id_is_3() {
    let root = TempPath::under_target("impl-watch-cli-wait");
    let home = home_in(&root);

    assert_eq!(waiting(&home, &["stopped", "--id", "a"]), gone("removed"));

    assert!(read_state(&home).participants.is_empty());
}
