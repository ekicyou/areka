#![allow(non_snake_case)]

use crate::api::*;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use windows::core::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WinStyle {
    pub(crate) style: WINDOW_STYLE,
    pub(crate) ex_style: WINDOW_EX_STYLE,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) parent: Option<HWND>,
}

impl WinStyle {
    //================================================================================
    // コミット
    //================================================================================

    /// SetWindowLongWへ反映
    pub fn commit(&self, hwnd: HWND) -> Result<()> {
        set_window_long_ptr(hwnd, GWL_STYLE, self.style.0 as _)?;
        set_window_long_ptr(hwnd, GWL_EXSTYLE, self.ex_style.0 as _)?;
        Ok(())
    }

    //================================================================================
    // コンストラクタ
    //================================================================================

    /// hwndから現在のスタイルを取得して生成します
    pub fn new(hwnd: HWND) -> Result<Self> {
        let style = WINDOW_STYLE(get_window_long_ptr(hwnd, GWL_STYLE)? as _);
        let ex_style = WINDOW_EX_STYLE(get_window_long_ptr(hwnd, GWL_EXSTYLE)? as _);
        Ok(Self {
            style,
            ex_style,
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT,
            parent: None,
        })
    }

    #[inline(always)]
    fn with_style(style: WINDOW_STYLE) -> WinStyle {
        WinStyle {
            style,
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT,
            parent: None,
            ..Default::default()
        }
    }

