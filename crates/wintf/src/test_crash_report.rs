//! 写しの相手: crates/areka/src/emo2_boot/crash_report_test_support.rs（こちらは許可の持ち手の表と許可の測りの口を持たない）
//!
//! テストのプロセスの落ちの報告（areka-test-binary-stability 設計「CrashReport」・要件 1.1・1.10）。
//!
//! プロセスの読み込みの初め（libtest の main より前）に `.CRT$XCU` の初期化子から最後の受け手を据える。
//! 捕まえられなかった例外が来たら、例外のコード・番地・落ちたスレッドの名前とモジュール・生きている
//! スレッドの一覧（wintf の lib は並ぶテストが少ないので許可の持ち手の表は持たず、一覧で読む）・積み上げを、
//! 決めた行の形で標準エラーの OS のハンドルへ直に書き（libtest の捕まえも `std` の標準エラーの錠も通らない）、次の受け手へ回す。ふだんの緑の走行では何も書かない。
//!
//! 書く順は割り当ての無い行から先に。後ろの段（スレッドの名前の取り出し・積み上げ）で錠やヒープが壊れて
//! 止まっても、前の行は残る。

use std::fs::File;
use std::io::{self, Write};
use std::mem::ManuallyDrop;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE};
use windows::Win32::System::Com::CoIncrementMTAUsage;
use windows::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameW, GetModuleHandleExW,
};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, GetCurrentThread, GetCurrentThreadId, GetThreadDescription, OpenThread,
    THREAD_QUERY_LIMITED_INFORMATION,
};
use windows::core::PCWSTR;

// ---------------------------------------------------------------------------
// 今の `windows` の機能（Win32_System_Diagnostics_* が無い）に無い kernel32 の関数と構造体（設計の議題 2・
// `Cargo.toml` は触らない）。
// ---------------------------------------------------------------------------

#[repr(C)]
#[allow(non_snake_case)]
struct EXCEPTION_RECORD {
    ExceptionCode: u32,
    ExceptionFlags: u32,
    ExceptionRecord: *mut EXCEPTION_RECORD,
    ExceptionAddress: *mut core::ffi::c_void,
    NumberParameters: u32,
    ExceptionInformation: [usize; 15],
}

#[repr(C)]
#[allow(non_snake_case)]
struct EXCEPTION_POINTERS {
    ExceptionRecord: *mut EXCEPTION_RECORD,
    ContextRecord: *mut core::ffi::c_void,
}

#[repr(C)]
#[allow(non_snake_case)]
struct THREADENTRY32 {
    dwSize: u32,
    cntUsage: u32,
    th32ThreadID: u32,
    th32OwnerProcessID: u32,
    tpBasePri: i32,
    tpDeltaPri: i32,
    dwFlags: u32,
}

type ExceptionFilter = unsafe extern "system" fn(*const EXCEPTION_POINTERS) -> i32;

const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
const TH32CS_SNAPTHREAD: u32 = 0x4;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(filter: Option<ExceptionFilter>) -> Option<ExceptionFilter>;
    fn RaiseException(code: u32, flags: u32, count: u32, arguments: *const usize);
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> HANDLE;
    fn Thread32First(snapshot: HANDLE, entry: *mut THREADENTRY32) -> i32;
    fn Thread32Next(snapshot: HANDLE, entry: *mut THREADENTRY32) -> i32;
}

// ---------------------------------------------------------------------------
// 据え付け
// ---------------------------------------------------------------------------

/// テストの実行ファイルの根のクレートの中に置くので、リンクで落ちない（較正で確かめる・`lib.rs` の `#[cfg(test)] mod` だけ）。
#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = install;

/// 据えた前の受け手（無ければ `None`）。
static PREVIOUS: OnceLock<Option<ExceptionFilter>> = OnceLock::new();

/// 据え付けが走った印（ふだんの走行で受け手が据わっていることを、落とさずに確かめるため）。
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// プロセスの読み込みの初め（libtest の main より前）に 1 度だけ走る。
extern "C" fn install() {
    // 測りの間だけ（X3・Y2）: プロセスの寿命の MTA を立てる口。5.1 の直しか 7.1 で外す（候補 B が原因なら
    // 口を外して無条件に呼ぶ形が直し）。クッキーは返さない（プロセスの終わりまで持つ）。
    if std::env::var_os("AREKA_TEST_MTA_KEEPER").is_some_and(|v| v == "1") {
        // SAFETY: 引数の無い COM の呼び出し。
        if let Err(e) = unsafe { CoIncrementMTAUsage() } {
            let _ = writeln!(raw_stderr(), "AREKA-MTA-KEEPER failed hr={:?}", e.code());
        }
    }
    // SAFETY: 受け手は `report` だけで、プロセスの寿命のあいだ生きている。
    let previous = unsafe { SetUnhandledExceptionFilter(Some(report)) };
    let _ = PREVIOUS.set(previous);
    INSTALLED.store(true, Ordering::Release);
}

/// 標準エラーの OS のハンドルを、閉じない `File` として包む。
fn raw_stderr() -> ManuallyDrop<File> {
    // SAFETY: ハンドルはプロセスの持ち物で、`ManuallyDrop` なので閉じない。
    ManuallyDrop::new(unsafe { File::from_raw_handle(io::stderr().as_raw_handle()) })
}

// ---------------------------------------------------------------------------
// 受け手
// ---------------------------------------------------------------------------

