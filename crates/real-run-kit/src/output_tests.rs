//! `output` の兄弟のテスト。手で書いたファイル（後から追記するスレッドつき）と、
//! 終わった子（`cmd /c exit 3`）で、待ちの分岐と締切切れの文を確かめる。

use super::*;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Command, Stdio};
use temp_path_kit::TempPath;

/// 見回りの間隔（試験は短く回す）。
const POLL: Duration = Duration::from_millis(10);
/// 見つかるはずの待ちの締切（届かない長さ・遅い机でも落ちない）。
const FAR: Duration = Duration::from_secs(30);

fn append(path: &Path, bytes: &[u8]) {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
}

/// 締切まで生き続ける子（終わりの分岐へ入らない相手）。試験の終わりに止める。
fn living_child() -> Child {
    Command::new("ping")
        .args(["-n", "60", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn appended_later_needle_ends_wait() {
    let dir = TempPath::under_target("output-later");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    append(&out, b"boot\n");
    let writer = {
        let err = err.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            append(&err, "MCP: 待受を始めた 127.0.0.1:9801\n".as_bytes());
        })
    };
    let mut child = living_child();
    let got = wait_any(
        [&out, &err],
        &["見つからない", "MCP: 待受を始めた"],
        &mut child,
        Instant::now() + FAR,
        POLL,
    );
    child.kill().unwrap();
    child.wait().unwrap();
    writer.join().unwrap();
    assert_eq!(got, Ok(1));
}

/// 2 回の追記の境目（多バイトの字の途中）をまたいだ文字列を拾う。
#[test]
fn needle_split_across_two_appends_is_found() {
    let dir = TempPath::under_target("output-split");
    let path = dir.child("stdout.txt");
    let needle = "待受を始めた";
    let cut = 4; // 「待」（3 バイト）の後、「受」の 1 バイト目の後ろで切る。
    let mut tail = Tail::new(path.clone());
    append(
        &path,
        &[b"head ".as_slice(), &needle.as_bytes()[..cut]].concat(),
    );
    assert_eq!(tail.scan(&[needle]), None);
    append(&path, &needle.as_bytes()[cut..]);
    assert_eq!(tail.scan(&[needle]), Some(0));
    // 読んだ位置は進んでいる（同じ分を 2 度は数えない）。
    assert_eq!(tail.scan(&[needle]), None);
}

#[test]
fn exited_child_ends_wait_with_exit_code() {
    let dir = TempPath::under_target("output-exit");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    let mut child = Command::new("cmd").args(["/c", "exit 3"]).spawn().unwrap();
    let got = wait_any(
        [&out, &err],
        &["来ない"],
        &mut child,
        Instant::now() + FAR,
        POLL,
    );
    match got {
        Err(WaitEnd::ChildExited(ChildExited { status })) => assert_eq!(status.code(), Some(3)),
        other => panic!("子の終わりのはず: {other:?}"),
    }
}

/// 子が終わる直前に書いた文字列は、子の終わりより先に拾う。
#[test]
fn needle_written_just_before_exit_is_found() {
    let dir = TempPath::under_target("output-last");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    let file = std::fs::File::create(&out).unwrap();
    let mut child = Command::new("cmd")
        .args(["/c", "echo LAST_LINE& exit 3"])
        .stdout(file)
        .spawn()
        .unwrap();
    let got = wait_any(
        [&out, &err],
        &["LAST_LINE"],
        &mut child,
        Instant::now() + FAR,
        POLL,
    );
    assert_eq!(got, Ok(0));
}

#[test]
fn deadline_ends_wait_and_text_has_paths_and_tail() {
    let dir = TempPath::under_target("output-deadline");
    let (out, err) = (dir.child("stdout.txt"), dir.child("stderr.txt"));
    // 標準出力は 64 KiB を超える（頭は文に入らず、末尾だけが入る）。
    let mut big = b"HEAD_MARK\n".to_vec();
    big.resize(TAIL_BYTES + 1024, b'.');
    big.extend_from_slice(b"\nTAIL_MARK\n");
    append(&out, &big);
    append(&err, b"ERR_MARK\n");

    let mut child = living_child();
    let start = Instant::now();
    let got = wait_any(
        [&out, &err],
        &["来ない"],
        &mut child,
        start + Duration::from_millis(200),
        POLL,
    );
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(got, Err(WaitEnd::Deadline));
    assert!(start.elapsed() >= Duration::from_millis(200));

    let text = failure_text(&["来ない"], start.elapsed(), [&out, &err]);
    for want in [
        "来ない",
        &out.display().to_string(),
        &err.display().to_string(),
        "TAIL_MARK",
        "ERR_MARK",
        "全文はファイルにある",
    ] {
        assert!(text.contains(want), "{want:?} が無い:\n{text}");
    }
    assert!(!text.contains("HEAD_MARK"), "末尾 64 KiB の外が入った");
}
