//! 別のプロセスの書き手 2 つが同じ層のファイルへ書いて、1 つも消えないことを確かめる。
//!
//! 書き手は、この試験の実行ファイルを子として起こし直したもの。子は `WRITER_INDEX` が立ったときだけ
//! 書き手として走り（普段の走りでは親として子を起こす）、子からはもう子を起こさない。

use areka_sylphya::store::FsBackend;
use areka_sylphya::{KvsDirect, PersistScope, ScopeRoots, WriteOutcome};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use temp_path_kit::TempPath;

const TEST_NAME: &str = "two_processes_lose_no_write";
/// 子の番号（立っていれば子の役）。
const WRITER_INDEX: &str = "AREKA_SYLPHYA_TWO_PROCESS_INDEX";
const WRITER_ROOT: &str = "AREKA_SYLPHYA_TWO_PROCESS_ROOT";
const WRITERS: usize = 2;
const KEYS_PER_WRITER: usize = 40;
/// 退行したときに止まらず赤になるための上限。
const MAX_ATTEMPTS: usize = 1000;

fn key(writer: usize, n: usize) -> String {
    format!("areka.window.scope({}).x", writer * KEYS_PER_WRITER + n)
}

fn kvs(root: &Path) -> KvsDirect {
    let roots = ScopeRoots {
        ghost: Some(root.to_path_buf()),
        ..ScopeRoots::default()
    };
    KvsDirect::open(roots, FsBackend::with_manual_clock())
}

fn write_as_child(writer: usize, root: &Path) {
    let kvs = kvs(root);
    for n in 0..KEYS_PER_WRITER {
        let k = key(writer, n);
        let mut last = None;
        let written = (0..MAX_ATTEMPTS).any(|_| {
            let outcome = kvs.put(PersistScope::Ghost, &[(&k, &n.to_string())]);
            let ok = outcome == WriteOutcome::Written;
            last = Some(outcome);
            ok
        });
        assert!(
            written,
            "{k} を {MAX_ATTEMPTS} 回試しても書けない: {last:?}"
        );
    }
}

#[test]
fn two_processes_lose_no_write() {
    if let Ok(index) = std::env::var(WRITER_INDEX) {
        let root = PathBuf::from(std::env::var(WRITER_ROOT).expect("子に置き場が渡っていない"));
        write_as_child(index.parse().expect("子の番号"), &root);
        return;
    }

    let dir = TempPath::new("sylphya-two-process");
    let exe = std::env::current_exe().expect("試験の実行ファイルの場所");
    let children: Vec<_> = (0..WRITERS)
        .map(|i| {
            Command::new(&exe)
                .args(["--exact", TEST_NAME, "--nocapture"])
                .env(WRITER_INDEX, i.to_string())
                .env(WRITER_ROOT, dir.path())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("子を起こせない")
        })
        .collect();
    for (i, child) in children.into_iter().enumerate() {
        let out = child.wait_with_output().expect("子の終わりを待てない");
        assert!(
            out.status.success(),
            "子 {i} が失敗した:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let kvs = kvs(dir.path());
    let lost: Vec<String> = (0..WRITERS)
        .flat_map(|w| (0..KEYS_PER_WRITER).map(move |n| (w, n)))
        .filter(|&(w, n)| kvs.get_in(PersistScope::Ghost, &key(w, n)) != Some(n.to_string()))
        .map(|(w, n)| key(w, n))
        .collect();
    assert!(lost.is_empty(), "読めない鍵: {lost:?}");
}
