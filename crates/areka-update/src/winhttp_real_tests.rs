//! 本物の取得口（WinHTTP）で実機の一周を回す（要件 3.3〜3.5・9.7・`#[ignore]`）。
//!
//! 実行: `cargo test -p areka-update --release winhttp_real -- --ignored --nocapture`
//!
//! 同じテストの中で `127.0.0.1` の静的配信を別スレッドに立てる（外のネットワークには出ない）。
//! `/r/…` は `302 Location: /…`、それ以外はパーセント復号したパスのファイルを `200`、
//! 無ければ `404`、`/status/500` は `500`。更新先 URL を `/r/` にして転送の追随も通す。

use super::*;
use crate::md5::md5_hex;
use crate::testkit::{dau, tree};
use crate::urlpath;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::SampleRoot;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// 書き換える辞書と、配信から外して `delete.txt` で取り除かせるファイル。
const DICT: &str = "ghost/master/dic/talk.pasta";
const GONE: &str = "readme.txt";
/// 配信側にだけ足す新しいファイル（非 ASCII と空白＝取得の URL がパーセント符号化される）。
const NEW: &str = "ghost/master/日本 語.txt";
/// `NEW` の符号化された綴り（`urlpath::encode` の出力を手で書いた照合値）。
const NEW_ENCODED: &str = "ghost/master/%E6%97%A5%E6%9C%AC%20%E8%AA%9E.txt";

/// 受けた要求のパス（受けた順）。
type Seen = Arc<Mutex<Vec<String>>>;

/// `served` を配る待受を別スレッドに立て、`http://127.0.0.1:<port>` を返す。
/// ponytail: スレッドはテストの終わりまで待ち続け、プロセスの終了で消える（1 本しか走らせない）。
fn serve(served: PathBuf, seen: Seen) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("待受を立てられる");
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            answer(stream.expect("接続を受けられる"), &served, &seen);
        }
    });
    base
}

/// 要求 1 本に答える（HTTP/1.1・`Connection: close`）。
fn answer(mut stream: TcpStream, served: &Path, seen: &Seen) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    // 見出しは空行まで読み捨てる（GET に本文は無い）。
    let mut header = String::new();
    while reader.read_line(&mut header).unwrap() > 2 {
        header.clear();
    }
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
    seen.lock().unwrap().push(path.clone());
    let (status, extra, body) = if let Some(rest) = path.strip_prefix("/r/") {
        ("302 Found", format!("Location: /{rest}\r\n"), Vec::new())
    } else if path == "/status/500" {
        ("500 Internal Server Error", String::new(), Vec::new())
    } else {
        let rel = String::from_utf8(urlpath::decode(&path[1..])).unwrap();
        match fs::read(served.join(rel)) {
            Ok(bytes) => ("200 OK", String::new(), bytes),
            Err(_) => ("404 Not Found", String::new(), Vec::new()),
        }
    };
    let head = format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(&body).unwrap();
}

/// 配信フォルダを組む: 辞書 1 つを書き換え、`GONE` を外し、`delete.txt` と `updates2.dau` を置く。
/// 定義ファイルは配信フォルダの全ファイル（直下の定義ファイル自身を除く）を符号化したパスで並べる。
fn build_served(served: &Path) {
    let dict = served.join(DICT);
    let mut bytes = fs::read(&dict).expect("辞書を読める");
    bytes.extend_from_slice(b"\r\n// areka-update winhttp_real\r\n");
    fs::write(&dict, bytes).unwrap();
    fs::remove_file(served.join(GONE)).expect("外すファイルが検体に在る");
    fs::write(served.join("delete.txt"), format!("{GONE}\r\n")).unwrap();
    fs::write(served.join(NEW), "新しい行\r\n".as_bytes()).unwrap();
    let files: Vec<(String, String)> = tree(served)
        .into_iter()
        .filter(|(rel, _)| !rel.ends_with('/') && rel != "updates2.dau" && rel != "updates.txt")
        .map(|(rel, bytes)| (urlpath::encode(&rel), md5_hex(&bytes)))
        .collect();
    let lines: Vec<[&str; 2]> = files
        .iter()
        .map(|(p, m)| [p.as_str(), m.as_str()])
        .collect();
    let lines: Vec<&[&str]> = lines.iter().map(|l| &l[..]).collect();
    fs::write(served.join("updates2.dau"), dau(&lines, true)).unwrap();
}

struct Round {
    result: Result<UpdateOutcome, UpdateError>,
    seen: Vec<Progress>,
    records: Vec<CapturedEvent>,
    requests: Vec<String>,
}