    //================================================================================
    // 座標値、親ウィンドウ
    //================================================================================

    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn size(mut self, width: i32, height: i32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn parent(mut self, parent: HWND) -> Self {
        self.parent = Some(parent);
        self
    }

    //================================================================================
    // 複数ビットがONの WINDOW_STYLE (複合フラグ)
    //================================================================================

    /// 標準的なトップレベルウィンドウを作成するための複合スタイルです。`WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX` の組み合わせです。
    pub fn WS_OVERLAPPEDWINDOW() -> Self {
        WinStyle::with_style(WS_OVERLAPPEDWINDOW)
    }

    /// `WS_OVERLAPPEDWINDOW` と同じです。
    pub fn WS_TILEDWINDOW() -> Self {
        WinStyle::with_style(WS_TILEDWINDOW)
    }

    /// 標準的なポップアップウィンドウを作成するための複合スタイルです。`WS_POPUP | WS_BORDER | WS_SYSMENU` の組み合わせです。
    pub fn WS_POPUPWINDOW() -> Self {
        WinStyle::with_style(WS_POPUPWINDOW)
    }

    //================================================================================
    // 1ビットだけONの WINDOW_STYLE (ウィンドウ基本属性)
    //================================================================================

    /// オーバーラップウィンドウを作成します。オーバーラップウィンドウは、タイトルバーと境界線を持つトップレベルウィンドウです。`WS_TILED` と同じです。
    pub fn WS_OVERLAPPED() -> Self {
        WinStyle::with_style(WS_OVERLAPPED)
    }

    /// ポップアップウィンドウを作成します。`WS_CHILD` とは併用できません。
    pub fn WS_POPUP() -> Self {
        WinStyle::with_style(WS_POPUP)
    }

    //================================================================================
    // 1ビットだけONの WINDOW_STYLE (その他の属性)
    //================================================================================

    /// ウィンドウのタイトルバーに最大化ボタンを表示します。`WS_SYSMENU` スタイルも指定する必要があります。
    pub fn WS_MAXIMIZEBOX(self, flag: bool) -> Self {
        set_style(self, WS_MAXIMIZEBOX, flag)
    }

    /// ウィンドウのタイトルバーに最小化ボタンを表示します。`WS_SYSMENU` スタイルも指定する必要があります。
    pub fn WS_MINIMIZEBOX(self, flag: bool) -> Self {
        set_style(self, WS_MINIMIZEBOX, flag)
    }

    /// サイズ変更可能な境界線を持つウィンドウを作成します。`WS_THICKFRAME` と同じです。
    pub fn WS_SIZEBOX(self, flag: bool) -> Self {
        set_style(self, WS_SIZEBOX, flag)
    }

    /// サイズ変更可能な境界線を持つウィンドウを作成します。`WS_SIZEBOX` と同じです。
    pub fn WS_THICKFRAME(self, flag: bool) -> Self {
        set_style(self, WS_THICKFRAME, flag)
    }

    /// ウィンドウにタイトルバーを追加または削除します。
    pub fn WS_CAPTION(self, flag: bool) -> Self {
        set_style(self, WS_CAPTION, flag)
    }

    /// ウィンドウのタイトルバーにシステムメニューを表示します。`WS_CAPTION` スタイルも指定する必要があります。
    pub fn WS_SYSMENU(self, flag: bool) -> Self {
        set_style(self, WS_SYSMENU, flag)
    }

    /// 水平スクロールバーを持つウィンドウを作成します。
    pub fn WS_HSCROLL(self, flag: bool) -> Self {
        set_style(self, WS_HSCROLL, flag)
    }

    /// 垂直スクロールバーを持つウィンドウを作成します。
    pub fn WS_VSCROLL(self, flag: bool) -> Self {
        set_style(self, WS_VSCROLL, flag)
    }

    /// タイトルバーを持たないダイアログボックス形式の境界線を持つウィンドウを作成します。
    pub fn WS_DLGFRAME(self, flag: bool) -> Self {
        set_style(self, WS_DLGFRAME, flag)
    }

    /// 細い線の境界線を持つウィンドウを作成します。
    pub fn WS_BORDER(self, flag: bool) -> Self {
        set_style(self, WS_BORDER, flag)
    }

    /// ウィンドウを最大化状態で作成します。
    pub fn WS_MAXIMIZE(self, flag: bool) -> Self {
        set_style(self, WS_MAXIMIZE, flag)
    }

    /// 親ウィンドウのクライアント領域内で子ウィンドウが描画される領域をクリップします。親ウィンドウが描画されるときに、子ウィンドウによって隠されている領域の描画を除外します。
    pub fn WS_CLIPCHILDREN(self, flag: bool) -> Self {
        set_style(self, WS_CLIPCHILDREN, flag)
    }

    /// 兄弟関係にある子ウィンドウ同士の描画領域をクリップします。特定の子ウィンドウが再描画される必要がある場合、他の子ウィンドウの領域には描画されません。
    pub fn WS_CLIPSIBLINGS(self, flag: bool) -> Self {
        set_style(self, WS_CLIPSIBLINGS, flag)
    }

    /// ウィンドウを無効状態で作成します。無効化されたウィンドウはユーザーからの入力を受け付けません。
    pub fn WS_DISABLED(self, flag: bool) -> Self {
        set_style(self, WS_DISABLED, flag)
    }

    /// ウィンドウを可視状態で作成します。このスタイルが指定されない場合、ウィンドウは非表示になります。
    pub fn WS_VISIBLE(self, flag: bool) -> Self {
        set_style(self, WS_VISIBLE, flag)
    }

    /// ウィンドウを最小化状態で作成します。`WS_MINIMIZE` と同じです。
    pub fn WS_ICONIC(self, flag: bool) -> Self {
        set_style(self, WS_ICONIC, flag)
    }

    /// ウィンドウを最小化状態で作成します。`WS_ICONIC` と同じです。
    pub fn WS_MINIMIZE(self, flag: bool) -> Self {
        set_style(self, WS_MINIMIZE, flag)
    }

    //================================================================================
    // WINDOW_EX_STYLE: 複数ビットが立っているフラグ (組み合わせ)
    //================================================================================

    /// ウィンドウに立体的な境界線を作成します。
    /// WS_EX_WINDOWEDGE (0x100) の単独ビットです。
    pub fn WS_EX_WINDOWEDGE(self) -> Self {
        set_ex(self, WS_EX_WINDOWEDGE, true)
    }

    /// オーバーラップウィンドウを作成します。
    /// WS_EX_WINDOWEDGE (0x100) と WS_EX_CLIENTEDGE (0x200) の組み合わせです。
    pub fn WS_EX_OVERLAPPEDWINDOW(self) -> Self {
        set_ex(self, WS_EX_OVERLAPPEDWINDOW, true)
    }

    /// パレットウィンドウを作成します。これは、WS_EX_WINDOWEDGE、WS_EX_TOOLWINDOW、WS_EX_TOPMOSTを組み合わせたものです。
    /// WS_EX_WINDOWEDGE (0x100) | WS_EX_TOOLWINDOW (0x80) | WS_EX_TOPMOST (0x8) の組み合わせです。
    pub fn WS_EX_PALETTEWINDOW(self) -> Self {
        set_ex(self, WS_EX_PALETTEWINDOW, true)
    }

    //================================================================================
    // WINDOW_EX_STYLE: 1ビットだけが立っているフラグ (単一機能)
    //================================================================================

    /// ウィンドウにモーダルダイアログボックスのような二重の境界線を作成します。
    pub fn WS_EX_DLGMODALFRAME(self, flag: bool) -> Self {
        set_ex(self, WS_EX_DLGMODALFRAME, flag)
    }

    /// 子ウィンドウが作成または破棄されたときに、その親ウィンドウにWM_PARENTNOTIFYメッセージを送信しないように指定します。
    pub fn WS_EX_NOPARENTNOTIFY(self, flag: bool) -> Self {
        set_ex(self, WS_EX_NOPARENTNOTIFY, flag)
    }

    /// ウィンドウを最前面ウィンドウとして作成します。システムは、非最前面ウィンドウの上にこのウィンドウを配置します。
    pub fn WS_EX_TOPMOST(self, flag: bool) -> Self {
        set_ex(self, WS_EX_TOPMOST, flag)
    }

    /// ウィンドウを透明として作成します。このウィンドウの下にある兄弟ウィンドウが描画されるまで、このウィンドウは描画されません。
    pub fn WS_EX_TRANSPARENT(self, flag: bool) -> Self {
        set_ex(self, WS_EX_TRANSPARENT, flag)
    }

    /// MDI（Multiple Document Interface）の子ウィンドウを作成します。
    pub fn WS_EX_MDICHILD(self, flag: bool) -> Self {
        set_ex(self, WS_EX_MDICHILD, flag)
    }

    /// フローティングツールバーとして使用されるツールウィンドウを作成します。
    pub fn WS_EX_TOOLWINDOW(self, flag: bool) -> Self {
        set_ex(self, WS_EX_TOOLWINDOW, flag)
    }

    /// ウィンドウに立体的な境界線を持つ、沈んだ外観を与えます。
    pub fn WS_EX_CLIENTEDGE(self, flag: bool) -> Self {
        set_ex(self, WS_EX_CLIENTEDGE, flag)
    }

    /// ウィンドウのタイトルバーに疑問符（？）のコンテキストヘルプボタンを追加します。
    pub fn WS_EX_CONTEXTHELP(self, flag: bool) -> Self {
        set_ex(self, WS_EX_CONTEXTHELP, flag)
    }

    /// Tabキーを使用してユーザーがコントロール間を移動できる子ウィンドウを持つウィンドウ（ダイアログボックスなど）を作成します。
    pub fn WS_EX_CONTROLPARENT(self, flag: bool) -> Self {
        set_ex(self, WS_EX_CONTROLPARENT, flag)
    }

    /// アイテムをドラッグアンドドロップで受け入れないウィンドウに、立体的な境界線スタイルを与えます。
    pub fn WS_EX_STATICEDGE(self, flag: bool) -> Self {
        set_ex(self, WS_EX_STATICEDGE, flag)
    }

    /// トップレベルウィンドウをタスクバーに表示します。
    pub fn WS_EX_APPWINDOW(self, flag: bool) -> Self {
        set_ex(self, WS_EX_APPWINDOW, flag)
    }

    /// レイヤードウィンドウを作成します。このスタイルは、ウィンドウ作成時またはSetWindowLong/Ex関数で設定する必要があります。
    pub fn WS_EX_LAYERED(self, flag: bool) -> Self {
        set_ex(self, WS_EX_LAYERED, flag)
    }

    /// 子ウィンドウが親ウィンドウのレイアウト（RTLまたはLTR）を継承しないようにします。
    pub fn WS_EX_NOINHERITLAYOUT(self, flag: bool) -> Self {
        set_ex(self, WS_EX_NOINHERITLAYOUT, flag)
    }

    /// このウィンドウとその子ウィンドウの描画を、画面外のビットマップにリダイレクトしません。
    pub fn WS_EX_NOREDIRECTIONBITMAP(self, flag: bool) -> Self {
        set_ex(self, WS_EX_NOREDIRECTIONBITMAP, flag)
    }

    /// ウィンドウをコンポジットウィンドウとして作成します。
    pub fn WS_EX_COMPOSITED(self, flag: bool) -> Self {
        set_ex(self, WS_EX_COMPOSITED, flag)
    }

    /// このスタイルで作成されたウィンドウは、ユーザーがクリックしてもフォアグラウンドウィンドウになりません。
    pub fn WS_EX_NOACTIVATE(self, flag: bool) -> Self {
        set_ex(self, WS_EX_NOACTIVATE, flag)
    }

    //================================================================================
    // WINDOW_EX_STYLE: ON/OFF関係
    //================================================================================

    /// ウィンドウは右側に垂直スクロールバーを持ちます（存在する場合）。これは既定値です。
    pub fn WS_EX_RIGHTSCROLLBAR(self) -> Self {
        set_ex(self, WS_EX_LEFTSCROLLBAR, false)
    }
    /// 垂直スクロールバーをウィンドウの左端に配置します。
    pub fn WS_EX_LEFTSCROLLBAR(self) -> Self {
        set_ex(self, WS_EX_LEFTSCROLLBAR, true)
    }

    /// ウィンドウは左揃えのプロパティを持ちます。これは既定値です。
    pub fn WS_EX_LEFT(self) -> Self {
        set_ex(self, WS_EX_RIGHT, false)
    }
    /// ウィンドウ自体が右揃えのプロパティを持ちます。
    pub fn WS_EX_RIGHT(self) -> Self {
        set_ex(self, WS_EX_RIGHT, true)
    }

    /// ウィンドウは左から右（LTR）への読み取り順序のプロパティを持ちます。これは既定値です。
    pub fn WS_EX_LTRREADING(self) -> Self {
        set_ex(self, WS_EX_RTLREADING, false)
    }
    /// ウィンドウのテキストを右から左（RTL）の読み取り順序で表示します。
    pub fn WS_EX_RTLREADING(self) -> Self {
        set_ex(self, WS_EX_RTLREADING, true)
    }

    /// ウィンドウのレイアウトを左から右（LTR）にします。これは既定値です。
    pub fn WS_EX_LAYOUTLTR(self) -> Self {
        set_ex(self, WS_EX_LAYOUTRTL, false)
    }
    /// ウィンドウのレイアウトを右から左（RTL）にします。
    pub fn WS_EX_LAYOUTRTL(self) -> Self {
        set_ex(self, WS_EX_LAYOUTRTL, true)
    }
}

//================================================================================
// クリック透過（WS_EX_TRANSPARENT 動的トグル）
//================================================================================

/// 対象 HWND の `WS_EX_TRANSPARENT` を `transparent` フラグに一致させ、`FRAMECHANGED` で反映する。
///
/// 現在の ex-style を読み戻し、`WS_EX_TRANSPARENT` ビットのみを set/clear して
/// `SetWindowLongPtr(GWL_EXSTYLE)` で書き戻し、`SetWindowPos(SWP_FRAMECHANGED | SWP_NOMOVE |
/// SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE)` で非クライアント領域へ反映する
/// (`apply_initial_state` のレシピ準拠)。他の ex-style ビット（`WS_EX_LAYERED` 等）には触れない。
pub fn apply_click_through(hwnd: HWND, transparent: bool) -> Result<()> {
    // 現在の ex-style を読み、TRANSPARENT ビットのみを目標へ一致させる。
    let current = get_window_long_ptr(hwnd, GWL_EXSTYLE)?;
    let mask = WS_EX_TRANSPARENT.0 as isize;
    let next = if transparent {
        current | mask
    } else {
        current & !mask
    };
    set_window_long_ptr(hwnd, GWL_EXSTYLE, next)?;

    // スタイル変更を非クライアント領域へ反映する。位置・サイズ・z オーダー・アクティブ化は
    // 変えず（NOMOVE/NOSIZE/NOZORDER/NOACTIVATE）、フレーム再計算（FRAMECHANGED）のみ要求する。
    // SAFETY: Win32 境界。SetWindowPos に安全ラッパは無いため直接呼ぶ。所有権・寿命に影響しない。
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}

/// 対象 HWND に `WS_EX_LAYERED` を同伴フラグとして立てる（クリック透過の必須条件）。
///
/// pilot 実証（`crates/pilot/examples/pilot-clickthrough-alpha-toggle/REPORT.md`）:
/// DComp（`WS_EX_NOREDIRECTIONBITMAP`）窓では `WS_EX_TRANSPARENT` 単独ではマウス透過が
/// 効かず窓が全クリックを吸う。`WS_EX_LAYERED` をフラグとして併設して初めて別プロセス
/// 透過が成立し、DComp 描画は LAYERED 併設でも消えず共存する（実測 ex_style 0x280028）。
///
/// 本関数は LAYERED ビットを **立てるのみ**（落とさない・冪等）で、レイヤード描画
/// （`UpdateLayeredWindow`／`SetLayeredWindowAttributes`）は呼ばない＝当たり判定を
/// 効かせる同伴フラグ用途に限定する（R6.2）。他の ex-style ビットには触れない。
/// 既に立っている場合は Win32 呼び出しを行わず `Ok(())` を返す。
pub fn apply_layered_companion(hwnd: HWND) -> Result<()> {
    let current = get_window_long_ptr(hwnd, GWL_EXSTYLE)?;
    let mask = WS_EX_LAYERED.0 as isize;
    if current & mask != 0 {
        // 既に同伴フラグ済み（冪等・無駄な FRAMECHANGED を撃たない）。
        return Ok(());
    }
    set_window_long_ptr(hwnd, GWL_EXSTYLE, current | mask)?;

    // スタイル変更の反映レシピは apply_click_through と同一（FRAMECHANGED のみ要求）。
    // SAFETY: Win32 境界。SetWindowPos に安全ラッパは無いため直接呼ぶ。所有権・寿命に影響しない。
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}

#[inline(always)]
fn set_style(src: WinStyle, style: WINDOW_STYLE, flag: bool) -> WinStyle {
    let org = src.style.0;
    let style = style.0;
    let style = if flag { org | style } else { org & !style };
    let style = WINDOW_STYLE(style);
    WinStyle { style, ..src }
}

#[inline(always)]
fn set_ex(src: WinStyle, ex_style: WINDOW_EX_STYLE, flag: bool) -> WinStyle {
    let org = src.ex_style.0;
    let ex_style = ex_style.0;
    let ex_style = if flag {
        org | ex_style
    } else {
        org & !ex_style
    };
    let ex_style = WINDOW_EX_STYLE(ex_style);
    WinStyle { ex_style, ..src }
}

#[cfg(test)]
mod tests {
    use super::*;