/// 捕まえられなかった例外の最後の受け手。書き終えたら次の受け手へ回す。
unsafe extern "system" fn report(info: *const EXCEPTION_POINTERS) -> i32 {
    let mut out = raw_stderr();
    let out: &mut File = &mut out;

    // ⑴ 割り当ての無い 1 行目。
    // SAFETY: OS が渡す例外の情報（null なら読まない）。
    let record = unsafe { info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) };
    let (code, addr) = record.map_or((0, core::ptr::null_mut()), |r| {
        (r.ExceptionCode, r.ExceptionAddress)
    });
    // SAFETY: 引数の無い呼び出し。
    let tid = unsafe { GetCurrentThreadId() };
    let _ = writeln!(out, "AREKA-CRASH code=0x{code:08X} addr={addr:p} tid={tid}");

    // ⑵ 落ちた番地のモジュールと、落ちたスレッドの名前。
    let mut path = [0u16; 520];
    let mut module = HMODULE::default();
    // SAFETY: FROM_ADDRESS の形では名前の引数を番地として読む。参照の数は変えない。
    let found = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(addr as *const u16),
            &mut module,
        )
    }
    .is_ok();
    let len = if found {
        // SAFETY: 上で得たモジュールと、手元の配列。
        unsafe { GetModuleFileNameW(Some(module), &mut path) as usize }
    } else {
        0
    };
    let offset = (addr as usize).wrapping_sub(module.0 as usize);
    let _ = write!(out, "AREKA-CRASH module=");
    write_wide(out, &path[..len.min(path.len())]);
    let _ = write!(out, "+0x{offset:X} thread=\"");
    // SAFETY: 疑似ハンドル（閉じなくてよい）。
    write_thread_name(out, unsafe { GetCurrentThread() });
    let _ = writeln!(out, "\"");

    // ⑶ 生きているスレッドの一覧。
    write_live_threads(out);

    // ⑷ 積み上げ（ここから割り当てる）。
    let stack = std::backtrace::Backtrace::force_capture().to_string();
    for line in stack.lines() {
        let _ = writeln!(out, "AREKA-CRASH-STACK {line}");
    }

    match PREVIOUS.get().copied().flatten() {
        // SAFETY: OS が据えていた前の受け手へ、受けた情報をそのまま渡す。
        Some(previous) => unsafe { previous(info) },
        None => EXCEPTION_CONTINUE_SEARCH,
    }
}

fn write_live_threads(out: &mut File) {
    // SAFETY: 引数の無い呼び出し。
    let pid = unsafe { GetCurrentProcessId() };
    // SAFETY: 全プロセスのスレッドの写し（自分のプロセスの分だけ読む）。
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot.is_invalid() {
        let _ = writeln!(out, "AREKA-CRASH-THREAD snapshot-failed");
        return;
    }
    let mut entry = THREADENTRY32 {
        dwSize: size_of::<THREADENTRY32>() as u32,
        cntUsage: 0,
        th32ThreadID: 0,
        th32OwnerProcessID: 0,
        tpBasePri: 0,
        tpDeltaPri: 0,
        dwFlags: 0,
    };
    // SAFETY: 上の写しと、dwSize を入れた手元の構造体。
    let mut more = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    while more {
        if entry.th32OwnerProcessID == pid {
            let _ = write!(out, "AREKA-CRASH-THREAD tid={} name=\"", entry.th32ThreadID);
            // SAFETY: 名前を読むだけの権限で開き、読んだら閉じる。
            if let Ok(thread) =
                unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, entry.th32ThreadID) }
            {
                write_thread_name(out, thread);
                // SAFETY: 上で開いたハンドル。
                let _ = unsafe { CloseHandle(thread) };
            }
            let _ = writeln!(out, "\"");
        }
        // SAFETY: 同上。
        more = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    // SAFETY: 上で開いた写し。
    let _ = unsafe { CloseHandle(snapshot) };
}

fn write_thread_name(out: &mut File, thread: HANDLE) {
    // shortcut: 返った文字列を LocalFree しない（落ちていくプロセスの数十バイト）。
    // SAFETY: 名前を読む権限のあるハンドル。
    if let Ok(name) = unsafe { GetThreadDescription(thread) }
        && !name.is_null()
    {
        // SAFETY: OS が返した 0 終わりの文字列。
        write_wide(out, unsafe { name.as_wide() });
    }
}

/// UTF-16 を割り当てずに UTF-8 で書く（壊れた並びは `?`）。
fn write_wide(out: &mut File, wide: &[u16]) {
    let mut buf = [0u8; 4];
    for c in char::decode_utf16(wide.iter().copied()) {
        let _ = out.write_all(c.unwrap_or('?').encode_utf8(&mut buf).as_bytes());
    }
}

// ---------------------------------------------------------------------------
// 檻と較正
// ---------------------------------------------------------------------------

/// ふだんの走行でも、受け手は libtest の main より前に据わっている（落とさずに確かめる）。
#[test]
fn crash_filter_is_installed_before_the_tests_start() {
    assert!(
        INSTALLED.load(Ordering::Acquire),
        "`.CRT$XCU` の初期化子が走っていない（受け手が据わらない）"
    );
}

/// 較正（要件 1.2）: 装置を作らずに、名前の無いスレッドで例外を起こす。出力に落ちたスレッド（名前なし）と
/// 生きているスレッドの一覧（走っているテストの名前のスレッド）が出ることを `load-flake.ps1` の summary で見る
/// （タスク 2.4）。
#[test]
#[ignore = "較正: わざとプロセスを落とす（要件 1.2）"]
fn crash_report_names_the_running_test() {
    // `std::thread::spawn` は名前を付けない（`Builder::name` を使わない）。
    let _ = std::thread::spawn(|| {
        // SAFETY: わざと捕まえられない例外を起こす（較正）。
        unsafe { RaiseException(0xC000_0005, 0, 0, core::ptr::null()) };
    })
    .join();
    unreachable!("例外でプロセスが落ちるはず");
}