#[allow(clippy::result_large_err)] // 公開の失敗の型をそのまま受ける（lib.rs の run と同じ）。
fn round(label: &str, homeurl: &str, target: &Path, fetch: &WinHttpFetch, log: &Seen) -> Round {
    log.lock().unwrap().clear();
    let mut seen = Vec::new();
    let (result, records) = capture(|| {
        run(
            &UpdateRequest { homeurl, target },
            fetch,
            &mut |p: &Progress| seen.push(p.clone()),
        )
    });
    let requests = log.lock().unwrap().clone();
    println!("== {label}: {result:?}");
    seen.iter().for_each(|p| println!("  progress {p:?}"));
    records.iter().for_each(|r| {
        println!(
            "  record {} {}",
            r.level,
            r.fields_map()
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    });
    requests.iter().for_each(|r| println!("  GET {r}"));
    Round {
        result,
        seen,
        records,
        requests,
    }
}

/// 記録は info 2 件（開始と成功の終了）だけで、終了の欄が結果と件数に合う（8.3）。
fn assert_records(records: &[CapturedEvent], outcome: &str, placed: usize, removed: usize) {
    let levels: Vec<tracing::Level> = records.iter().map(|r| r.level).collect();
    assert_eq!(levels, [tracing::Level::INFO; 2], "{records:#?}");
    assert_eq!(records[0].message(), "[areka_update] update started");
    let end = &records[1];
    assert_eq!(end.message(), "[areka_update] update finished");
    assert_eq!(end.field_str("outcome"), Some(outcome));
    assert_eq!(end.field("placed"), Some(placed.to_string().as_str()));
    assert_eq!(end.field("removed"), Some(removed.to_string().as_str()));
    assert_eq!(end.field("leftovers"), Some("0"));
}

#[test]
#[ignore = "実機の一周（WinHTTP・ローカル HTTP）。cargo test -p areka-update --release winhttp_real -- --ignored --nocapture"]
fn winhttp_real_two_rounds_update_then_unchanged() {
    let target_sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let served_sample = SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let target = target_sample.folder();
    let served = served_sample.folder();
    build_served(served);
    let log: Seen = Arc::default();
    let base = serve(served.to_path_buf(), log.clone());
    let homeurl = format!("{base}/r/");
    let fetch = WinHttpFetch::new().expect("WinHTTP のセッションを開ける");

    // 状態コードの判定（3.5）: 404 は「無い」、500 はコード付き。
    assert_eq!(
        fetch.get(&format!("{base}/missing.txt")),
        Err(FetchError::NotFound)
    );
    assert_eq!(
        fetch.get(&format!("{base}/status/500")),
        Err(FetchError::Status { code: 500 })
    );

    // 1 周目の期待: 定義の順（delete.txt → 辞書 → 新しいファイル）に 3 件置き、GONE を 1 件取り除く。
    let before = tree(target);
    let files = ["delete.txt", DICT, NEW];
    let served_tree = tree(served);
    let mut expected_tree = before.clone();
    for f in files {
        expected_tree.insert(f.to_owned(), served_tree[f].clone());
    }
    expected_tree.insert("updates2.dau".into(), served_tree["updates2.dau"].clone());
    expected_tree.remove(GONE);

    let r1 = round("round 1", &homeurl, target, &fetch, &log);
    let Ok(UpdateOutcome::Updated {
        manifest,
        placed,
        removed,
        undeletable,
        leftovers,
    }) = &r1.result
    else {
        panic!("1 周目は Updated のはず: {:?}", r1.result);
    };
    assert_eq!(*manifest, ManifestName::Updates2Dau);
    assert_eq!(placed, &files.map(String::from));
    assert_eq!(removed, &[target.join(GONE)]);
    assert!(undeletable.is_empty() && leftovers.is_empty());
    let mut progress = vec![
        Progress::ManifestFetched {
            name: ManifestName::Updates2Dau,
        },
        Progress::DiffDecided {
            files: files.map(String::from).to_vec(),
        },
    ];
    for (index, f) in files.into_iter().enumerate() {
        let md5 = md5_hex(&served_tree[f]);
        progress.push(Progress::DownloadBegin {
            file: f.to_owned(),
            index,
            total: 3,
        });
        progress.push(Progress::Md5Compared {
            file: f.to_owned(),
            expected: md5.clone(),
            actual: md5,
            matched: true,
        });
    }
    progress.push(Progress::Committed {
        placed: files.map(String::from).to_vec(),
    });
    progress.push(Progress::Deleted {
        removed: vec![target.join(GONE)],
    });
    assert_eq!(r1.seen, progress, "1 周目の段の順序");
    assert_records(&r1.records, "updated", 3, 1);
    // 取得はどれも 302 を経て本体へ届いた（3.4）。
    let hops = |names: &[&str]| -> Vec<String> {
        names
            .iter()
            .flat_map(|n| [format!("/r/{n}"), format!("/{n}")])
            .collect()
    };
    // 非 ASCII のパスは符号化されて届き、配信側の復号で本体に当たった（1.15）。
    assert_eq!(urlpath::encode(NEW), NEW_ENCODED);
    assert_eq!(
        r1.requests,
        hops(&["updates2.dau", "delete.txt", DICT, NEW_ENCODED])
    );
    assert_eq!(
        fs::read(target.join(NEW)).unwrap(),
        "新しい行\r\n".as_bytes()
    );
    assert!(tree(target) == expected_tree, "1 周目の後の木が期待と違う");

    // 2 周目: 差分 0。何も取得せず、何も書かない。
    let r2 = round("round 2", &homeurl, target, &fetch, &log);
    assert!(
        matches!(
            r2.result,
            Ok(UpdateOutcome::Unchanged {
                manifest: ManifestName::Updates2Dau
            })
        ),
        "2 周目は Unchanged のはず: {:?}",
        r2.result
    );
    assert_eq!(
        r2.seen,
        progress[..1]
            .iter()
            .cloned()
            .chain([Progress::DiffDecided { files: vec![] }])
            .collect::<Vec<_>>()
    );
    assert_records(&r2.records, "unchanged", 0, 0);
    assert_eq!(r2.requests, hops(&["updates2.dau"]));
    assert!(tree(target) == expected_tree, "2 周目で木が変わった");
}
