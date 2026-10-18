//! 窓口の待ち・閉じ方・止め方・環境を偽の子で確かめる（spec: areka-P0-real-run-mcp-kit タスク 5.1・
//! 要件 1.5〜1.7・3.1・3.2・3.4・3.5・5.1〜5.6・8.2・9.1・10.1）。
//!
//! 公開の API だけを使う。子は `real-run-fake-areka`（手順は `REAL_RUN_FAKE_STEPS`）。試験は並んで走るので、
//! 試験の側の環境変数は変えない。わざと落とす試験は、残った回の置き場を最後に自分で消す。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use real_run_kit::{EndedBy, RealRun, Run};
use temp_path_kit::TempPath;

/// 偽の子を手順つきで立てる指定（32bit の仲介は揃えない）。
fn fake(steps: &str) -> RealRun {
    RealRun::new(env!("CARGO_BIN_EXE_real-run-fake-areka"))
        .without_host32_helper()
        .env("REAL_RUN_FAKE_STEPS", steps)
}

/// その番号のプロセスが偽の子として生きているか（`tasklist` の行に実行ファイルの名前が在るか）。
fn alive(pid: u32) -> bool {
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .expect("tasklist を起こせない");
    String::from_utf8_lossy(&out.stdout).contains("real-run-fake-areka")
}

/// `f` の中の panic を捕まえて文を返す。`f` は立てた子の番号と回の置き場を `seen` へ控える。
/// 残った回の置き場は消す（落ちた回の証拠＝6.1 は 5.3 の試験が確かめる）。
fn expect_panic(f: impl FnOnce(&mut Option<(u32, PathBuf)>)) -> (String, u32) {
    let mut seen = None;
    let err = catch_unwind(AssertUnwindSafe(|| f(&mut seen))).expect_err("panic するはず");
    let (pid, run_dir) = seen.expect("子を立てる前に落ちた");
    let _ = std::fs::remove_dir_all(&run_dir);
    let text = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    (text, pid)
}

fn note(run: &Run, seen: &mut Option<(u32, PathBuf)>) {
    *seen = Some((run.pid(), run.run_dir().to_path_buf()));
}

/// 3.2: 出力に現れた文字列を待って見つける。
#[test]
fn wait_for_finds_string() {
    let mut run = fake("out:ここにいる;window").spawn();
    run.wait_for("ここにいる").expect("子は生きている");
    assert_eq!(run.close().by, EndedBy::UserClose);
}

/// 3.5: 待ちの途中で子が終われば、締切切れでなく子の終わりと終了コード。
#[test]
fn child_exit_during_wait_is_reported_with_code() {
    let mut run = fake("exit:3").spawn();
    let exited = run.wait_for("現れない").expect_err("子が先に終わる");
    assert_eq!(exited.status.code(), Some(3));
}

/// 3.4・5.4: 生き続ける子に短い締切で panic し、その後に子は止まっている。
#[test]
fn hang_hits_deadline_and_child_is_stopped() {
    let (text, pid) = expect_panic(|seen| {
        let mut run = fake("hang").deadline(Duration::from_secs(1)).spawn();
        note(&run, seen);
        let _ = run.wait_for("現れない");
    });
    assert!(
        text.contains("現れない"),
        "待っていた文字列が文に無い: {text}"
    );
    assert!(!alive(pid), "締切の後も子（PID {pid}）が生きている");
}

/// 3.1: 大量の出力でも子は詰まらずに終わる。
#[test]
fn flood_does_not_block_child() {
    let mut run = fake("flood:4096").spawn();
    let ended = run.wait_exit();
    assert_eq!(ended.status.code(), Some(0));
    assert!(run.stdout().len() >= 4096 * 1024, "出力が足りない");
}

