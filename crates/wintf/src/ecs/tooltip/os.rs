//! OS の境界。OS の値を読み、標準のツールチップ 1 枚を作って出す・消す。
//!
//! この機能の `unsafe` と Win32 の呼び出しはこのファイルだけに置く。計算（置き場所・
//! 最大の幅・改行の揃え・はみ出す行の割り方）は `geometry` に置き、ここは OS に渡すだけ。

use super::geometry::{
    PlaceInput, PointPx, RectPx, SizePx, force_break, logical_max_width, max_tip_width,
    normalize_newlines, place,
};
use super::turn::FALLBACK_HOVER_TIME;
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::warn;
use windows::Win32::Foundation::{COLORREF, E_FAIL, HWND, LPARAM, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DeleteObject, GetDC, GetMonitorInfoW, GetTextExtentExPointW, HFONT,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, ReleaseDC, SelectObject,
};
use windows::Win32::UI::Controls::{
    CCM_GETVERSION, ICC_BAR_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, TOOLTIPS_CLASS,
    TTF_ABSOLUTE, TTF_TRACK, TTF_TRANSPARENT, TTM_ADDTOOLW, TTM_ADJUSTRECT, TTM_SETMAXTIPWIDTH,
    TTM_TRACKACTIVATE, TTM_TRACKPOSITION, TTM_UPDATETIPTEXTW, TTS_ALWAYSTIP, TTS_NOANIMATE,
    TTS_NOFADE, TTS_NOPREFIX, TTTOOLINFOW,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, GetDpiForWindow, GetSystemMetricsForDpi, MDT_EFFECTIVE_DPI,
    SystemParametersInfoForDpi,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON, VK_XBUTTON1, VK_XBUTTON2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetCursorPos, GetWindowRect, HWND_TOPMOST, IsWindowVisible,
    LWA_ALPHA, NONCLIENTMETRICSW, SM_CYCURSOR, SPI_GETMOUSEHOVERTIME, SPI_GETNONCLIENTMETRICS,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SendMessageW, SetLayeredWindowAttributes, SetWindowPos, SystemParametersInfoW, WINDOW_STYLE,
    WM_SETFONT, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, PWSTR};

