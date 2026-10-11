//! 本物の実行ファイルを子プロセスで立てて、机の往復を実時間で通す実機テスト（要件 12.5・`#[ignore]`）。
//!
//! 実行: `cargo test -p areka-impl-watch --test real -- --ignored --nocapture`（約 30 秒）
//! 短い形: `cargo test -p areka-impl-watch --test real -- --ignored --nocapture --skip a_waiter_reclaims`
//! （5〜8 秒）
//!
//! 常時テストは判断を偽の口で確かめ、本物の 1 秒の眠りには入らない。ここだけが、本物の
//! 実行ファイル・本物のロックファイル・本物の 1 秒の読み直し・本物のログのファイルを通す。
//! 30 秒ごとの周期の一回りを待つ 1 本（[`a_waiter_reclaims_a_dead_holder_on_its_periodic_round`]）
//! だけは約 30 秒掛かる（短い形はこの 1 本を外す）。
//!
//! 決まり:
//! - 置き場所は必ずワークツリーの `target\test-roots\` の下（[`Desk`]）。どの子プロセスにも
//!   `AREKA_IMPL_WATCH_HOME` を明示して渡す（開発者の機械には本物のユーザー環境変数が在る）。
//! - 立てた子プロセスは [`support::Running`] が持ち、落とすときに自分の子だけを止めて待つ
//!   （名前で探さない）。`Running` は [`Desk`] を借りているので、置き場所より先に落ちる。
//! - 待ちは全部上限つき（支えの `LIMIT`）。終わるはずの子が終わらなければ、待ち続けずに
//!   赤にする。
//! - テストは並んで走るので、テストごとに別の置き場所を使う（識別は置き場所ごとに独立）。
//! - コマンドは空白で区切った 1 行で書く（引数の値に空白を入れない）。
//! - 識別は小文字で書く（実行ファイルが小文字に寄せる）。大文字で渡すのは、寄せることを
//!   確かめる 1 本だけ。
//!
//! 置き場所・子プロセス・状態ファイルの読み取りの支えは `real/mod.rs` に在る（1 ファイル
//! 1,000 行の目安のために分けた。別のテストの入口にならないよう、フォルダの下に置く）。

#![cfg(windows)]

use std::fs::{self, File};
use std::io::Read;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// 支え（一時の置き場所・立てた子プロセス・状態ファイルの読み取り）。
#[cfg(test)]
#[path = "real/mod.rs"]
mod support;

use support::{
    Desk, HOME_ENV, assert_granted_after, count, done, gone, has_participant, has_wait, ok, read,
};

/// 本物の読み直し（1 秒）を 1 回は挟む長さ。「まだ終わっていない」を確かめる前に置く。
const ONE_POLL: Duration = Duration::from_millis(1200);
/// 机が空いていて、直前のマージの記録も無いときの、マージの番の 1 行。
const GRANTED: &str = "granted merge repo=areka; last: none\n";
/// 版の合わない状態ファイルを読んだときの失敗の文。
const MISMATCH: &str = "state file version mismatch: file has 2, this exe knows 1. Do not mix old and new exes; see doc/impl-watch.md\n";
/// 他へ許すのは読みだけ（消し＝置き換えを許さない）で開く、の指定。
const SHARE_READ: u32 = 1;
const SHARE_WRITE: u32 = 2;

