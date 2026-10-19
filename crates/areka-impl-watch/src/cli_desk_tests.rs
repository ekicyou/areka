//! 机の申し込みと返し（`request`・`return`・`loadrunning`）の入口の往復のテスト: 出力の 1 行と
//! 終了コード・断りの `not applied: …` と 3・`--wait` 付きの断りは待ちに入らず待ちの記録を
//! 残さないこと・`--desk` の形の誤りは何も読み書きしないこと。
//!
//! 本物の口（状態ファイル・ロックファイル・本物の時計）を、ワークツリーの `target\` の下の
//! 一時の置き場所に向けて通す。端末へ出る文が ASCII だけであることは、どの呼び出しでも
//! [`call`] が判定する。2 つ以上の識別を使うテストは、呼ぶ前に全員の見張りの居る印を握る。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::test_support::{call, done, entries, hold_watches, home_in, read_state, state_bytes};
use crate::state::{WaitKind, WaitRecord};

const PURPOSE: &str = "全体テスト";

/// 終了コード・標準出力・標準エラー。
type Told = (u8, String, String);

/// 机の申し込み（`desks` は `--desk` の値）。`extra` は後ろに足す引数（`--wait` など）。
fn request(home: &Path, id: &str, desks: &str, extra: &[&str]) -> (u8, String, String) {
    let words = ["request", "--id", id, "--repo", "areka", "--desk", desks];
    call(home, &[&words[..], &["--purpose", PURPOSE], extra].concat())
}

fn merge(home: &Path, id: &str, extra: &[&str]) -> (u8, String, String) {
    let words = ["merge", "--id", id, "--repo", "areka", "--spec", "x"];
    call(home, &[&words[..], extra].concat())
}

fn give_back(home: &Path, id: &str, desks: Option<&str>) -> (u8, String, String) {
    let named = desks.map_or(Vec::new(), |desks| vec!["--desk", desks]);
    call(home, &[&["return", "--id", id][..], &named].concat())
}

fn running(home: &Path, id: &str) -> (u8, String, String) {
    let words = ["loadrunning", "--id", id, "--repo", "areka"];
    call(home, &[&words[..], &["--purpose", PURPOSE]].concat())
}

/// 当てはまらなかった: 終了コード 3・標準出力は空・標準エラーにその 1 行。
fn not_applied(why: &str) -> (u8, String, String) {
    (3, String::new(), format!("not applied: {why}\n"))
}

/// 待ちの記録（識別, 種類）。
fn waits(home: &Path) -> Vec<(String, WaitKind)> {
    let key = |wait: &WaitRecord| (wait.id.clone(), wait.kind);
    read_state(home).waits.iter().map(key).collect()
}

/// 断りが `--wait` の有無によらず同じ 1 行と 3 で、状態ファイルを 1 バイトも変えず、待ちの記録を
/// 残さないこと（`--wait` 付きが待ちに入れば、本物の口の眠りに当たって panic する）。
fn refused_with_and_without_wait(home: &Path, send: &dyn Fn(&[&str]) -> Told, why: &str) {
    let before = state_bytes(home);
    for extra in [&[][..], &["--wait"]] {
        assert_eq!(send(extra), not_applied(why), "{extra:?}");
        assert_eq!(state_bytes(home), before, "{extra:?}: 状態が変わった");
        assert_eq!(waits(home), [], "{extra:?}: 待ちの記録が残った");
    }
}

// ---- 番と並び・返し ----