/// OS の表示の失敗。
#[derive(Debug, thiserror::Error)]
pub enum TooltipOsError {
    #[error("ツールチップの窓を作れなかった: {0}")]
    Create(#[source] windows::core::Error),
    #[error("ツールチップを出せなかった（{stage}）")]
    Show { stage: &'static str },
}

/// 1 回の判定で読む OS の値。
pub(crate) struct OsSample {
    /// マウスの画面の位置。読めなければ None。
    pub cursor: Option<PointPx>,
    /// 左・右・中・拡張 2 つのどれかが押されているか。
    pub button_down: bool,
}

/// マウスの位置とボタンの押下を読む。
pub(crate) fn sample() -> OsSample {
    let mut pt = POINT::default();
    // SAFETY: Win32 境界。書き込み先は手元の POINT。
    let cursor = unsafe { GetCursorPos(&mut pt) }
        .ok()
        .map(|()| PointPx { x: pt.x, y: pt.y });
    // 物理のボタンを見る（左右の入れ替えの設定に関わらず、5 つ全部を見るので同じ）。
    let buttons: [VIRTUAL_KEY; 5] = [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_XBUTTON1, VK_XBUTTON2];
    // SAFETY: Win32 境界。引数は仮想キーの番号だけ。最上位のビットが立っていれば押されている。
    let button_down = buttons
        .iter()
        .any(|vk| unsafe { GetAsyncKeyState(i32::from(vk.0)) } < 0);
    OsSample {
        cursor,
        button_down,
    }
}

/// 読めなかった `warn` を出したか（プロセスで 1 回だけにする）。
static HOVER_TIME_WARNED: AtomicBool = AtomicBool::new(false);

/// OS のマウスを止めたと見なす待ち時間の設定を読む。読めなければ代わりの値（要件 1.6）。
pub(crate) fn read_hover_time() -> Duration {
    let mut ms: u32 = 0;
    // SAFETY: Win32 境界。SPI_GETMOUSEHOVERTIME は書き込み先の u32 に値を書くだけ。
    let read = unsafe {
        SystemParametersInfoW(
            SPI_GETMOUSEHOVERTIME,
            0,
            Some((&raw mut ms).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    match read {
        Ok(()) => Duration::from_millis(u64::from(ms)),
        Err(e) => {
            // 数え始めるたびに読むので、毎回出すと記録が埋まる。
            if !HOVER_TIME_WARNED.swap(true, Ordering::Relaxed) {
                warn!(
                    error = %e,
                    fallback_ms = FALLBACK_HOVER_TIME.as_millis() as u64,
                    "[tooltip_hover_time_unreadable] 待ち時間の設定を読めないので代わりの値で続ける"
                );
            }
            FALLBACK_HOVER_TIME
        }
    }
}

/// 窓が見えているか。
pub(crate) fn is_visible(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。壊れた HWND を渡しても偽が返るだけ。
    unsafe { IsWindowVisible(hwnd) }.as_bool()
}

/// UTF-16 の先頭 units 個に丸ごと入る `char` の数（サロゲート対の途中で切らない）。
///
/// `GetTextExtentExPointW` の入る数は UTF-16 の数、`geometry::force_break` は `char` の数で受ける。
pub(crate) fn chars_within_utf16(text: &str, units: usize) -> usize {
    let mut used = 0;
    text.chars()
        .take_while(|c| {
            used += c.len_utf16();
            used <= units
        })
        .count()
}

/// 道具の番号（窓に道具は 1 つだけ）。
const TOOL_ID: usize = 1;
/// 大きさを測る間に置く、どの画面にも掛からない位置（最小化した窓と同じ慣例の座標）。
const PARKED: PointPx = PointPx {
    x: -32000,
    y: -32000,
};
/// 画面の情報が読めないときの作業領域（収める処理が効かない広さ）。
const UNBOUNDED: RectPx = RectPx {
    left: i32::MIN / 4,
    top: i32::MIN / 4,
    right: i32::MAX / 4,
    bottom: i32::MAX / 4,
};

/// 標準のツールチップの窓（UI スレッドに 1 枚）。最初に出すときに作り、`Drop` で壊す。
///
/// wintf の窓のエンティティにはしない（窓の数・終了の判断・クリック透過の登録に入らない）。
#[derive(Default)]
pub(crate) struct TipWindow {
    hwnd: Option<HWND>,
    /// 渡してある字体と、それを作った DPI。
    font: Option<(HFONT, u32)>,
    /// 標準の部品の版 6 か。版 6 は `TTM_SETMAXTIPWIDTH` の幅に窓の DPI の倍率を掛けて折り返す
    /// （版 5 は渡した幅のまま）。窓を作ったときに `CCM_GETVERSION` で見る（版 5 は 0、版 6 は 6
    /// を返す。窓ごとに見るので、同じプロセスに両方の版があっても取り違えない）。
    v6: bool,
}

impl TipWindow {
    /// 無ければ作る。anchor の真上に text を出し、出た矩形を返す。
    ///
    /// `Err` を返したとき、ツールチップは出ていない（途中まで出ていれば消してから返す）。
    pub(crate) fn show(&mut self, text: &str, anchor: PointPx) -> Result<RectPx, TooltipOsError> {
        let shown = self.try_show(text, anchor);
        if shown.is_err() {
            self.hide();
        }
        shown
    }

    /// 追跡をやめて消す。出ていなくても、窓が無くても安全。
    pub(crate) fn hide(&mut self) {
        if let Some(hwnd) = self.hwnd {
            let mut ti = tool_info(hwnd, PWSTR::null());
            // SAFETY: Win32 境界。自分で作ったツールチップの窓へ、手元の TTTOOLINFOW を渡す。
            unsafe {
                SendMessageW(
                    hwnd,
                    TTM_TRACKACTIVATE,
                    Some(WPARAM(0)),
                    Some(LPARAM((&raw mut ti) as isize)),
                );
            }
        }
    }

    /// いつも手前の窓の中の最前へ当て直す（入力先は変えない）。窓が無ければ何もしない。
    ///
    /// 後から最前面へ当て直された別のいつも手前の窓はツールチップを覆う（最初の試し S7）。
    /// 出す番の間の見回りのたびに呼んで手前へ戻す（設計の S7 の逃げ道）。
    pub(crate) fn keep_on_top(&self) -> Result<(), TooltipOsError> {
        let Some(hwnd) = self.hwnd else {
            return Ok(());
        };
        // SAFETY: Win32 境界。自分のツールチップの窓の重なりの順だけを変える。
        unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            )
        }
        .map_err(|_| TooltipOsError::Show { stage: "topmost" })
    }

    fn try_show(&mut self, text: &str, anchor: PointPx) -> Result<RectPx, TooltipOsError> {
        let hwnd = self.ensure_window()?;
        let (work_area, dpi) = monitor_at(anchor).unwrap_or_else(|| {
            warn!(
                x = anchor.x,
                y = anchor.y,
                "[tooltip_monitor_unreadable] 画面の情報を読めないので作業領域に収めずに出す"
            );
            (UNBOUNDED, 96)
        });
        // ⓪ 窓の DPI を出す画面の DPI に揃える。版 6 は今の窓の DPI で並べ、窓が別の DPI の画面へ
        // 動くとその DPI で並べ直す。画面の外 (PARKED) では最後に居た画面の DPI のまま。揃えずに
        // 測ると、出した所で大きさが変わり、置き場所がずれる。隠したまま出す所へ動かすと、窓の DPI
        // はその画面の DPI になり、画面の外へ移っても保たれる（6.3 の実機）。
        // SAFETY: Win32 境界。自分の窓の DPI を読むだけ。
        if unsafe { GetDpiForWindow(hwnd) } != dpi {
            self.hide();
            // SAFETY: Win32 境界。隠れている自分の窓の位置だけを変える（見せない・入力先を取らない）。
            unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    anchor.x,
                    anchor.y,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
                )
            }
            .map_err(|_| TooltipOsError::Show { stage: "dpi" })?;
        }
        // ① 字体（DPI が前回と同じなら作り直さない）。
        let font = self.ensure_font(hwnd, dpi)?;
        // ② 最大の幅。版 6 には窓の DPI の倍率で割り戻した幅を渡す（OS が掛け戻して物理の幅で折る）。
        let max_width = max_tip_width(dpi, work_area);
        // SAFETY: Win32 境界。自分の窓の DPI を読むだけ。
        let window_dpi = unsafe { GetDpiForWindow(hwnd) };
        let os_max_width = if self.v6 {
            logical_max_width(max_width, window_dpi)
        } else {
            max_width
        };
        // SAFETY: Win32 境界。自分のツールチップの窓へ値だけを渡す。
        unsafe {
            SendMessageW(
                hwnd,
                TTM_SETMAXTIPWIDTH,
                Some(WPARAM(0)),
                Some(LPARAM(os_max_width as isize)),
            );
        }
        // ③ CR LF に揃えた文字 → ④ 大きさを測る。
        let mut text = normalize_newlines(text);
        let mut size = set_text_and_measure(hwnd, &text)?;
        // 最大の幅は文字の幅なので、ふちを足した窓の幅と比べる（測った大きさと同じ窓の矩形の単位）。
        // 越えていれば分けられない語が残っている。
        if size.width > width_limit(hwnd, max_width, window_dpi) {
            text = break_long_lines(hwnd, font, &text, max_width)?;
            size = set_text_and_measure(hwnd, &text)?;
        }
        // ⑤ 置き場所。
        // SAFETY: Win32 境界。引数は番号と DPI だけ。
        let cursor_height = unsafe { GetSystemMetricsForDpi(SM_CYCURSOR, dpi) };
        let pos = place(&PlaceInput {
            anchor,
            tip: size,
            work_area,
            offset_above: scale(20, dpi),
            offset_below: cursor_height,
        });
        // ⑥ 出す（置き場所へ動かす）。
        track_at(hwnd, pos);
        // ⑦ いつも手前の最前へ。
        self.keep_on_top()?;
        // ⑧ 実際の矩形。
        window_rect(hwnd)
    }

    /// 窓が無ければ作り、道具を 1 つ登録する。
    fn ensure_window(&mut self) -> Result<HWND, TooltipOsError> {
        if let Some(hwnd) = self.hwnd {
            return Ok(hwnd);
        }
        let icc = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        };
        // SAFETY: Win32 境界。標準の部品を読み込み、ツールチップの窓の種類を登録させる。
        // 失敗しても次の CreateWindowExW が失敗して誤りになるので、結果は見ない。
        let _ = unsafe { InitCommonControlsEx(&icc) };
        // 持ち主の窓は付けない。マウスを素通しし、入力先を取らず、タスクバーに出さない。
        //
        // 素通しは WS_EX_LAYERED と WS_EX_TRANSPARENT の組で効かせる（最初の試し S4）。ツールチップは
        // マウスの当たりの問い合わせに「透明」と答えるが、OS がそれで下へ回すのは同じスレッドの窓だけ
        // で、別のアプリ（メモ帳）の窓への押下は捨てられる。層の窓にして初めて別のプロセスへ素通しする。
        // 標準のツールチップはフェードが有効だと出すたび（TTM_TRACKACTIVATE）に WS_EX_LAYERED を
        // 外すので、TTS_NOFADE と TTS_NOANIMATE を付けて作り、外されないようにする。
        // SAFETY: Win32 境界。標準の種類の窓を作るだけ。
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOOLWINDOW
                    | WS_EX_TRANSPARENT
                    | WS_EX_LAYERED,
                TOOLTIPS_CLASS,
                PCWSTR::null(),
                WINDOW_STYLE(
                    WS_POPUP.0 | TTS_NOPREFIX | TTS_ALWAYSTIP | TTS_NOFADE | TTS_NOANIMATE,
                ),
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .map_err(TooltipOsError::Create)?;
        // 層の窓は不透明度を決めるまで描かれないので、不透明（255）にする。
        // SAFETY: Win32 境界。作ったばかりの自分の窓の不透明度を決める。
        if let Err(e) = unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA) } {
            // SAFETY: Win32 境界。作ったばかりの自分の窓を壊す。
            let _ = unsafe { DestroyWindow(hwnd) };
            return Err(TooltipOsError::Create(e));
        }
        let mut empty = [0u16];
        let mut ti = tool_info(hwnd, PWSTR(empty.as_mut_ptr()));
        // SAFETY: Win32 境界。手元の TTTOOLINFOW を渡す（文字は窓の側に写される）。
        let added = unsafe {
            SendMessageW(
                hwnd,
                TTM_ADDTOOLW,
                Some(WPARAM(0)),
                Some(LPARAM((&raw mut ti) as isize)),
            )
        };
        if added.0 == 0 {
            // SAFETY: Win32 境界。作ったばかりの自分の窓を壊す。
            let _ = unsafe { DestroyWindow(hwnd) };
            return Err(TooltipOsError::Create(windows::core::Error::new(
                E_FAIL,
                "TTM_ADDTOOLW",
            )));
        }
        // SAFETY: Win32 境界。自分の窓に標準の部品の版を問うだけ。
        let version = unsafe { SendMessageW(hwnd, CCM_GETVERSION, None, None) }.0;
        self.v6 = version >= 6;
        self.hwnd = Some(hwnd);
        Ok(hwnd)
    }

    /// DPI に合った字体を渡す。前回と同じ DPI なら作り直さない（要件 3.7）。
    fn ensure_font(&mut self, hwnd: HWND, dpi: u32) -> Result<HFONT, TooltipOsError> {
        if let Some((font, made_for)) = self.font
            && made_for == dpi
        {
            return Ok(font);
        }
        let mut ncm = NONCLIENTMETRICSW {
            cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        // SAFETY: Win32 境界。書き込み先は大きさを書いた手元の NONCLIENTMETRICSW。
        unsafe {
            SystemParametersInfoForDpi(
                SPI_GETNONCLIENTMETRICS.0,
                ncm.cbSize,
                Some((&raw mut ncm).cast()),
                0,
                dpi,
            )
        }
        .map_err(|_| TooltipOsError::Show { stage: "font" })?;
        // SAFETY: Win32 境界。手元の LOGFONTW から字体を作る。
        let font = unsafe { CreateFontIndirectW(&ncm.lfStatusFont) };
        if font.is_invalid() {
            return Err(TooltipOsError::Show { stage: "font" });
        }
        // SAFETY: Win32 境界。字体を渡し（窓は持ち主にならない）、前の字体は渡し終えてから消す。
        unsafe {
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
            if let Some((old, _)) = self.font.replace((font, dpi)) {
                let _ = DeleteObject(old.into());
            }
        }
        Ok(font)
    }
}

