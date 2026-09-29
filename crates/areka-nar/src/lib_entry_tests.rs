//! 書庫の中を読む口（[`NarArchive::entry_bytes`]）の兄弟テスト。
//!
//! 判定するのは、在る・大文字小文字違い・フォルダ・無い・同梱バルーンのフォルダの
//! 中のファイルの 5 通りと、読み直し・書き込み・記録が 0 であること。

use super::*;
use log_capture_kit::capture;
use sample_ghost_kit::{NarBuilder, WorkDir, install_txt};
use std::fs;

/// ゴースト 1 本。フォルダのエントリ `ghost/` と、同梱バルーン `kaku` の中の
/// 利用条件のファイルを持つ。
fn ghost() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "type,ghost",
                "name,テスト",
                "directory,tester",
                "balloon.directory,kaku",
                "balloon.source.directory,kaku",
            ]),
        )
        .done()
        .dir("ghost/")
        .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
        .done()
        .file("terms.txt", b"terms\r\n")
        .deflate()
        .done()
        .file("kaku/terms.md", b"# balloon terms\r\n")
        .done()
}

/// 書庫を一時の根に置いて開く。根を返すのは、開いた後に原本を潰すテストのため。
fn open(work: &WorkDir) -> (NarArchive, PathBuf) {
    let path = work.path().join("ghost.nar");
    fs::write(&path, ghost().bytes()).expect("固定入力を置ける");
    (NarArchive::open(&path).expect("無傷の書庫は開ける"), path)
}

#[test]
fn a_file_that_exists_returns_its_inflated_bytes() {
    let work = WorkDir::new().expect("根を借りられる");
    let (archive, _) = open(&work);
    assert_eq!(
        archive.entry_bytes("terms.txt"),
        Some(&b"terms\r\n"[..]),
        "deflate のエントリも伸長済みの中身で返る"
    );
    assert_eq!(
        archive.entry_bytes("ghost/master/descript.txt"),
        Some(&b"charset,Shift_JIS\r\n"[..])
    );
}

#[test]
fn the_path_ignores_ascii_case() {
    let work = WorkDir::new().expect("根を借りられる");
    let (archive, _) = open(&work);
    assert_eq!(archive.entry_bytes("TERMS.TXT"), Some(&b"terms\r\n"[..]));
    assert_eq!(
        archive.entry_bytes("Ghost/Master/Descript.txt"),
        Some(&b"charset,Shift_JIS\r\n"[..])
    );
}

#[test]
fn a_folder_is_none() {
    let work = WorkDir::new().expect("根を借りられる");
    let (archive, _) = open(&work);
    // エントリとして在るフォルダ。
    assert_eq!(archive.entry_bytes("ghost"), None);
    assert_eq!(archive.entry_bytes("ghost/"), None);
    // エントリは無いがファイルの親として見えるフォルダ。
    assert_eq!(archive.entry_bytes("ghost/master"), None);
    assert_eq!(archive.entry_bytes("kaku"), None);
}

#[test]
fn a_missing_path_is_none() {
    let work = WorkDir::new().expect("根を借りられる");
    let (archive, _) = open(&work);
    assert_eq!(archive.entry_bytes("terms.md"), None, "最上位には無い");
    assert_eq!(archive.entry_bytes(""), None);
    assert_eq!(archive.entry_bytes("terms"), None, "前方一致では当てない");
}

#[test]
fn a_file_inside_the_bundled_balloon_folder_is_found() {
    let work = WorkDir::new().expect("根を借りられる");
    let (archive, _) = open(&work);
    assert_eq!(
        archive.entry_bytes("kaku/terms.md"),
        Some(&b"# balloon terms\r\n"[..])
    );
}

/// 開いた後に原本を潰しても同じ中身が返り、根の木も記録も増えない。
#[test]
fn reading_touches_neither_the_archive_nor_the_root_nor_the_log() {
    let work = WorkDir::new().expect("根を借りられる");
    let (archive, path) = open(&work);
    fs::write(&path, "これはもう zip ではない").expect("原本を潰せる");
    let before: Vec<_> = fs::read_dir(work.path())
        .expect("根を読める")
        .map(|e| e.expect("要素").file_name())
        .collect();

    let (bytes, records) = capture(|| archive.entry_bytes("terms.txt").map(<[u8]>::to_vec));

    assert_eq!(
        bytes.as_deref(),
        Some(&b"terms\r\n"[..]),
        "読み直していない"
    );
    assert!(records.is_empty(), "記録は 0: {records:?}");
    let after: Vec<_> = fs::read_dir(work.path())
        .expect("根を読める")
        .map(|e| e.expect("要素").file_name())
        .collect();
    assert_eq!(before, after, "何も書いていない");
}
