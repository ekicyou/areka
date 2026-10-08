//! `os_tests` の子。版 5 と版 6 の両方で走らせる実機のテスト（どれも `#[ignore]`）。
//!
//! テストの exe にはマニフェストが無く、標準の部品は版 5 で動く。版 6 の窓は、版 6 を申告する
//! サンプルのマニフェスト（`examples.manifest`）から作った実行の文脈を有効にした間に作る。窓の
//! 種類は作るときの文脈で決まり、文脈を外した後も版 6 のまま動くので、同じ exe の中で版 5 と
//! 版 6 の窓を並べて確かめられる（`build.rs` を替えずに済む）。
//!
//! 走らせ方: `os_tests` と同じ（`tooltip::os` に含まれる）。

use super::*;
use windows::Win32::Foundation::{HANDLE, HMODULE};
use windows::Win32::System::LibraryLoader::GetProcAddress;
use windows::Win32::UI::Controls::TTM_GETTEXTW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{DLLGETVERSIONPROC, DLLVERSIONINFO};
use windows::Win32::UI::WindowsAndMessaging::{GCLP_HMODULE, GetClassLongPtrW};
use windows::core::s;

/// `ACTCTXW`（根の `windows` クレートで有効な機能に無いので、テストの中だけで宣言する）。
#[repr(C)]
struct ActCtxW {
    cb_size: u32,
    dw_flags: u32,
    lp_source: PCWSTR,
    w_processor_architecture: u16,
    w_lang_id: u16,
    lp_assembly_directory: PCWSTR,
    lp_resource_name: PCWSTR,
    lp_application_name: PCWSTR,
    h_module: isize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateActCtxW(actctx: *const ActCtxW) -> HANDLE;
    fn ActivateActCtx(h: HANDLE, cookie: *mut usize) -> BOOL;
    fn DeactivateActCtx(flags: u32, cookie: usize) -> BOOL;
    fn ReleaseActCtx(h: HANDLE);
}

/// 版 6 を指す実行の文脈。生きている間、このスレッドで作る標準の部品の窓は版 6 になる。
struct V6Context {
    handle: HANDLE,
    cookie: usize,
}

impl V6Context {
    fn activate() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples.manifest");
        let wide: Vec<u16> = manifest
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain([0])
            .collect();
        let ctx = ActCtxW {
            cb_size: size_of::<ActCtxW>() as u32,
            dw_flags: 0,
            lp_source: PCWSTR(wide.as_ptr()),
            w_processor_architecture: 0,
            w_lang_id: 0,
            lp_assembly_directory: PCWSTR::null(),
            lp_resource_name: PCWSTR::null(),
            lp_application_name: PCWSTR::null(),
            h_module: 0,
        };
        // SAFETY: Win32 境界。手元の構造体（マニフェストの道は呼び出しの間だけ生きる）から文脈を
        // 作り、このスレッドで有効にする。
        unsafe {
            let handle = CreateActCtxW(&ctx);
            assert!(handle.0 as isize != -1, "版 6 の文脈を作れた");
            let mut cookie = 0;
            assert!(
                ActivateActCtx(handle, &mut cookie).as_bool(),
                "文脈を有効にした"
            );
            Self { handle, cookie }
        }
    }
}

impl Drop for V6Context {
    fn drop(&mut self) {
        // SAFETY: Win32 境界。同じスレッドで有効にした文脈を外して放す。
        unsafe {
            let _ = DeactivateActCtx(0, self.cookie);
            ReleaseActCtx(self.handle);
        }
    }
}

/// 窓の種類を登録した comctl32 の `DllGetVersion` の主版（`CCM_GETVERSION` と別の道で版を確かめる）。
fn class_dll_major(hwnd: HWND) -> u32 {
    // SAFETY: Win32 境界。窓の種類の持ち主の DLL から版を問う関数を引き、手元の構造体へ書かせる。
    unsafe {
        let module = HMODULE(GetClassLongPtrW(hwnd, GCLP_HMODULE) as *mut _);
        let get: DLLGETVERSIONPROC =
            std::mem::transmute(GetProcAddress(module, s!("DllGetVersion")));
        let mut info = DLLVERSIONINFO {
            cbSize: size_of::<DLLVERSIONINFO>() as u32,
            ..Default::default()
        };
        get.expect("DllGetVersion")(&mut info)
            .ok()
            .expect("版を読む");
        info.dwMajorVersion
    }
}