impl Drop for TipWindow {
    fn drop(&mut self) {
        // SAFETY: Win32 境界。自分で作った窓と字体を、窓→字体の順に壊す（同じ UI スレッドで持つ）。
        unsafe {
            if let Some(hwnd) = self.hwnd.take() {
                let _ = DestroyWindow(hwnd);
            }
            if let Some((font, _)) = self.font.take() {
                let _ = DeleteObject(font.into());
            }
        }
    }
}

/// 道具 1 つを指す TTTOOLINFOW。
///
/// 道具の窓（`hwnd`）はツールチップの窓自身。`TTF_TRANSPARENT` は受けたマウスのメッセージを道具の
/// 窓へ回すので自分へ回りうるが、層の窓で素通しするため、ツールチップのスレッドへマウスの
/// メッセージは届かず、その回し先の道は通らない（最初の試し S4 の上での移動で確かめた）。
fn tool_info(hwnd: HWND, text: PWSTR) -> TTTOOLINFOW {
    TTTOOLINFOW {
        // 末尾の lpReserved を含めない大きさにする。含めると、版 6 の申告の無い exe（古い版）で
        // TTM_ADDTOOLW が失敗する。
        cbSize: std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32,
        uFlags: TTF_TRACK | TTF_ABSOLUTE | TTF_TRANSPARENT,
        hwnd,
        uId: TOOL_ID,
        lpszText: text,
        ..Default::default()
    }
}