    //================================================================================
    // コンストラクタ
    //================================================================================

    #[test]
    fn default_is_all_zero() {
        let s = WinStyle::default();
        assert_eq!(s.style, WINDOW_STYLE(0));
        assert_eq!(s.ex_style, WINDOW_EX_STYLE(0));
        // 特性化: default() の座標は 0（with_style 系コンストラクタの CW_USEDEFAULT とは異なる）
        assert_eq!((s.x, s.y, s.width, s.height), (0, 0, 0, 0));
        assert_eq!(s.parent, None);
    }

    #[test]
    fn ws_overlappedwindow_sets_composite_bits_and_default_geometry() {
        let s = WinStyle::WS_OVERLAPPEDWINDOW();
        assert_eq!(s.style, WS_OVERLAPPEDWINDOW);
        assert_eq!(
            s.style.0,
            WS_OVERLAPPED.0
                | WS_CAPTION.0
                | WS_SYSMENU.0
                | WS_THICKFRAME.0
                | WS_MINIMIZEBOX.0
                | WS_MAXIMIZEBOX.0
        );
        assert_eq!(s.ex_style, WINDOW_EX_STYLE(0));
        assert_eq!(
            (s.x, s.y, s.width, s.height),
            (CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT)
        );
        assert_eq!(s.parent, None);
    }

