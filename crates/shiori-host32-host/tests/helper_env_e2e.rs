//! 親に `PASTA_DEBUG=1` が立っていても、helper の下では `0` になることを確かめる
//! （audit-shiori-host32 件 2・U4・要件 4.1・4.4・8.5・design.md「件 2 の試験」）。
//!
//! 親のテスト [`pasta_debug_is_zero_under_the_helper`] は自分の実行ファイルを
//! `--exact <観測の本体> --ignored` で起こし直し、その子にだけ `PASTA_DEBUG=1` を渡す
//! （自分のプロセスの環境は書き換えない＝並んで走るテストを乱さない。
//! `src/process_host_console_tests.rs` と同じ形）。
//!
//! 観測の本体 [`pasta_debug_probe_child`] は前提（自分の環境が `1`）を確かめてから、本番の
//! [`spawn`] で実の i686 helper を起こし、握手・LOAD（試験用の DLL）・`GET OnTestEnv` で
//! helper の下の DLL が見る `PASTA_DEBUG` を読み、結果の行を標準出力に書く。
//! `process_host.rs` の `helper_command` から `.env("PASTA_DEBUG", "0")` を消すと値が `1` になって赤。
//!
//! 成果物（i686 helper・testdll）が無ければ `common` の探し方が panic する（要件 4.4）。
//!
//! ## 実行前提（PowerShell・Git Bash 不可）
//! ```powershell
//! cargo build -p shiori-host32-helper -p shiori-host32-testdll --target i686-pc-windows-msvc
//! cargo test  -p shiori-host32-host --test helper_env_e2e
//! ```

use std::process::Command;

use shiori_host32_host::process_host::LOAD_ACK_TIMEOUT;
use shiori_host32_host::{Charset, CharsetNegotiator, ParentMessageWindow, Shiori3Client, spawn};
use shiori_host32_ipc::MsgTag;

mod common;
use common::{HANDSHAKE_TIMEOUT, TestRoot, resolve_helper_exe, resolve_testdll};

/// 観測の本体が標準出力に書く結果の行の印（後ろに helper の下の値が続く）。
const VALUE_MARKER: &str = "HELPER_PASTA_DEBUG=";

/// 試験用の DLL の試しの ID（testdll `TEST_ENV_ID` と一致・host のテストは testdll へ依存しないので写す）。
const TEST_ENV_ID: &str = "OnTestEnv";

/// 観測の本体。[`pasta_debug_is_zero_under_the_helper`] が子として起こす。
#[test]
#[ignore = "pasta_debug_is_zero_under_the_helper が子として起こす観測の本体"]
fn pasta_debug_probe_child() {
    // 前提: 親が渡した 1 が自分に届いている（届いていなければ試験が空になる）。
    assert_eq!(
        std::env::var("PASTA_DEBUG").as_deref(),
        Ok("1"),
        "観測の子に PASTA_DEBUG=1 が渡っていない（試験の前提が崩れている）"
    );

    let helper_exe = resolve_helper_exe();
    let testdll = resolve_testdll();
    let root = TestRoot::new("helper-env");
    std::fs::copy(&testdll, root.path().join("shiori.dll"))
        .expect("testdll をテストの根へ写せない");

    let parent = ParentMessageWindow::create().expect("親 message-only 窓を作れない");
    // panic しても `HelperHandle` の Drop が helper を終わらせて待つ。
    let handle = spawn(&helper_exe, root.path(), "shiori.dll", parent.hwnd_u32())
        .expect("i686 helper を本番の起こし方で起こせない");
    assert!(
        parent.pump_until_hello_or(HANDSHAKE_TIMEOUT).is_some(),
        "上限時間内に helper から HELLO が届かない"
    );
    let load_ack = parent
        .send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)
        .expect("LOAD の送りが失敗");
    assert_eq!(load_ack, vec![1u8], "LOAD が成功の [1] を返さない");

    let mut negotiator = CharsetNegotiator::new(Charset::UTF_8, false);
    let value = Shiori3Client::new(&parent, &mut negotiator)
        .get(TEST_ENV_ID, &[], None)
        .expect("GET OnTestEnv が Err");
    println!("{VALUE_MARKER}{}", value.unwrap_or_default());

    drop(handle);
    drop(parent);
}

#[test]
fn pasta_debug_is_zero_under_the_helper() {
    let exe = std::env::current_exe().expect("テストの実行体のパスが取れる");
    // 子の締切は子の中の上限（握手 30 秒・LOAD・GET の上限）が持つ。
    let output = Command::new(exe)
        .args([
            "--exact",
            "pasta_debug_probe_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PASTA_DEBUG", "1")
        .output()
        .expect("観測の子を起こせる");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "観測の子が正常に終わらない（{}）: stdout={stdout:?} stderr={stderr:?}",
        output.status
    );
    // libtest がテスト名の行に続けて書くので行頭とは限らない。値は行末まで。
    let value = stdout.find(VALUE_MARKER).map(|at| {
        stdout[at + VALUE_MARKER.len()..]
            .lines()
            .next()
            .unwrap_or("")
            .trim()
    });
    assert_eq!(
        value,
        Some("0"),
        "helper の下の PASTA_DEBUG が 0 でない（親の 1 を受け継いでいる・行が無いなら観測の本体が走っていない）: {stdout:?}"
    );
}