/// 追跡型の道具を pos（左上）に置いて追跡を始める（出ていればその場で動かす）。
fn track_at(hwnd: HWND, pos: PointPx) {
    let mut ti = tool_info(hwnd, PWSTR::null());
    // SAFETY: Win32 境界。自分のツールチップの窓へ、位置と手元の TTTOOLINFOW を渡す。
    unsafe {
        SendMessageW(
            hwnd,
            TTM_TRACKPOSITION,
            Some(WPARAM(0)),
            Some(LPARAM(make_lparam(pos.x, pos.y))),
        );
        SendMessageW(
            hwnd,
            TTM_TRACKACTIVATE,
            Some(WPARAM(1)),
            Some(LPARAM((&raw mut ti) as isize)),
        );
    }
}

/// 文字を入れ直し、置き場所へ動かす前の窓の大きさを測る。
///
/// 大きさは追跡の始まった窓の `GetWindowRect` で測る（設計の S2 の逃げ道）。`TTM_GETBUBBLESIZE` は
/// 使わない: 古い版（版 5）は追跡を始める前に送るとプロセスごと落ち、新しい版（版 6）は実際の窓より
/// 幅・高さとも 1 大きい値を返す（最初の試し S2・6.1 の実機）。出ていなければ画面の外で追跡を始めて
/// から測り、置き場所へは後で動かす。出ていればその場で入れ替えて測る（画面の外へ外すと、出ている
/// ものが一瞬消えうる）。追跡は文字が入ってからでないと始まらないので、文字の後に始める。始まらな
/// ければ測らずに誤りにする。
fn set_text_and_measure(hwnd: HWND, text: &str) -> Result<SizePx, TooltipOsError> {
    let mut wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
    let mut ti = tool_info(hwnd, PWSTR(wide.as_mut_ptr()));
    // SAFETY: Win32 境界。手元の TTTOOLINFOW と文字を渡す（文字は窓の側に写される）。
    unsafe {
        SendMessageW(
            hwnd,
            TTM_UPDATETIPTEXTW,
            Some(WPARAM(0)),
            Some(LPARAM((&raw mut ti) as isize)),
        );
    }
    if !is_visible(hwnd) {
        track_at(hwnd, PARKED);
        if !is_visible(hwnd) {
            return Err(TooltipOsError::Show { stage: "track" });
        }
    }
    let rc = window_rect(hwnd)?;
    let size = SizePx {
        width: rc.right - rc.left,
        height: rc.bottom - rc.top,
    };
    if size.width <= 0 || size.height <= 0 {
        return Err(TooltipOsError::Show { stage: "size" });
    }
    Ok(size)
}