    #[test]
    fn ws_tiledwindow_equals_ws_overlappedwindow() {
        assert_eq!(WinStyle::WS_TILEDWINDOW(), WinStyle::WS_OVERLAPPEDWINDOW());
    }

    #[test]
    fn ws_popupwindow_is_popup_border_sysmenu() {
        let s = WinStyle::WS_POPUPWINDOW();
        assert_eq!(s.style, WS_POPUPWINDOW);
        assert_eq!(s.style.0, WS_POPUP.0 | WS_BORDER.0 | WS_SYSMENU.0);
    }

    #[test]
    fn ws_overlapped_constructor_is_zero_style_with_default_geometry() {
        let s = WinStyle::WS_OVERLAPPED();
        // WS_OVERLAPPED 定数は 0（スタイルビットなし）だが座標は CW_USEDEFAULT
        assert_eq!(s.style, WINDOW_STYLE(0));
        assert_eq!(s.x, CW_USEDEFAULT);
    }

    #[test]
    fn ws_popup_constructor_sets_single_bit() {
        assert_eq!(WinStyle::WS_POPUP().style, WS_POPUP);
    }

    //================================================================================
    // 座標値、親ウィンドウ
    //================================================================================

    #[test]
    fn position_size_parent_builders_update_only_their_fields() {
        let parent = HWND(0x1234 as *mut _);
        let s = WinStyle::WS_OVERLAPPEDWINDOW()
            .position(10, -20)
            .size(640, 480)
            .parent(parent);
        assert_eq!((s.x, s.y), (10, -20));
        assert_eq!((s.width, s.height), (640, 480));
        assert_eq!(s.parent, Some(parent));
        // スタイルビットは座標・親設定で変化しない
        assert_eq!(s.style, WS_OVERLAPPEDWINDOW);
        assert_eq!(s.ex_style, WINDOW_EX_STYLE(0));
    }