#[test]
fn a_test_desk_round_trip_prints_the_design_lines_and_exit_codes() {
    let root = TempPath::under_target("impl-watch-cli-desk");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b"]);

    assert_eq!(request(&home, "a", "test", &[]), done("granted test"));
    assert_eq!(request(&home, "b", "test", &[]), done("queued test pos=1"));
    // 同じ組の申し込み直しは二重にならない。
    assert_eq!(request(&home, "b", "test", &[]), done("queued test pos=1"));
    assert_eq!(read_state(&home).requests.len(), 1);

    assert_eq!(give_back(&home, "a", Some("test")), done("returned test"));
    // 返した同じ呼び出しで b に渡っている。持っていれば申し込み直しは直ちに番の 1 行。
    assert_eq!(request(&home, "b", "test", &[]), done("granted test"));
    assert_eq!(
        request(&home, "b", "test", &["--wait"]),
        done("granted test")
    );
    assert_eq!(waits(&home), []);
    assert_eq!(give_back(&home, "b", None), done("returned test"));
    assert_eq!(
        give_back(&home, "b", None),
        not_applied("holds no desk to return")
    );
}

#[test]
fn a_merge_holder_adds_the_test_desk_and_returns_only_it() {
    let root = TempPath::under_target("impl-watch-cli-desk");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a"]);

    assert_eq!(
        merge(&home, "a", &["--wait"]),
        done("granted merge repo=areka; last: none")
    );
    assert_eq!(
        request(&home, "a", "test", &["--wait"]),
        done("granted test")
    );
    // 名指しなしでも、マージの机は返さない。
    assert_eq!(give_back(&home, "a", None), done("returned test"));

    let state = read_state(&home);
    let holder = state.merge["areka"].holder.as_ref().map(|h| h.id.as_str());
    assert_eq!((holder, state.test), (Some("a"), None));
    assert_eq!(state.waits, []);
}

// ---- 断り（`--wait` の有無によらず待たずに 3） ----

#[test]
fn refused_requests_and_merges_are_3_and_never_enter_the_wait() {
    let root = TempPath::under_target("impl-watch-cli-desk");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b", "c"]);
    // a はマージの机と排他テストの机を持ち、b は排他テストの机を待ち、c はマージの机を待つ。
    merge(&home, "a", &[]);
    assert_eq!(request(&home, "a", "test", &[]), done("granted test"));
    assert_eq!(request(&home, "b", "test", &[]), done("queued test pos=1"));
    assert_eq!(
        merge(&home, "c", &[]),
        done("queued merge repo=areka pos=1")
    );

    // 2.8: 待っている申し込みと違う組。
    let b_sole = |extra: &[&str]| request(&home, "b", "test,sole", extra);
    refused_with_and_without_wait(
        &home,
        &b_sole,
        "a different desk request is already waiting",
    );
    // 4.2: 机を持ちながら、マージの持ち主の `test` のほかの組。
    let a_sole = |extra: &[&str]| request(&home, "a", "test,sole", extra);
    let held = "desks are held; only a merge holder may add the test desk";
    refused_with_and_without_wait(&home, &a_sole, held);
    let a_only_sole = |extra: &[&str]| request(&home, "a", "sole", extra);
    refused_with_and_without_wait(&home, &a_only_sole, held);
    // 4.8: マージの机を待つ者の申し込み。
    let c_test = |extra: &[&str]| request(&home, "c", "test", extra);
    let merge_waiting = "a merge request is waiting; request desks after the merge desk is granted";
    refused_with_and_without_wait(&home, &c_test, merge_waiting);
    // 4.7: 排他テストの机を持つ者の `merge`。
    let a_merge = |extra: &[&str]| merge(&home, "a", extra);
    let return_first = "return the test and sole desks before merge";
    refused_with_and_without_wait(&home, &a_merge, return_first);
    // 2.8（2026-10-18 の裁定）: 机の申し込みを待つ者の `merge`。
    let b_merge = |extra: &[&str]| merge(&home, "b", extra);
    let desk_waiting = "a desk request is waiting; merge after it is granted and returned";
    refused_with_and_without_wait(&home, &b_merge, desk_waiting);
}

