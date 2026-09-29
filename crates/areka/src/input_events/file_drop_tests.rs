// =============================================================================
// 投げ込みの振り分けの決定論テスト（areka-P0-file-drop・設計のテスト 9）
// =============================================================================
//
// `sort_drops` の規則 1〜4 を、フォルダかどうかと目次の有無を閉包で注入して判定する。
// 目次の問い合わせ（`has_install_txt`）を受けたパスを記録し、フォルダと拡張子の合わない物では
// 問い合わせが 0 回であることまで確かめる（要件 2.2）。どのテストでも 3 つの数の和が入力の数と
// 等しいことを見る。

use std::io;
use std::path::{Path, PathBuf};

use areka_nar::{NarError, RefuseReason};

use super::{DropSort, ProbeNote, sort_drops};

/// 注入するフォルダの答え。
#[derive(Clone, Copy)]
enum Dir {
    Yes,
    No,
    Fail,
}

/// 注入する目次の答え。
#[derive(Clone, Copy)]
enum Peek {
    Has,
    Lacks,
    Unreadable,
}

fn corrupt(path: &Path) -> NarError {
    NarError::Refused {
        archive: path.to_path_buf(),
        reason: RefuseReason::CorruptArchive {
            detail: "EOCD が無い".to_owned(),
        },
    }
}

/// `(パス, フォルダの答え, 目次の答え)` の並びで振り分け、結果と目次を問われたパスの並びを返す。
fn run(items: &[(&str, Dir, Peek)]) -> (DropSort, Vec<PathBuf>) {
    let paths: Vec<PathBuf> = items.iter().map(|(p, ..)| PathBuf::from(p)).collect();
    let lookup = |p: &Path| {
        *items
            .iter()
            .find(|(q, ..)| Path::new(q) == p)
            .expect("注入していないパスを問われた")
    };
    let mut peeked = Vec::new();
    let sort = sort_drops(
        paths,
        |p| match lookup(p).1 {
            Dir::Yes => Ok(true),
            Dir::No => Ok(false),
            Dir::Fail => Err(io::Error::from(io::ErrorKind::NotFound)),
        },
        |p| {
            peeked.push(p.to_path_buf());
            match lookup(p).2 {
                Peek::Has => Ok(true),
                Peek::Lacks => Ok(false),
                Peek::Unreadable => Err(corrupt(p)),
            }
        },
    );
    assert_eq!(
        sort.files.len() + sort.dirs.len() + sort.installs.len(),
        items.len(),
        "3 つの数の和は入力の数"
    );
    (sort, peeked)
}