    //================================================================================
    // WINDOW_STYLE フラグの ON/OFF（set_style 経路）
    //================================================================================

    #[test]
    fn style_flag_on_off_roundtrip_preserves_other_bits() {
        let base = WinStyle::WS_OVERLAPPEDWINDOW();
        let on = base.WS_VISIBLE(true);
        assert_eq!(on.style.0, WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0);
        let off = on.WS_VISIBLE(false);
        assert_eq!(off.style, WS_OVERLAPPEDWINDOW);
    }

    #[test]
    fn style_flag_off_clears_only_target_bits() {
        let s = WinStyle::WS_OVERLAPPEDWINDOW().WS_THICKFRAME(false);
        assert_eq!(s.style.0, WS_OVERLAPPEDWINDOW.0 & !WS_THICKFRAME.0);
    }

    #[test]
    fn ws_caption_off_clears_border_and_dlgframe_bits() {
        // WS_CAPTION は WS_BORDER | WS_DLGFRAME の複合ビット。OFF は両ビットを同時に落とす
        let s = WinStyle::default()
            .WS_BORDER(true)
            .WS_DLGFRAME(true)
            .WS_CAPTION(false);
        assert_eq!(s.style, WINDOW_STYLE(0));
    }

    #[test]
    fn sizebox_and_thickframe_are_aliases() {
        assert_eq!(
            WinStyle::default().WS_SIZEBOX(true),
            WinStyle::default().WS_THICKFRAME(true)
        );
    }