/// 流れ ①②: 見張りを立て、マージの待ちが番で終わる。「済んだ」で見張りが 3 で終わる。
/// 待っている間、同じ識別・同じ種類の 2 つ目は 1 で、状態を変えない。識別は小文字に寄る:
/// 大文字で渡した `A` と小文字の `a` は、記録・居る印・出力のどこでも同じ参加者（要件 2.6）。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn a_merge_wait_ends_by_its_turn_and_merged_ends_the_watch() {
    let desk = Desk::new("impl-watch-real-merge");
    // 大文字で立てた見張りも、記録の鍵と居る印のファイルの名前は小文字。
    let mut watch_a = desk.waiter("a", "watch", "watch --id A --repo areka");
    let mut watch_b = desk.watch("b");
    let signs = fs::read_dir(desk.file("alive")).expect("居る印のフォルダを読める");
    let mut signs: Vec<_> = signs
        .map(|sign| sign.expect("読める").file_name())
        .collect();
    signs.sort();
    assert_eq!(signs, ["a.watch.lock", "b.watch.lock"]);

    // 見張りが走っている間、同じ識別の 2 つ目の見張りは 1（握った印を持ち続けている）。
    let before = desk.state_text();
    let busy = done(1, "", "a watch wait for a is already running\n");
    assert_eq!(desk.call("watch --id a --repo areka"), busy);
    assert_eq!(desk.call("watch --id A --repo areka"), busy);
    assert_eq!(desk.state_text(), before, "断られた見張りは状態を変えない");

    // 机が空いていれば、待ちは直ちに番で終わる（待ちの記録は置かない）。
    let merge_a = "merge --id A --repo areka --spec sa --wait";
    assert_eq!(desk.call(merge_a), ok(GRANTED));
    let state = desk.state();
    assert_eq!(state["merge"]["areka"]["holder"]["id"], "a", "{state}");
    assert!(!has_wait(&state, "a", "merge") && !has_participant(&state, "A"));

    // b は a の後ろに並んで待つ。待っている間、同じ識別の 2 つ目の待ちは 1。
    let merge_b = "merge --id b --repo areka --spec sb --bug --wait";
    let mut wait_b = desk.waiter("b", "merge", merge_b);
    let before = desk.state_text();
    let busy = done(1, "", "a merge wait for b is already running\n");
    assert_eq!(desk.call(merge_b), busy);
    assert_eq!(desk.state_text(), before, "断られた待ちは状態を変えない");
    std::thread::sleep(ONE_POLL);
    wait_b.assert_waiting_silently();
    watch_a.assert_waiting_silently();
    watch_b.assert_waiting_silently();

    // a が「済んだ」→ 同じ呼び出しで b の番が出て、b の待ちが直前のマージつきで終わる。
    let merged = desk.call("merged --id a --pr 281 --sha 414d43eb");
    assert_eq!(merged, ok("merged repo=areka\n"));
    assert_granted_after(&wait_b.end(), "PR#281 414d43eb sa");
    // 「済んだ」は離脱を兼ねる: a の見張りは 3 で終わる。b の見張りは机を持っても走り続ける。
    assert_eq!(watch_a.end(), gone("removed"));
    watch_b.assert_waiting_silently();

    // 持ち主でない「済んだ」は 3（a はもう居ない）。
    let refused = desk.call("merged --id a --pr 9 --sha 0000000");
    assert_eq!(refused, done(3, "", "not applied: not the merge holder\n"));

    let merged = desk.call("merged --id b --pr 282 --sha 226109e8");
    assert_eq!(merged, ok("merged repo=areka\n"));
    assert_eq!(watch_b.end(), gone("removed"));

    // 正常に終わった待ち・見張りは記録を残さない（要件 6.7）。
    let state = desk.state();
    assert_eq!(count(&state["waits"]), 0, "{state}");
    assert_eq!(state["participants"], json!({}), "{state}");
    assert_eq!(state["merge"]["areka"]["last"]["pr"], "282", "{state}");

    desk.assert_logged("watch", "Joined { id: \"a\" }");
    desk.assert_logged("merge", "MergeRequested { repo: \"areka\", id: \"b\"");
    desk.assert_logged("merged", "MergeGranted { repo: \"areka\", id: \"b\" }");
    desk.assert_logged("merged", "Left { id: \"a\", why: \"merged\" }");
    let ended = "WaitRemoved { id: \"b\", kind: Merge, why: \"ended\" }";
    desk.assert_logged("merge", ended);
}

