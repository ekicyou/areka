//! 写しの相手: crates/wintf/src/test_crash_report.rs（許可の持ち手の表を持たない写し）
//!
//! テストのプロセスの落ちの報告（areka-test-binary-stability 設計「CrashReport」・要件 1.1・1.10）。
//!
//! プロセスの読み込みの初め（libtest の main より前）に `.CRT$XCU` の初期化子から、最初の受け手（AV）と
//! 最後の受け手（それ以外）を据える。AV は最初の受け手で書く（途中の枠の受け手がプロセスを終わらせたり、枠を
//! 巻き戻せずに最後の受け手まで届かなかったりする落ちがあった・2026-11-01 の測り）。それ以外の捕まえられなかった
//! 例外は最後の受け手で書く。書く中身は、例外のコード・番地・落ちたスレッドの名前とモジュール・GPU の装置の
//! 許可の持ち手・生きているスレッドの一覧・積み上げを、決めた行の形で標準エラーの OS のハンドルへ直に書き
//! （libtest の捕まえも `std` の標準エラーの錠も通らない）、次の受け手へ回す。ふだんの緑の走行では何も書かない。
//!
//! 同じ初期化子で、プロセスの寿命の MTA を立てる（`CoIncrementMTAUsage`・件 3 の直し）。テストのスレッドが
//! 足場ごとに入って出る一時的な MTA が解体され、放された DLL を後から呼んで落ちるのを防ぐ（2026-11-01 の対照の
//! 走行で候補 B と決めた）。そのため、COM を初期化していないテストのスレッドも暗黙の MTA に居る。
//!
//! 書く順は割り当ての無い行から先に。後ろの段（スレッドの名前の取り出し・積み上げ）で錠やヒープが壊れて
//! 止まっても、前の行は残る。

use std::fmt::{self, Write as _};
use std::fs::File;
use std::io::{self, Write};
use std::mem::ManuallyDrop;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, TryLockError};

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
struct MEMORY_BASIC_INFORMATION {
    BaseAddress: *mut core::ffi::c_void,
    AllocationBase: *mut core::ffi::c_void,
    AllocationProtect: u32,
    #[cfg(target_pointer_width = "64")]
    PartitionId: u16,
    RegionSize: usize,
    State: u32,
    Protect: u32,
    Type: u32,
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

// 手で宣言した並びが SDK の大きさと合うことを、ビルドの時に確かめる。
const _: () = assert!(
    size_of::<MEMORY_BASIC_INFORMATION>()
        == if cfg!(target_pointer_width = "64") {
            48
        } else {
            28
        }
);

type ExceptionFilter = unsafe extern "system" fn(*const EXCEPTION_POINTERS) -> i32;

const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
const EXCEPTION_ACCESS_VIOLATION: u32 = 0xC000_0005;
const TH32CS_SNAPTHREAD: u32 = 0x4;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetUnhandledExceptionFilter(filter: Option<ExceptionFilter>) -> Option<ExceptionFilter>;
    fn AddVectoredExceptionHandler(
        first: u32,
        handler: Option<ExceptionFilter>,
    ) -> *mut core::ffi::c_void;
    fn RaiseException(code: u32, flags: u32, count: u32, arguments: *const usize);
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> HANDLE;
    fn Thread32First(snapshot: HANDLE, entry: *mut THREADENTRY32) -> i32;
    fn Thread32Next(snapshot: HANDLE, entry: *mut THREADENTRY32) -> i32;
    fn VirtualQuery(
        address: *const core::ffi::c_void,
        buffer: *mut MEMORY_BASIC_INFORMATION,
        length: usize,
    ) -> usize;
}

// ---------------------------------------------------------------------------
// 据え付け
// ---------------------------------------------------------------------------

/// テストの実行ファイルの根のクレートの中に置くので、リンクで落ちない（較正で確かめる・`main.rs` は触らない）。
#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = install;

/// 据えた前の受け手（無ければ `None`）。
static PREVIOUS: OnceLock<Option<ExceptionFilter>> = OnceLock::new();

/// 据え付けが走り、最初の受け手まで据わった印（ふだんの走行で受け手が据わっていることを、落とさずに確かめるため）。
static INSTALLED: AtomicBool = AtomicBool::new(false);

