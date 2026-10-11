//! 建てた exe（`CARGO_BIN_EXE_areka-mcp-bridge`）を起こす通しのテスト。
//!
//! 起こす exe には必ず `AREKA_MCP_PORT`（偽の受け口の番号か、テストが束ねたまま待ち受けない番号か、
//! `0`）と `AREKA_PROFILE_DIR`（`target` の下の一時フォルダ）の両方を渡す（机の本物の番号と本物の
//! 控えの置き場へ落ちない）。読めない値と未設定は既定の 20 個（机の本物の番号）へつなぎに行くので、
//! exe ごしには回さない（`config_tests.rs` の値だけの判定が持つ）。

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use areka_mcp::SERVER_NAME;
use areka_mcp_bridge::cache::Cache;
use serde_json::{Value, json};
use temp_path_kit::TempPath;

use fake_server::{FakeServer, Reply, Script};

/// 部品のテスト（クレートの中）と同じ偽の受け口。ここで使わない部品の警告は止める。
#[allow(dead_code)]
#[path = "common/fake_server.rs"]
mod fake_server;

/// exe が標準入力を読み切るまでと、標準入力を閉じてから exe が終わるまでを待つ上限。
/// 過ぎたら exe を止めてテストを赤にする。
const EXIT_WAIT: Duration = Duration::from_secs(20);

/// 終わった exe の終了コードと、標準出力・標準エラーの全部。
struct Finished {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    /// 標準入力を閉じてから exe が終わるまで。
    waited: Duration,
}

/// exe を引数なしで起こし、`input` を標準入力へ流して閉じ、終わるまで待つ。
/// `RUST_LOG` は外す（記録の既定の絞りで回す）。
fn run_bridge(port: u16, profile_dir: &Path, input: &str) -> Finished {
    run(port, profile_dir, input, true)
}

