//! 窓書込指令（`command.rs`）のテストが共有する道具。
//!
//! 一括投入（`command_batch_tests.rs`）・合流（`command_coalesce_tests.rs`）・遷移観測
//! （`command_transition_tests.rs`）の 3 本で写しになっていた 7 項目をここに 1 つずつ置く。
//! 写しのままだと、偽ハンドルの潰れのような不具合を全部の写しで別々に直す必要がある。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

use super::super::transition_diag::{self, RECORD_PREFIX_TAG, TRANSITION_TARGET};
use super::drain_window_pos_commands;

/// 実機サインオフが用いる directive のうち、観測チャネルを点灯させるもの。
pub(super) const SIGNOFF_DIRECTIVES: &str = "info,wintf::transition=debug";

/// 捕捉が生きている証拠として同じ窓の中で必ず出す対照行。
pub(super) const CONTROL_LINE: &str = "[transition-probe] alive";

/// 対照行を出す（`info` 水準ゆえどの directive でも通る）。
pub(super) fn emit_control() {
    tracing::info!(target: TRANSITION_TARGET, "{CONTROL_LINE}");
}

/// テスト冒頭の初期化——キューの残りと刻印の写しを落とす（要件 7.7）。
pub(super) fn clean_slate() {
    let _residue = drain_window_pos_commands();
    transition_diag::reset_for_test();
}

/// 実在し得ない窓ハンドルを作る。
///
/// 64bit Windows の `HWND` は 32bit 値の**符号拡張**であり、`0xFFFF_FF01` のように上位
/// 32bit が 0 のまま下位が負域にある値はハンドル表に存在し得ない。それでも他プロセスの
/// 実窓を掴んでいないことを `IsWindow` で毎回確かめる。
pub(super) fn fake_hwnd(tag: u8) -> HWND {
    // 下位に識別子を載せ、必ず奇数にする（ハンドル値の粒度に載らない）。
    //
    // 識別子は**左へ 1 桁ずらしてから**奇数化する。`usize::from(tag) | 1` だと 0x20 と
    // 0x21 が同じ値へ潰れ、「別の窓」を意図した呼び分けが同一窓を指してしまう
    // （合流の導入で `drain_returns_the_enqueued_commands_verbatim_without_executing_them`
    //  が偽の失敗を出して露見した・task 5.3）。
    let value = 0xFFFF_FE00_usize | (usize::from(tag) << 1) | 1;
    let hwnd = HWND(value as *mut core::ffi::c_void);
    // SAFETY: `IsWindow` は任意のハンドル値に対して安全に真偽を返す読み取り専用 API で
    // あり、無効値でも未定義動作を起こさない。
    assert!(
        !unsafe { IsWindow(Some(hwnd)) }.as_bool(),
        "偽ハンドル 0x{value:X} が実窓を掴んでいる——このまま書込を撃つと無関係の窓を動かす"
    );
    hwnd
}

/// 捕捉出力から観測チャネルの行だけを取り出す。
pub(super) fn transition_lines(captured: &str) -> Vec<&str> {
    captured
        .lines()
        .filter(|line| line.contains(RECORD_PREFIX_TAG))
        .collect()
}

/// 種別語 `kind=<kind>` を持つ行だけを取り出す。
pub(super) fn lines_of_kind<'a>(captured: &'a str, kind: &str) -> Vec<&'a str> {
    let needle = format!("kind={kind} ");
    transition_lines(captured)
        .into_iter()
        .filter(|line| line.contains(&needle))
        .collect()
}