/// 1.5〜1.7・8.2: 子に届いた環境変数と引数（呼び手の `env` が最後に勝つ）。
#[test]
fn child_receives_env_and_args() {
    let mut run = fake("env")
        .safety_exit(Duration::from_millis(1234))
        .env("AREKA_NO_ALERT", "上書き")
        .arg("--一つめ")
        .arg("二つめ")
        .spawn();
    assert_eq!(run.wait_exit().status.code(), Some(0));
    let out = run.stdout();
    let run_dir = run.run_dir().to_path_buf();
    let value = |name: &str| -> String {
        let head = format!("env {name}=");
        out.lines()
            .find_map(|l| l.strip_prefix(&head))
            .unwrap_or_else(|| panic!("{name} が子に届いていない:\n{out}"))
            .to_owned()
    };
    let under_run = |name: &str, leaf: &str| {
        let got = PathBuf::from(value(name));
        assert_eq!(got, run_dir.join(leaf), "{name}");
        assert!(got.is_absolute() && !got.to_string_lossy().starts_with(r"\\?\"));
        assert!(
            got.to_string_lossy().contains(r"\target\test-roots\"),
            "{name} が target\\test-roots の下に無い: {}",
            got.display()
        );
    };
    under_run("AREKA_ROOT", "root");
    under_run("AREKA_PROFILE_DIR", "profile");
    assert!(
        out.lines().any(|l| l == "env AREKA_USER_ROOT 無い"),
        "{out}"
    );
    assert_eq!(value("AREKA_MCP_PORT"), run.mcp_port().to_string());
    assert_eq!(value("AREKA_NO_ALERT"), "上書き");
    assert_eq!(value("NO_COLOR"), "1");
    assert_eq!(value("AREKA_APP_SMOKE_EXIT_MS"), "1234");
    assert_eq!(value("RUST_LOG"), "info");
    assert!(out.lines().any(|l| l == "arg 1=--一つめ"), "{out}");
    assert!(out.lines().any(|l| l == "arg 2=二つめ"), "{out}");
    assert!(!out.contains("arg 3="), "{out}");
}

/// 5.1・5.3: 窓に閉じる知らせで、利用者と同じ閉じ方と終了コード 0。2 度目は同じ終わり。
#[test]
fn close_window_is_user_close() {
    let mut run = fake("window").spawn();
    let ended = run.close();
    assert_eq!(ended.by, EndedBy::UserClose);
    assert_eq!(ended.status.code(), Some(0));
    assert_eq!(run.close(), ended);
}

/// 5.4: 知らせを無視する窓は締切 → 止める → 止めたことと終了コードを含む panic。
#[test]
fn deaf_window_is_stopped_after_deadline() {
    let (text, pid) = expect_panic(|seen| {
        let mut run = fake("window-deaf").deadline(Duration::from_secs(2)).spawn();
        note(&run, seen);
        run.close();
    });
    assert!(text.contains(&format!("PID {pid}")), "{text}");
    assert!(text.contains("を止めた"), "止めたことが文に無い: {text}");
    assert!(
        text.contains("回収した終わり方 exit code: 1"),
        "終了コードが文に無い: {text}"
    );
    assert!(!alive(pid), "止めた後も子（PID {pid}）が生きている");
}

/// 5.2・5.3: 閉じる知らせを送らずに待つと、保険の自動終了で終わる。
#[test]
fn wait_exit_ends_by_safety_exit() {
    let mut run = fake("smoke")
        .safety_exit(Duration::from_millis(200))
        .spawn();
    let ended = run.wait_exit();
    assert_eq!(ended.by, EndedBy::SafetyExit);
    assert_eq!(ended.status.code(), Some(0));
}

/// 5.5: 閉じずに取っ手を捨てると、破棄が閉じる知らせで子を終わらせる（目印のファイル）。
#[test]
fn drop_closes_running_window() {
    let marks = TempPath::under_target("rr-mark");
    std::fs::create_dir_all(marks.path()).unwrap();
    let mark = marks.child("mark.txt");
    let run = fake("window").env("REAL_RUN_FAKE_MARK", &mark).spawn();
    let pid = run.pid();
    drop(run);
    assert_eq!(read(&mark), "closed");
    assert!(!alive(pid), "破棄の後も子（PID {pid}）が生きている");
}

/// 9.1: 2 つの走りを同時に立てると、置き場と番号が別。
#[test]
fn two_runs_at_once_are_separate() {
    let mut a = fake("window").spawn();
    let mut b = fake("window").spawn();
    assert_ne!(a.run_dir(), b.run_dir());
    assert_ne!(a.mcp_port(), b.mcp_port());
    assert_eq!(a.close().by, EndedBy::UserClose);
    assert_eq!(b.close().by, EndedBy::UserClose);
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} を読めない: {e}", path.display()))
        .trim()
        .to_owned()
}