/// 流れ ③: 負荷テストの待ち → 別の参加者の見張りが 0 で終わる → 「止まった」と再開の待ち →
/// 負荷テストの番 → 「済んだ」で再開。止まっている間に停止要請が出し直されたら、再開の待ちが
/// それを知らせて 0 で終わる（要件 5.16）。止まっている参加者の負荷テストの申し込みは、その
/// 参加者の状態も候補の番も変えない（要件 5.9）。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn a_load_test_stops_the_others_and_done_resumes_them() {
    let desk = Desk::new("impl-watch-real-stop");
    let status_md = desk.file("status.md");
    let mut watch_a = desk.watch("a");
    let named = "watch --id b --repo areka --name 作業B";
    let mut watch_b = desk.waiter("b", "watch", named);
    // 負荷テストの机はマシンに 1 つ: 別のリポジトリの c の申し込みで、areka の a と b が止まる。
    let mut watch_c = desk.waiter("c", "watch", "watch --id c --repo pasta");
    let load = "loadtest --id c --repo pasta --purpose 計測5回 --wait";
    let mut load_c = desk.waiter("c", "load", load);

    // a と b の見張りが停止要請で終わる。端末には識別と読み物の道筋だけ（内容は読み物に在る）。
    let told = format!("stop requested by c\ndetails: {}\n", status_md.display());
    assert_eq!(watch_a.end(), ok(&told));
    assert_eq!(watch_b.end(), ok(&told));
    let summary = desk.status();
    let line = "\nb areka stop-requested";
    assert!(summary.contains(line), "{summary}");
    let reading = read(&status_md);
    let both = reading.contains("計測5回") && reading.contains("作業B");
    assert!(both, "{reading}");

    // 停止要請中の `resume` は待たない: 誰の停止要請かを添えて直ちに 0 で終わり、状態を変えない
    // （「止まった」を記録するのは `stopped`）。
    let before = desk.state_text();
    let again = ok("stop requested again by c\n");
    assert_eq!(desk.call("resume --id b"), again);
    assert_eq!(desk.state_text(), before, "resume は状態を変えない");

    // b が「止まった」。a がまだなので c の番は来ない。再開の待ちが走っている間、同じ識別の
    // 2 つ目（`resume` も `stopped --wait` も）は 1 で、状態を変えない。
    let stopped = "stopped --id b --wait";
    let mut stopped_b = desk.waiter("b", "resume", stopped);
    let before = desk.state_text();
    let busy = done(1, "", "a resume wait for b is already running\n");
    assert_eq!(desk.call("resume --id b"), busy);
    assert_eq!(desk.call(stopped), busy);
    assert_eq!(desk.state_text(), before, "断られた待ちは状態を変えない");
    assert_eq!(desk.state()["participants"]["b"]["status"], "stopped");

    // 止まっている間に停止要請を取り消しても、c の申し込みが残っているので、同じ呼び出しの中で
    // 出し直される。b の見張りはもう終わっている: 知らせるのは再開の待ちで、0 で終わる。
    assert_eq!(desk.call("unstop"), ok("unstopped n=2\n"));
    assert_eq!(stopped_b.end(), again);
    let state = desk.state();
    let status_b = &state["participants"]["b"]["status"];
    assert_eq!(status_b, "stop-requested", "{state}");
    assert!(!has_wait(&state, "b", "resume"), "{state}");
    // もう一度「止まった」と言う（前の待ちは終わっているので、2 本は重ならない）。
    let mut stopped_b = desk.waiter("b", "resume", stopped);

    // 止まっている b が負荷テストを申し込んでも、b は「止まった」のまま並ぶだけ。
    let queued = desk.call("loadtest --id b --repo areka --purpose p");
    assert_eq!(queued, ok("queued load pos=2\n"));
    let state = desk.state();
    assert_eq!(state["participants"]["b"]["status"], "stopped", "{state}");
    std::thread::sleep(ONE_POLL);
    stopped_b.assert_waiting_silently();
    load_c.assert_waiting_silently();

    // a も「止まった」→ 同じ呼び出しで c の番が出て、c の待ちが止まった識別つきで終わる
    // （b の申し込みは c の番を止めない）。
    let mut stopped_a = desk.waiter("a", "resume", "stopped --id a --wait");
    assert_eq!(load_c.end(), ok("granted load; stopped: a, b\n"));
    // 負荷テストの候補・持ち主に停止要請は出ない: c の見張りは走り続ける。
    watch_c.assert_waiting_silently();
    let state = desk.state();
    assert_eq!(state["participants"]["b"]["status"], "stopped", "{state}");
    assert_eq!(state["load"]["holder"]["id"], "c", "{state}");

    // 殺された再開の待ちの記録は、次の状態を変える呼び出しが消す。待ちは `resume` で始め直せる。
    stopped_b.kill();
    let what = "殺した再開の待ちの記録が消える";
    desk.tick_until(what, |state| !has_wait(state, "b", "resume"));
    let absent = "WaitRemoved { id: \"b\", kind: Resume, why: \"absent\" }";
    desk.assert_logged("tick", absent);
    let mut resume_b = desk.waiter("b", "resume", "resume --id b");
    assert_eq!(desk.call("cancel --id b"), ok("cancelled\n"));
    std::thread::sleep(ONE_POLL);
    resume_b.assert_waiting_silently();

    // 持ち主でない「済んだ」は 3。持ち主の「済んだ」で a と b が再開する。
    let refused = done(3, "", "not applied: not the load-test holder\n");
    assert_eq!(desk.call("loaddone --id b"), refused);
    assert_eq!(desk.call("loaddone --id c"), ok("load done\n"));
    assert_eq!(resume_b.end(), ok("resumed\n"));
    assert_eq!(stopped_a.end(), ok("resumed\n"));

    // 再開した直後（見張りを立て直す前）の参加者は回収されない。
    assert_eq!(desk.call("tick"), ok("tick\n"));
    let summary = desk.status();
    let line = "\nb areka working awaiting-watch\n";
    assert!(summary.contains(line), "{summary}");
    // 見張りを立て直すと印が消える。
    let mut watch_b = desk.watch("b");
    let what = "b の見張り待ちの印が消える";
    desk.until(what, |state| {
        state["participants"]["b"]["awaiting_watch_since"].is_null()
    });

    assert_eq!(desk.call("leave --id c"), ok("left\n"));
    assert_eq!(watch_c.end(), gone("removed"));
    assert_eq!(desk.call("leave --id b"), ok("left\n"));
    assert_eq!(watch_b.end(), gone("removed"));

    desk.assert_logged("loadtest", "StopRequested { id: \"b\", by: \"c\" }");
    desk.assert_logged("unstop", "StopRequested { id: \"b\", by: \"c\" }");
    desk.assert_logged("stopped", "Stopped { id: \"b\" }");
    let granted = "LoadGranted { id: \"c\", stopped: [\"a\", \"b\"] }";
    desk.assert_logged("stopped", granted);
    desk.assert_logged("loaddone", "Resumed { id: \"b\", why: \"no-load\" }");
}

