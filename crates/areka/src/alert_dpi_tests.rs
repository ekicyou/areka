//! 画面を出す間だけスレッドを v2 に囲む部品のテスト（alert-dialog-per-monitor-dpi）。
//!
//! 関門 0（要件 4.6）: 実機でメッセージボックスを出し、その窓の DPI の扱いを areka のコードの中から
//! 読む 2 本（`#[ignore]`・排他テストの机を取ってから 1 本ずつ別のプロセスで走らせる）。
//! 画面が出ている間に同じスレッドのタイマーの呼び戻しで題名 [`PROBE_TITLE`] の `#32770` の窓を探し、
//! 読んだら閉じる（メッセージボックスは `WM_CLOSE`・タスクダイアログは `TDM_CLICK_BUTTON`）。
//! 場合ごとに ASCII の `ALERT-DPI-PROBE ...` を 1 行出す。案 B の測り（タスクダイアログ）も同じ道で読む。
//!
//! 測りが直に出すメッセージボックス・タイマー・窓の読み取り・スレッドの局所の置き場の `unsafe` は
//! このファイルに閉じ、`alert.rs` には置かない（`MessageBoxW` は本番の 1 か所のまま）。

use std::cell::Cell;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentThreadId};
use windows::Win32::UI::Controls::{TD_ERROR_ICON, TDCBF_OK_BUTTON, TDM_CLICK_BUTTON, TaskDialog};
use windows::Win32::UI::HiDpi::{
    AreDpiAwarenessContextsEqual, DPI_AWARENESS_CONTEXT,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, DPI_AWARENESS_CONTEXT_UNAWARE,
    GetAwarenessFromDpiAwarenessContext, GetDpiAwarenessContextForProcess, GetDpiForMonitor,
    GetDpiForSystem, GetDpiForWindow, GetThreadDpiAwarenessContext, GetWindowDpiAwarenessContext,
    MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumThreadWindows, GetClassNameW, GetWindowTextW, IDOK, KillTimer, MB_OK, MessageBoxW,
    PostMessageW, PostQuitMessage, SetTimer, WM_CLOSE,
};
use windows::core::{BOOL, HSTRING, PCWSTR};

// ---------------------------------------------------------------- 関門 0 の測り（実機・#[ignore]）

/// 測りの画面の題名（窓を探す鍵）。
const PROBE_TITLE: &str = "areka-alert-dpi-probe";
/// 測りの画面の本文。
const PROBE_BODY: &str = "areka alert DPI probe (closes by itself)";
/// 窓を探す間隔（ミリ秒）。
const TICK_MS: u32 = 300;
/// 探す回数の上限（およそ 6 秒）。超えたらモーダルを抜けて「見つからない」とする。
const MAX_TICKS: u32 = 20;

/// 画面が出ている間に読んだ値。
#[derive(Debug, Clone, Copy)]
struct WindowRead {
    thread_during: i32,
    window: i32,
    window_v2: bool,
    window_dpi: u32,
    monitor_dpi: u32,
    system_dpi: u32,
}

/// 1 つの場合の結果。
#[derive(Debug, Clone)]
struct ProbeRow {
    line: String,
    read: Option<WindowRead>,
}

thread_local! {
    /// 呼び戻しが探した回数。場合ごとに新しいスレッドなので戻さない。
    static TICKS: Cell<u32> = const { Cell::new(0) };
    /// 見つけた窓から読んだ値。
    static FOUND: Cell<Option<WindowRead>> = const { Cell::new(None) };
    /// 読んだ後に窓へ送る閉じる知らせ（メッセージ, wParam）。`WM_COMMAND`＋`IDOK` では
    /// メッセージボックスは閉じない（実機で確かめた）。タスクダイアログは `WM_CLOSE` で閉じない。
    static CLOSE: Cell<(u32, usize)> = const { Cell::new((WM_CLOSE, 0)) };
}

/// 扱いの値を 0＝非対応・1＝system・2＝モニタごとの数にする（無効なら -1）。
fn awareness(context: DPI_AWARENESS_CONTEXT) -> i32 {
    // SAFETY: 値を読むだけの関数。無効な値には DPI_AWARENESS_INVALID（-1）が返る。
    unsafe { GetAwarenessFromDpiAwarenessContext(context) }.0
}

/// 呼んだスレッドの今の扱い。
fn thread_awareness() -> i32 {
    // SAFETY: 引数なしで呼んだスレッドの値を読むだけ。
    awareness(unsafe { GetThreadDpiAwarenessContext() })
}