/// 最初の受け手が書いている印（書く途中の AV で書き直さない）。
static WRITING: AtomicBool = AtomicBool::new(false);

/// プロセスの読み込みの初め（libtest の main より前）に 1 度だけ走る。
extern "C" fn install() {
    // 一時的な MTA の解体の後に放された DLL を呼ぶ落ち（件 3）を防ぐ: プロセスの寿命の MTA を立てる
    // （クッキーは返さない・プロセスの終わりまで持つ）。
    // SAFETY: 引数の無い COM の呼び出し。
    if let Err(e) = unsafe { CoIncrementMTAUsage() } {
        let _ = writeln!(raw_stderr(), "AREKA-MTA-KEEPER failed hr={:?}", e.code());
    }
    // SAFETY: 受け手は `report` と `first_chance` だけで、プロセスの寿命のあいだ生きている。
    let previous = unsafe { SetUnhandledExceptionFilter(Some(report)) };
    let _ = PREVIOUS.set(previous);
    // SAFETY: 同上（外さない）。
    let vectored = unsafe { AddVectoredExceptionHandler(0, Some(first_chance)) };
    INSTALLED.store(!vectored.is_null(), Ordering::Release);
}

/// 標準エラーの OS のハンドルを、閉じない `File` として包む。
fn raw_stderr() -> ManuallyDrop<File> {
    // SAFETY: ハンドルはプロセスの持ち物で、`ManuallyDrop` なので閉じない。
    ManuallyDrop::new(unsafe { File::from_raw_handle(io::stderr().as_raw_handle()) })
}

// ---------------------------------------------------------------------------
// GPU の装置の許可の持ち手の表（`GpuPermit` の取る・返す所から載せ降ろしする・割り当て無し）
// ---------------------------------------------------------------------------

/// 表の行の数。プロセスの数えの許可 4 つの他に、檻が別の数えで 2 つまで取るので、余らせて置く。
pub(crate) const HOLDER_ROWS: usize = 16;
const NAME_BYTES: usize = 160;

#[derive(Clone, Copy)]
struct Holder {
    tid: u32,
    len: usize,
    name: [u8; NAME_BYTES],
}

impl Holder {
    const EMPTY: Self = Self {
        tid: 0,
        len: 0,
        name: [0; NAME_BYTES],
    };
}

static HOLDERS: Mutex<[Holder; HOLDER_ROWS]> = Mutex::new([Holder::EMPTY; HOLDER_ROWS]);

/// 表に入りきらなかった許可の数（受け手が `missed=` で出す）。
static MISSED: AtomicUsize = AtomicUsize::new(0);

/// 表に入らなかった許可の行の番号。
pub(crate) const NO_ROW: usize = usize::MAX;

/// 今のスレッドを許可の持ち手として載せ、行の番号を返す（入りきらなければ [`NO_ROW`]）。
pub(crate) fn hold() -> usize {
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("");
    let mut len = name.len().min(NAME_BYTES);
    while !name.is_char_boundary(len) {
        len -= 1;
    }
    let mut rows = HOLDERS.lock().unwrap_or_else(|e| e.into_inner());
    let Some((row, slot)) = rows.iter_mut().enumerate().find(|(_, h)| h.tid == 0) else {
        MISSED.fetch_add(1, Ordering::Relaxed);
        return NO_ROW;
    };
    // SAFETY: 引数の無い呼び出し。
    slot.tid = unsafe { GetCurrentThreadId() };
    slot.len = len;
    slot.name[..len].copy_from_slice(&name.as_bytes()[..len]);
    row
}

/// 許可を返す所: 表から降ろす。
pub(crate) fn release(row: usize) {
    if let Some(slot) = HOLDERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(row)
    {
        *slot = Holder::EMPTY;
    }
}

// ---------------------------------------------------------------------------
// 受け手
// ---------------------------------------------------------------------------

/// 最初の受け手が書く例外か（AV だけ・ふだんの走行で捕まえられて続く例外は書かない）。
fn should_write_first_chance(code: u32) -> bool {
    code == EXCEPTION_ACCESS_VIOLATION
}