/// 流れ ④: 見張りを殺すと、次の状態を変える呼び出しがその参加者を回収し、同じ呼び出しで
/// 次の番が出る。殺された待ちの記録も消え、始め直した待ちは元の申し込みを引き継ぐ。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn a_killed_watch_is_reclaimed_by_the_next_call() {
    let desk = Desk::new("impl-watch-real-reclaim");
    let mut watch_a = desk.watch("a");
    assert_eq!(
        desk.call("merge --id a --repo areka --spec sa"),
        ok(GRANTED)
    );
    let mut watch_b = desk.watch("b");
    let merge_b = "merge --id b --repo areka --spec sb --wait";
    let mut wait_b = desk.waiter("b", "merge", merge_b);
    let mut watch_c = desk.watch("c");
    let merge_c = "merge --id c --repo areka --spec sc --wait";
    let mut wait_c = desk.waiter("c", "merge", merge_c);

    // 持ち主 a の見張りが落ちる。`status` は読むだけ: 「居ない」の印を付け、回収はしない。
    let before = desk.state_text();
    watch_a.kill();
    let line = "\na areka working absent\n";
    desk.status_until("a に absent の印が付く", |summary| {
        summary.contains(line)
    });
    std::thread::sleep(ONE_POLL);
    wait_b.assert_waiting_silently();
    assert_eq!(desk.state_text(), before, "status は状態を変えない");

    // c の待ちも落ちる（c の見張りは生きている）。次の状態を変える呼び出しが両方を片付ける。
    wait_c.kill();
    let what = "a の回収と、c の待ちの記録の抹消";
    let state = desk.tick_until(what, |state| {
        !has_participant(state, "a") && !has_wait(state, "c", "merge")
    });
    // 回収で空いた机の次の番は、同じ呼び出しで出る。
    assert_eq!(wait_b.end(), ok(GRANTED));
    let areka = &state["merge"]["areka"];
    assert_eq!(areka["holder"]["id"], "b", "{state}");
    // 見張りの生きている c は回収されず、申し込みも残る。
    assert_eq!(count(&areka["queue"]), 1, "{state}");
    assert_eq!(areka["queue"][0]["id"], "c", "{state}");

    // 回収は、ログと状態の確認の両方で読める。
    desk.assert_logged("tick", "Reclaimed { id: \"a\", why: \"watch absent\" }");
    desk.assert_logged("tick", "MergeGranted { repo: \"areka\", id: \"b\" }");
    let absent = "WaitRemoved { id: \"c\", kind: Merge, why: \"absent\" }";
    desk.assert_logged("tick", absent);
    let summary = desk.status();
    assert!(!summary.contains("\na "), "{summary}");
    let reading = read(&desk.file("status.md"));
    let row = "| reclaimed | a | watch absent |";
    assert!(reading.contains(row), "{reading}");

    // 待ちを始め直すと、元の申し込みを引き継ぐ（二重に並ばない）。
    let mut wait_c = desk.waiter("c", "merge", merge_c);
    let state = desk.state();
    assert_eq!(count(&state["merge"]["areka"]["queue"]), 1, "{state}");
    let merged = desk.call("merged --id b --pr 7 --sha abcdef0");
    assert_eq!(merged, ok("merged repo=areka\n"));
    assert_granted_after(&wait_c.end(), "PR#7 abcdef0 sb");
    assert_eq!(watch_b.end(), gone("removed"));

    assert_eq!(desk.call("leave --id c"), ok("left\n"));
    assert_eq!(watch_c.end(), gone("removed"));
}

/// 要件 7.4: 記録に載っているプロセス番号が生きている別のプロセスのものでも、見張りの印
/// （ロック）が無ければ「居る」と見なさない。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn a_recorded_pid_of_another_living_process_does_not_count_as_present() {
    let desk = Desk::new("impl-watch-real-pid");
    // このテストのプロセス（生きている）の番号を、見張りの番号として書いておく。
    let pid = std::process::id();
    let state = json!({
        "version": 1,
        "participants": { "z": {
            "id": "z", "name": "z", "repo": "areka", "status": "working", "since": 1,
            "watch": { "pid": pid, "since": 1 }
        } },
        "waits": [ { "id": "z", "kind": "watch", "repo": "areka", "pid": pid, "since": 1 } ]
    });
    desk.put_state(&state.to_string());

    assert_eq!(desk.call("tick"), ok("tick\n"));
    let state = desk.state();
    assert!(!has_participant(&state, "z"), "{state}");
    assert_eq!(count(&state["waits"]), 0, "{state}");
    desk.assert_logged("tick", "Reclaimed { id: \"z\", why: \"watch absent\" }");
}

