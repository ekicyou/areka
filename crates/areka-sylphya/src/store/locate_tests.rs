use super::*;
use crate::store::{Located, fs_backend};
use rusqlite::Connection;
use std::path::PathBuf;
use temp_path_kit::TempPath;

#[test]
fn drive_path_becomes_verbatim_uri() {
    let uri = file_uri(Path::new(r"C:\a\b\sylphya.sqlite"), Access::ReadWrite).unwrap();
    assert_eq!(
        uri,
        "file:%5C%5C%3F%5CC:%5Ca%5Cb%5Csylphya.sqlite?vfs=win32-longpath&mode=rw"
    );
}

#[test]
fn unc_path_gets_verbatim_unc_prefix() {
    let uri = file_uri(Path::new(r"\\server\share\x"), Access::ReadOnly).unwrap();
    assert_eq!(
        uri,
        "file:%5C%5C%3F%5CUNC%5Cserver%5Cshare%5Cx?vfs=win32-longpath&mode=ro"
    );
}

#[test]
fn verbatim_path_is_not_prefixed_twice() {
    let uri = file_uri(Path::new(r"\\?\C:\a"), Access::Create).unwrap();
    assert_eq!(uri, "file:%5C%5C%3F%5CC:%5Ca?vfs=win32-longpath&mode=rwc");
}

#[test]
fn percent_and_hash_are_escaped_and_japanese_is_kept() {
    let uri = file_uri(Path::new(r"C:\100%#記憶"), Access::ReadWrite).unwrap();
    assert_eq!(
        uri,
        "file:%5C%5C%3F%5CC:%5C100%25%23記憶?vfs=win32-longpath&mode=rw"
    );
}

#[test]
fn forward_slashes_and_relative_paths_become_absolute() {
    let slashed = file_uri(Path::new("C:/a/b"), Access::ReadWrite).unwrap();
    assert_eq!(
        slashed,
        "file:%5C%5C%3F%5CC:%5Ca%5Cb?vfs=win32-longpath&mode=rw"
    );

    let relative = file_uri(Path::new("rel"), Access::ReadWrite).unwrap();
    let cwd = std::env::current_dir().unwrap().join("rel");
    let expected = file_uri(&cwd, Access::ReadWrite).unwrap();
    assert_eq!(relative, expected);
}

fn japanese_root(base: &TempPath) -> PathBuf {
    base.path().join("ゴースト").join("記憶の置き場")
}

fn long_root(base: &TempPath) -> PathBuf {
    let mut root = base.path().to_path_buf();
    for i in 0..6 {
        root.push(format!(
            "{i}-深いフォルダの名前をとても長くしておく-{}",
            "x".repeat(30)
        ));
    }
    assert!(root.to_string_lossy().chars().count() > 260);
    root
}

fn uri_of(located: Located) -> String {
    match located {
        Located::Uri(uri) => uri,
        Located::Missing => panic!("記憶のファイルの在りかが返るはず"),
    }
}

/// 書いて、別の接続で開き直して読み、同じ置き場で今の std の保存も通ることを測る。
fn write_reopen_read_at(root: &Path) {
    let backend = fs_backend();

    assert!(matches!(
        backend.locate(root, Access::ReadWrite).unwrap(),
        Located::Missing
    ));
    assert!(matches!(
        backend.locate(root, Access::ReadOnly).unwrap(),
        Located::Missing
    ));
    assert!(!root.exists(), "Create のほかはフォルダを作らない");

    backend.ensure_dir(root).unwrap();
    let create = uri_of(backend.locate(root, Access::Create).unwrap());
    {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("ATTACH ?1 AS layer", [&create]).unwrap();
        conn.execute_batch(
            "CREATE TABLE layer.kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO layer.kv VALUES('areka.last.ghost', 'えも');",
        )
        .unwrap();
        conn.execute_batch("DETACH layer").unwrap();
    }
    assert!(root.join(DB_FILE).is_file());

    for access in [Access::ReadWrite, Access::ReadOnly] {
        let uri = uri_of(backend.locate(root, access).unwrap());
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("ATTACH ?1 AS layer", [&uri]).unwrap();
        let value: String = conn
            .query_row(
                "SELECT value FROM layer.kv WHERE key = 'areka.last.ghost'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "えも");
    }

    let legacy = root.join("sylphya.toml");
    std::fs::write(&legacy, "[areka]\n").unwrap();
    assert_eq!(std::fs::read_to_string(&legacy).unwrap(), "[areka]\n");
}

#[test]
fn japanese_named_root_round_trips() {
    let base = TempPath::under_target("sylphya-locate-ja");
    write_reopen_read_at(&japanese_root(&base));
}

#[test]
fn root_longer_than_260_chars_round_trips() {
    let base = TempPath::under_target("sylphya-locate-long");
    write_reopen_read_at(&long_root(&base));
}