/// `EnumThreadWindows` の呼び戻し: クラス `#32770`・題名 [`PROBE_TITLE`] の窓を `lparam` の先へ書いて止める。
unsafe extern "system" fn find_probe(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let (mut class, mut title) = ([0u16; 64], [0u16; 64]);
    // SAFETY: `hwnd` は OS が列挙で渡した窓、バッファはこの関数の中で生きている。
    let (class_len, title_len) = unsafe {
        (
            GetClassNameW(hwnd, &mut class),
            GetWindowTextW(hwnd, &mut title),
        )
    };
    let class = String::from_utf16_lossy(&class[..class_len.max(0) as usize]);
    let title = String::from_utf16_lossy(&title[..title_len.max(0) as usize]);
    if class == "#32770" && title == PROBE_TITLE {
        // SAFETY: `lparam` は `on_tick` の局所の `HWND` を指し、列挙の間ずっと生きている。
        unsafe { *(lparam.0 as *mut HWND) = hwnd };
        return BOOL(0);
    }
    BOOL(1)
}

/// タイマーの呼び戻し（モーダルの間も同じスレッドで配られる）。見つけたら読んで閉じ、
/// [`MAX_TICKS`] 回で見つからなければモーダルを抜けさせる。
unsafe extern "system" fn on_tick(_: HWND, _: u32, timer: usize, _: u32) {
    let mut hwnd = HWND::default();
    // SAFETY: 呼び戻しは上の `find_probe`、`lparam` は呼び出しの間生きている局所の `hwnd` を指す。
    // 呼び戻しが列挙を止めると FALSE が返るので戻り値は見ない。
    let _ = unsafe {
        EnumThreadWindows(
            GetCurrentThreadId(),
            Some(find_probe),
            LPARAM(&mut hwnd as *mut HWND as isize),
        )
    };
    if !hwnd.is_invalid() {
        let mut monitor_dpi = (0u32, 0u32);
        // SAFETY: `hwnd` は今このスレッドで出ている画面の窓。読むだけの関数と、
        // この関数の中で生きている出力先だけを渡す。
        let read = unsafe {
            let context = GetWindowDpiAwarenessContext(hwnd);
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let _ = GetDpiForMonitor(
                monitor,
                MDT_EFFECTIVE_DPI,
                &mut monitor_dpi.0,
                &mut monitor_dpi.1,
            );
            WindowRead {
                thread_during: thread_awareness(),
                window: awareness(context),
                window_v2: AreDpiAwarenessContextsEqual(
                    context,
                    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                )
                .as_bool(),
                window_dpi: GetDpiForWindow(hwnd),
                monitor_dpi: monitor_dpi.0,
                system_dpi: GetDpiForSystem(),
            }
        };
        FOUND.set(Some(read));
        // SAFETY: 窓は生きている画面の窓、`timer` は OS が渡したこのスレッドのタイマー。
        unsafe {
            let (message, wparam) = CLOSE.get();
            let _ = PostMessageW(Some(hwnd), message, WPARAM(wparam), LPARAM(0));
            let _ = KillTimer(None, timer);
        }
        return;
    }
    let ticks = TICKS.get() + 1;
    TICKS.set(ticks);
    if ticks >= MAX_TICKS {
        // SAFETY: このスレッドのタイマーを止め、モーダルを抜けさせる。残る WM_QUIT は
        // 場合ごとの新しいスレッドが終わるときに一緒に消える。
        unsafe {
            let _ = KillTimer(None, timer);
            PostQuitMessage(0);
        }
    }
}

/// 新しいスレッドで `setup` の後に `show` で画面を 1 つ出して測り、1 行を出して返す。
/// `show` の戻り値は行の `box=`（メッセージボックスは押されたボタン・タスクダイアログは HRESULT）。
fn probe_case(case: &'static str, setup: fn(), show: fn() -> i32) -> ProbeRow {
    let row = std::thread::spawn(move || {
        setup();
        // SAFETY: 呼んだプロセスの疑似ハンドルで扱いを読むだけ。
        let process =
            awareness(unsafe { GetDpiAwarenessContextForProcess(GetCurrentProcess()) });
        let thread_before = thread_awareness();
        // SAFETY: 窓なしのタイマー。呼び戻しは上の `on_tick`。
        let timer = unsafe { SetTimer(None, 0, TICK_MS, Some(on_tick)) };
        assert_ne!(timer, 0, "測りのタイマーを立てられない（case={case}）");
        let result = show();
        let read = FOUND.get();
        let head = format!(
            "ALERT-DPI-PROBE case={case} process={process} thread_before={thread_before}"
        );
        let line = match read {
            Some(r) => format!(
                "{head} thread_during={} window={} window_v2={} window_dpi={} monitor_dpi={} system_dpi={} box={}",
                r.thread_during,
                r.window,
                r.window_v2,
                r.window_dpi,
                r.monitor_dpi,
                r.system_dpi,
                result
            ),
            None => format!("{head} found=false box={result}"),
        };
        ProbeRow { line, read }
    })
    .join()
    .expect("測りのスレッドが panic した");
    println!("{}", row.line);
    row
}