/// 例外のコードと番地（情報が null なら 0 と null）。
///
/// # Safety
/// `info` は OS が受け手へ渡した例外の情報か null。
unsafe fn code_and_address(info: *const EXCEPTION_POINTERS) -> (u32, *mut core::ffi::c_void) {
    // SAFETY: 呼び手の約束（null なら読まない）。
    let record = unsafe { info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) };
    record.map_or((0, core::ptr::null_mut()), |r| {
        (r.ExceptionCode, r.ExceptionAddress)
    })
}

/// 最初の受け手（AV だけ書く）。いつも `EXCEPTION_CONTINUE_SEARCH` を返す＝プロセスの振る舞いと終了コードは変えない。
unsafe extern "system" fn first_chance(info: *const EXCEPTION_POINTERS) -> i32 {
    // SAFETY: OS が渡す例外の情報。
    let (code, _) = unsafe { code_and_address(info) };
    if should_write_first_chance(code) && !WRITING.swap(true, Ordering::AcqRel) {
        // SAFETY: 同上。
        unsafe { write_report(info) };
        WRITING.store(false, Ordering::Release);
    }
    EXCEPTION_CONTINUE_SEARCH
}

/// 捕まえられなかった例外の最後の受け手（AV は最初の受け手が書いたので書かない）。次の受け手へ回す。
unsafe extern "system" fn report(info: *const EXCEPTION_POINTERS) -> i32 {
    // SAFETY: OS が渡す例外の情報。
    let (code, _) = unsafe { code_and_address(info) };
    if !should_write_first_chance(code) {
        // SAFETY: 同上。
        unsafe { write_report(info) };
    }
    match PREVIOUS.get().copied().flatten() {
        // SAFETY: OS が据えていた前の受け手へ、受けた情報をそのまま渡す。
        Some(previous) => unsafe { previous(info) },
        None => EXCEPTION_CONTINUE_SEARCH,
    }
}

/// 報告の本体を書く。行はどれも 1 行ずつ 1 度の書き込みで出す（libtest の出力が行の途中に入らない）。
///
/// # Safety
/// `info` は OS が受け手へ渡した例外の情報か null。
unsafe fn write_report(info: *const EXCEPTION_POINTERS) {
    let mut out = raw_stderr();
    let out: &mut File = &mut out;
    let mut line = Line::new();

    // ⑴ 1 行目。
    // SAFETY: 呼び手の約束。
    let (code, addr) = unsafe { code_and_address(info) };
    // SAFETY: 引数の無い呼び出し。
    let tid = unsafe { GetCurrentThreadId() };
    let _ = write!(
        line,
        "AREKA-CRASH code=0x{code:08X} addr={addr:p} tid={tid}"
    );
    line.emit(out);

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
    let _ = write!(line, "AREKA-CRASH module=");
    write_wide(&mut line, &path[..len.min(path.len())]);
    let _ = write!(line, "+0x{offset:X} thread=\"");
    // SAFETY: 疑似ハンドル（閉じなくてよい）。
    write_thread_name(&mut line, unsafe { GetCurrentThread() });
    let _ = write!(line, "\"");
    line.emit(out);
    if len == 0 {
        // モジュールに当たらない番地は、領域の様子を書く（外されたモジュールの跡かを読む）。
        write_region(&mut line, addr);
        line.emit(out);
    }

    // ⑶ 許可の持ち手の表（錠が取れなければ取らずに busy と書く）。
    let rows = match HOLDERS.try_lock() {
        Ok(rows) => Some(rows),
        Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    };
    match rows {
        Some(rows) => {
            for h in rows.iter().filter(|h| h.tid != 0) {
                let name = core::str::from_utf8(&h.name[..h.len]).unwrap_or("?");
                let _ = write!(line, "AREKA-CRASH-HOLDER tid={} name=\"{name}\"", h.tid);
                line.emit(out);
            }
        }
        None => {
            let _ = write!(line, "AREKA-CRASH-HOLDER busy");
            line.emit(out);
        }
    }
    let missed = MISSED.load(Ordering::Relaxed);
    if missed > 0 {
        let _ = write!(line, "AREKA-CRASH-HOLDER missed={missed}");
        line.emit(out);
    }

    // ⑷ 生きているスレッドの一覧。
    write_live_threads(out, &mut line);

    // ⑸ 積み上げ（ここから割り当てる）。
    let stack = std::backtrace::Backtrace::force_capture().to_string();
    for frame in stack.lines() {
        let _ = write!(line, "AREKA-CRASH-STACK {frame}");
        line.emit(out);
    }
}

