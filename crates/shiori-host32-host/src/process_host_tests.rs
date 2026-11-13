//! `process_host` の兄弟テスト（終了の分類・非ブロッキングの問い合わせ・Drop・起動の契約）。

use super::*;
use std::time::{Duration, Instant};

/// stand-in（`cmd.exe /c exit N`）を組んだ Command を返す。`lifecycle` のテストもこれを使う。
pub(crate) fn cmd_exit(code: i32) -> Command {
    let mut command = Command::new("cmd.exe");
    command.args(["/c", "exit", &code.to_string()]);
    command
}

/// handle が終了するまで非ブロッキング poll を回して終了種別を得る。
///
/// 各 poll がブロックしないこと（要件 1.2）を、poll 単体の所要時間が
/// ごく短いことで担保しつつ、全体は上限時間で bounded に待つ。
fn wait_kind(handle: &mut HelperHandle) -> ExitKind {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(kind) = poll_exit_kind(handle) {
            return kind;
        }
        assert!(
            Instant::now() < deadline,
            "helper が上限時間内に終了しなかった"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// `try_wait` の失敗は `ERROR` を 1 行書いてから「動いている」（`None`）とみなす（指摘 10）。
#[test]
fn a_try_wait_failure_is_logged() {
    let failure = Err(std::io::Error::other("try_wait failed"));
    let (kind, events) = log_capture_kit::capture(|| note_try_wait(failure));
    assert_eq!(kind, None);
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.target == LOG_TARGET && e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "{events:?}");
}

/// job を持たない `HelperHandle` でも、捨てれば子が終わっている（Drop が終わらせて待つ・指摘 14）。
#[test]
fn dropping_a_handle_without_job_ends_the_child() {
    use std::os::windows::io::{AsHandle, AsRawHandle};
    use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::WaitForSingleObject;

    let child = Command::new("ping.exe")
        .args(["-n", "60", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("ping stand-in を spawn できる");
    let process = child
        .as_handle()
        .try_clone_to_owned()
        .expect("取っ手を複製できる");
    drop(HelperHandle {
        child,
        _job: None,
        stderr_reader: None,
    });
    // SAFETY: `process` は生きているプロセスの取っ手の複製。待ちは 0 ミリ秒（問い合わせだけ）。
    let state = unsafe { WaitForSingleObject(HANDLE(process.as_raw_handle()), 0) };
    assert_eq!(state, WAIT_OBJECT_0, "Drop の後も子が動いている");
}

// --- ExitKind::classify 純関数（要件 1.3 / 1.4）---

#[test]
fn classify_clean_from_exit_zero() {
    // 実プロセスで code() == Some(0) を作り Clean へ分類（要件 1.3）。
    let status = cmd_exit(0)
        .status()
        .expect("cmd.exe /c exit 0 が起動できる");
    assert_eq!(ExitKind::classify(&status), ExitKind::Clean);
}

#[test]
fn classify_abnormal_from_nonzero_exit() {
    // 実プロセスで code() == Some(3) を作り Abnormal(3) へ分類（要件 1.4）。
    let status = cmd_exit(3)
        .status()
        .expect("cmd.exe /c exit 3 が起動できる");
    assert_eq!(ExitKind::classify(&status), ExitKind::Abnormal(3));
}

// --- spawn + 非ブロッキング poll（要件 1.1 / 1.2 / 1.3 / 1.4）---

#[test]
fn spawn_command_then_poll_classifies_clean() {
    // stand-in を spawn（下位 seam）→ 終了後に Clean 分類（要件 1.1/1.3）。
    let mut handle = spawn_command(cmd_exit(0)).expect("stand-in を spawn できる");
    assert!(handle.pid() > 0, "spawn 成功で helper への参照を保持する");
    assert_eq!(wait_kind(&mut handle), ExitKind::Clean);
}

#[test]
fn spawn_command_then_poll_classifies_abnormal() {
    // stand-in を spawn → 終了後に Abnormal(5) 分類（要件 1.4）。
    let mut handle = spawn_command(cmd_exit(5)).expect("stand-in を spawn できる");
    assert_eq!(wait_kind(&mut handle), ExitKind::Abnormal(5));
}

// --- spawn の起動パラメーター契約（arg1..3・env 3 種・cwd＝design §SpawnContract）---

/// env キー名が design §SpawnContract どおりに固定されていること（契約回帰防止）。
#[test]
fn spawn_env_key_names_match_contract() {
    assert_eq!(PARENT_HWND_ENV, "HOST32_PARENT_HWND");
    assert_eq!(LOAD_DIR_ENV, "HOST32_LOAD_DIR");
    assert_eq!(SHIORI_NAME_ENV, "HOST32_SHIORI_NAME");
}

/// LOAD 用 ack 待機の推奨既定 timeout が 30 秒であること（R5.2・design §Load timeout）。
#[test]
fn load_ack_timeout_default_is_30s() {
    assert_eq!(LOAD_ACK_TIMEOUT, Duration::from_secs(30));
}

// --- request timeout 定数 + env seam 解決（R4.3 / R5.1・design §Shiori3Client）---

/// request 往復の既定 timeout が 60 秒で、env キー名が契約どおりであること（回帰防止）。
#[test]
fn request_timeout_default_is_60s_and_env_key_matches() {
    assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(60));
    assert_eq!(REQUEST_TIMEOUT_ENV, "AREKA_SHIORI_REQUEST_TIMEOUT_MS");
}

/// env 不在／空／空白のみ／parse 不能 → 既定 60s へ安全側フォールバック（無限へ昇格させない）。
#[test]
fn resolve_request_timeout_defaults_to_60s_when_absent_or_garbage() {
    // 不在。
    assert_eq!(resolve_request_timeout(None), Some(REQUEST_TIMEOUT));
    assert_eq!(resolve_request_timeout(None), Some(Duration::from_secs(60)));
    // 空文字・空白のみ。
    assert_eq!(
        resolve_request_timeout(Some(String::new())),
        Some(Duration::from_secs(60))
    );
    assert_eq!(
        resolve_request_timeout(Some("   ".to_string())),
        Some(Duration::from_secs(60))
    );
    // parse 不能（非数値・負値・小数）はすべて既定へ（無限待ちへ暗黙昇格しない）。
    assert_eq!(
        resolve_request_timeout(Some("abc".to_string())),
        Some(Duration::from_secs(60))
    );
    assert_eq!(
        resolve_request_timeout(Some("-1".to_string())),
        Some(Duration::from_secs(60))
    );
    assert_eq!(
        resolve_request_timeout(Some("1.5".to_string())),
        Some(Duration::from_secs(60))
    );
}

/// env `"0"` → `None`（無限待ち・デバッグ opt-in）。
#[test]
fn resolve_request_timeout_zero_means_infinite() {
    assert_eq!(resolve_request_timeout(Some("0".to_string())), None);
    // 前後空白付き "0" も trim 後に 0 として無限待ちへ。
    assert_eq!(resolve_request_timeout(Some("  0  ".to_string())), None);
}

/// env `"<n>"`（n > 0・ミリ秒） → `Some(Duration::from_millis(n))`。
#[test]
fn resolve_request_timeout_positive_millis_are_used() {
    assert_eq!(
        resolve_request_timeout(Some("1500".to_string())),
        Some(Duration::from_millis(1500))
    );
    // trim 済みで採用されること。
    assert_eq!(
        resolve_request_timeout(Some("  250  ".to_string())),
        Some(Duration::from_millis(250))
    );
}

/// `spawn` が使う [`helper_command`] の中身（引数・環境・作業フォルダ）を読んで確かめる
/// （R3.1〜3.3・要件 8.1・8.2・10.4）。
#[test]
fn helper_command_sets_args_env_cwd_and_pasta_debug_zero() {
    let parent_hwnd: u32 = 4_294_967_295; // u32::MAX（10進境界値）
    let load_dir = Path::new(r"C:\ghost\master");
    let command = helper_command(Path::new("helper.exe"), load_dir, "shiori.dll", parent_hwnd);

    let args: Vec<&std::ffi::OsStr> = command.get_args().collect();
    assert_eq!(
        args,
        ["4294967295", r"C:\ghost\master", "shiori.dll"].map(std::ffi::OsStr::new)
    );
    let envs: Vec<(&std::ffi::OsStr, Option<&std::ffi::OsStr>)> = command.get_envs().collect();
    for (key, value) in [
        (PARENT_HWND_ENV, "4294967295"),
        (LOAD_DIR_ENV, r"C:\ghost\master"),
        (SHIORI_NAME_ENV, "shiori.dll"),
        ("PASTA_DEBUG", "0"),
    ] {
        assert!(
            envs.contains(&(key.as_ref(), Some(value.as_ref()))),
            "環境に ({key:?}, Some({value:?})) が無い: {envs:?}"
        );
    }
    assert_eq!(command.get_current_dir(), Some(load_dir));
}
