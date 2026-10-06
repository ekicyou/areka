//! `os` の兄弟テスト。
//!
//! 上の 2 つは窓を作らない純粋な部分。下は明示したときだけ走る（`#[ignore]`）実機の最初の試し
//! S2〜S7（design.md「最初の試し」）。ツールチップを実際に出して確かめる。
//!
//! 走らせ方: `cargo test -p wintf --lib tooltip::os -- --ignored --test-threads=1 --nocapture`
//!
//! この試しの限界: テストの exe にはマニフェストが無いので、標準の部品は古い版（版 5）で動く。
//! 見た目・大きさ・ふち・折り返しの位置は 6.1 で新しい版の下で確かめ直す。S7 は wintf の窓も
//! World も無いので、別の窓への素の最前面の当て直しで近似する。S4 はメモ帳を 1 つ起こし、
//! 本物のマウスを短く動かして押す（終わったら位置を戻し、起こしたメモ帳の窓だけを閉じる）。

use super::*;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Instant;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetObjectW, GetPixel, GetTextExtentPoint32W, HDC, HMONITOR, LOGFONTW,
};
use windows::Win32::UI::Controls::TTM_GETTEXTW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GA_ROOT, GUITHREADINFO, GW_HWNDPREV, GWL_EXSTYLE, GetAncestor,
    GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetLayeredWindowAttributes, GetWindow,
    GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, IsWindow, MSG, PM_REMOVE,
    PeekMessageW, PostMessageW, SW_SHOWNOACTIVATE, SWP_SHOWWINDOW, SetCursorPos, ShowWindow,
    TranslateMessage, WM_CLOSE, WS_BORDER, WindowFromPoint,
};
use windows::core::{BOOL, w};

#[test]
fn utf16_fit_counts_whole_chars() {
    assert_eq!(chars_within_utf16("abc", 2), 2);
    assert_eq!(chars_within_utf16("aあb", 2), 2);
    assert_eq!(chars_within_utf16("abc", 0), 0);
    assert_eq!(chars_within_utf16("abc", 10), 3);
}

#[test]
fn utf16_fit_does_not_split_surrogate_pair() {
    // "a😀b" は UTF-16 で a・上位・下位・b の 4 つ。
    let s = "a😀b";
    assert_eq!(chars_within_utf16(s, 1), 1);
    assert_eq!(chars_within_utf16(s, 2), 1, "対の途中では切らない");
    assert_eq!(chars_within_utf16(s, 3), 2);
    assert_eq!(chars_within_utf16(s, 4), 3);
}

// ---- 実機の最初の試し（ここから下は `#[ignore]`） ----

