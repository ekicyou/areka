//! 控え（`cache.rs`）のテスト。
//!
//! 置き場はどれもテストごとの一時フォルダ（ワークツリーの `target` の下）。利用者の本物の控えと
//! SSP の控えには触らない（要件 8.1）。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use temp_path_kit::TempPath;

use super::*;

fn info(name: &str) -> ServerPeerInfo {
    serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": name, "version": "0.0.2" },
        "instructions": "hello",
    }))
    .expect("サーバーの情報の見本")
}

fn tools(count: usize) -> ListToolsResult {
    let tools: Vec<_> = (0..count)
        .map(|n| json!({ "name": format!("tool_{n}"), "inputSchema": { "type": "object" } }))
        .collect();
    serde_json::from_value(json!({ "tools": tools })).expect("ツールの一覧の見本")
}

/// 置き場にあるファイルの名前（並べ替え済み）。一時の名前が残っていないことを見る。
fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("置き場を開く")
        .map(|entry| {
            entry
                .expect("置き場の 1 件")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// 書いた控えは、起動し直した中継（別の `Cache`）が読める（要件 3.3）。2 度目の書きは前のを置き換える。
#[test]
fn saved_caches_are_read_back_by_another_cache() {
    let root = TempPath::under_target("bridge-cache-roundtrip");
    // まだ無い深いフォルダ（最初の書きが作る）。
    let dir = root.child("profile").join("mcp-bridge");
    assert_eq!(Cache::new(Some(dir.clone())).load_server_info(), None);
    assert_eq!(Cache::new(Some(dir.clone())).load_tools(), None);

    let writer = Cache::new(Some(dir.clone()));
    writer.save_server_info(&info("old"));
    writer.save_tools(&tools(1));
    writer.save_server_info(&info("areka-mcp-server"));
    writer.save_tools(&tools(3));

    let reader = Cache::new(Some(dir.clone()));
    assert_eq!(reader.load_server_info(), Some(info("areka-mcp-server")));
    assert_eq!(reader.load_tools(), Some(tools(3)));
    assert_eq!(names_in(&dir), [SERVER_INFO_FILE, TOOLS_FILE]);
}

/// 壊れた・途中までの控えは無いものとして扱い、次の書きで作り直せる（要件 3.10）。
#[test]
fn unreadable_content_is_treated_as_absent() {
    let root = TempPath::under_target("bridge-cache-corrupt");
    let cache = Cache::new(Some(root.path().to_path_buf()));
    let whole_info = serde_json::to_vec(&info("areka-mcp-server")).unwrap();
    let whole_tools = serde_json::to_vec(&tools(3)).unwrap();

    let broken: [(&[u8], &[u8]); 4] = [
        (b"", b""),
        (b"not json", b"\xff\xfe"),
        // 途中まで。
        (
            &whole_info[..whole_info.len() / 2],
            &whole_tools[..whole_tools.len() / 2],
        ),
        // JSON だが形が違う。
        (b"{}", b"[]"),
    ];
    for (info_bytes, tools_bytes) in broken {
        std::fs::write(root.child(SERVER_INFO_FILE), info_bytes).unwrap();
        std::fs::write(root.child(TOOLS_FILE), tools_bytes).unwrap();
        assert_eq!(cache.load_server_info(), None, "{info_bytes:?}");
        assert_eq!(cache.load_tools(), None, "{tools_bytes:?}");
    }

    cache.save_server_info(&info("areka-mcp-server"));
    cache.save_tools(&tools(3));
    assert_eq!(cache.load_server_info(), Some(info("areka-mcp-server")));
    assert_eq!(cache.load_tools(), Some(tools(3)));
}

/// 書けない置き場でも中継は続く（要件 3.10）: 置き場の名前がファイル・控えの名前がフォルダ。
#[test]
fn unwritable_place_keeps_going() {
    let root = TempPath::under_target("bridge-cache-unwritable");

    // 置き場の名前に既にファイルが居る（フォルダを作れない）。
    let blocker = root.child("blocker");
    std::fs::write(&blocker, b"not a folder").unwrap();
    let cache = Cache::new(Some(blocker.clone()));
    cache.save_server_info(&info("areka-mcp-server"));
    cache.save_tools(&tools(1));
    assert_eq!(cache.load_server_info(), None);
    assert_eq!(cache.load_tools(), None);
    assert_eq!(std::fs::read(&blocker).unwrap(), b"not a folder");

    // 控えの名前に既にフォルダが居る（置き換えられない・読めない）。一時の名前は残さない。
    let dir = root.child("place");
    std::fs::create_dir_all(dir.join(TOOLS_FILE)).unwrap();
    let cache = Cache::new(Some(dir.clone()));
    cache.save_tools(&tools(1));
    assert_eq!(cache.load_tools(), None);
    assert_eq!(names_in(&dir), [TOOLS_FILE]);
    // もう片方の控えは巻き込まれない。
    cache.save_server_info(&info("areka-mcp-server"));
    assert_eq!(cache.load_server_info(), Some(info("areka-mcp-server")));
}

/// 置き場が無ければ読まず書かず（起こされた場所にも書かない）。
#[test]
fn no_place_reads_and_writes_nothing() {
    let cache = Cache::new(None);
    cache.save_server_info(&info("areka-mcp-server"));
    cache.save_tools(&tools(1));
    assert_eq!(cache.load_server_info(), None);
    assert_eq!(cache.load_tools(), None);
    assert!(!Path::new(SERVER_INFO_FILE).exists());
    assert!(!Path::new(TOOLS_FILE).exists());
}

/// 2 つの中継が同じ控えを同時に書いても、置き場にあるのはいつもどちらか 1 つの全体（要件 3.11）。
///
/// 2 つの中継は別のプロセスなので一時の名前（プロセス番号入り）が違う。テストは 1 つのプロセスの
/// 2 つのスレッドで、置き換えの関数に別々の番号を渡して同じ形を作る。書いている間じゅう別の
/// スレッドが読み続け、読めた中身がいつも全体であることを見る（最後の 1 回だけでは、途中までの
/// 中身が見える書き方でも通ってしまう）。
#[test]
fn two_writers_always_leave_one_whole_answer() {
    const ROUNDS: usize = 200;
    let root = TempPath::under_target("bridge-cache-race");
    let dir = root.path();
    // 長さの違う 2 つ（混ざれば JSON として読めないか、どちらとも違う）。
    let answers = [tools(1), tools(40)];
    let bytes = answers
        .each_ref()
        .map(|answer| serde_json::to_vec(answer).unwrap());
    let is_whole = |read: &[u8]| {
        let got: ListToolsResult =
            serde_json::from_slice(read).expect("読めた控えはいつも JSON の全体");
        assert!(answers.contains(&got), "どちらの答えとも違う控え");
    };
    replace(dir, TOOLS_FILE, &bytes[0], 1).expect("最初の控え");

    let done = AtomicBool::new(false);
    let (wrote, read) = std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            let mut read = 0;
            while !done.load(Ordering::Relaxed) {
                // 置き換えの瞬間に開けないことはあってよい（本番では「無いもの」になる）。
                if let Ok(content) = std::fs::read(dir.join(TOOLS_FILE)) {
                    is_whole(&content);
                    read += 1;
                }
            }
            read
        });
        let writers = [(&bytes[0], 1), (&bytes[1], 2)].map(|(content, pid)| {
            scope.spawn(move || {
                // 相手の置き換えとぶつかって失敗する回はあってよい（本番では warn して続ける）。
                (0..ROUNDS)
                    .filter(|_| replace(dir, TOOLS_FILE, content, pid).is_ok())
                    .count()
            })
        });
        let wrote = writers.map(|writer| writer.join().expect("書き手"));
        done.store(true, Ordering::Relaxed);
        (wrote, reader.join().expect("読み手"))
    });

    assert!(wrote.iter().all(|&count| count > 0), "書けた回数 {wrote:?}");
    assert!(read > 0, "読み手が 1 度も読めていない");
    let last = Cache::new(Some(dir.to_path_buf())).load_tools();
    assert!(last.is_some_and(|last| answers.contains(&last)));
    assert_eq!(names_in(dir), [TOOLS_FILE], "一時の名前が残っている");
}