/// 窓の矩形（画面の物理ピクセル）。
fn window_rect(hwnd: HWND) -> Result<RectPx, TooltipOsError> {
    let mut rc = RECT::default();
    // SAFETY: Win32 境界。書き込み先は手元の RECT。
    unsafe { GetWindowRect(hwnd, &mut rc) }.map_err(|_| TooltipOsError::Show { stage: "rect" })?;
    Ok(RectPx {
        left: rc.left,
        top: rc.top,
        right: rc.right,
        bottom: rc.bottom,
    })
}

/// 最大の幅 max_width の行を並べた窓の幅の上限。これを越えた窓には、OS が分けられなかった語がある。
///
/// ふちと余白は `TTM_ADJUSTRECT(TRUE)` で足し、遊びを 2 論理ピクセル足す。OS が並べた窓の幅は、
/// ADJUSTRECT の答えより版 5 で 0〜1、版 6 で 1〜2 ピクセル広い（版 6 はふちの換算の差、両方とも
/// 行の終わりの字のはみ出しの分。6.3 の実機の 200%・150%）。遊びが無いと、OS が最大の幅の内で
/// 折り返せた英語の行まで越えたと見て、語の途中で割ってしまう。
// ponytail: 遊びの分（2 論理ピクセル）だけ最大の幅を越える分けられない語は割らない（窓がその分
// 広くなるだけ）。ぴったりにするなら、字体を作るたびに 1 行の見本を OS に並べさせてふちを測る。
fn width_limit(hwnd: HWND, max_width: i32, window_dpi: u32) -> i32 {
    window_width_for(hwnd, max_width) + scale(2, window_dpi)
}

