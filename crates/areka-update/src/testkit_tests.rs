//! 試験の道具そのものの較正（ネットワークに触れない）。

use super::*;
use sample_ghost_kit::WorkDir;
use std::cell::Cell;
use std::rc::Rc;

// ---- FakeFetch ----

#[test]
fn fake_fetch_returns_the_table_and_records_every_call_in_order() {
    let fetch = FakeFetch::new()
        .serve("http://x/a", b"AAA")
        .fail("http://x/b", FetchError::Timeout);

    assert_eq!(fetch.get("http://x/a"), Ok(b"AAA".to_vec()));
    assert_eq!(fetch.get("http://x/b"), Err(FetchError::Timeout));
    assert_eq!(fetch.get("http://x/a"), Ok(b"AAA".to_vec()));
    assert_eq!(fetch.calls(), ["http://x/a", "http://x/b", "http://x/a"]);
}

#[test]
fn fake_fetch_answers_not_found_for_urls_outside_the_table_and_records_them() {
    let fetch = FakeFetch::new().serve("http://x/a", b"A");

    assert_eq!(fetch.get("http://x/missing"), Err(FetchError::NotFound));
    assert_eq!(fetch.calls(), ["http://x/missing"]);
}

#[test]
fn fake_fetch_calls_the_hook_before_answering_each_get() {
    let seen = Rc::new(RefCell::new(Vec::<String>::new()));
    let log = Rc::clone(&seen);
    let fetch = FakeFetch::new()
        .serve("http://x/a", b"A")
        .on_get(move |url| log.borrow_mut().push(url.to_owned()));

    let _ = fetch.get("http://x/a");
    let _ = fetch.get("http://x/none");

    assert_eq!(*seen.borrow(), ["http://x/a", "http://x/none"]);
}

#[test]
fn fake_fetch_hook_runs_before_the_answer_is_returned() {
    // 差し込みは「取得の最中」に木を書き換える口なので、戻りより先に走っていなければならない。
    let ran = Rc::new(Cell::new(false));
    let flag = Rc::clone(&ran);
    let fetch = FakeFetch::new().on_get(move |_| flag.set(true));

    assert!(!ran.get());
    let _ = fetch.get("http://x/a");
    assert!(ran.get());
}

// ---- tree ----

#[test]
fn tree_copies_files_byte_for_byte_and_marks_folders_with_a_slash() {
    let work = WorkDir::new().expect("作業フォルダ");
    let root = work.path();
    fs::create_dir_all(root.join("sub/empty")).unwrap();
    fs::write(root.join("a.txt"), b"a\r\nb").unwrap();
    fs::write(root.join("sub/b.bin"), [0u8, 1, 255]).unwrap();

    let expected: BTreeMap<String, Vec<u8>> = [
        ("a.txt", b"a\r\nb".to_vec()),
        ("sub/", Vec::new()),
        ("sub/b.bin", vec![0u8, 1, 255]),
        ("sub/empty/", Vec::new()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    assert_eq!(tree(root), expected);

    // 1 バイトの違いも見分ける（写しが中身を見ていることの較正）。
    fs::write(root.join("sub/b.bin"), [0u8, 1, 254]).unwrap();
    assert_ne!(tree(root), expected);
}

#[test]
fn tree_of_a_missing_folder_is_empty() {
    let work = WorkDir::new().expect("作業フォルダ");
    assert!(tree(&work.path().join("none")).is_empty());
}

// ---- hold ----

#[test]
fn rename_fails_while_held_and_succeeds_after_release() {
    let work = WorkDir::new().expect("作業フォルダ");
    let from = work.path().join("held.dll");
    let to = work.path().join("moved.dll");
    fs::write(&from, b"x").unwrap();

    let handle = hold(&from);
    assert!(
        fs::rename(&from, &to).is_err(),
        "掴んでいる間は rename できない"
    );
    // 読みは共有しているので中身は読める（読み込まれた DLL と同じ状態）。
    assert_eq!(fs::read(&from).unwrap(), b"x");

    drop(handle);
    fs::rename(&from, &to).expect("放せば rename できる");
}

// ---- 定義ファイルの組み立て ----

#[test]
fn dau_joins_fields_with_0x01_and_ends_every_line() {
    let lines: &[&[&str]] = &[&["a.txt", "0123", "size=3"], &["b/c.txt", "4567"]];
    assert_eq!(
        dau(lines, true),
        b"a.txt\x010123\x01size=3\r\nb/c.txt\x014567\r\n".to_vec()
    );
    assert_eq!(
        dau(lines, false),
        b"a.txt\x010123\x01size=3\nb/c.txt\x014567\n".to_vec()
    );
}

#[test]
fn txt_ends_every_line_with_crlf() {
    assert_eq!(
        txt(&["charset,UTF-8", "file,a.txt\x010123"]),
        b"charset,UTF-8\r\nfile,a.txt\x010123\r\n".to_vec()
    );
}

#[test]
fn sjis_encodes_japanese_as_shift_jis_bytes() {
    // 「あ」＝ 0x82 0xA0（Shift_JIS）。ASCII はそのまま。
    assert_eq!(sjis("a\u{3042}"), vec![b'a', 0x82, 0xA0]);
}

// ---- junction ----

#[test]
fn junction_points_outside_and_reads_through() {
    let work = WorkDir::new().expect("作業フォルダ");
    let outside = work.path().join("outside");
    let target = work.path().join("target");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(outside.join("o.txt"), b"out").unwrap();

    let link = target.join("link");
    junction(&link, &outside);

    assert_eq!(fs::read(link.join("o.txt")).unwrap(), b"out");
    assert_eq!(
        fs::canonicalize(&link).unwrap(),
        fs::canonicalize(&outside).unwrap(),
        "実パスは対象の外"
    );
}