/// 版を選んでツールチップの窓を作る（版 6 は文脈を有効にした間に作る）。
fn tip_for(v6: bool) -> TipWindow {
    per_monitor_aware();
    let mut tip = TipWindow::default();
    {
        let _ctx = v6.then(V6Context::activate);
        tip.show("版を見る", center(primary().0)).expect("窓を作る");
    }
    tip.hide();
    let major = class_dll_major(tip.hwnd.expect("窓"));
    println!(
        "[版] 求めた版 6={v6} 見分けた版 6={} DLL の主版={major}",
        tip.v6
    );
    assert_eq!(major >= 6, v6, "求めた版の窓を作った");
    assert_eq!(tip.v6, v6, "窓の版を見分けた");
    tip
}

fn window_dpi(hwnd: HWND) -> u32 {
    // SAFETY: Win32 境界。読むだけ。
    unsafe { GetDpiForWindow(hwnd) }
}

/// 語の切れ目のある英語の長い文。
fn english() -> String {
    "The tooltip wraps this English sentence at spaces between words. ".repeat(4)
}

fn screen_center(m: RECT) -> PointPx {
    center(RectPx {
        left: m.left,
        top: m.top,
        right: m.right,
        bottom: m.bottom,
    })
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s2_on_v6() {
    s2_check(&mut tip_for(true));
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s5_long_japanese_line_and_url_wrap_within_max_width() {
    s5_check(&mut tip_for(false));
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn s5_on_v6() {
    s5_check(&mut tip_for(true));
}

fn s5_check(tip: &mut TipWindow) {
    let (work, dpi) = primary();
    let anchor = center(work);
    let one_line = size_of_rect(tip.show("あ", anchor).expect("1 行")).height;
    let hwnd = tip.hwnd.expect("窓");
    let max_width = max_tip_width(dpi, work);
    let limit = width_limit(hwnd, max_width, dpi);
    let japanese =
        "ツールチップの折り返しを確かめるための、句読点の少ない長い日本語の一行です".repeat(4);
    let url = format!("https://example.com/{}", "abcdefghij0123456789".repeat(15));
    let english = english();
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
            "[S5] {name}: 版 6={} dpi={dpi} max_width={max_width} limit={limit} rect={size:?} \
             1行={one_line} OS だけの幅={} force_break={}",
            tip.v6,
            raw.width,
            raw.width > limit
        );
        assert!(size.width <= limit, "{name}: 最大の幅（＋ふち）に収まる");
        assert!(size.height > one_line, "{name}: 複数の行に折り返した");
        if name == "英語" {
            assert!(
                raw.width <= limit,
                "英語は OS の折り返しだけで収まる（割らない）"
            );
        }
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

/// OS が折り返した一番長い行が最大の幅ちょうどのとき、語の途中で割らない。分けられない語が
/// 遊びより長く最大の幅を越えれば割る（比べが働いている）。版 6 は分けられない語も OS 自身が
/// 最大の幅で割るので（6.3 の実機。URL も同じ）、`force_break` に回らず、窓が上限に収まることだけを見る。
///
/// 版 6 は最大の幅を DPI の倍率で広げ（割り戻して渡さないと OS の幅がいつも越えて見える）、
/// `TTM_ADJUSTRECT` は実際の窓より 1〜2 狭い。どちらかが崩れると、ちょうどの語の行が割られる。
fn boundary_check(tip: &mut TipWindow) {
    let (work, dpi) = primary();
    let anchor = center(work);
    tip.show("あ", anchor).expect("字体を作る");
    let hwnd = tip.hwnd.expect("窓");
    let (font, _) = tip.font.expect("字体");
    let max_width = max_tip_width(dpi, work);
    let limit = width_limit(hwnd, max_width, dpi);
    // 続く 2 語は、それぞれは 1 行に入り、2 つ並べると入らない長さ。割られると 3 語目の途中で切れる。
    let mut rest = String::new();
    while text_width(hwnd, font, &rest) * 2 <= max_width {
        rest.push_str("wrap");
    }
    for (over, want_broken) in [(0, false), (scale(4, dpi), true)] {
        let first = word_exactly(hwnd, font, max_width + over).expect("幅を決めた語を作れる");
        let text = format!("{first} {rest} {rest}");
        let rect = tip.show(&text, anchor).expect("出す");
        pump_for(Duration::from_millis(150));
        let shown = tip_text(hwnd);
        tip.hide();
        let width = size_of_rect(rect).width;
        println!(
            "[境い目] 版 6={} dpi={dpi} max_width={max_width} 越えた分={over} limit={limit} \
             窓の幅={width} 割った={}",
            tip.v6,
            shown != text
        );
        assert!(width <= limit, "窓は上限に収まる");
        if want_broken {
            if !tip.v6 {
                assert_ne!(shown, text, "遊びより長く越えた語は割る");
            }
        } else {
            assert!(
                width >= window_width_for(hwnd, max_width),
                "一番長い行が最大の幅ちょうど"
            );
            assert_eq!(shown, text, "語の途中で割らない（OS の折り返しのまま）");
        }
    }
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn line_wrapped_to_exactly_max_width_is_not_force_broken() {
    boundary_check(&mut tip_for(false));
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn line_wrapped_to_exactly_max_width_is_not_force_broken_on_v6() {
    boundary_check(&mut tip_for(true));
}

/// どの画面（このマシンでは 200% と 150%）でも、英語の長い文は OS が語の切れ目で折り返し、
/// どの行も最大の幅を越えない。最大の幅の近くまで使う（割り戻しすぎていない）。
fn english_check(tip: &mut TipWindow) {
    let text = english();
    let mut seen = Vec::new();
    for m in monitor_rects() {
        let anchor = screen_center(m);
        let (work, dpi) = monitor_at(anchor).expect("画面の情報");
        let rect = tip.show(&text, anchor).expect("出す");
        pump_for(Duration::from_millis(150));
        let hwnd = tip.hwnd.expect("窓");
        let (font, _) = tip.font.expect("字体");
        let shown = tip_text(hwnd);
        let max_width = max_tip_width(dpi, work);
        let longest_word = text
            .split_inclusive(' ')
            .map(|w| text_width(hwnd, font, w))
            .max()
            .unwrap_or(0);
        let size = size_of_rect(rect);
        let full = window_width_for(hwnd, max_width - longest_word);
        let limit = width_limit(hwnd, max_width, dpi);
        println!(
            "[英語] 版 6={} dpi={dpi} 窓の DPI={} max_width={max_width} 窓={size:?} \
             下限={full} 上限={limit} 割った={}",
            tip.v6,
            window_dpi(hwnd),
            shown != text
        );
        assert_eq!(window_dpi(hwnd), dpi, "出した画面の DPI で並べた");
        assert_eq!(
            shown, text,
            "語の途中で割らない（OS が語の切れ目で折り返したまま）"
        );
        assert!(size.width <= limit, "どの行も最大の幅（＋ふち）を越えない");
        assert!(size.width >= full, "最大の幅の近くまで使って折り返した");
        assert!(
            size.height >= text_height(hwnd, font, "x") * 3,
            "3 行以上に折り返した"
        );
        seen.push(dpi);
        tip.hide();
    }
    println!("[英語] 確かめた画面の DPI={seen:?}");
    assert!(!seen.is_empty(), "画面がある");
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn english_wraps_at_word_boundaries_within_max_width() {
    english_check(&mut tip_for(false));
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn english_wraps_at_word_boundaries_within_max_width_on_v6() {
    english_check(&mut tip_for(true));
}

/// DPI の違う画面へ続けて出しても（消えている所から・出ているまま）、測った大きさで出る。
///
/// 版 6 は窓の DPI で並べ、画面の外では最後の画面の DPI のまま。前の画面の DPI で測ると、出した
/// 所で並べ直されて大きさが変わり、左上がずれる。
fn dpi_switch_check(tip: &mut TipWindow) {
    let screens: Vec<(PointPx, u32)> = monitor_rects()
        .into_iter()
        .map(|m| {
            let a = screen_center(m);
            (a, monitor_at(a).expect("画面の情報").1)
        })
        .collect();
    let a = *screens.first().expect("画面がある");
    let Some(&b) = screens.iter().find(|s| s.1 != a.1) else {
        println!("[DPI の切り替え] DPI の違う画面が無いので確かめられない（not available）");
        return;
    };
    let text = english();
    for (from, to) in [(a, b), (b, a)] {
        for keep_shown in [false, true] {
            tip.show(&text, from.0).expect("前の画面に出す");
            if !keep_shown {
                tip.hide();
            }
            let rect = tip.show(&text, to.0).expect("次の画面に出す");
            pump_for(Duration::from_millis(150));
            let hwnd = tip.hwnd.expect("窓");
            let want = expected_top_left(to.0, size_of_rect(rect));
            println!(
                "[DPI の切り替え] 版 6={} {}→{} 出ているまま={keep_shown} 窓の DPI={} \
                 rect={rect:?} want={want:?}",
                tip.v6,
                from.1,
                to.1,
                window_dpi(hwnd)
            );
            assert_eq!(hwnd_rect(hwnd), rect, "出した後に大きさが変わらない");
            assert_eq!(window_dpi(hwnd), to.1, "出した画面の DPI");
            assert_eq!(
                (rect.left, rect.top),
                (want.x, want.y),
                "測った大きさで置いた"
            );
            assert_eq!(tip_text(hwnd), text, "語の途中で割らない");
            tip.hide();
        }
    }
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn dpi_switch_between_screens_shows_measured_size() {
    dpi_switch_check(&mut tip_for(false));
}

#[test]
#[ignore = "実機の画面とマウスを使う（--ignored で明示して走らせる）"]
fn dpi_switch_between_screens_shows_measured_size_on_v6() {
    dpi_switch_check(&mut tip_for(true));
}
