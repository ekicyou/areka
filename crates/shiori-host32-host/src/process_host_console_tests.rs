//! 補助プロセスをコンソール窓なしで起こすことの兄弟テスト（network-update タスク 11）。
//!
//! 補助プロセスはコンソールの実行体である。GUI の `areka.exe` から窓の指定なしで起こすと
//! Windows が新しいコンソール窓を開き、利用者がそれを閉じると補助プロセスが殺される。
//! 本番の起こし方（[`spawn_command`]）が子をコンソール窓なしで起こすことを、実際に起こした
//! 子の中で `GetConsoleWindow` を問うて確かめる。
//!
//! 子にはこのテストの実行体そのもの（コンソールの実行体）を使い、[`console_window_probe_child`]
//! だけを走らせる。子は結果を標準出力へ書く（標準出力の受け取りが今日どおり届くことも同時に
//! 確かめる）。窓の指定が外れたときの赤は親の環境で 2 通りに分かれるので、両方を見る:
//! - 親にコンソールが無い（本番の GUI の `areka.exe`）→ 子に新しい窓が開く＝`GetConsoleWindow` が窓を返す
//! - 親にコンソールが在る（端末やツールから走らせたテスト）→ 子は親のコンソールを継ぐ＝
//!   `GetConsoleProcessList` に親たちが並ぶ（親のコンソールに窓が無い環境でもこちらで赤になる）
//!
//! `CREATE_NO_WINDOW` 付きなら子は窓の無い自分だけのコンソールを持つ＝窓なし・並ぶのは自分 1 つ。

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::{ExitKind, poll_exit_kind, spawn_command};

#[link(name = "kernel32")]
unsafe extern "system" {
    /// 呼び出したプロセスに結びついたコンソールの窓を返す（無ければ NULL）。
    fn GetConsoleWindow() -> *mut core::ffi::c_void;
    /// 呼び出したプロセスのコンソールにつながるプロセスの数を返す（失敗は 0）。
    fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
}

/// 子が書く行の頭。値は `NULL`（窓なし）か `PRESENT`（窓あり）。
const WINDOW_MARKER: &str = "CONSOLE_WINDOW=";
/// 子が書く行の頭。値は子のコンソールにつながるプロセスの数。
const PROCESSES_MARKER: &str = "CONSOLE_PROCESSES=";

/// 子の側で走る観測の本体（親から `--exact --ignored` で名指しされたときだけ走る）。
///
/// 判定はしない（親のテスト実行で `--include-ignored` が付いても赤にならない）。
/// 自分のコンソールの窓の有無とつながるプロセスの数を書くだけで、判定は親の
/// [`spawned_helper_has_no_console_window`] が行う。
#[test]
#[ignore = "spawned_helper_has_no_console_window が子として起こす観測の本体"]
fn console_window_probe_child() {
    // SAFETY: 引数なし・副作用なしの問い合わせ。
    let window = unsafe { GetConsoleWindow() };
    let mut list = [0u32; 64];
    // SAFETY: 渡す領域は 64 要素で、数は領域の大きさと一致している。
    let processes = unsafe { GetConsoleProcessList(list.as_mut_ptr(), 64) };
    let state = if window.is_null() { "NULL" } else { "PRESENT" };
    println!("{WINDOW_MARKER}{state}");
    println!("{PROCESSES_MARKER}{processes}");
}

/// 子の標準出力から `marker` の後ろの値（行末まで）を取り出す（`=1` が `=12` に当たらないよう
/// 値で比べる。libtest がテスト名の行に続けて書くので行頭とは限らない）。
fn probe_value<'a>(stdout: &'a str, marker: &str) -> Option<&'a str> {
    let rest = &stdout[stdout.find(marker)? + marker.len()..];
    Some(rest.lines().next().unwrap_or("").trim())
}

#[test]
fn spawned_helper_has_no_console_window() {
    let exe = std::env::current_exe().expect("テストの実行体のパスが取れる");
    let mut command = Command::new(exe);
    command
        .args([
            "--exact",
            "process_host::console_tests::console_window_probe_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .stdout(Stdio::piped());

    let mut handle = spawn_command(command).expect("観測の子を起こせる");
    let mut stdout = String::new();
    handle
        .child
        .stdout
        .take()
        .expect("標準出力を受け取る管がある")
        .read_to_string(&mut stdout)
        .expect("子の標準出力を読める");

    let deadline = Instant::now() + Duration::from_secs(30);
    let kind = loop {
        if let Some(kind) = poll_exit_kind(&mut handle) {
            break kind;
        }
        assert!(Instant::now() < deadline, "観測の子が締切内に終わらない");
        std::thread::sleep(Duration::from_millis(5));
    };

    assert_eq!(kind, ExitKind::Clean, "観測の子が正常に終わる: {stdout:?}");
    // 子が観測の本体を走らせたこと（フィルタの取り違えで 0 件のまま緑にならない）を先に確かめる。
    let window = probe_value(&stdout, WINDOW_MARKER);
    let processes = probe_value(&stdout, PROCESSES_MARKER);
    assert!(
        window.is_some() && processes.is_some(),
        "観測の子が結果の行を書いていない: {stdout:?}"
    );
    assert_eq!(
        window,
        Some("NULL"),
        "本番の起こし方で起こした子にコンソール窓がある（CREATE_NO_WINDOW が外れている）: {stdout:?}"
    );
    assert_eq!(
        processes,
        Some("1"),
        "本番の起こし方で起こした子が親のコンソールを継いでいる（CREATE_NO_WINDOW が外れている）: {stdout:?}"
    );
}
