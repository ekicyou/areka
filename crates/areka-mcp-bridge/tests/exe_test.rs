//! 建てた exe（`CARGO_BIN_EXE_areka-mcp-bridge`）を起こす通しのテスト。
//!
//! 起こす exe には必ず `AREKA_MCP_PORT`（偽の受け口の番号）と `AREKA_PROFILE_DIR`（`target` の下の
//! 一時フォルダ）の両方を渡す（机の本物の番号と本物の控えの置き場へ落ちない）。

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use areka_mcp::SERVER_NAME;
use serde_json::{Value, json};
use temp_path_kit::TempPath;

use fake_server::{FakeServer, Script};

/// 部品のテスト（クレートの中）と同じ偽の受け口。ここで使わない部品の警告は止める。
#[allow(dead_code)]
#[path = "common/fake_server.rs"]
mod fake_server;

/// 標準入力を閉じてから exe が終わるまでを待つ上限。過ぎたら exe を止めてテストを赤にする。
const EXIT_WAIT: Duration = Duration::from_secs(20);

/// 終わった exe の終了コードと、標準出力・標準エラーの全部。
struct Finished {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// exe を引数なしで起こし、`input` を標準入力へ流して閉じ、終わるまで待つ。
/// `RUST_LOG` は外す（記録の既定の絞りで回す）。
fn run_bridge(port: u16, profile_dir: &Path, input: &str) -> Finished {
    let mut child = Command::new(env!("CARGO_BIN_EXE_areka-mcp-bridge"))
        .env(areka_mcp::PORT_ENV, port.to_string())
        .env("AREKA_PROFILE_DIR", profile_dir)
        .env_remove("RUST_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the bridge");
    // 出力は別のスレッドで読み切る（管が詰まって exe が止まらないように）。
    let stdout = read_all(child.stdout.take().expect("stdout of the bridge"));
    let stderr = read_all(child.stderr.take().expect("stderr of the bridge"));
    let mut stdin = child.stdin.take().expect("stdin of the bridge");
    stdin.write_all(input.as_bytes()).expect("write to stdin");
    drop(stdin);
    let closed = Instant::now();
    let status = loop {
        match child.try_wait().expect("wait for the bridge") {
            Some(status) => break Some(status),
            None if closed.elapsed() > EXIT_WAIT => {
                child.kill().expect("stop the bridge");
                child.wait().expect("wait for the stopped bridge");
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let stdout = stdout.join().expect("the reader of stdout");
    let stderr = stderr.join().expect("the reader of stderr");
    let Some(status) = status else {
        panic!(
            "the bridge did not exit within {EXIT_WAIT:?} after stdin closed\nstdout: {stdout}\nstderr: {stderr}"
        );
    };
    Finished {
        code: status.code(),
        stdout,
        stderr,
    }
}

/// 管を終わりまで読むスレッド。
fn read_all(mut pipe: impl Read + Send + 'static) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let mut text = String::new();
        pipe.read_to_string(&mut text)
            .expect("read a pipe of the bridge");
        text
    })
}

/// 要件 1.9・2.8・5.1・5.2: 引数なしで起こした exe に `initialize` の 1 行を流して閉じると、
/// 標準出力は応答の 1 行だけ・記録は標準エラー・終了コード 0。応答は標準入力が閉じた後に
/// 書かれる（探索と握手は行を読んだ後に走る）。
#[test]
fn one_initialize_line_then_eof_gives_one_answer_line_and_exit_code_0() {
    let areka = FakeServer::start(Script::areka());
    let profile = TempPath::under_target("bridge-exe");
    let request = json!({"jsonrpc": "2.0", "id": 7, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": {"name": "exe-test", "version": "0"},
    }});

    let done = run_bridge(areka.port(), profile.path(), &format!("{request}\n"));
    let Finished { stdout, stderr, .. } = &done;

    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    // 標準出力は、areka の名乗りを持つ応答の 1 行だけ（要件 5.1）。
    let line = stdout
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("stdout does not end with LF: {stdout:?}\nstderr: {stderr}"));
    assert!(!line.contains('\n'), "stdout is not one line: {stdout:?}");
    let answer: Value = serde_json::from_str(line).expect("the line of stdout is JSON");
    assert_eq!(answer["id"], 7);
    assert_eq!(answer["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(answer["result"]["serverInfo"]["name"], SERVER_NAME);
    // 記録は標準エラー（要件 5.2）: 起動時の 3 行（候補の列・控えの置き場・3 つの時間）と、つないだこと、
    // 最初の問い合わせに答えたこと（相手が話した版と返した版）。
    let cache_dir = profile.child("mcp-bridge");
    let expected = [
        format!("ports=[{}]", areka.port()),
        cache_dir.display().to_string(),
        "connect=300ms verify=3s reply=30s".to_owned(),
        "connected to areka".to_owned(),
        "answered the first inquiry".to_owned(),
        "requested=2025-06-18 answered=2025-06-18".to_owned(),
    ];
    for expected in expected {
        assert!(
            stderr.contains(&expected),
            "stderr lacks {expected:?}: {stderr}"
        );
    }
    assert!(
        !stderr.contains("\"jsonrpc\""),
        "an answer line leaked to stderr: {stderr}"
    );
    // 既定の絞りでは、rmcp 自身の `info`（つないだ相手の情報の全部・通知の本文）を出さない。
    assert!(
        !stderr.contains("Service initialized"),
        "the default filter lets the info lines of rmcp through: {stderr}"
    );
    // 控えは `AREKA_PROFILE_DIR` の下に書かれた（環境を読む口から控えまでの通し）。
    assert!(cache_dir.join("server-info.json").is_file());
}