fn pb(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

/// 規則 1: `foo.nar` という名のフォルダはフォルダ。目次は問わない（要件 2.1 ⑴・10.3）。
#[test]
fn folder_wins_over_archive_extension() {
    let (sort, peeked) = run(&[(r"C:\d\foo.nar", Dir::Yes, Peek::Has)]);
    assert_eq!(sort.dirs, pb(&[r"C:\d\foo.nar"]));
    assert!(sort.files.is_empty() && sort.installs.is_empty());
    assert!(sort.notes.is_empty());
    assert!(peeked.is_empty(), "フォルダの目次は問わない");
}

/// 規則 1: `install.txt` を持つフォルダもフォルダ（要件 2.3）。
#[test]
fn folder_holding_install_txt_is_a_folder() {
    let (sort, peeked) = run(&[(r"C:\d\ghost", Dir::Yes, Peek::Has)]);
    assert_eq!(sort.dirs, pb(&[r"C:\d\ghost"]));
    assert!(sort.installs.is_empty());
    assert!(peeked.is_empty());
}

/// 規則 4: 拡張子は ASCII 大小無視・`.nar` と `.zip` は同じ扱いでインストール対象（要件 2.1 ⑵）。
#[test]
fn archives_with_install_txt_are_installs_case_insensitively() {
    let items = [
        (r"C:\d\a.nar", Dir::No, Peek::Has),
        (r"C:\d\b.ZIP", Dir::No, Peek::Has),
        (r"C:\d\c.Nar", Dir::No, Peek::Has),
        (r"C:\d\d.zip", Dir::No, Peek::Has),
    ];
    let (sort, peeked) = run(&items);
    let all = pb(&[r"C:\d\a.nar", r"C:\d\b.ZIP", r"C:\d\c.Nar", r"C:\d\d.zip"]);
    assert_eq!(sort.installs, all);
    assert_eq!(peeked, all, "書庫はどれも 1 回ずつ問う");
    assert!(sort.files.is_empty() && sort.dirs.is_empty() && sort.notes.is_empty());
}

/// 規則 4: `install.txt` の無い書庫はファイル（`.nar`・`.zip` とも・要件 2.4・10.7）。
#[test]
fn archives_without_install_txt_are_files() {
    let (sort, peeked) = run(&[
        (r"C:\d\a.nar", Dir::No, Peek::Lacks),
        (r"C:\d\b.zip", Dir::No, Peek::Lacks),
    ]);
    assert_eq!(sort.files, pb(&[r"C:\d\a.nar", r"C:\d\b.zip"]));
    assert_eq!(peeked.len(), 2);
    assert!(sort.installs.is_empty() && sort.notes.is_empty());
}

/// 規則 4: 目次の読めない書庫は覚え書き 1 件を積んでファイルへ倒す（要件 2.4）。
#[test]
fn unreadable_archive_is_a_file_with_one_note() {
    let (sort, _) = run(&[(r"C:\d\broken.nar", Dir::No, Peek::Unreadable)]);
    assert_eq!(sort.files, pb(&[r"C:\d\broken.nar"]));
    assert!(sort.installs.is_empty());
    match sort.notes.as_slice() {
        [ProbeNote::ArchiveUnreadable { path, error }] => {
            assert_eq!(path, Path::new(r"C:\d\broken.nar"));
            assert!(matches!(error, NarError::Refused { .. }));
        }
        other => panic!("覚え書きは ArchiveUnreadable 1 件のはず: {other:?}"),
    }
}

/// 規則 3: 拡張子なし・`.txt`・`.nar` を名前に含むだけの物はファイル。目次は問わない（要件 2.2）。
#[test]
fn non_archive_extensions_are_files_without_peeking() {
    let (sort, peeked) = run(&[
        (r"C:\d\README", Dir::No, Peek::Has),
        (r"C:\d\memo.txt", Dir::No, Peek::Has),
        (r"C:\d\a.nar.txt", Dir::No, Peek::Has),
        (r"C:\d\.nar", Dir::No, Peek::Has),
    ]);
    assert_eq!(
        sort.files,
        pb(&[
            r"C:\d\README",
            r"C:\d\memo.txt",
            r"C:\d\a.nar.txt",
            r"C:\d\.nar"
        ])
    );
    assert!(peeked.is_empty(), "拡張子の合わない物の目次は問わない");
    assert!(sort.notes.is_empty());
}

/// 規則 2: フォルダかどうかを問えなかった物は覚え書きを積み、拡張子で決める（要件 2.5・10.6）。
#[test]
fn is_dir_failure_notes_and_falls_through_to_extension() {
    let (sort, peeked) = run(&[
        (r"C:\d\gone.nar", Dir::Fail, Peek::Has),
        (r"C:\d\gone.txt", Dir::Fail, Peek::Has),
    ]);
    assert_eq!(sort.installs, pb(&[r"C:\d\gone.nar"]), ".nar は目次へ進む");
    assert_eq!(sort.files, pb(&[r"C:\d\gone.txt"]));
    assert!(sort.dirs.is_empty());
    assert_eq!(peeked, pb(&[r"C:\d\gone.nar"]));
    let noted: Vec<&Path> = sort
        .notes
        .iter()
        .map(|n| match n {
            ProbeNote::IsDirFailed { path, error } => {
                assert_eq!(error.kind(), io::ErrorKind::NotFound);
                path.as_path()
            }
            other => panic!("IsDirFailed のはず: {other:?}"),
        })
        .collect();
    assert_eq!(
        noted,
        [Path::new(r"C:\d\gone.nar"), Path::new(r"C:\d\gone.txt")]
    );
}

/// 規則 2 → 4: 問えずに消えた書庫は目次も読めず、覚え書き 2 件でファイルへ倒れる（要件 2.5 の括弧）。
#[test]
fn vanished_archive_notes_twice_and_is_a_file() {
    let (sort, _) = run(&[(r"C:\d\gone.zip", Dir::Fail, Peek::Unreadable)]);
    assert_eq!(sort.files, pb(&[r"C:\d\gone.zip"]));
    assert!(matches!(
        sort.notes.as_slice(),
        [
            ProbeNote::IsDirFailed { .. },
            ProbeNote::ArchiveUnreadable { .. }
        ]
    ));
}

/// 混ざった 6 件: 3 つの並びはどれも入力の順（要件 2.6）。
#[test]
fn mixed_six_keep_input_order_in_each_kind() {
    let (sort, peeked) = run(&[
        (r"C:\d\2.nar", Dir::No, Peek::Has),
        (r"C:\d\b.png", Dir::No, Peek::Has),
        (r"C:\d\dir2", Dir::Yes, Peek::Has),
        (r"C:\d\1.zip", Dir::No, Peek::Has),
        (r"C:\d\a.nar", Dir::No, Peek::Lacks),
        (r"C:\d\dir1", Dir::Yes, Peek::Has),
    ]);
    assert_eq!(sort.installs, pb(&[r"C:\d\2.nar", r"C:\d\1.zip"]));
    assert_eq!(sort.files, pb(&[r"C:\d\b.png", r"C:\d\a.nar"]));
    assert_eq!(sort.dirs, pb(&[r"C:\d\dir2", r"C:\d\dir1"]));
    assert_eq!(peeked, pb(&[r"C:\d\2.nar", r"C:\d\1.zip", r"C:\d\a.nar"]));
    assert!(sort.notes.is_empty());
}

/// 0 件は 0 件（問い合わせも 0 回）。
#[test]
fn empty_input_sorts_to_nothing() {
    let (sort, peeked) = run(&[]);
    assert!(sort.files.is_empty() && sort.dirs.is_empty() && sort.installs.is_empty());
    assert!(sort.notes.is_empty() && peeked.is_empty());
}

// =============================================================================
// Reference の形（設計のテスト 10・11・要件 4.2〜4.4・4.6・5.2）
// =============================================================================

/// `OnFileDrop2`: 2 ファイルで長さ 3・区切りは byte 値 1・拡張子なしの MIME は空でも区切りが残る。
#[test]
fn file_drop2_references_are_three_with_separators_kept_for_empty_mime() {
    let refs = super::file_drop2_references(&pb(&[r"C:\d\a.png", r"C:\d\b"]), 1);
    assert_eq!(
        refs,
        [
            "C:\\d\\a.png\u{1}C:\\d\\b".to_owned(),
            "1".to_owned(),
            "image/png\u{1}".to_owned(),
        ]
    );
    assert_eq!(
        refs[0].split('\u{1}').count(),
        refs[2].split('\u{1}').count(),
        "Reference0 と Reference2 は同じ数"
    );
}

/// `OnFileDrop2`: 1 ファイルなら区切りは 0 個。スコープは十進。
#[test]
fn file_drop2_references_single_file_has_no_separator() {
    let refs = super::file_drop2_references(&pb(&[r"C:\d\x.NAR"]), 0);
    assert_eq!(
        refs,
        [
            r"C:\d\x.NAR".to_owned(),
            "0".to_owned(),
            "application/zip".to_owned()
        ]
    );
    assert!(refs.iter().all(|r| !r.contains('\u{1}')));
}

/// `OnDirectoryDrop`: 長さ 2・[パス, スコープ]。
#[test]
fn directory_drop_references_are_path_and_scope() {
    let refs = super::directory_drop_references(Path::new(r"C:\d\ghost"), 12);
    assert_eq!(refs, [r"C:\d\ghost".to_owned(), "12".to_owned()]);
}

/// 送るイベント名の 2 定数は正典の綴り。
#[test]
fn event_name_constants_are_canon_spelling() {
    assert_eq!(super::ON_FILE_DROP2, "OnFileDrop2");
    assert_eq!(super::ON_DIRECTORY_DROP, "OnDirectoryDrop");
}