/// 流れ ⑤: `clear` で、走っていた見張り・マージの待ち・負荷テストの待ち・再開の待ちが
/// 全部 3 で終わる。消す前の状態は別名で残る。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn clear_ends_every_running_wait_and_watch_with_3() {
    let desk = Desk::new("impl-watch-real-clear");
    let mut watch_a = desk.watch("a");
    assert_eq!(
        desk.call("merge --id a --repo areka --spec sa"),
        ok(GRANTED)
    );
    let mut watch_b = desk.watch("b");
    let merge_b = "merge --id b --repo areka --spec sb --wait";
    let mut wait_b = desk.waiter("b", "merge", merge_b);
    let mut watch_c = desk.watch("c");
    let mut watch_l = desk.watch("l");

    // l の負荷テストの申し込みで止まるのは c だけ（マージの持ち主 a とマージ待ちの b は対象外）。
    let load = "loadtest --id l --repo areka --purpose p --wait";
    let mut load_l = desk.waiter("l", "load", load);
    let status_md = desk.file("status.md");
    let told = format!("stop requested by l\ndetails: {}\n", status_md.display());
    assert_eq!(watch_c.end(), ok(&told));
    let mut stopped_c = desk.waiter("c", "resume", "stopped --id c --wait");
    // マージの持ち主が「済んだ」と言うまで、負荷テストの番は来ない。
    std::thread::sleep(ONE_POLL);
    let state = desk.state();
    assert!(state["load"]["holder"].is_null(), "{state}");
    assert_eq!(state["participants"]["c"]["status"], "stopped", "{state}");
    assert_eq!(state["participants"]["a"]["status"], "working", "{state}");
    assert_eq!(state["participants"]["b"]["status"], "working", "{state}");
    let (watches, requests) = (
        [&mut watch_a, &mut watch_b, &mut watch_l, &mut stopped_c],
        [&mut wait_b, &mut load_l],
    );
    for running in watches.into_iter().chain(requests) {
        running.assert_waiting_silently();
    }

    let cleared = desk.call("clear");
    let backup = cleared.out.strip_prefix("cleared; backup: ");
    let backup = backup.and_then(|rest| rest.strip_suffix('\n'));
    let backup = backup.unwrap_or_else(|| panic!("{cleared:?}"));
    assert_eq!((cleared.code, cleared.err.as_str()), (0, ""), "{cleared:?}");
    let name = desk.file("state.json.cleared-20");
    assert!(backup.starts_with(&*name.to_string_lossy()), "{backup}");
    let kept: Value = serde_json::from_str(&read(Path::new(backup))).expect("退避は読める");
    assert!(has_participant(&kept, "a"), "{kept}");

    // 参加者の記録を待つ見張り・再開の待ちは `removed`、申し込みを待つ待ちは `request gone`。
    assert_eq!(watch_a.end(), gone("removed"));
    assert_eq!(watch_b.end(), gone("removed"));
    assert_eq!(watch_l.end(), gone("removed"));
    assert_eq!(stopped_c.end(), gone("removed"));
    assert_eq!(wait_b.end(), gone("request gone"));
    assert_eq!(load_l.end(), gone("request gone"));

    // 残るのは版と「消した記録」だけ。
    let state = desk.state();
    assert_eq!(state["version"], 1, "{state}");
    assert_eq!(state["participants"], json!({}), "{state}");
    assert_eq!(state["merge"], json!({}), "{state}");
    assert_eq!(state["load"], json!({ "holder": null, "queue": [] }));
    assert_eq!(count(&state["waits"]), 0, "{state}");
    assert_eq!(count(&state["recent"]), 1, "{state}");
    assert_eq!(state["recent"][0]["kind"], "cleared", "{state}");
    assert_eq!(state["recent"][0]["detail"], backup, "{state}");
    desk.assert_logged("clear", "Cleared { backup: Some(");
    let summary = desk.status();
    let first = "participants=0 load-holder=none load-queued=0 merging=0 merge-queued=0 not-working=0 waits=0\n";
    assert!(summary.starts_with(first), "{summary}");
}

/// 要件 8.1: 同時に始まった 8 本の見張りの登録が、1 本も失われない。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn simultaneous_changes_are_all_kept() {
    let desk = Desk::new("impl-watch-real-together");
    let ids = ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7"];
    // 記録を待たずに続けて立てる。
    let start = |id: &&str| desk.start(&format!("watch --id {id} --repo areka"));
    let mut watches: Vec<_> = ids.iter().map(start).collect();
    let recorded =
        |state: &Value, id: &&str| has_participant(state, id) && has_wait(state, id, "watch");
    let state = desk.until("8 本の見張りが全部載る", |state| {
        ids.iter().all(|id| recorded(state, id))
    });
    assert_eq!(count(&state["waits"]), ids.len(), "{state}");
    // 8 本目の登録は、読み物をまだ書いている途中かもしれない。
    desk.settle();
    for watch in &mut watches {
        watch.assert_waiting_silently();
    }
    desk.assert_no_temp_files();
}