/// 番地の領域の様子の 1 行（`VirtualQuery`）。
fn write_region(line: &mut Line, addr: *const core::ffi::c_void) {
    // SAFETY: 整数と生ポインタだけの構造体（0 で正しい）。
    let mut info: MEMORY_BASIC_INFORMATION = unsafe { core::mem::zeroed() };
    // SAFETY: 手元の構造体へ、番地の領域の様子を読むだけ。
    let n = unsafe { VirtualQuery(addr, &mut info, size_of::<MEMORY_BASIC_INFORMATION>()) };
    if n == 0 {
        let _ = write!(line, "AREKA-CRASH region query-failed");
        return;
    }
    let _ = write!(
        line,
        "AREKA-CRASH region base=0x{:X} alloc_base=0x{:X} size=0x{:X} state=0x{:X} protect=0x{:X} type=0x{:X}",
        info.BaseAddress as usize,
        info.AllocationBase as usize,
        info.RegionSize,
        info.State,
        info.Protect,
        info.Type,
    );
}

fn write_live_threads(out: &mut File, line: &mut Line) {
    // SAFETY: 引数の無い呼び出し。
    let pid = unsafe { GetCurrentProcessId() };
    // SAFETY: 全プロセスのスレッドの写し（自分のプロセスの分だけ読む）。
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot.is_invalid() {
        let _ = write!(line, "AREKA-CRASH-THREAD snapshot-failed");
        line.emit(out);
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
            let _ = write!(
                line,
                "AREKA-CRASH-THREAD tid={} name=\"",
                entry.th32ThreadID
            );
            // SAFETY: 名前を読むだけの権限で開き、読んだら閉じる。
            if let Ok(thread) =
                unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, entry.th32ThreadID) }
            {
                write_thread_name(line, thread);
                // SAFETY: 上で開いたハンドル。
                let _ = unsafe { CloseHandle(thread) };
            }
            let _ = write!(line, "\"");
            line.emit(out);
        }
        // SAFETY: 同上。
        more = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    // SAFETY: 上で開いた写し。
    let _ = unsafe { CloseHandle(snapshot) };
}

fn write_thread_name(line: &mut Line, thread: HANDLE) {
    // shortcut: 返った文字列を LocalFree しない（落ちていくプロセスの数十バイト）。
    // SAFETY: 名前を読む権限のあるハンドル。
    if let Ok(name) = unsafe { GetThreadDescription(thread) }
        && !name.is_null()
    {
        // SAFETY: OS が返した 0 終わりの文字列。
        write_wide(line, unsafe { name.as_wide() });
    }
}

/// UTF-16 を割り当てずに UTF-8 で書く（壊れた並びは `?`）。
fn write_wide(line: &mut Line, wide: &[u16]) {
    for c in char::decode_utf16(wide.iter().copied()) {
        let _ = line.write_char(c.unwrap_or('?'));
    }
}

// ---------------------------------------------------------------------------
// 1 行の溜め（割り当て無し）
// ---------------------------------------------------------------------------

/// 1 行の上限のバイト数（改行と切りの印を含む）。
const LINE_BYTES: usize = 1024;
/// 上限で切った行の終わりに付ける印。
const CUT_MARK: &str = " <cut>";

/// 1 行を手元の配列に溜め、1 度の書き込みで出す。上限を越えた分は字の境目で切って印を付ける。
struct Line {
    buf: [u8; LINE_BYTES],
    len: usize,
    cut: bool,
}

impl Line {
    const fn new() -> Self {
        Self {
            buf: [0; LINE_BYTES],
            len: 0,
            cut: false,
        }
    }

    /// 切りの印と改行を付けた 1 行。
    fn finish(&mut self) -> &[u8] {
        let mut end = self.len;
        if self.cut {
            self.buf[end..end + CUT_MARK.len()].copy_from_slice(CUT_MARK.as_bytes());
            end += CUT_MARK.len();
        }
        self.buf[end] = b'\n';
        &self.buf[..=end]
    }

