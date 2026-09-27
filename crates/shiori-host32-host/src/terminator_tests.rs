//! `HelperTerminator` の兄弟テスト（要件 2.4・3.5）。
//!
//! 長命の子を `Child` を持たない別スレッドから終わらせ、締切内に子の終了が観測でき、
//! 終わった後にもう一度呼んでも成功する（冪等）ことを実プロセスで確かめる。

use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::HANDLE;

use super::HelperTerminator;
use crate::lifecycle::{HelperLifecycle, HelperStatus};
use crate::process_host::{ExitKind, HelperHandle, poll_exit_kind, spawn_command};

/// 約 60 秒生きる子（`ping` を直接起こす。`cmd /c` を挟むと孫が残るため挟まない）。
/// 終わらせなければ締切（10 秒）を大きく超えて生き続ける。
fn long_lived() -> Command {
    let mut command = Command::new("ping.exe");
    command
        .args(["-n", "60", "127.0.0.1"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// 締切（10 秒）内に子の終了を非ブロッキングで待ち、終了の種別を返す。
fn wait_exit(handle: &mut HelperHandle) -> ExitKind {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(kind) = poll_exit_kind(handle) {
            return kind;
        }
        assert!(
            Instant::now() < deadline,
            "terminate の後も子が締切内に終わらなかった"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 型の約束: 複製でき、スレッドをまたいで渡せ、共有できる。
#[test]
fn terminator_is_clone_send_sync() {
    fn assert_bounds<T: Clone + Send + Sync + 'static>() {}
    assert_bounds::<HelperTerminator>();
}

/// `HelperHandle::terminator` の複製を別スレッドから `terminate` すると子が締切内に終わり
/// （終了コード 1＝`Abnormal(1)`）、終わった後に呼んでも（複製元・複製先どちらからでも）成功する。
#[test]
fn terminate_from_another_thread_ends_child_and_is_idempotent() {
    let mut handle = spawn_command(long_lived()).expect("長命の子を spawn できる");
    let terminator = handle.terminator().expect("取っ手を複製できる");
    // 対照: 終わらせる前は生きている。
    assert_eq!(
        poll_exit_kind(&mut handle),
        None,
        "終わらせる前は生きている"
    );

    let remote = terminator.clone();
    std::thread::spawn(move || remote.terminate())
        .join()
        .expect("別スレッドが panic しない")
        .expect("1 度目の terminate は成功");

    assert_eq!(wait_exit(&mut handle), ExitKind::Abnormal(1));
    terminator
        .terminate()
        .expect("終わった後の 2 度目の terminate も成功（冪等）");
    terminator
        .clone()
        .terminate()
        .expect("複製からの 3 度目も成功（冪等）");
}

/// 1 度目の直後、終了を待たずに 2 度目を呼んでも成功する（終わりかけのプロセスでも冪等）。
/// 終わりかけの間だけ開く隙間を踏むため、別々の子で数回くり返す。
#[test]
fn terminate_twice_back_to_back_is_ok() {
    for round in 0..10 {
        let mut handle = spawn_command(long_lived()).expect("長命の子を spawn できる");
        let terminator = handle.terminator().expect("取っ手を複製できる");
        terminator
            .terminate()
            .unwrap_or_else(|e| panic!("{round} 回目の組の 1 度目が失敗: {e:?}"));
        terminator
            .terminate()
            .unwrap_or_else(|e| panic!("{round} 回目の組の直後の 2 度目が失敗: {e:?}"));
        assert_eq!(wait_exit(&mut handle), ExitKind::Abnormal(1));
    }
}

/// `HelperLifecycle::terminator` は保持する `HelperHandle` へ委譲し、同じ子を終わらせる。
#[test]
fn lifecycle_terminator_delegates_to_handle() {
    let handle = spawn_command(long_lived()).expect("長命の子を spawn できる");
    let mut lc = HelperLifecycle::new(handle);
    assert_eq!(
        lc.status(),
        HelperStatus::Running,
        "終わらせる前は生きている"
    );

    let terminator = lc.terminator().expect("取っ手を複製できる");
    terminator.terminate().expect("1 度目の terminate は成功");

    let deadline = Instant::now() + Duration::from_secs(10);
    let kind = loop {
        if let HelperStatus::Exited(kind) = lc.status() {
            break kind;
        }
        assert!(
            Instant::now() < deadline,
            "terminate の後も子が締切内に終わらなかった"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(kind, ExitKind::Abnormal(1));
    terminator
        .terminate()
        .expect("終わった後の 2 度目の terminate も成功（冪等）");
}

// --- 往復の最中に宛先のプロセスを終わらせると同期の送信が戻る（要件 1.3・2.4・7.2）---
//
// 上限に達したときは補助プロセスを終わらせて、止まっている `SendMessageTimeoutW` を解く。
// その前提を x64 の別プロセスの窓で固定する。本物の補助プロセス（i686）に対する同じ形は、
// テスト DLL に「固まる」応答が無いので常設しない（実機確認で固まる SHIORI を用意できる場合に限る）。
// 終わらせた「後」に送る形は `tests/lifecycle_kill_e2e.rs` が固定しており、ここはその前段（最中）。

/// 子の入口だけが読む旗（本番コードは読まない）。値は「送信が届いた」を親へ知らせる名前つきの
/// イベントの名前。
const HOLD_CHILD_ENV: &str = "AREKA_TEST_TERMINATOR_HOLD_CHILD";

/// 子の窓の題（pid 入りで一意）。
fn hold_title(pid: u32) -> String {
    format!("areka-terminator-hold-{pid}")
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// `HANDLE` を閉じ忘れないよう `OwnedHandle` へ移す。
fn owned(handle: HANDLE) -> OwnedHandle {
    // SAFETY: 呼び手は作ったばかりで他に持ち主の居ない有効な取っ手を渡す。
    unsafe { OwnedHandle::from_raw_handle(handle.0) }
}

/// 題 `title` の `STATIC`・`HWND_MESSAGE`（message-only）の窓を今のスレッドに作る。
fn create_hold_window(title: &str) -> windows::Win32::Foundation::HWND {
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
    };
    use windows::core::{PCWSTR, w};

    let title = wide(title);
    // SAFETY: 題の文字列は呼び出しの間生きている。親は HWND_MESSAGE（message-only）。
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            PCWSTR(title.as_ptr()),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )
    }
    .expect("message-only 窓を作れる")
}

/// 窓へ `WM_GETTEXTLENGTH` を期限 30 秒・応答なし判定なし（`SMTO_NORMAL`）で同期に送り、
/// `(戻り値, 窓の応答, 最後のエラー)` を返す。窓が応答すれば題の長さ（> 0）が返る。
fn send_get_text_length(hwnd: isize) -> (isize, usize, windows::Win32::Foundation::WIN32_ERROR) {
    use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        SMTO_NORMAL, SendMessageTimeoutW, WM_GETTEXTLENGTH,
    };

    let mut result = 0usize;
    // SAFETY: ポインタを運ばない問い合わせ。宛先が消えても送り手は安全に戻る。
    let ret = unsafe {
        SendMessageTimeoutW(
            HWND(hwnd as *mut _),
            WM_GETTEXTLENGTH,
            WPARAM(0),
            LPARAM(0),
            SMTO_NORMAL,
            30_000,
            Some(&mut result as *mut usize),
        )
    };
    // SAFETY: 同じスレッドの直前の呼び出しの最後のエラーを読むだけ。
    (ret.0, result, unsafe { GetLastError() })
}

/// 子の入口。旗が無ければ何もせず緑。旗があれば `STATIC`・`HWND_MESSAGE` の窓を作り、
/// メッセージを配らずに送信の到着を `GetQueueStatus` で見て（有界に覗く）、到着したら旗の名前の
/// イベントを立てて、配らないまま眠る（親に終わらされる）。到着の知らせは窓を通らないイベントにする
/// （窓の呼び出しで知らせると、その中で届いた送信を配ってしまうおそれがある）。
#[test]
fn hold_child_entry() {
    use windows::Win32::System::Threading::{EVENT_MODIFY_STATE, OpenEventW, SetEvent};
    use windows::Win32::UI::WindowsAndMessaging::{GetQueueStatus, QS_SENDMESSAGE};
    use windows::core::PCWSTR;

    let Some(event_name) = std::env::var_os(HOLD_CHILD_ENV) else {
        return;
    };
    let event_name = wide(event_name.to_str().expect("イベントの名前は UTF-8"));
    // SAFETY: 名前の文字列は呼び出しの間生きている。
    let held = owned(
        unsafe { OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(event_name.as_ptr())) }
            .expect("親のイベントを開ける"),
    );
    create_hold_window(&hold_title(std::process::id()));
    // 送信が届くまで覗く（親が来なければ 60 秒で諦めて緑で終わる）。下位語＝今キューにある種類。
    let deadline = Instant::now() + Duration::from_secs(60);
    // SAFETY: キューの状態を読むだけで、送られてきたメッセージは配らない。
    while unsafe { GetQueueStatus(QS_SENDMESSAGE) } & QS_SENDMESSAGE.0 == 0 {
        if Instant::now() >= deadline {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // SAFETY: `held` は開いたばかりの有効なイベントの取っ手。
    unsafe { SetEvent(HANDLE(held.as_raw_handle())) }.expect("イベントを立てられる");
    std::thread::sleep(Duration::from_secs(60));
}

/// 自分で起こした子を、テストがどこで終わっても残さない。
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// 題で子の message-only 窓を有界（30 秒）に探す。子が先に終われば赤。
fn find_child_window(child: &mut KillOnDrop, title: &str) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, HWND_MESSAGE};
    use windows::core::{PCWSTR, w};

    let name = wide(title);
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        // SAFETY: 題の文字列は呼び出しの間生きている。
        if let Ok(hwnd) = unsafe {
            FindWindowExW(
                Some(HWND_MESSAGE),
                None,
                w!("STATIC"),
                PCWSTR(name.as_ptr()),
            )
        } {
            return hwnd.0 as isize;
        }
        let exited = child.0.try_wait().expect("子の状態を読める");
        assert!(
            exited.is_none(),
            "子が窓 {title} を出す前に終わった: {exited:?}"
        );
        assert!(
            Instant::now() < deadline,
            "子の窓 {title} が 30 秒で見つからない"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// 別スレッドから期限 30 秒・応答なし判定なし（`SMTO_NORMAL`）で同期に送り、子が送信の到着を
/// 見た（配らずに抱えている＝往復の最中）後に取っ手で子を終わらせると、送信が期限切れでなく戻る。
///
/// 判定は時計に依らない: 期限切れは必ず「戻り値 0・最後のエラー `ERROR_TIMEOUT`」なので、その組で
/// ないことを見る。前提が崩れれば 30 秒後にその組で戻って赤になる（止まらない）。
/// 実測（2026-09-27・x64）では宛先のスレッドが消えた送信は「戻り値 1・窓の応答 0・最後のエラー 0」で
/// 戻る（受け手が消えた送信を OS が応答 0 で閉じる）。窓の応答が 0 であること（子の窓が答えれば題の
/// 長さが返る＝同じスレッドから送る対照で確かめる）で、子が配ってしまった回と見分ける。
#[test]
fn terminate_mid_send_releases_synchronous_send() {
    use windows::Win32::Foundation::{ERROR_TIMEOUT, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
    use windows::core::PCWSTR;

    // 対照: 同じ形の窓が答えれば、応答は題の長さ（> 0）になる。
    let control_title = format!("{}-control", hold_title(std::process::id()));
    let control = create_hold_window(&control_title);
    let (ret, result, _) = send_get_text_length(control.0 as isize);
    assert_ne!(ret, 0, "対照の送信は成功する");
    assert_eq!(
        result,
        control_title.encode_utf16().count(),
        "対照の窓は題の長さを答える"
    );

    let event_name = format!(r"Local\areka-terminator-held-{}", std::process::id());
    let wide_event = wide(&event_name);
    // SAFETY: 名前の文字列は呼び出しの間生きている。手動リセット・初期は下り。
    let held = owned(
        unsafe { CreateEventW(None, true, false, PCWSTR(wide_event.as_ptr())) }
            .expect("イベントを作れる"),
    );

    let exe = std::env::current_exe().expect("テストの実行体の場所を読める");
    let entry = format!(
        "{}::hold_child_entry",
        module_path!()
            .split_once("::")
            .expect("crate 名の後の経路")
            .1
    );
    let mut child = KillOnDrop(
        Command::new(exe)
            .args([entry.as_str(), "--exact", "--test-threads=1"])
            .env(HOLD_CHILD_ENV, &event_name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("テストの実行体を子として起こせる"),
    );
    let title = hold_title(child.0.id());
    let hwnd = find_child_window(&mut child, &title);

    let sender = std::thread::spawn(move || send_get_text_length(hwnd));

    // 子が送信の到着を見た＝往復の最中（送信は 30 秒の期限までまだ戻らない）。
    // SAFETY: `held` は作ったばかりの有効なイベントの取っ手。
    let wait = unsafe { WaitForSingleObject(HANDLE(held.as_raw_handle()), 30_000) };
    assert_eq!(wait, WAIT_OBJECT_0, "子が 30 秒で送信の到着を知らせない");
    HelperTerminator::from_child(&child.0)
        .expect("取っ手を複製できる")
        .terminate()
        .expect("往復の最中の terminate は成功");

    let (ret, result, last_error) = sender.join().expect("送信のスレッドが panic しない");
    assert!(
        !(ret == 0 && last_error == ERROR_TIMEOUT),
        "送信が期限切れで戻った＝終わらせても同期の送信が解けていない（戻り値 {ret}・{last_error:?}）"
    );
    assert_eq!(
        result, 0,
        "子の窓は答えていない（題の長さが返れば子が配ってしまった）"
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    while child.0.try_wait().expect("子の状態を読める").is_none() {
        assert!(
            Instant::now() < deadline,
            "terminate の後も子が締切内に終わらなかった"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