/// 流れ ⑥（要件 8.3）: 別のプロセス（このテスト）が状態ファイルを開いたままでも置き換え書きが
/// 通り、読む側は書きかけを見ない。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn replacing_goes_through_while_another_process_holds_the_state_file_open() {
    let desk = Desk::new("impl-watch-real-open");
    let _watch_a = desk.watch("a");
    let state_json = desk.file("state.json");
    let (merge, cancel) = ("merge --id a --repo areka --spec sa", "cancel --id a");

    // ふつうの読み手（Rust の `File::open`＝読み・書き・消しを他へ許して開く）が開いたまま。
    let mut reader = File::open(&state_json).expect("状態ファイルを開ける");
    assert_eq!(desk.call(merge), ok(GRANTED));
    assert_eq!(desk.state()["merge"]["areka"]["holder"]["id"], "a");
    // 開いたままの手は、置き換えられる前の中身を最後まで読む（書きかけも、新旧の混ざりも無い）。
    let mut old = String::new();
    let read_old = reader.read_to_string(&mut old);
    read_old.expect("開いたままの手で読める");
    let old: Value = serde_json::from_str(&old).expect("古い中身は丸ごとの JSON");
    assert_eq!(old["merge"], json!({}), "{old}");
    drop(reader);

    // 消しを他へ許さずに開いたままの読み手が居る間は、置き換えが通らない（20 ms × 5 回の試しの
    // 後に 1）。黙って成功にはならず、元のファイルは無傷で、一時ファイルも残らない。読み手が
    // 閉じれば通る。
    for share in [SHARE_READ | SHARE_WRITE, SHARE_READ] {
        let mut options = File::options();
        let reader = options.read(true).share_mode(share).open(&state_json);
        let reader = reader.expect("状態ファイルを開ける");
        let before = desk.state_text();
        let refused = desk.call(cancel);
        let told = refused.err.starts_with("io write state.json: ") && refused.err.is_ascii();
        assert_eq!((refused.code, refused.out.as_str()), (1, ""), "{refused:?}");
        assert!(told, "share={share} {refused:?}");
        assert_eq!(desk.state_text(), before, "share={share}");
        desk.assert_no_temp_files();
        drop(reader);
        assert_eq!(desk.call(cancel), ok("cancelled\n"));
        assert_eq!(desk.call(merge), ok(GRANTED));
    }
    let failed = "[store] state change failed error=io write state.json: ";
    assert_eq!(desk.log_lines("ERROR", failed), 2, "{}", desk.log());

    // 読み手が中身を読み続ける間に 20 回置き換える。読みは 1 度も失敗せず、書きかけも見ない
    // （実行ファイルの読むだけの口と同じ `fs::read`。置き換えと重なった読みに試し直しは要らない）。
    let stop = AtomicBool::new(false);
    let (reads, failures) = std::thread::scope(|scope| {
        let reading = scope.spawn(|| {
            let (mut reads, mut failures) = (0_u32, Vec::new());
            while !stop.load(Ordering::Relaxed) {
                reads += 1;
                match fs::read(&state_json) {
                    Ok(bytes) if serde_json::from_slice::<Value>(&bytes).is_ok() => {}
                    Ok(bytes) => failures.push(format!("torn read of {} bytes", bytes.len())),
                    Err(err) => failures.push(format!("{:?}: {err}", err.kind())),
                }
            }
            (reads, failures)
        });
        for round in 0..20 {
            let line = if round % 2 == 0 { cancel } else { merge };
            assert_eq!(desk.call(line).code, 0, "round {round}");
        }
        stop.store(true, Ordering::Relaxed);
        reading.join().expect("読み手が終わる")
    });
    eprintln!("[measure] reads overlapping 20 replaces: {reads}, failures: {failures:?}");
    assert_eq!(failures, Vec::<String>::new(), "reads={reads}");
    desk.assert_no_temp_files();
}