#[test]
fn a_refused_request_wait_removes_the_record_of_a_killed_wait() {
    let root = TempPath::under_target("impl-watch-cli-desk");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a"]);
    assert_eq!(request(&home, "a", "test", &[]), done("granted test"));
    // 前の `request --wait` は殺され、その記録だけが残っている（居る印は解けている）。
    let mut state = read_state(&home);
    state.waits.push(WaitRecord {
        id: "a".to_owned(),
        kind: WaitKind::Request,
        repo: None,
        pid: 4242,
        since: 1,
    });
    let text = serde_json::to_string(&state).expect("状態を書ける");
    std::fs::write(home.join("state.json"), text).expect("書ける");

    let held = "desks are held; only a merge holder may add the test desk";
    assert_eq!(
        request(&home, "a", "test,sole", &["--wait"]),
        not_applied(held)
    );

    // 断りと同じ排他の中で、自分の待ちの記録が消えている。机はそのまま。
    assert_eq!(waits(&home), []);
    assert_eq!(
        read_state(&home).test.map(|holder| holder.id),
        Some("a".into())
    );
}

#[test]
fn refused_returns_and_running_records_are_3_and_change_nothing() {
    let root = TempPath::under_target("impl-watch-cli-desk");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "b"]);
    assert_eq!(running(&home, "a"), done("recorded running load"));
    let before = state_bytes(&home);

    // 4.3 の但し書き（2026-10-18 の裁定）: 走っている負荷テストは 2 つを 1 組で返す。
    assert_eq!(
        give_back(&home, "a", Some("test")),
        not_applied("a running load test returns test and sole together")
    );
    // 4.5: 持っていない机。
    let not_holder = not_applied("not the holder of that desk");
    assert_eq!(give_back(&home, "b", Some("test")), not_holder);
    assert_eq!(give_back(&home, "b", Some("sole")), not_holder);
    assert_eq!(
        give_back(&home, "b", None),
        not_applied("holds no desk to return")
    );
    // 5.6: どちらかの机に持ち主が居る。
    assert_eq!(
        running(&home, "b"),
        not_applied("the test desk or the sole desk already has a holder")
    );
    assert_eq!(state_bytes(&home), before);

    // 名指しなし（または 2 つを名指し）なら 1 組で返る。
    assert_eq!(give_back(&home, "a", None), done("returned test,sole"));
}

#[test]
fn loadrunning_by_a_merge_waiter_is_3_and_by_the_merge_holder_is_recorded() {
    let root = TempPath::under_target("impl-watch-cli-desk");
    let home = home_in(&root);
    let _held = hold_watches(&home, &["a", "c"]);
    merge(&home, "a", &[]);
    assert_eq!(
        merge(&home, "c", &[]),
        done("queued merge repo=areka pos=1")
    );
    let before = state_bytes(&home);

    // 5.7（2026-10-18 の裁定）: マージの机を待つ者の記録は断る。
    assert_eq!(
        running(&home, "c"),
        not_applied(
            "a merge request is waiting; record the running load after the merge desk is granted"
        )
    );
    assert_eq!(state_bytes(&home), before);
    // マージの机の持ち主は記録できる（5.5）。
    assert_eq!(running(&home, "a"), done("recorded running load"));
}

// ---- `--desk` の形の誤り ----

#[test]
fn a_malformed_desk_is_2_and_reads_or_writes_nothing() {
    let root = TempPath::under_target("impl-watch-cli-desk");
    let home = home_in(&root);
    for desks in ["test,test", "merge", ""] {
        for extra in [&[][..], &["--wait"]] {
            let (code, out, err) = request(&home, "a", desks, extra);
            assert_eq!((code, out.as_str()), (2, ""), "{desks:?} {extra:?}");
            assert!(err.starts_with("usage error: "), "{err:?}");
            // 置き場所の解決（フォルダを作る）にも進まない。
            assert!(!home.exists(), "{desks:?}: 置き場所を作った");
        }
    }
    assert_eq!(entries(root.path()), Vec::<PathBuf>::new());
}