    #[test]
    fn iconic_and_minimize_are_aliases() {
        assert_eq!(
            WinStyle::default().WS_ICONIC(true),
            WinStyle::default().WS_MINIMIZE(true)
        );
    }

    #[test]
    fn style_and_ex_style_setters_are_independent() {
        let s = WinStyle::default().WS_EX_TOPMOST(true).WS_VISIBLE(true);
        assert_eq!(s.style, WS_VISIBLE);
        assert_eq!(s.ex_style, WS_EX_TOPMOST);
    }

    //================================================================================
    // WINDOW_EX_STYLE フラグの ON/OFF（set_ex 経路）
    //================================================================================

    #[test]
    fn ex_flag_on_off_roundtrip_preserves_other_bits() {
        let on = WinStyle::default().WS_EX_TOPMOST(true).WS_EX_LAYERED(true);
        assert_eq!(on.ex_style.0, WS_EX_TOPMOST.0 | WS_EX_LAYERED.0);
        let off = on.WS_EX_TOPMOST(false);
        assert_eq!(off.ex_style, WS_EX_LAYERED);
    }

    #[test]
    fn ws_ex_overlappedwindow_is_windowedge_plus_clientedge() {
        let s = WinStyle::default().WS_EX_OVERLAPPEDWINDOW();
        assert_eq!(s.ex_style, WS_EX_OVERLAPPEDWINDOW);
        assert_eq!(s.ex_style.0, WS_EX_WINDOWEDGE.0 | WS_EX_CLIENTEDGE.0);
    }