/// 流れ ⑦（要件 10.1〜10.3）: 置き場所の下のログに、出来事の行（コマンド・識別・変化）と失敗の
/// 行が、プロセスをまたいで追記される。色の制御文字は無い。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn the_log_keeps_event_lines_and_failure_lines_across_processes() {
    const OTHER_VERSION: &str = "{\"version\": 2}";
    let desk = Desk::new("impl-watch-real-log");
    let state_json = desk.file("state.json");

    // 状態ファイルが無ければ作り、そのことを残す。
    assert_eq!(desk.call("tick"), ok("tick\n"));
    let first = desk.log();
    assert_eq!(first.lines().count(), 1, "{first}");
    desk.assert_logged("tick", "Recovered { backup: None }");
    let mut watch_a = desk.watch("a");
    desk.assert_logged("watch", "Joined { id: \"a\" }");
    desk.assert_logged("watch", "WaitRegistered { id: \"a\", kind: Watch }");

    // 版の合わない状態ファイル: 読まず、上書きせず、失敗をログに残して 1。走っていた見張りも、
    // 読めなくなった時点で 1 で終わる（黙って待ち続けない）。
    desk.put_state(OTHER_VERSION);
    assert_eq!(watch_a.end(), done(1, "", MISMATCH));
    assert_eq!(desk.call("tick"), done(1, "", MISMATCH));
    assert_eq!(desk.call("status"), done(1, "", MISMATCH));
    assert_eq!(read(&state_json), OTHER_VERSION);
    let mismatch = "error=state file version mismatch: file has 2, this exe knows 1";
    let changing = format!("[store] state change failed {mismatch}");
    let reading = format!("[store] read failed {mismatch}");
    assert_eq!(desk.log_lines("ERROR", &changing), 1, "{}", desk.log());
    // 見張りの読み直しと `status` の 1 行ずつ。
    assert_eq!(desk.log_lines("ERROR", &reading), 2, "{}", desk.log());

    // `clear` の失敗（消しを他へ許さない読み手が開いたままで、退避できない）も残る。
    let mut options = File::options();
    let reader = options.read(true).share_mode(SHARE_READ).open(&state_json);
    let reader = reader.expect("状態ファイルを開ける");
    let refused = desk.call("clear");
    let told = "io set aside state.json: ";
    assert_eq!((refused.code, refused.out.as_str()), (1, ""), "{refused:?}");
    assert!(
        refused.err.starts_with(told) && refused.err.is_ascii(),
        "{refused:?}"
    );
    drop(reader);
    let clearing = format!("[store] clear failed error={told}");
    assert_eq!(desk.log_lines("ERROR", &clearing), 1, "{}", desk.log());
    assert_eq!(read(&state_json), OTHER_VERSION);

    // `clear` は版の合わないファイルも読まずに退避して、空からやり直せる。
    let cleared = desk.call("clear");
    assert_eq!((cleared.code, cleared.err.as_str()), (0, ""), "{cleared:?}");
    let backups = desk.files_named("state.json.cleared-");
    assert_eq!(backups.len(), 1, "{backups:?}");
    assert_eq!(read(&backups[0]), OTHER_VERSION);
    desk.assert_logged("clear", "Cleared { backup: Some(");

    // 壊れた状態ファイル: 別名で残して空から始め、そのことを警告の行で残す。
    desk.put_state("not json");
    assert_eq!(desk.call("tick"), ok("tick\n"));
    let broken = desk.files_named("state.json.broken-");
    assert_eq!(broken.len(), 1, "{broken:?}");
    assert_eq!(read(&broken[0]), "not json");
    let set_aside =
        "[store] state file was broken; set aside command=\"tick\" event=Recovered { backup: Some(";
    assert_eq!(desk.log_lines("WARN", set_aside), 1, "{}", desk.log());
    let reading = read(&desk.file("status.md"));
    let row = "| recovered | - | backed up: ";
    assert!(reading.contains(row), "{reading}");

    // 追記（最初の行が頭に残っている）・色の制御文字なし・どの行も時刻で始まり出どころを持つ。
    let log = desk.log();
    assert!(log.starts_with(&first), "{log}");
    assert!(!log.contains('\u{1b}'), "{log}");
    let shaped = |line: &str| line.starts_with("20") && line.contains(" areka_impl_watch::");
    assert!(log.lines().all(shaped), "{log}");
}

/// 読み物（`status.md`）が書けなくても、状態を変える 1 回・状態の確認・全部消すのどれも、結果と
/// 終了コードを変えずに、ログへ警告を 1 行ずつ残す（要件 10.2 の例外。正本は状態ファイル）。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn an_unwritable_status_md_changes_no_result_and_leaves_a_warning() {
    let desk = Desk::new("impl-watch-real-reading");
    // 読み物の名前をフォルダで塞ぐ（置き換えの改名が通らない）。
    let status_md = desk.file("status.md");
    fs::create_dir_all(&status_md).expect("読み物の名前をフォルダで塞げる");
    let warned = || desk.log_lines("WARN", ": [store] status.md not written");

    // 状態を変える 1 回: 結果の行は標準出力・標準エラーは空・変化は状態ファイルに入っている。
    let merge = "merge --id a --repo areka --spec sa";
    assert_eq!(desk.call(merge), ok(GRANTED));
    assert_eq!(desk.state()["merge"]["areka"]["holder"]["id"], "a");
    assert_eq!(warned(), 1, "{}", desk.log());
    // 状態の確認: 要約と、読み物の道筋の行を出す。
    let summary = desk.status();
    let path = format!("status: {}\n", status_md.display());
    let shown = summary.starts_with("participants=1 ") && summary.ends_with(&path);
    assert!(shown, "{summary}");
    assert_eq!(warned(), 2, "{}", desk.log());
    // 全部消す: 退避の道筋の行を出す。
    let cleared = desk.call("clear");
    assert_eq!((cleared.code, cleared.err.as_str()), (0, ""), "{cleared:?}");
    assert!(cleared.out.starts_with("cleared; backup: "), "{cleared:?}");
    assert_eq!(desk.state()["participants"], json!({}));
    assert_eq!(warned(), 3, "{}", desk.log());

    // 失敗の行は 1 本も無い。読み物の名前はフォルダのままで、書きかけも残らない。
    assert_eq!(desk.log_lines("ERROR", ""), 0, "{}", desk.log());
    assert!(status_md.is_dir());
    desk.assert_no_temp_files();
}