    /// 1 度の書き込みで出し、空に戻す。
    fn emit(&mut self, out: &mut File) {
        let _ = out.write_all(self.finish());
        self.len = 0;
        self.cut = false;
    }
}

impl fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.cut {
            return Ok(());
        }
        let room = LINE_BYTES - CUT_MARK.len() - 1 - self.len;
        let mut n = s.len().min(room);
        while !s.is_char_boundary(n) {
            n -= 1;
        }
        self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        self.cut = n < s.len();
        Ok(())
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

/// 初期化子はプロセスの寿命の MTA を立てている（件 3 の直し）: COM を初期化していない新しいスレッドも、
/// 暗黙の MTA に居る。
#[test]
fn initializer_keeps_a_process_lifetime_mta() {
    use windows::Win32::System::Com::{
        APTTYPE, APTTYPE_MTA, APTTYPEQUALIFIER, APTTYPEQUALIFIER_IMPLICIT_MTA, CoGetApartmentType,
    };
    let got = std::thread::spawn(|| {
        let mut kind = APTTYPE::default();
        let mut qualifier = APTTYPEQUALIFIER::default();
        // SAFETY: 出力先は生きている局所変数。
        let r = unsafe { CoGetApartmentType(&mut kind, &mut qualifier) };
        (r.map_err(|e| e.code()), kind, qualifier)
    })
    .join()
    .expect("スレッドが落ちた");
    assert_eq!(
        got,
        (Ok(()), APTTYPE_MTA, APTTYPEQUALIFIER_IMPLICIT_MTA),
        "プロセスの寿命の MTA が立っていない"
    );
}

/// 最初の受け手が書くのは AV だけ（ふだんの走行で捕まえられて続く例外は書かない・要件 10.3）。
#[test]
fn first_chance_writes_only_access_violations() {
    assert!(should_write_first_chance(0xC000_0005));
    assert!(!should_write_first_chance(0x4001_0006), "OutputDebugString");
    assert!(!should_write_first_chance(0xE06D_7363), "C++ の例外");
}

/// 1 行の溜めは短い行をそのまま改行つきで出し、長い行は字の境目で切って印を付ける（上限を越えない）。
#[test]
fn line_cuts_an_overlong_line_at_a_char_boundary_with_a_mark() {
    let mut line = Line::new();
    let _ = write!(line, "AREKA-CRASH code=0x{:08X}", 0xC000_0005u32);
    assert_eq!(line.finish(), b"AREKA-CRASH code=0xC0000005\n");

    let mut line = Line::new();
    let _ = write!(line, "{}", "あ".repeat(LINE_BYTES));
    let _ = write!(line, "tail");
    let bytes = line.finish();
    assert!(bytes.len() <= LINE_BYTES);
    let text = core::str::from_utf8(bytes).expect("字の境目で切る");
    assert!(text.ends_with(" <cut>\n"), "{text}");
    assert!(!text.contains("tail"), "切った後の書き足しは捨てる");
}

/// 較正（要件 1.2）: 許可を取り `GraphicsCore` を作った後（GPU の DLL を読み込んだ後も受け手が生きていることを
/// 見るため）、名前の無いスレッドで例外を起こす。出力に落ちたスレッド（名前なし）・持ち手の行のこのテストの名前・
/// 生きているスレッドの一覧が出ることを `load-flake.ps1` の summary で見る（タスク 2.4）。
#[test]
#[ignore = "較正: わざとプロセスを落とす（要件 1.2）"]
fn crash_report_names_the_running_test() {
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let _gpu = crate::emo2_boot::spine::GpuPermit::take();
    let _core = wintf::ecs::GraphicsCore::new().expect("GraphicsCore::new 失敗");
    // `std::thread::spawn` は名前を付けない（`Builder::name` を使わない）。
    let _ = std::thread::spawn(|| {
        // SAFETY: わざと捕まえられない例外を起こす（較正）。
        unsafe { RaiseException(0xC000_0005, 0, 0, core::ptr::null()) };
    })
    .join();
    unreachable!("例外でプロセスが落ちるはず");
}
