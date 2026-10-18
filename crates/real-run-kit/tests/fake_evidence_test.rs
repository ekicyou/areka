//! 落ちた回の証拠の残し方を偽の子で確かめる（spec: areka-P0-real-run-mcp-kit タスク 5.3・
//! 要件 6.1〜6.4・10.1）。
//!
//! 窓口は残した場所をプロセスの標準エラーへ 1 行書く（6.2）。同じプロセスの中では試験の枠が
//! `eprintln!` を捕まえて読めないので、落ちる筋書きはこの試験の実行ファイルの写し（子のプロセス）で
//! 走らせ、その標準エラーを読む。子のプロセスでは `REAL_RUN_EVIDENCE_CHILD` が立ったときだけ
//! 筋書きの試験が中身を走らせる（普段の走りでは何もせず緑）。残った置き場は親が最後に消す。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;

use real_run_kit::{EndedBy, Launch, RealRun};

/// 子のプロセスで落ちる筋書きを走らせる印。
const CHILD_FLAG: &str = "REAL_RUN_EVIDENCE_CHILD";
/// 窓口が残した場所を名指す行の頭（evidence.rs の `finish`）。
const KEPT_HEAD: &str = "落ちた回の証拠を残しました: ";

fn fake(steps: &str) -> RealRun {
    RealRun::new(env!("CARGO_BIN_EXE_real-run-fake-areka"))
        .without_host32_helper()
        .env("REAL_RUN_FAKE_STEPS", steps)
}

/// ワークスペースの `target\test-roots`（temp-path-kit の置き場と同じ決め方: このクレートの 2 つ上の根）。
/// `CARGO_TARGET_DIR` や `--target` で成果物の場所が変わっても、窓口の置き場はここに決まっている。
fn target_test_roots() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/real-run-kit の 2 つ上がワークスペースの根")
        .join("target")
        .join("test-roots")
}

fn alive(pid: u32) -> bool {
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .expect("tasklist を起こせない");
    String::from_utf8_lossy(&out.stdout).contains("real-run-fake-areka")
}

/// 6.3: 成功で終わった回は、取っ手を捨てると置き場が消える。
#[test]
fn success_removes_run_dir() {
    let mut run = fake("window").spawn();
    let dir = run.run_dir().to_path_buf();
    assert!(dir.is_dir());
    assert_eq!(run.close().by, EndedBy::UserClose);
    drop(run);
    assert!(!dir.exists(), "成功の回の置き場が残った: {}", dir.display());
}

/// 6.1・6.2・6.4: 走りの途中の失敗で、置き場（根・記憶・出力・検体の写しの複写）が `target\` の下に
/// 残り、標準エラーの 1 行が場所を名指す。筋書きは子のプロセス（`kept_on_panic_child`）が走らせる。
#[test]
fn panic_keeps_run_dir_and_names_it() {
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "kept_on_panic_child", "--nocapture"])
        .env(CHILD_FLAG, "1")
        .output()
        .expect("試験の実行ファイルの写しを起こせない");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let kept: Vec<PathBuf> = stderr
        .lines()
        .filter_map(|l| l.strip_prefix(KEPT_HEAD))
        .map(PathBuf::from)
        .collect();
    assert!(out.status.success(), "子のプロセス:\n{stdout}\n{stderr}");
    assert_eq!(kept.len(), 1, "場所を名指す行が 1 行でない:\n{stderr}");
    let dir = &kept[0];
    let checks = std::panic::catch_unwind(|| {
        assert!(
            dir.starts_with(target_test_roots()),
            "target\\test-roots の下に無い: {}",
            dir.display()
        );
        for leaf in ["root", "profile", "stdout.log", "stderr.log"] {
            assert!(dir.join(leaf).exists(), "{leaf} が残っていない");
        }
        assert!(
            dir.join("sample").join("ghost").join("emo2").is_dir(),
            "検体の写しが sample\\ に残っていない"
        );
        let pid: u32 = stdout
            .lines()
            .find_map(|l| l.strip_prefix("pid "))
            .and_then(|p| p.trim().parse().ok())
            .expect("子の番号が出ていない");
        assert!(!alive(pid), "落ちた回の偽の子（PID {pid}）が生きている");
    });
    let _ = std::fs::remove_dir_all(dir);
    if let Err(e) = checks {
        std::panic::resume_unwind(e);
    }
}

/// 子のプロセスでだけ走る筋書き: 検体の写しで偽の子を立て、取っ手が生きている間に落ちる
/// （`catch_unwind` の中の失敗＝呼び手の確かめが落ちた形）。印が無い普段の走りでは何もしない。
#[test]
fn kept_on_panic_child() {
    if std::env::var_os(CHILD_FLAG).is_none() {
        return;
    }
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let run = fake("window")
            .launch(Launch::Sample {
                name: "emo2".to_owned(),
                balloon: None,
            })
            .spawn();
        println!("pid {}", run.pid());
        panic!("わざと落とす");
    }));
    assert!(caught.is_err());
}