/// 置き場所の環境変数が無い・空・絶対パスでないなら、警告を出して 1 で終わり、何も作らない
/// （要件 1.3・1.6）。使い方の誤り（2）と `--help`（0）も置き場所に触らない。
#[test]
#[ignore = "実機: 本物の実行ファイルを子プロセスで立てる。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn nothing_is_created_without_a_home_or_for_a_usage_error() {
    let desk = Desk::new("impl-watch-real-nohome");
    let cwd = desk.root.child("cwd");
    fs::create_dir(&cwd).expect("子のカレントを作れる");
    // 子のカレントは空のフォルダ（既定の場所や相対の場所へ倒れて何かを作れば、ここに残る）。
    let created = || fs::read_dir(&cwd).expect("フォルダを読める").count();

    // 相対の値は、断られなければ子のカレント（このテストの一時フォルダの中）にフォルダができる。
    for (value, said) in [
        (None, "is not set or empty"),
        (Some(""), "is not set or empty"),
        (Some("rel-home"), "must be an absolute path"),
    ] {
        let mut command = desk.command("tick");
        command.current_dir(&cwd);
        match value {
            // このテストのプロセスが本物のユーザー環境変数を継いでいても、子には渡さない。
            None => command.env_remove(HOME_ENV),
            Some(value) => command.env(HOME_ENV, value),
        };
        let end = desk.spawn(command).end();
        let told = end.err.starts_with(&format!("{HOME_ENV} {said}"))
            && (1..=2).contains(&end.err.lines().count());
        assert_eq!((end.code, end.out.as_str()), (1, ""), "{value:?} {end:?}");
        assert!(told && end.err.is_ascii(), "{value:?} {end:?}");
        assert_eq!(created(), 0, "{value:?}");
    }

    let usage = desk.call("merge --id a");
    let told = usage.err.starts_with("usage error: merge: missing --repo");
    assert_eq!((usage.code, usage.out.as_str()), (2, ""), "{usage:?}");
    assert!(told && usage.err.lines().count() == 1, "{usage:?}");
    let help = desk.call("--help");
    assert_eq!((help.code, help.err.as_str()), (0, ""), "{help:?}");
    for code in ["\n  0  ", "\n  1  ", "\n  2  ", "\n  3  "] {
        assert!(help.out.contains(code), "{help:?}");
    }
    assert!(!desk.home().exists(), "置き場所は作られない");
}

/// 待っている者しか居ないとき、落ちた持ち主は、待ち・見張りの 30 秒ごとの周期の一回りが回収
/// する（ほかの呼び出しを待たない）。本物の 1 秒の眠りを 30 回通すので 30 秒あまり掛かる。
#[test]
#[ignore = "実機・遅い（約 30 秒）: 30 秒ごとの周期の一回りを待つ。cargo test -p areka-impl-watch --test real -- --ignored --nocapture"]
fn a_waiter_reclaims_a_dead_holder_on_its_periodic_round() {
    let desk = Desk::new("impl-watch-real-round");
    let mut watch_a = desk.watch("a");
    assert_eq!(
        desk.call("merge --id a --repo areka --spec sa"),
        ok(GRANTED)
    );
    let mut watch_b = desk.watch("b");
    let merge_b = "merge --id b --repo areka --spec sb --wait";
    let mut wait_b = desk.waiter("b", "merge", merge_b);

    // 持ち主 a の見張りが落ちる。この後、テストからは何も呼ばない。
    watch_a.kill();
    let killed = Instant::now();
    assert_eq!(wait_b.end_within(Duration::from_secs(60)), ok(GRANTED));
    let waited = killed.elapsed();
    // 回収したのは、b の見張りか待ちの周期の一回り（始めてから 30 回目の眠りの後）。
    let reclaimed = "Reclaimed { id: \"a\", why: \"watch absent\" }";
    let by_round = desk.logged("watch", reclaimed) || desk.logged("merge", reclaimed);
    assert!(by_round, "{}", desk.log());
    assert!(waited > Duration::from_secs(20), "{waited:?}");
    watch_b.assert_waiting_silently();
}
