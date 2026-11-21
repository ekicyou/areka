//! 落ちの報告の本体の檻と、装置なしの較正。

use super::*;

#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = super::install;

/// ふだんの走行でも、受け手は libtest の main より前に据わっている（落とさずに確かめる）。
#[test]
fn crash_filter_is_installed_before_the_tests_start() {
    assert!(
        installed(),
        "`.CRT$XCU` の初期化子が走っていない（受け手が据わらない）"
    );
}

/// 初期化子はプロセスの寿命の MTA を立てている（一時的な MTA の解体の後に放された DLL を呼ぶ落ちの直し）:
/// COM を初期化していない新しいスレッドも、暗黙の MTA に居る。
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

/// 最初の受け手が書くのは AV だけ（ふだんの走行で捕まえられて続く例外は書かない）。
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

/// 較正: 名前の無いスレッドで例外を起こす。出力に落ちたスレッド（名前なし）と生きているスレッドの一覧が出ることを見る。
#[test]
#[ignore = "較正: わざとプロセスを落とす"]
fn crash_report_names_the_running_test() {
    super::crash_on_an_unnamed_thread();
}