    #[test]
    fn ws_ex_palettewindow_is_windowedge_toolwindow_topmost() {
        let s = WinStyle::default().WS_EX_PALETTEWINDOW();
        assert_eq!(s.ex_style, WS_EX_PALETTEWINDOW);
        assert_eq!(
            s.ex_style.0,
            WS_EX_WINDOWEDGE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0
        );
    }

    #[test]
    fn ws_ex_windowedge_sets_only_windowedge_bit() {
        // WS_EX_WINDOWEDGE 定数は 0x100 の単独ビットであり CLIENTEDGE は立たない
        let s = WinStyle::default().WS_EX_WINDOWEDGE();
        assert_eq!(s.ex_style, WS_EX_WINDOWEDGE);
        assert_eq!(s.ex_style.0 & WS_EX_CLIENTEDGE.0, 0);
    }

    //================================================================================
    // WINDOW_EX_STYLE: ON/OFF 対（既定値へ戻す系）
    //================================================================================

    #[test]
    fn rightscrollbar_clears_leftscrollbar_bit() {
        let s = WinStyle::default()
            .WS_EX_LEFTSCROLLBAR()
            .WS_EX_RIGHTSCROLLBAR();
        assert_eq!(s.ex_style, WINDOW_EX_STYLE(0));
    }

    #[test]
    fn left_clears_right_bit() {
        let s = WinStyle::default().WS_EX_RIGHT().WS_EX_LEFT();
        assert_eq!(s.ex_style, WINDOW_EX_STYLE(0));
    }

    #[test]
    fn ltrreading_clears_rtlreading_bit() {
        let s = WinStyle::default().WS_EX_RTLREADING().WS_EX_LTRREADING();
        assert_eq!(s.ex_style, WINDOW_EX_STYLE(0));
    }

    #[test]
    fn layoutltr_clears_layoutrtl_bit() {
        let s = WinStyle::default().WS_EX_LAYOUTRTL().WS_EX_LAYOUTLTR();
        assert_eq!(s.ex_style, WINDOW_EX_STYLE(0));
    }

    //================================================================================
    // Win32 API 依存経路（GUI 非依存のエラー伝播のみ検証）
    //================================================================================

    #[test]
    fn new_with_null_hwnd_returns_err() {
        // 無効 HWND では GetWindowLongPtrW が失敗し Err が伝播する（ウィンドウ生成不要）
        assert!(WinStyle::new(HWND::default()).is_err());
    }

    #[test]
    fn commit_to_null_hwnd_returns_err() {
        assert!(
            WinStyle::WS_OVERLAPPEDWINDOW()
                .commit(HWND::default())
                .is_err()
        );
    }

    //================================================================================
    // apply_click_through: WS_EX_TRANSPARENT 動的トグル
    //================================================================================

    #[test]
    fn apply_click_through_toggles_only_transparent_bit() {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WINDOW_STYLE,
        };
        use windows::core::w;

        // 実所有 HWND を用意（"Static" 定義済クラス・非表示ポップアップ）。
        // 初期 ex-style に WS_EX_TRANSPARENT 以外のビット（TOOLWINDOW/NOREDIRECTIONBITMAP）を
        // 立てておき、トグル後に「TRANSPARENT のみが変化し他ビットは保存」を検証する。
        let initial_ex = WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP;

        // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ポップアップを生成する。
        // hinstance は現行モジュール。失敗時は hwnd が無効となるため下で検証する。
        let hwnd = unsafe {
            let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
            CreateWindowExW(
                initial_ex,
                w!("Static"),
                w!("wintf-click-through-test"),
                WINDOW_STYLE(WS_POPUP.0),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .expect("CreateWindowExW should create a hidden test window");

        // テスト本体を closure に包み、途中で panic しても必ず DestroyWindow する。
        let result = std::panic::catch_unwind(|| {
            let before =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style before") as u32;
            // 前提: 初期状態で TRANSPARENT は立っておらず、他ビットは立っている
            assert_eq!(
                before & WS_EX_TRANSPARENT.0,
                0,
                "TRANSPARENT must start clear"
            );
            let non_transparent_before = before & !WS_EX_TRANSPARENT.0;

            // ON: TRANSPARENT を立てる
            apply_click_through(hwnd, true).expect("apply true");
            let on = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style on") as u32;
            assert_ne!(
                on & WS_EX_TRANSPARENT.0,
                0,
                "TRANSPARENT must be set after true"
            );
            assert_eq!(
                on & !WS_EX_TRANSPARENT.0,
                non_transparent_before,
                "non-TRANSPARENT bits must be preserved after ON"
            );

            // OFF: TRANSPARENT を落とす
            apply_click_through(hwnd, false).expect("apply false");
            let off = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style off") as u32;
            assert_eq!(
                off & WS_EX_TRANSPARENT.0,
                0,
                "TRANSPARENT must be clear after false"
            );
            assert_eq!(
                off & !WS_EX_TRANSPARENT.0,
                non_transparent_before,
                "non-TRANSPARENT bits must be preserved after OFF"
            );

            // 冪等: 既に OFF の状態で false を再適用しても不変
            apply_click_through(hwnd, false).expect("apply false idempotent");
            let off2 = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style off2") as u32;
            assert_eq!(off2, off, "idempotent OFF must not change ex-style");
        });

        // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する（テスト後始末）。
        unsafe {
            let _ = DestroyWindow(hwnd);
        }

        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    #[test]
    fn apply_click_through_null_hwnd_returns_err() {
        // 無効 HWND では GetWindowLongPtrW が失敗し Err が伝播する（ウィンドウ生成不要）
        assert!(apply_click_through(HWND::default(), true).is_err());
    }

    //================================================================================
    // apply_layered_companion: WS_EX_LAYERED 同伴フラグ（pilot 必須条件）
    //================================================================================

    #[test]
    fn apply_layered_companion_sets_layered_and_preserves_other_bits() {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WINDOW_STYLE,
        };
        use windows::core::w;

        // LAYERED 以外のビット（TOOLWINDOW/NOREDIRECTIONBITMAP）を立てた実所有 HWND を用意し、
        // 同伴フラグ適用後に「LAYERED のみが加わり他ビットは保存」を検証する。
        let initial_ex = WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP;

        // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ポップアップを生成する。
        let hwnd = unsafe {
            let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
            CreateWindowExW(
                initial_ex,
                w!("Static"),
                w!("wintf-layered-companion-test"),
                WINDOW_STYLE(WS_POPUP.0),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .expect("CreateWindowExW should create a hidden test window");

        let result = std::panic::catch_unwind(|| {
            let before =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style before") as u32;
            assert_eq!(before & WS_EX_LAYERED.0, 0, "LAYERED must start clear");
            let non_layered_before = before & !WS_EX_LAYERED.0;

            // 同伴フラグ適用: LAYERED が立つ。
            apply_layered_companion(hwnd).expect("apply companion");
            let on = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style on") as u32;
            assert_ne!(
                on & WS_EX_LAYERED.0,
                0,
                "LAYERED must be set after companion"
            );
            assert_eq!(
                on & !WS_EX_LAYERED.0,
                non_layered_before,
                "non-LAYERED bits must be preserved"
            );

            // 冪等: 再適用しても不変（既フラグ時は API を撃たない early-return 経路）。
            apply_layered_companion(hwnd).expect("apply companion idempotent");
            let on2 = get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style on2") as u32;
            assert_eq!(on2, on, "idempotent companion must not change ex-style");

            // TRANSPARENT トグルと共存: トグル後も LAYERED は保持される。
            apply_click_through(hwnd, true).expect("toggle on");
            let toggled =
                get_window_long_ptr(hwnd, GWL_EXSTYLE).expect("read ex-style toggled") as u32;
            assert_ne!(
                toggled & WS_EX_LAYERED.0,
                0,
                "LAYERED must survive TRANSPARENT toggle"
            );
            assert_ne!(toggled & WS_EX_TRANSPARENT.0, 0, "TRANSPARENT must be set");
        });

        // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する（テスト後始末）。
        unsafe {
            let _ = DestroyWindow(hwnd);
        }

        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    #[test]
    fn apply_layered_companion_null_hwnd_returns_err() {
        // 無効 HWND では GetWindowLongPtrW が失敗し Err が伝播する（ウィンドウ生成不要）
        assert!(apply_layered_companion(HWND::default()).is_err());
    }
}