/// 呼んだスレッドを非対応にする。
fn thread_unaware() {
    super::set_thread_dpi_context(DPI_AWARENESS_CONTEXT_UNAWARE);
}

/// 呼んだスレッドをモニタごとの v2 にする。
fn thread_v2() {
    super::set_thread_dpi_context(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
}

/// 何も替えない（プロセスの既定を受ける）。
fn thread_inherit() {}

/// `MessageBoxW` を直に出す。
fn box_direct() -> i32 {
    let (title, body) = (HSTRING::from(PROBE_TITLE), HSTRING::from(PROBE_BODY));
    // SAFETY: `title`／`body` は終端 0 付きで呼び出しの間ずっと生存し、他の引数は None と定数。
    unsafe { MessageBoxW(None, &body, &title, MB_OK) }.0
}

/// 本番の道（[`super::show_box`]）で出す。
fn box_wrapped() -> i32 {
    super::show_box("alert", PROBE_TITLE, PROBE_BODY, MB_OK)
        .result
        .0
}

/// タスクダイアログ（OK だけ・エラーの絵）を直に出し、HRESULT を返す（失敗も行に載せる）。
fn task_dialog() -> i32 {
    CLOSE.set((TDM_CLICK_BUTTON.0 as u32, IDOK.0 as usize));
    let (title, body) = (HSTRING::from(PROBE_TITLE), HSTRING::from(PROBE_BODY));
    // SAFETY: `title`／`body` は終端 0 付きで呼び出しの間ずっと生存し、絵は OS の定数、
    // 主の指示は空、持ち主・インスタンス・押されたボタンの出力先は None。
    match unsafe {
        TaskDialog(
            None,
            None,
            &title,
            PCWSTR::null(),
            &body,
            TDCBF_OK_BUTTON,
            TD_ERROR_ICON,
            None,
        )
    } {
        Ok(()) => 0,
        Err(e) => e.code().0,
    }
}

/// タスクダイアログを出す間だけスレッドを v2 に囲む（本番の部品）。
fn task_dialog_wrapped() -> i32 {
    super::with_thread_per_monitor("alert", super::set_thread_dpi_context, task_dialog)
}

/// 関門 0 の初めの 3 つの場合。判定は `wrapped_from_unaware` の窓が v2 であること。
#[test]
#[ignore = "real dialog: shows a message box on the desktop"]
fn probe_alert_box_window_dpi() {
    let rows = [
        probe_case("unwrapped_unaware", thread_unaware, box_direct),
        probe_case("wrapped_from_unaware", thread_unaware, box_wrapped),
        probe_case("unwrapped_thread_v2", thread_v2, box_direct),
    ];
    for row in &rows {
        assert!(row.read.is_some(), "画面の窓が見つからない: {}", row.line);
    }
    let wrapped = rows[1].read.expect("上で確かめた");
    assert!(
        wrapped.thread_during == 2 && wrapped.window_v2,
        "囲んだ画面の窓が v2 でない（関門 0 を通らない）: {}",
        rows[1].line
    );
}

/// 関門 0 の 4 つ目の場合: プロセスを先に v2 に立て、スレッドは替えずに直に出す。
/// プロセスの扱いは戻せないので別の 1 本（別のプロセスで走らせる）。
#[test]
#[ignore = "real dialog: shows a message box on the desktop"]
fn probe_alert_box_window_dpi_process_v2() {
    wintf::ensure_process_dpi_awareness();
    let row = probe_case("process_v2_default", thread_inherit, box_direct);
    assert!(row.read.is_some(), "画面の窓が見つからない: {}", row.line);
}

/// 案 B の測り: タスクダイアログ（comctl32 v6）の窓の扱い。測るだけで v2 は判定しない（進行役が読む）。
#[test]
#[ignore = "real dialog: shows a task dialog on the desktop"]
fn probe_alert_task_dialog_window_dpi() {
    let rows = [
        probe_case("taskdialog_unaware", thread_unaware, task_dialog),
        probe_case(
            "taskdialog_wrapped_from_unaware",
            thread_unaware,
            task_dialog_wrapped,
        ),
        probe_case("taskdialog_thread_v2", thread_v2, task_dialog),
    ];
    for row in &rows {
        assert!(row.read.is_some(), "画面の窓が見つからない: {}", row.line);
    }
}