/// 本番（wintf の実行時）と同じく、画面ごとの DPI を受ける。2 回目以降は失敗するが害は無い。
fn per_monitor_aware() {
    // SAFETY: Win32 境界。プロセスの DPI の受け方を決めるだけ。
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// cond が真になるか timeout まで、自分のスレッドのメッセージを回しながら待つ。真になれば true。
///
/// ツールチップの窓はこのスレッドのものなので、回さないと描画もマウスの受け取りも進まない。
fn pump_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let end = Instant::now() + timeout;
    loop {
        let mut msg = MSG::default();
        // SAFETY: Win32 境界。自分のスレッドの待ち行列から取り出して配るだけ。
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if cond() {
            return true;
        }
        if Instant::now() >= end {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn pump_for(d: Duration) {
    pump_until(d, || false);
}

fn size_of_rect(r: RectPx) -> SizePx {
    SizePx {
        width: r.right - r.left,
        height: r.bottom - r.top,
    }
}

fn center(r: RectPx) -> PointPx {
    PointPx {
        x: r.left + (r.right - r.left) / 2,
        y: r.top + (r.bottom - r.top) / 2,
    }
}

fn hwnd_rect(hwnd: HWND) -> RectPx {
    window_rect(hwnd).expect("窓の矩形")
}

/// 主画面の作業領域と DPI。
fn primary() -> (RectPx, u32) {
    monitor_at(PointPx { x: 0, y: 0 }).expect("主画面の情報")
}

/// 全部の画面の矩形（物理ピクセル）。
fn monitor_rects() -> Vec<RECT> {
    unsafe extern "system" fn visit(_: HMONITOR, _: HDC, rc: *mut RECT, data: LPARAM) -> BOOL {
        // SAFETY: data は下で渡した手元の Vec、rc は OS が渡す画面の矩形。
        unsafe { (*(data.0 as *mut Vec<RECT>)).push(*rc) };
        BOOL(1)
    }
    let mut rects: Vec<RECT> = Vec::new();
    // SAFETY: Win32 境界。手元の Vec へ画面の矩形を集めさせる（呼び出しの中で同期に回る）。
    let _ =
        unsafe { EnumDisplayMonitors(None, None, Some(visit), LPARAM((&raw mut rects) as isize)) };
    rects
}

/// show の中と同じ入力で、置き場所（左上）を計算し直す。
fn expected_top_left(anchor: PointPx, tip: SizePx) -> PointPx {
    let (work_area, dpi) = monitor_at(anchor).expect("画面の情報");
    // SAFETY: Win32 境界。引数は番号と DPI だけ。
    let offset_below = unsafe { GetSystemMetricsForDpi(SM_CYCURSOR, dpi) };
    place(&PlaceInput {
        anchor,
        tip,
        work_area,
        offset_above: scale(20, dpi),
        offset_below,
    })
}

/// 手前の窓と、その窓のスレッドの入力先。
fn foreground_and_focus() -> (HWND, HWND) {
    // SAFETY: Win32 境界。読むだけ。書き込み先は大きさを書いた手元の GUITHREADINFO。
    unsafe {
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, None);
        let mut gti = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        let _ = GetGUIThreadInfo(tid, &mut gti);
        (fg, gti.hwndFocus)
    }
}

/// 前提: 利用者の画面が入力を受けている（スクリーンセーバーや錠の画面の間は、手前の窓が無く、
/// 送ったマウスの入力も届かないので、S3・S4 は確かめられない）。
fn assert_desktop_in_use(foreground: HWND) {
    assert!(
        !foreground.is_invalid(),
        "前提: 手前の窓がある（スクリーンセーバーや錠の画面の間は確かめられない。画面を使える状態で走らせ直す）"
    );
}

/// upper が lower より手前（重なりの順で上）にあるか。lower から手前へたどって探す。
fn is_above(upper: HWND, lower: HWND) -> bool {
    let mut h = lower;
    // SAFETY: Win32 境界。重なりの順をたどって読むだけ。
    while let Ok(prev) = unsafe { GetWindow(h, GW_HWNDPREV) } {
        if prev == upper {
            return true;
        }
        h = prev;
    }
    false
}

/// 印の窓（標準の STATIC）を作り、入力先を取らずに見せる。押せるよう SS_NOTIFY を付ける。
fn marker_window(r: RectPx, ex_noactivate: bool) -> HWND {
    const SS_NOTIFY: u32 = 0x0100;
    let mut ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
    if ex_noactivate {
        ex |= WS_EX_NOACTIVATE;
    }
    // SAFETY: Win32 境界。標準の種類の窓を作り、入力先を取らずに見せる。
    unsafe {
        let hwnd = CreateWindowExW(
            ex,
            w!("STATIC"),
            w!("tooltip os_tests"),
            WINDOW_STYLE(WS_POPUP.0 | WS_BORDER.0 | SS_NOTIFY),
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            None,
            None,
            None,
            None,
        )
        .expect("印の窓");
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        hwnd
    }
}

/// 本物のマウスを pt で左ボタンを押して離す（位置は SetCursorPos で動かす）。
fn click_at(pt: PointPx) {
    let button = |flags: MOUSE_EVENT_FLAGS| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    // SAFETY: Win32 境界。カーソルを動かし、押す・離すの 2 つの入力を送るだけ。
    unsafe {
        SetCursorPos(pt.x, pt.y).expect("カーソルを動かす");
        let sent = SendInput(
            &[button(MOUSEEVENTF_LEFTDOWN), button(MOUSEEVENTF_LEFTUP)],
            size_of::<INPUT>() as i32,
        );
        assert_eq!(sent, 2, "押す・離すの入力が送れた");
    }
}

/// 走り終えないときにプロセスを止める見張り（ハングしたまま本物のマウスを握り続けないため）。
struct Watchdog(Arc<AtomicBool>);

impl Watchdog {
    fn start(name: &'static str, limit: Duration) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let seen = done.clone();
        std::thread::spawn(move || {
            let end = Instant::now() + limit;
            while Instant::now() < end {
                if seen.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            eprintln!("[os_tests] {name} が {limit:?} で終わらないので止める");
            std::process::abort();
        });
        Self(done)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s2_given_position_is_top_left_and_size_is_known_before_placing() {
    per_monitor_aware();
    let mut tip = TipWindow::default();
    let (work, dpi) = primary();
    let anchor = center(work);
    // 消えている所から（画面の外で追跡を始めて測る経路）・出ている所から（その場で測る経路）・
    // 消した後にもう一度（また画面の外から）の 3 回。
    let cases = [
        (
            "最初の試し S2
ふたつめの行",
            anchor,
            false,
        ),
        ("出ているまま文字を長くする（その場で測る）", anchor, true),
        (
            "消した後に別の所へ",
            PointPx {
                x: work.left + 300,
                y: work.bottom - 300,
            },
            false,
        ),
    ];
    for (i, &(text, anchor, _)) in cases.iter().enumerate() {
        let rect = tip.show(text, anchor).expect("出す");
        pump_for(Duration::from_millis(150));
        let hwnd = tip.hwnd.expect("窓");
        assert!(is_visible(hwnd), "出ている");
        assert_eq!(hwnd_rect(hwnd), rect, "返した矩形は出ている窓の矩形");
        // 出た窓の本当の大きさで置き場所を計算し直す。show の中で測った大きさが本当の大きさと
        // 1 でも違えば（版 6 の TTM_GETBUBBLESIZE は 1 大きい）、真上に出す回の上の位置がずれる。
        let want = expected_top_left(anchor, size_of_rect(rect));
        // 消えている所から同じ文字を測り直す（画面の外の経路）。出ていた大きさと同じになる。
        tip.hide();
        let parked = set_text_and_measure(hwnd, &normalize_newlines(text)).expect("測り直す");
        println!(
            "[S2] dpi={dpi} anchor={anchor:?} rect={rect:?} want={want:?} 画面の外で測った={parked:?}"
        );
        assert_eq!(
            (rect.left, rect.top),
            (want.x, want.y),
            "測った大きさで置いた（渡した位置が左上）"
        );
        assert_eq!(parked, size_of_rect(rect), "画面の外で測った大きさで出た");
        if cases
            .get(i + 1)
            .is_some_and(|&(_, _, keep_shown)| keep_shown)
        {
            // 次の回は出ているまま入れ替える経路なので、出し直しておく。
            assert_eq!(
                tip.show(text, anchor).expect("出し直す"),
                rect,
                "同じ所に出る"
            );
        }
    }
    tip.hide();
    let hwnd = tip.hwnd.expect("窓");
    assert!(!is_visible(hwnd), "消える");
    // 出ていないときに測ると、本当に画面の外（PARKED）で追跡を始めている。
    set_text_and_measure(hwnd, "画面の外で測る").expect("測る");
    let parked = hwnd_rect(hwnd);
    println!("[S2] 出ていないときに測った位置={parked:?}");
    assert_eq!(
        (parked.left, parked.top),
        (PARKED.x, PARKED.y),
        "画面の外に置いて測った"
    );
    tip.hide();
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn negative_coordinate_screen_keeps_given_top_left() {
    per_monitor_aware();
    let Some(m) = monitor_rects()
        .into_iter()
        .find(|r| r.left < 0 || r.top < 0)
    else {
        println!("[負の座標] 主画面の左・上に画面が無いので確かめられない（not available）");
        return;
    };
    let mut tip = TipWindow::default();
    // 画面の左上の近く（座標の絶対値が大きい所）に出す。
    let anchor = PointPx {
        x: m.left + 200,
        y: m.top + 200,
    };
    let rect = tip.show("負の座標の画面", anchor).expect("出す");
    pump_for(Duration::from_millis(200));
    let want = expected_top_left(anchor, size_of_rect(rect));
    let (work, _) = monitor_at(anchor).expect("画面の情報");
    println!("[負の座標] monitor={m:?} anchor={anchor:?} rect={rect:?} want={want:?}");
    assert_eq!(
        (rect.left, rect.top),
        (want.x, want.y),
        "負の座標でも渡した位置が左上"
    );
    assert!(
        rect.left >= work.left && rect.right <= work.right,
        "その画面の作業領域の中"
    );
    tip.hide();
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s3_showing_keeps_foreground_and_focus() {
    per_monitor_aware();
    let mut tip = TipWindow::default();
    let (work, _) = primary();
    let before = foreground_and_focus();
    assert_desktop_in_use(before.0);
    let rect = tip.show("最初の試し S3", center(work)).expect("出す");
    pump_for(Duration::from_millis(300));
    let after_first = foreground_and_focus();
    // 出ている間にもう一度出す（文字の入れ替えと最前面の当て直しの経路）。
    tip.show("最初の試し S3（出し直し）", center(work))
        .expect("出し直す");
    pump_for(Duration::from_millis(300));
    let after_second = foreground_and_focus();
    let hwnd = tip.hwnd.expect("窓");
    println!(
        "[S3] tip={hwnd:?} rect={rect:?} before={before:?} after={after_first:?} again={after_second:?}"
    );
    assert_eq!(after_first, before, "出しても手前の窓と入力先が変わらない");
    assert_eq!(after_second, before, "出し直しても変わらない");
    assert_ne!(before.0, hwnd, "ツールチップが手前の窓になっていない");
    tip.hide();
}

/// S4 の下に置くメモ帳。起こしたもの（新しく現れた窓）だけを持ち、Drop で閉じる。
struct Notepad {
    child: Child,
    hwnd: HWND,
    file: PathBuf,
}

impl Notepad {
    /// 中身の空のファイルを `target\` の下に作って開く。新しく現れた窓だけを自分のものにする。
    fn open() -> Self {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/tooltip_os_tests");
        std::fs::create_dir_all(&dir).expect("target の下の作業場所");
        let stem = format!("tooltip-s4-{}", std::process::id());
        let file = dir.join(format!("{stem}.txt"));
        std::fs::write(&file, "").expect("空のファイル");
        let existing = top_windows();
        let child = Command::new("notepad.exe")
            .arg(&file)
            .spawn()
            .expect("メモ帳を起こす");
        let mut found = None;
        let appeared = pump_until(Duration::from_secs(10), || {
            found = top_windows()
                .into_iter()
                .find(|(h, class, title)| {
                    !existing.iter().any(|(e, ..)| e == h)
                        && class == "Notepad"
                        && title.contains(&stem)
                })
                .map(|(h, ..)| h);
            found.is_some()
        });
        let Some(hwnd) = found.filter(|_| appeared) else {
            let mut child = child;
            // 起こした子だけを止める（Store 版は既に抜けている）。
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&file);
            panic!(
                "メモ帳が新しい窓で開かなかった（既存の窓のタブに入った可能性。既存の窓には触れない）"
            );
        };
        Self { child, hwnd, file }
    }
}

impl Drop for Notepad {
    fn drop(&mut self) {
        // SAFETY: Win32 境界。自分が起こして新しく現れた窓にだけ、閉じる知らせを送る。
        let _ = unsafe { PostMessageW(Some(self.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        // SAFETY: Win32 境界。窓がまだあるかを読むだけ。
        let closed = pump_until(Duration::from_secs(5), || unsafe {
            !IsWindow(Some(self.hwnd)).as_bool()
        });
        if !closed {
            eprintln!("[S4] メモ帳の窓が閉じない（確認の問いが出ている可能性。手で閉じる）");
        }
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.file);
    }
}

/// 見えている最上位の窓の (HWND, 種類の名前, 題名)。
fn top_windows() -> Vec<(HWND, String, String)> {
    unsafe extern "system" fn visit(h: HWND, data: LPARAM) -> BOOL {
        // SAFETY: data は下で渡した手元の Vec。
        unsafe { (*(data.0 as *mut Vec<HWND>)).push(h) };
        BOOL(1)
    }
    let mut all: Vec<HWND> = Vec::new();
    // SAFETY: Win32 境界。手元の Vec へ最上位の窓を集めさせる（呼び出しの中で同期に回る）。
    let _ = unsafe { EnumWindows(Some(visit), LPARAM((&raw mut all) as isize)) };
    all.into_iter()
        .filter(|h| is_visible(*h))
        .map(|h| {
            let (mut class, mut title) = ([0u16; 64], [0u16; 260]);
            // SAFETY: Win32 境界。手元の配列へ名前と題名を書かせる。
            let (c, t) = unsafe { (GetClassNameW(h, &mut class), GetWindowTextW(h, &mut title)) };
            (
                h,
                String::from_utf16_lossy(&class[..c.max(0) as usize]),
                String::from_utf16_lossy(&title[..t.max(0) as usize]),
            )
        })
        .collect()
}

/// 画面に映っている r の内側の色を、縦 4 × 横 8 の格子の点で読む（ふちを避けて内側だけ）。
fn screen_colours(r: RectPx) -> Vec<u32> {
    let mut out = Vec::new();
    // SAFETY: Win32 境界。画面の DC を借りて点の色を読み、返す。
    unsafe {
        let hdc = GetDC(None);
        for row in 0..4 {
            for col in 0..8 {
                let x = r.left + 3 + (r.right - r.left - 6) * col / 7;
                let y = r.top + 3 + (r.bottom - r.top - 6) * row / 3;
                out.push(GetPixel(hdc, x, y).0);
            }
        }
        ReleaseDC(None, hdc);
    }
    out
}

fn root_at(pt: PointPx) -> HWND {
    // SAFETY: Win32 境界。点の下の窓と、その最上位の窓を読むだけ。
    unsafe { GetAncestor(WindowFromPoint(POINT { x: pt.x, y: pt.y }), GA_ROOT) }
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s4_press_on_tooltip_reaches_notepad_beneath_and_hover_does_not_recurse() {
    per_monitor_aware();
    let _watch = Watchdog::start("S4", Duration::from_secs(60));
    let mut home = POINT::default();
    // SAFETY: Win32 境界。終わったら戻すカーソルの位置を読む。
    let _ = unsafe { GetCursorPos(&mut home) };
    let (work, _) = primary();
    // SAFETY: Win32 境界。読むだけ。
    assert_desktop_in_use(unsafe { GetForegroundWindow() });
    let notepad = Notepad::open();
    let mut pid = 0u32;
    // SAFETY: Win32 境界。読むだけ。
    unsafe { GetWindowThreadProcessId(notepad.hwnd, Some(&mut pid)) };
    // メモ帳を主画面の左上へ置き、重なりの先頭へ（入力先は変えない）。
    // SAFETY: Win32 境界。新しく現れたメモ帳の窓を動かすだけ。
    unsafe {
        SetWindowPos(
            notepad.hwnd,
            Some(windows::Win32::UI::WindowsAndMessaging::HWND_TOP),
            work.left + 40,
            work.top + 40,
            ((work.right - work.left) / 2).max(600),
            ((work.bottom - work.top) * 2 / 3).max(400),
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    }
    .expect("メモ帳を動かす");
    pump_for(Duration::from_millis(500));
    let pad = hwnd_rect(notepad.hwnd);

    // 前提: 押す前の手前の窓をメモ帳以外にする（右の印の窓を押して手前にする）。
    let marker = marker_window(
        RectPx {
            left: work.right - 260,
            top: work.top + 60,
            right: work.right - 60,
            bottom: work.top + 180,
        },
        false,
    );
    let marker_rect = hwnd_rect(marker);
    assert!(
        marker_rect.left >= pad.right,
        "印の窓とメモ帳が重ならない（主画面が狭すぎる）: pad={pad:?} marker={marker_rect:?}"
    );
    pump_for(Duration::from_millis(200));
    click_at(center(marker_rect));
    // SAFETY: Win32 境界。読むだけ。
    pump_until(Duration::from_secs(2), || unsafe {
        GetForegroundWindow() == marker
    });
    // SAFETY: 同上。
    let fg_before = unsafe { GetForegroundWindow() };
    assert_ne!(
        fg_before, notepad.hwnd,
        "前提: 押す前はメモ帳が手前の窓ではない"
    );

    // メモ帳の真ん中より少し下を anchor にし、ツールチップをメモ帳の文字の欄の上に出す。
    let anchor = PointPx {
        x: pad.left + (pad.right - pad.left) / 2,
        y: pad.top + (pad.bottom - pad.top) * 2 / 3,
    };
    let text = "最初の試し S4（この上を押すと下のメモ帳へ届く）";
    let mut tip = TipWindow::default();
    let rect = tip.show(text, anchor).expect("出す");
    let hwnd = tip.hwnd.expect("窓");
    let press = center(rect);
    // 前提: ツールチップを除けば、押す点の下はメモ帳。
    tip.hide();
    pump_for(Duration::from_millis(100));
    assert_eq!(root_at(press), notepad.hwnd, "前提: 押す点の下はメモ帳");
    let hidden = screen_colours(rect);
    let rect_again = tip.show(text, anchor).expect("出し直す");
    assert_eq!(rect_again, rect);
    pump_for(Duration::from_millis(200));
    // 別のプロセスへの素通しは層の窓でだけ効く。出すたびに外されていないか（フェードが外す）。
    // SAFETY: Win32 境界。自分の窓の拡張の形を読むだけ。
    let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    assert!(
        ex & WS_EX_LAYERED.0 != 0,
        "出した後も層の窓のまま: ex={ex:#x}"
    );
    // 層の窓は不透明度を決めないと描かれない（見えないまま素通しだけする）。不透明と、本当に映ったかを見る。
    let (mut alpha, mut flags) = (0u8, Default::default());
    // SAFETY: Win32 境界。自分の窓の層の設定を手元の変数へ読むだけ。
    unsafe { GetLayeredWindowAttributes(hwnd, None, Some(&mut alpha), Some(&mut flags)) }
        .expect("層の設定を読む");
    assert!(
        alpha == 255 && flags.contains(LWA_ALPHA),
        "不透明の層の窓: alpha={alpha} flags={flags:?}"
    );
    let shown = screen_colours(rect);
    let distinct = |v: &[u32]| v.iter().collect::<std::collections::HashSet<_>>().len();
    println!(
        "[S4] 色の数 隠した時={} 出した時={}",
        distinct(&hidden),
        distinct(&shown)
    );
    assert!(
        shown.iter().any(|c| !hidden.contains(c)),
        "ツールチップが画面に映った（隠した時に無い色がある）: hidden={hidden:x?} shown={shown:x?}"
    );
    let from_point = root_at(press);

    // ツールチップの上でマウスを何度か動かす（道具の窓＝自分への再帰が無いことを見る）。
    for i in 0..8 {
        let p = PointPx {
            x: rect.left + 2 + (rect.right - rect.left - 4) * i / 7,
            y: press.y,
        };
        // SAFETY: Win32 境界。カーソルを動かすだけ。
        unsafe { SetCursorPos(p.x, p.y) }.expect("カーソルを動かす");
        pump_for(Duration::from_millis(40));
    }
    assert!(is_visible(hwnd), "上でマウスを動かしても出たまま");

    click_at(press);
    // SAFETY: Win32 境界。読むだけ。
    let reached = pump_until(Duration::from_secs(3), || unsafe {
        GetForegroundWindow() == notepad.hwnd
    });
    // SAFETY: 同上。
    let fg_after = unsafe { GetForegroundWindow() };
    println!(
        "[S4] notepad={:?} pid={pid} pad={pad:?} tip={hwnd:?} rect={rect:?} press={press:?} \
         WindowFromPoint の最上位={from_point:?} before={fg_before:?} after={fg_after:?} reached={reached}",
        notepad.hwnd
    );
    tip.hide();
    // SAFETY: Win32 境界。自分で作った印の窓を壊し、カーソルを元の位置へ戻す。
    unsafe {
        let _ = DestroyWindow(marker);
        let _ = SetCursorPos(home.x, home.y);
    }
    drop(notepad);
    assert!(
        reached,
        "ツールチップの上の押下が下のメモ帳へ届き、メモ帳が手前の窓になる"
    );
}

/// hwnd の DC で、選んである字体のまま text の高さを測る。
fn text_height(hwnd: HWND, font: HFONT, text: &str) -> i32 {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    // SAFETY: Win32 境界。自分の窓の DC を借り、字体を選んで測り、元に戻して返す。
    unsafe {
        let hdc = GetDC(Some(hwnd));
        let old = SelectObject(hdc, font.into());
        let _ = GetTextExtentPoint32W(hdc, &wide, &mut size);
        SelectObject(hdc, old);
        ReleaseDC(Some(hwnd), hdc);
    }
    size.cy
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s5_long_japanese_line_and_url_wrap_within_max_width() {
    per_monitor_aware();
    let mut tip = TipWindow::default();
    let (work, dpi) = primary();
    let anchor = center(work);
    let one_line = size_of_rect(tip.show("あ", anchor).expect("1 行")).height;
    let hwnd = tip.hwnd.expect("窓");
    let max_width = max_tip_width(dpi, work);
    let limit = window_width_for(hwnd, max_width);
    let japanese =
        "ツールチップの折り返しを確かめるための、句読点の少ない長い日本語の一行です".repeat(4);
    let url = format!("https://example.com/{}", "abcdefghij0123456789".repeat(15));
    // 比べのための英語（語の切れ目がある）。OS だけで収まり、語の途中で割られないことを見る。
    let english = "The tooltip wraps this English sentence at spaces between words. ".repeat(4);
    for (name, text) in [
        ("日本語", japanese.as_str()),
        ("URL", url.as_str()),
        ("英語", english.as_str()),
    ] {
        let rect = tip.show(text, anchor).expect("出す");
        pump_for(Duration::from_millis(150));
        let size = size_of_rect(rect);
        tip.hide();
        // OS の折り返しだけで収まったか（force_break が要ったか）を記録に残す。
        let raw = set_text_and_measure(hwnd, &normalize_newlines(text)).expect("生の大きさ");
        println!(
            "[S5] {name}: dpi={dpi} max_width={max_width} limit={limit} rect={size:?} 1行={one_line} \
             OS だけの幅={} force_break={}",
            raw.width,
            raw.width > limit
        );
        assert!(size.width <= limit, "{name}: 最大の幅（＋ふち）に収まる");
        assert!(size.height > one_line, "{name}: 複数の行に折り返した");
    }
}

/// hwnd の DC で、font のまま text の幅を測る。
fn text_width(hwnd: HWND, font: HFONT, text: &str) -> i32 {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();
    // SAFETY: Win32 境界。自分の窓の DC を借り、字体を選んで測り、元に戻して返す。
    unsafe {
        let hdc = GetDC(Some(hwnd));
        let old = SelectObject(hdc, font.into());
        let _ = GetTextExtentPoint32W(hdc, &wide, &mut size);
        SelectObject(hdc, old);
        ReleaseDC(Some(hwnd), hdc);
    }
    size.cx
}

/// 英字だけで、幅がちょうど target の 1 語を作る（字ごとの幅の組み合わせを数え上げる）。
fn word_exactly(hwnd: HWND, font: HFONT, target: i32) -> Option<String> {
    let letters: Vec<(char, usize)> = ('a'..='z')
        .chain('A'..='Z')
        .map(|c| (c, text_width(hwnd, font, &c.to_string()) as usize))
        .filter(|&(_, w)| w > 0)
        .collect();
    let target = usize::try_from(target).ok()?;
    // last[n] = 幅 n を作れたときの最後の字。
    let mut last: Vec<Option<char>> = vec![None; target + 1];
    for n in 1..=target {
        last[n] = letters
            .iter()
            .find(|&&(_, w)| w <= n && (w == n || last[n - w].is_some()))
            .map(|&(c, _)| c);
    }
    let mut word = String::new();
    let mut n = target;
    while n > 0 {
        let c = last[n]?;
        word.push(c);
        n -= letters.iter().find(|&&(l, _)| l == c)?.1;
    }
    // 字を並べた幅が字ごとの幅の和と同じ（字の間の詰めが無い）ことを確かめてから使う。
    (text_width(hwnd, font, &word) == target as i32).then_some(word)
}

/// 出ているツールチップの文字を読み戻す。
fn tip_text(hwnd: HWND) -> String {
    // 古い版は受け皿の大きさを見ずに写すので、十分に大きく取る。
    let mut buf = vec![0u16; 8192];
    let mut ti = tool_info(hwnd, PWSTR(buf.as_mut_ptr()));
    // SAFETY: Win32 境界。手元の TTTOOLINFOW と受け皿を渡す。
    unsafe {
        SendMessageW(
            hwnd,
            TTM_GETTEXTW,
            Some(WPARAM(buf.len())),
            Some(LPARAM((&raw mut ti) as isize)),
        );
    }
    let len = buf.iter().position(|&u| u == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// OS が折り返した一番長い行が最大の幅ちょうどのとき、窓は上限ちょうどの幅で、語の途中で割らない。
///
/// 測った大きさが本当の窓より 1 でも大きいと（版 6 の `TTM_GETBUBBLESIZE`）、上限を越えたと見て
/// `force_break` が残りの行を語の途中で割る。版 5 のテストの exe では差が出ないので、ここで檻に
/// 入れるのは境い目の比べ（越えたら割る・ちょうどなら割らない）。
#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn line_wrapped_to_exactly_max_width_is_not_force_broken() {
    per_monitor_aware();
    let mut tip = TipWindow::default();
    let (work, dpi) = primary();
    let anchor = center(work);
    tip.show("あ", anchor).expect("字体を作る");
    let hwnd = tip.hwnd.expect("窓");
    let (font, _) = tip.font.expect("字体");
    let max_width = max_tip_width(dpi, work);
    let limit = window_width_for(hwnd, max_width);
    let first = word_exactly(hwnd, font, max_width).expect("最大の幅ちょうどの語を作れる");
    // 続く 2 語は、それぞれは 1 行に入り、2 つ並べると入らない長さ。割られると 3 語目の途中で切れる。
    let mut rest = String::new();
    while text_width(hwnd, font, &rest) * 2 <= max_width {
        rest.push_str("wrap");
    }
    let text = format!("{first} {rest} {rest}");
    let rect = tip.show(&text, anchor).expect("出す");
    pump_for(Duration::from_millis(150));
    let shown = tip_text(hwnd);
    tip.hide();
    println!(
        "[境い目] dpi={dpi} max_width={max_width} limit={limit} rect={:?} 1語目={} 割った={}",
        size_of_rect(rect),
        first.len(),
        shown != text
    );
    assert_eq!(
        size_of_rect(rect).width,
        limit,
        "一番長い行が最大の幅ちょうど"
    );
    assert_eq!(shown, text, "語の途中で割らない（OS の折り返しのまま）");
}

/// 字体の em の高さ（LOGFONTW の lfHeight の絶対値）。
fn em_height(font: HFONT) -> i32 {
    let mut lf = LOGFONTW::default();
    // SAFETY: Win32 境界。手元の LOGFONTW へ字体の定義を書かせる。
    unsafe {
        GetObjectW(
            font.into(),
            size_of::<LOGFONTW>() as i32,
            Some((&raw mut lf).cast()),
        )
    };
    lf.lfHeight.abs()
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s6_font_follows_each_screen_dpi() {
    per_monitor_aware();
    let mut tip = TipWindow::default();
    // (DPI, 字の em の高さ, 描いた字の高さ, 窓の高さ)
    let mut seen: Vec<(u32, i32, i32, i32)> = Vec::new();
    for m in monitor_rects() {
        let anchor = center(RectPx {
            left: m.left,
            top: m.top,
            right: m.right,
            bottom: m.bottom,
        });
        let (_, dpi) = monitor_at(anchor).expect("画面の情報");
        let rect = tip.show("あいうABC", anchor).expect("出す");
        pump_for(Duration::from_millis(150));
        let (font, made_for) = tip.font.expect("字体");
        assert_eq!(made_for, dpi, "その画面の DPI で字体を作った");
        let em = em_height(font);
        let drawn = text_height(tip.hwnd.expect("窓"), font, "あいうABC");
        let window = rect.bottom - rect.top;
        println!("[S6] monitor={m:?} dpi={dpi} em={em} 描いた字の高さ={drawn} 窓の高さ={window}");
        seen.push((dpi, em, drawn, window));
    }
    tip.hide();
    // 画面の間で DPI が違えば、字の高さの比は DPI の比に合う（em は ±5%、描いた高さは画素の丸めで ±15%）。
    for a in &seen {
        for b in &seen {
            if a.0 < b.0 {
                let want = f64::from(b.0) / f64::from(a.0);
                let em = f64::from(b.1) / f64::from(a.1);
                let drawn = f64::from(b.2) / f64::from(a.2);
                println!("[S6] DPI の比={want:.3} em の比={em:.3} 描いた字の比={drawn:.3}");
                assert!((em / want - 1.0).abs() < 0.05, "em の比が DPI の比に合う");
                assert!(
                    (drawn / want - 1.0).abs() < 0.15,
                    "描いた字の比が DPI の比に近い"
                );
            }
        }
    }
    // 画面に無い組み合わせも、100% と 150% の字体を作り分けて比を確かめる（設計の基準）。
    let hwnd = tip.hwnd.expect("窓");
    let f96 = tip.ensure_font(hwnd, 96).expect("96");
    let (em96, drawn96) = (em_height(f96), text_height(hwnd, f96, "あ"));
    let f144 = tip.ensure_font(hwnd, 144).expect("144");
    let (em144, drawn144) = (em_height(f144), text_height(hwnd, f144, "あ"));
    let ratio = f64::from(em144) / f64::from(em96);
    println!(
        "[S6] 96dpi: em={em96} 描いた={drawn96} / 144dpi: em={em144} 描いた={drawn144} / em の比={ratio:.3}"
    );
    assert!((ratio - 1.5).abs() < 0.075, "150% の字は 100% の約 1.5 倍");
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s7_stays_in_front_after_another_topmost_window_is_raised() {
    per_monitor_aware();
    let mut tip = TipWindow::default();
    let (work, _) = primary();
    let rect = tip.show("最初の試し S7", center(work)).expect("出す");
    let hwnd = tip.hwnd.expect("窓");
    // ツールチップを覆う、いつも手前の別の窓を出し、最前面へ当て直す（重なりの立て直しの近似）。
    let other = marker_window(
        RectPx {
            left: rect.left - 20,
            top: rect.top - 20,
            right: rect.right + 20,
            bottom: rect.bottom + 20,
        },
        true,
    );
    // SAFETY: Win32 境界。自分で作った窓を最前面へ当て直すだけ（入力先は変えない）。
    unsafe {
        SetWindowPos(
            other,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        )
    }
    .expect("最前面へ");
    pump_for(Duration::from_millis(150));
    let covered = is_above(other, hwnd);
    // 見回りで当て直す（逃げ道）。
    tip.keep_on_top().expect("当て直す");
    pump_for(Duration::from_millis(150));
    let in_front = is_above(hwnd, other);
    println!("[S7] 当て直しの直後に覆われた={covered} 見回りの当て直しの後に手前={in_front}");
    tip.hide();
    // SAFETY: Win32 境界。自分で作った窓を壊す。
    let _ = unsafe { DestroyWindow(other) };
    assert!(in_front, "見回りの当て直しの後はツールチップが手前");
}