/// [`run_bridge`] の本体。`read_stdout` が偽なら、標準出力の管を開いたまま 1 バイトも読まない
/// （標準入力だけ閉じて読まない相手。`Finished::stdout` は空）。
fn run(port: u16, profile_dir: &Path, input: &str, read_stdout: bool) -> Finished {
    let mut child = Command::new(env!("CARGO_BIN_EXE_areka-mcp-bridge"))
        .env(areka_mcp::PORT_ENV, port.to_string())
        .env("AREKA_PROFILE_DIR", profile_dir)
        .env_remove("RUST_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the bridge");
    // 出力は別のスレッドで読み切る（管が詰まって exe が止まらないように）。読まない標準出力の管は
    // `child` が持ったまま（exe が終わるまで開いている）。
    let stdout = read_stdout.then(|| read_all(child.stdout.take().expect("stdout of the bridge")));
    let stderr = read_all(child.stderr.take().expect("stderr of the bridge"));
    // 流すのも別のスレッド（exe が読まなくなっても、このテストは止まらない。詰まった書きは下で
    // exe を止めるとほどける）。書き終えたら、失敗していても標準入力を閉じてから知らせる。
    let mut stdin = child.stdin.take().expect("stdin of the bridge");
    let input = input.to_owned();
    let (fed, feeding) = mpsc::channel();
    std::thread::spawn(move || {
        let written = stdin.write_all(input.as_bytes());
        // 閉じた時刻は閉じる直前にここで取る（主スレッドが起きるのが遅れても、待った長さが縮まない）。
        let closed = Instant::now();
        drop(stdin);
        let _ = fed.send((written, closed));
    });
    let (fed, closed) = match feeding.recv_timeout(EXIT_WAIT) {
        Ok((written, closed)) => (Ok(written), closed),
        Err(error) => (Err(error), Instant::now()),
    };
    let status = loop {
        match child.try_wait().expect("wait for the bridge") {
            Some(status) => break Some(status),
            None if !matches!(fed, Ok(Ok(()))) || closed.elapsed() > EXIT_WAIT => {
                child.kill().expect("stop the bridge");
                child.wait().expect("wait for the stopped bridge");
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let waited = closed.elapsed();
    let stdout = stdout.map(|reader| reader.join().expect("the reader of stdout"));
    let stdout = stdout.unwrap_or_default();
    let stderr = stderr.join().expect("the reader of stderr");
    let Some(status) = status.filter(|_| matches!(fed, Ok(Ok(())))) else {
        panic!(
            "the bridge did not take its stdin ({fed:?}) or did not exit within {EXIT_WAIT:?} after stdin closed\nstdout: {stdout}\nstderr: {stderr}"
        );
    };
    Finished {
        code: status.code(),
        stdout,
        stderr,
        waited,
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

/// 要求の 1 行（LF 付き）。`params` が `Null` なら欄ごと載せない。
fn request(id: impl Into<Value>, method: &str, params: Value) -> String {
    let mut line = json!({"jsonrpc": "2.0", "id": id.into(), "method": method});
    if !params.is_null() {
        line["params"] = params;
    }
    format!("{line}\n")
}

/// 最初の問い合わせの 1 行（版 `2025-06-18`）。
fn initialize(id: impl Into<Value>) -> String {
    let params = json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": {"name": "exe-test", "version": "0"},
    });
    request(id, "initialize", params)
}

/// ツールの呼び出しの 1 行。
fn call(id: impl Into<Value>) -> String {
    let params = json!({"name": "get_status", "arguments": {}});
    request(id, "tools/call", params)
}

/// 通知の 1 行（`id` なし）。
fn notification(method: &str) -> String {
    format!("{}\n", json!({"jsonrpc": "2.0", "method": method}))
}

/// 偽の受け口が `tools/call` に返す成功の結果。
fn called() -> Reply {
    Reply::Result(json!({"content": [{"type": "text", "text": "done"}]}))
}

/// 標準出力を応答の行に分ける。応答の行（LF で終わる、`id` と結果かエラーを持つ JSON-RPC の
/// 1 行）のほかに 1 バイトでもあれば panic（要件 5.1: 記録・空の行・BOM・CR が無い）。
fn answer_lines(done: &Finished) -> Vec<Value> {
    let Finished { stdout, stderr, .. } = done;
    let lines = stdout
        .strip_suffix('\n')
        .unwrap_or_else(|| panic!("stdout does not end with LF: {stdout:?}\nstderr: {stderr}"));
    assert!(
        !stdout.contains('\r') && !stdout.starts_with('\u{feff}'),
        "stdout has a CR or a BOM: {stdout:?}"
    );
    let answer = |line: &str| {
        let answer: Value = serde_json::from_str(line).unwrap_or_else(|error| {
            panic!("a line of stdout is not JSON ({error}): {line:?}\nstdout: {stdout}\nstderr: {stderr}")
        });
        assert_eq!(answer["jsonrpc"], "2.0", "not an answer line: {line}");
        assert!(!answer["id"].is_null(), "an answer without id: {line}");
        assert!(
            answer.get("result").is_some() != answer.get("error").is_some(),
            "neither a result nor an error: {line}"
        );
        answer
    };
    lines.split('\n').map(answer).collect()
}

/// この `id` への応答。ちょうど 1 行でなければ panic（応答の順は決まっていない＝設計 D-13）。
fn answer_to(answers: &[Value], id: impl Into<Value>) -> &Value {
    let id = id.into();
    let mut found = answers.iter().filter(|answer| answer["id"] == id);
    match (found.next(), found.next()) {
        (Some(answer), None) => answer,
        _ => panic!("not exactly one answer to id {id}: {answers:?}"),
    }
}

/// 標準エラーの記録のうち、この文を含む行。無ければ panic。
fn record<'a>(stderr: &'a str, words: &str) -> &'a str {
    let mut lines = stderr.lines();
    lines
        .find(|line| line.contains(words))
        .unwrap_or_else(|| panic!("stderr lacks {words:?}: {stderr}"))
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

/// 要件 5.1・5.2: 要求・通知・空の行・空白だけの行を混ぜて流しても、標準出力は `id` を持つ要求の
/// 数だけの応答の行だけ（順は決まっていない）。記録は標準エラーに出て、応答の行は混ざらない。
#[test]
fn several_lines_give_only_answer_lines_on_stdout_and_records_on_stderr() {
    let areka = FakeServer::start(Script::areka().on("tools/call", called()));
    let profile = TempPath::under_target("bridge-exe");
    let input = [
        initialize(1),
        notification("notifications/initialized"),
        "\n".to_owned(),
        " \t \r\n".to_owned(),
        request(2, "tools/list", Value::Null),
        notification("notifications/roots/list_changed"),
        request("p-1", "ping", Value::Null),
        call(3),
    ];

    let done = run_bridge(areka.port(), profile.path(), &input.concat());
    let Finished { stdout, stderr, .. } = &done;

    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    let answers = answer_lines(&done);
    assert_eq!(answers.len(), 4, "stdout: {stdout}\nstderr: {stderr}");
    let first = answer_to(&answers, 1);
    assert_eq!(first["result"]["serverInfo"]["name"], SERVER_NAME);
    assert_eq!(answer_to(&answers, 2)["result"]["tools"], json!([]));
    assert_eq!(answer_to(&answers, "p-1")["result"], json!({}));
    let called = answer_to(&answers, 3);
    assert_eq!(called["result"]["content"][0]["text"], "done");
    // 記録は標準エラーだけにある（中継の行は `[bridge]` で始まる文を持つ）。
    assert!(!stdout.contains("[bridge]"), "a record on stdout: {stdout}");
    for words in [
        "[bridge] connected to areka",
        "[bridge] answered the first inquiry",
        "[bridge] finished",
    ] {
        record(stderr, words);
    }
    assert!(
        !stderr.contains("\"jsonrpc\""),
        "an answer line leaked to stderr: {stderr}"
    );
}

/// 要件 2.6: `AREKA_MCP_PORT=0` では、`id` を持つ要求の全部（`ping` も）に同じエラーで答える。
/// 使える控えがあっても使わない。エラーの中身を rmcp の記録が写さない（既定の絞りの `rmcp=error`）。
#[test]
fn port_0_answers_every_request_with_the_error_and_ignores_a_valid_cache() {
    let profile = TempPath::under_target("bridge-exe");
    // 控えを本物の形で用意する: areka につないだ exe に 2 つとも書かせる。
    let areka = FakeServer::start(Script::areka());
    let seed = [initialize(1), request(2, "tools/list", Value::Null)];
    let seeded = run_bridge(areka.port(), profile.path(), &seed.concat());
    assert_eq!(seeded.code, Some(0), "stderr: {}", seeded.stderr);
    let cache = Cache::new(Some(profile.child("mcp-bridge")));
    assert!(cache.load_server_info().is_some() && cache.load_tools().is_some());
    let input = [
        initialize(1),
        request(2, "tools/list", Value::Null),
        notification("notifications/initialized"),
        request(3, "ping", Value::Null),
        call(4),
    ];

    let done = run_bridge(0, profile.path(), &input.concat());
    let Finished { stdout, stderr, .. } = &done;

    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    let answers = answer_lines(&done);
    assert_eq!(answers.len(), 4, "stdout: {stdout}\nstderr: {stderr}");
    for id in 1..=4 {
        let error = &answer_to(&answers, id)["error"];
        assert_eq!(error["code"], -1, "id {id}: {stdout}");
        assert_eq!(
            error["message"], "Server not available: AREKA_MCP_PORT=0 (areka does not listen)",
            "id {id}: {stdout}"
        );
    }
    record(
        stderr,
        "AREKA_MCP_PORT=0: the bridge will not look for areka",
    );
    assert!(
        !stderr.contains("response error"),
        "the default filter lets the warn lines of rmcp through: {stderr}"
    );
}

/// 要件 2.4: 番号を指定すると、候補はその番号だけ。areka ならつなぎ、areka でない相手なら「居ない」
/// エラー（文に載る候補はその 1 つ）で、要求はその相手へ届けない。
/// 要件 2.3・5.2: 飛ばした番号と名乗り、控えで答えたことが標準エラーに残る。
#[test]
fn a_given_port_is_the_only_one_tried() {
    let areka = FakeServer::start(Script::areka());
    let profile = TempPath::under_target("bridge-exe");
    let input = [initialize(1), request(2, "ping", Value::Null)];

    let done = run_bridge(areka.port(), profile.path(), &input.concat());

    let Finished { stdout, stderr, .. } = &done;
    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    let answers = answer_lines(&done);
    assert_eq!(answers.len(), 2, "stdout: {stdout}\nstderr: {stderr}");
    let first = answer_to(&answers, 1);
    assert_eq!(first["result"]["serverInfo"]["name"], SERVER_NAME);
    assert_eq!(answer_to(&answers, 2)["result"], json!({}));
    assert_eq!(areka.requests_of("ping").len(), 1);
    record(stderr, &format!("ports=[{}]", areka.port()));

    // areka でない相手の番号を指定する。最初の問い合わせには、上の回が書いた控えで答える。
    let other = FakeServer::start(Script::named("ssp-mcp-server"));
    let port = other.port();
    let input = [call(1), initialize(2)];

    let done = run_bridge(port, profile.path(), &input.concat());

    let Finished { stdout, stderr, .. } = &done;
    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    let answers = answer_lines(&done);
    assert_eq!(answers.len(), 2, "stdout: {stdout}\nstderr: {stderr}");
    let error = &answer_to(&answers, 1)["error"];
    assert_eq!(error["code"], -1, "stdout: {stdout}");
    assert_eq!(
        error["message"],
        format!(
            "Server not available: {SERVER_NAME} not found on 127.0.0.1 ports {port}; port {port} answered as \"ssp-mcp-server\""
        ),
        "stderr: {stderr}"
    );
    assert_eq!(other.requests_of("tools/call").len(), 0);
    let cached = answer_to(&answers, 2);
    assert_eq!(cached["result"]["serverInfo"]["name"], SERVER_NAME);
    // 飛ばした候補は、番号と名乗りを残す。控えで答えたことは、その方法を残す。
    let skipped = record(stderr, "skipped a candidate: it answered as another server");
    assert!(
        skipped.contains(&format!("port={port}")) && skipped.contains("\"ssp-mcp-server\""),
        "{skipped}"
    );
    let cached = record(stderr, "[bridge] areka not found; answered from the cache");
    assert!(cached.contains("method=\"initialize\""), "{cached}");
}

/// 要件 5.4・3.11: 同じ受け口と同じ控えの置き場に exe を 2 つ同時に起こしても、それぞれの標準出力は
/// 自分の要求への応答だけで、控えは 2 つとも読める答えの全体のまま（書きかけの一時ファイルも残らない）。
#[test]
fn two_bridges_at_once_keep_their_answers_apart_and_the_cache_readable() {
    let slow = Reply::Delayed(Duration::from_millis(30), Box::new(called()));
    let areka = FakeServer::start(Script::areka().on("tools/call", slow));
    let profile = TempPath::under_target("bridge-exe");
    let (port, dir) = (areka.port(), profile.path());
    // `<名前>-1` が最初の問い合わせ、後は `tools/list`（控えを書く）と `tools/call` を交互に。
    let input = |who: &str| {
        let mut lines = initialize(format!("{who}-1"));
        for n in 2..=20 {
            let id = format!("{who}-{n}");
            lines += &match n % 2 {
                0 => request(id, "tools/list", Value::Null),
                _ => call(id),
            };
        }
        lines
    };

    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(|| run_bridge(port, dir, &input("a")));
        let b = scope.spawn(|| run_bridge(port, dir, &input("b")));
        (a.join().expect("bridge a"), b.join().expect("bridge b"))
    });

    for (who, done) in [("a", &a), ("b", &b)] {
        let Finished { stdout, stderr, .. } = done;
        assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
        let answers = answer_lines(done);
        assert_eq!(answers.len(), 20, "stdout: {stdout}\nstderr: {stderr}");
        for n in 1..=20 {
            let answer = answer_to(&answers, format!("{who}-{n}"));
            assert!(answer.get("result").is_some(), "{answer}\nstderr: {stderr}");
        }
    }
    let cache_dir = profile.child("mcp-bridge");
    let cache = Cache::new(Some(cache_dir.clone()));
    assert!(cache.load_server_info().is_some());
    assert!(cache.load_tools().is_some());
    let left: Vec<_> = std::fs::read_dir(&cache_dir)
        .expect("list the cache dir")
        .map(|entry| entry.expect("an entry of the cache dir").file_name())
        .filter(|name| name.to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(left.is_empty(), "temporary files are left: {left:?}");
}

/// 要件 1.9: 相手が標準入力だけ閉じて標準出力を読まないと、詰まった応答の書きは終わらない。
/// 中継は rmcp の 5 秒を待ってから 6 秒で見切り、理由を標準エラーに残して終了コード 0 で終わる
/// （このテストは約 6 秒かかる。待つだけで、計算はしない）。
#[test]
fn stdin_closed_while_stdout_is_never_read_exits_with_code_0_by_the_bound() {
    let profile = TempPath::under_target("bridge-exe");
    // 応答は 1 行 100 バイトを越えるので、3 万行で標準出力の管が確実に詰まる。
    let input: String = (0..30_000)
        .map(|n| request(n, "ping", Value::Null))
        .collect();

    let done = run(0, profile.path(), &input, false);

    let stderr = &done.stderr;
    assert_eq!(done.code, Some(0), "stderr: {stderr}");
    let quit = record(stderr, "[bridge] stdout is still not read");
    assert!(quit.contains("quitting"), "{quit}");
    let waited = done.waited;
    assert!(
        (Duration::from_secs(5)..Duration::from_secs(15)).contains(&waited),
        "the bridge exited {waited:?} after stdin closed\nstderr: {stderr}"
    );
}

/// 要件 3.5・3.12・設計 D-18: areka も控えも無い（誰も待ち受けていない番号・空の控えの置き場）とき、
/// `initialize` と `tools/list` に、中継が自分で持っている答えで答え、方法ごとに記録を残す。
/// 組み込みの答えは控えに書かない（置き場にファイルができない）。終了コード 0。
#[test]
fn without_areka_or_a_cache_the_built_in_definitions_answer_and_leave_a_record() {
    // 束ねたまま待ち受けないソケットの番号（落とすまで、並んで走るテストに取られない）。
    let held = tokio::net::TcpSocket::new_v4().expect("make a socket");
    let any_port = "127.0.0.1:0".parse().expect("an address");
    held.bind(any_port).expect("bind a free port");
    let port = held.local_addr().expect("the bound port").port();
    let profile = TempPath::under_target("bridge-exe");
    let input = [initialize(1), request(2, "tools/list", Value::Null)];

    let done = run_bridge(port, profile.path(), &input.concat());

    let Finished { stdout, stderr, .. } = &done;
    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    let answers = answer_lines(&done);
    assert_eq!(answers.len(), 2, "stdout: {stdout}\nstderr: {stderr}");
    let first = answer_to(&answers, 1);
    assert_eq!(
        first["result"]["serverInfo"]["name"], SERVER_NAME,
        "stdout: {stdout}\nstderr: {stderr}"
    );
    let tools = answer_to(&answers, 2)["result"]["tools"].as_array();
    assert!(
        tools.is_some_and(|tools| !tools.is_empty()),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    for method in ["initialize", "tools/list"] {
        built_in_record(stderr, method);
    }
    let cache_dir = profile.child("mcp-bridge");
    let left = std::fs::read_dir(&cache_dir).map_or(0, Iterator::count);
    assert_eq!(left, 0, "files are left in {cache_dir:?}");
}

/// 標準エラーに、組み込みの答えで答えた記録の行が、この方法で在ること。
fn built_in_record(stderr: &str, method: &str) {
    let words = "[bridge] areka not found; answered from the built-in definitions";
    let method = format!("method=\"{method}\"");
    let found = |line: &str| line.contains(words) && line.contains(&method);
    assert!(
        stderr.lines().any(found),
        "stderr lacks {words:?} with {method}: {stderr}"
    );
}

/// 要件 5.2・3.10・4.2: 失敗の理由は標準エラーに残る。読めない控えは無いものとして扱い（最初の
/// 問い合わせには組み込みの答えで答える＝要件 3.5）、そのファイルと理由を残す。areka の JSON でない
/// 応答は、状態と本文の頭を残す（要求は送り直さない）。
#[test]
fn a_broken_cache_and_a_broken_answer_leave_their_reason_on_stderr() {
    let profile = TempPath::under_target("bridge-exe");
    // 壊れた控えと、areka でない相手だけ。
    let other = FakeServer::start(Script::named("ssp-mcp-server"));
    let broken = profile.child("mcp-bridge").join("server-info.json");
    std::fs::create_dir_all(broken.parent().expect("the cache dir")).expect("make the cache dir");
    std::fs::write(&broken, b"\xff\xfe not json").expect("write a broken cache");

    let done = run_bridge(other.port(), profile.path(), &initialize(1));

    let Finished { stdout, stderr, .. } = &done;
    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    let answers = answer_lines(&done);
    // 壊れた控えは使わず、組み込みの答えで答える。壊れた控えの警告は残り、ファイルは書き直さない。
    let first = answer_to(&answers, 1);
    assert_eq!(
        first["result"]["serverInfo"]["name"], SERVER_NAME,
        "stdout: {stdout}\nstderr: {stderr}"
    );
    built_in_record(stderr, "initialize");
    let warned = record(stderr, "[bridge] the cache is not readable JSON");
    assert!(
        warned.contains("WARN") && warned.contains(&broken.display().to_string()),
        "{warned}"
    );
    let kept = std::fs::read(&broken).expect("read the broken cache");
    assert_eq!(kept, b"\xff\xfe not json", "the broken cache was rewritten");

    // areka が `tools/call` に 500 の平文で答える。
    let areka = FakeServer::start(Script::areka().on("tools/call", Reply::Text(500, "boom")));
    let input = [initialize(1), call(2)];

    let done = run_bridge(areka.port(), profile.path(), &input.concat());

    let Finished { stdout, stderr, .. } = &done;
    assert_eq!(done.code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    let answers = answer_lines(&done);
    assert_eq!(answers.len(), 2, "stdout: {stdout}\nstderr: {stderr}");
    let error = &answer_to(&answers, 2)["error"];
    assert_eq!(error["code"], -1, "stdout: {stdout}");
    assert_eq!(
        error["message"], "Invalid JSON response from server: HTTP 500 boom",
        "stderr: {stderr}"
    );
    let warned = record(stderr, "[bridge] the answer of areka is not JSON");
    assert!(
        warned.contains("WARN") && warned.contains("500") && warned.contains("boom"),
        "{warned}"
    );
    assert_eq!(areka.requests_of("tools/call").len(), 1);
}