/// 文字の幅 text_width を出すのに要る窓の幅（ふちと余白を足す）。読めなければ text_width。
fn window_width_for(hwnd: HWND, text_width: i32) -> i32 {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: text_width,
        bottom: 0,
    };
    // SAFETY: Win32 境界。手元の RECT を文字の矩形から窓の矩形へ直させる。
    let ok = unsafe {
        SendMessageW(
            hwnd,
            TTM_ADJUSTRECT,
            Some(WPARAM(1)),
            Some(LPARAM((&raw mut rc) as isize)),
        )
    };
    if ok.0 == 0 {
        text_width
    } else {
        rc.right - rc.left
    }
}

/// 最大の幅を越える行を、入る文字数ごとに割る（入る数は字体で測る）。
fn break_long_lines(
    hwnd: HWND,
    font: HFONT,
    text: &str,
    max_width: i32,
) -> Result<String, TooltipOsError> {
    // SAFETY: Win32 境界。自分の窓の DC を借り、字体を選んで測り、元に戻して返す。
    let hdc = unsafe { GetDC(Some(hwnd)) };
    if hdc.is_invalid() {
        return Err(TooltipOsError::Show { stage: "measure" });
    }
    // SAFETY: 同上。
    let old = unsafe { SelectObject(hdc, font.into()) };
    let failed = Cell::new(false);
    let fit = |line: &str| {
        let wide: Vec<u16> = line.encode_utf16().collect();
        let mut fit_units = 0i32;
        let mut extent = SIZE::default();
        // SAFETY: Win32 境界。手元の UTF-16 の並びを長さ付きで渡す。
        let ok = unsafe {
            GetTextExtentExPointW(
                hdc,
                PCWSTR(wide.as_ptr()),
                wide.len() as i32,
                max_width,
                Some(&mut fit_units),
                None,
                &mut extent,
            )
        };
        if ok.as_bool() {
            chars_within_utf16(line, fit_units.max(0) as usize)
        } else {
            failed.set(true);
            usize::MAX
        }
    };
    let broken = force_break(text, &fit);
    // SAFETY: 同上。
    unsafe {
        SelectObject(hdc, old);
        ReleaseDC(Some(hwnd), hdc);
    }
    if failed.get() {
        return Err(TooltipOsError::Show { stage: "measure" });
    }
    Ok(broken)
}

/// 点のある画面の作業領域と DPI。読めなければ None。
fn monitor_at(p: PointPx) -> Option<(RectPx, u32)> {
    // SAFETY: Win32 境界。いちばん近い画面を引き、手元の構造体に情報を書かせる。
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: p.x, y: p.y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut mi).as_bool() {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).ok()?;
        let wa = mi.rcWork;
        Some((
            RectPx {
                left: wa.left,
                top: wa.top,
                right: wa.right,
                bottom: wa.bottom,
            },
            dpi_x,
        ))
    }
}

/// 論理の長さを DPI の物理ピクセルへ（四捨五入）。
fn scale(logical: i32, dpi: u32) -> i32 {
    ((i64::from(logical) * i64::from(dpi) + 48) / 96) as i32
}

/// 2 つの座標を LPARAM の下位・上位の 16 ビットに詰める（負の座標は 2 の補数のまま）。
fn make_lparam(x: i32, y: i32) -> isize {
    ((u32::from(y as u16) << 16) | u32::from(x as u16)) as isize
}

#[cfg(test)]
#[path = "os_tests.rs"]
mod os_tests;
