use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use windows::core::*;

/// GetWindowLongPtrWのラッパー
#[inline(always)]
pub(crate) fn get_window_long_ptr(hwnd: HWND, index: WINDOW_LONG_PTR_INDEX) -> Result<isize> {
    unsafe {
        SetLastError(ERROR_SUCCESS);
        let res = GetWindowLongPtrW(hwnd, index);
        let err = Error::from_thread();
        if err.code() != S_OK {
            return Err(err);
        }
        Ok(res)
    }
}

/// SetWindowLongPtrWのラッパー
#[inline(always)]
pub(crate) fn set_window_long_ptr(
    hwnd: HWND,
    index: WINDOW_LONG_PTR_INDEX,
    value: isize,
) -> Result<isize> {
    unsafe {
        SetLastError(ERROR_SUCCESS);
        let res = SetWindowLongPtrW(hwnd, index, value);
        if res == 0 {
            let err = Error::from_thread();
            if err.code() != S_OK {
                return Err(err);
            }
        }
        Ok(res)
    }
}

/// GetWindow で z オーダー上の隣接窓を得る共通実装。
///
/// 隣接窓が無いこと（z オーダーの端に居る）は**失敗ではなく正常な不在**であり、
/// `Ok(None)` として返す。`windows` crate の `GetWindow` は戻り値 HWND が NULL の
/// 時点で一律 `Err(Error::from_thread())` にするため、そのままでは不在と真の失敗を
/// 区別できない。呼び出し前に `SetLastError(ERROR_SUCCESS)` でスレッドの残留エラーを
/// 消し、`Err` のコードが `S_OK`（＝API がエラーを立てていない＝隣が無いだけ）なら
/// `Ok(None)`、それ以外は失敗として `Err` を呼び出し側へ返す（握り潰さない）。
#[inline(always)]
fn get_window_relative(hwnd: HWND, ucmd: GET_WINDOW_CMD) -> Result<Option<HWND>> {
    unsafe {
        SetLastError(ERROR_SUCCESS);
        match GetWindow(hwnd, ucmd) {
            Ok(found) => Ok(Some(found)),
            Err(err) if err.code() == S_OK => Ok(None),
            Err(err) => Err(err),
        }
    }
}

/// 指定窓の 1 つ手前（z オーダー上でより前面側）にある窓を返す。
///
/// Win32 の z オーダー語彙では `GW_HWNDPREV` が「前面側の隣」を、`GW_HWNDNEXT` が
/// 「背面側の隣」を指す（"PREV/NEXT" は列挙順であって前後関係の直感とは逆になる）。
/// 「バルーンはキャラのすぐ手前」という本フィーチャーの不変条件判定はこの対応関係の
/// 上に立つため、名前と定数の対応をここで固定する。
///
/// 最前面でこれ以上手前が無い場合は `Ok(None)`。失敗は `Err` で返し、記録は呼び出し側が行う。
#[inline(always)]
pub(crate) fn get_window_above(hwnd: HWND) -> Result<Option<HWND>> {
    get_window_relative(hwnd, GW_HWNDPREV)
}

/// 指定窓の 1 つ背後（z オーダー上でより背面側）にある窓を返す。
///
/// 対応関係は [`get_window_above`] の doc を参照（背面側＝`GW_HWNDNEXT`）。
/// 最背面でこれ以上背後が無い場合は `Ok(None)`。失敗は `Err` で返す。
#[inline(always)]
pub(crate) fn get_window_below(hwnd: HWND) -> Result<Option<HWND>> {
    get_window_relative(hwnd, GW_HWNDNEXT)
}

/// 現在の前面窓（フォアグラウンドウィンドウ）を返す。
///
/// 前面窓が無い状態——他プロセスが前面を手放した直後や、前面窓が別のデスクトップに居る場合
/// ——では OS が NULL を返す。これは**失敗ではなく正常な不在**であり `None` になる。
/// `Result` で包まないのは、`GetForegroundWindow` が失敗そのものを持たない API だからである
/// （不在と失敗の区別が存在しないため、`Err` を作ると呼び出し側が偽の失敗を見ることになる）。
///
/// 読み取り専用であり、前面窓を変えることは決してない（要件 4.1／4.3）。
#[inline(always)]
pub(crate) fn get_foreground_window() -> Option<HWND> {
    // SAFETY: Win32 境界。前面窓の識別子を読むだけで、窓の状態は一切変えない。
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() { None } else { Some(hwnd) }
}

/// 指定窓の owner を `owner` に設定する（`SetWindowLongPtrW(GWLP_HWNDPARENT)`）。
///
/// トップレベル窓に対する `GWLP_HWNDPARENT` の書込は親子化（`SetParent`）ではなく
/// **owner の付け替え**である。owner を張ると OS が owner 窓の浮上にあわせて被 owner 窓を
/// 前面へ連れて上がる（要件 1.1 の確立手段）。
/// 失敗は握り潰さず `Err` で返す（記録は呼び出し側・要件 6.2）。
#[inline(always)]
pub(crate) fn set_window_owner(hwnd: HWND, owner: HWND) -> Result<()> {
    set_window_long_ptr(hwnd, GWLP_HWNDPARENT, owner.0 as isize).map(|_| ())
}

/// 指定窓の owner を外す（`GWLP_HWNDPARENT` へ 0 を書く）。
///
/// owner を張ったままペアの一方を破棄すると OS の破棄カスケードがもう一方を巻き込み、
/// 後発の破棄が無効 HWND を撃つ。破棄に先立って本関数で切り離すことで独立破棄の
/// semantics へ戻し、要件 5.9（破棄が重複しても異常終了しない）を構造的に満たす。
/// 失敗は握り潰さず `Err` で返す。
#[inline(always)]
pub(crate) fn clear_window_owner(hwnd: HWND) -> Result<()> {
    set_window_long_ptr(hwnd, GWLP_HWNDPARENT, 0).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    // set_window_long_ptr の成功経路は自プロセス所有の実在 HWND を要するため GUI 非依存の
    // ユニットテストでは検証不能（統合的な検証はウィンドウ生成を伴う examples / 実機動作に
    // 委ねる）。get_window_long_ptr の成功経路はウィンドウ生成不要の GetDesktopWindow()
    // （常に実在する読み取り可能な HWND）で検証する。null HWND による決定的なエラー経路
    // （SetLastError → Error::from_thread のエラー変換ロジック）も併せて固定する。

    #[test]
    fn get_window_long_ptr_with_null_hwnd_returns_invalid_window_handle() {
        let err = get_window_long_ptr(HWND::default(), GWL_STYLE).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn set_window_long_ptr_with_null_hwnd_returns_invalid_window_handle() {
        let err = set_window_long_ptr(HWND::default(), GWL_EXSTYLE, 0).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn get_window_long_ptr_succeeds_on_desktop_window() {
        // GetDesktopWindow はウィンドウ生成不要で常に有効な HWND を返す（GUI 非依存）。
        // デスクトップウィンドウは常に WS_VISIBLE を持つためスタイル値は非 0。
        let hwnd = unsafe { GetDesktopWindow() };
        let style = get_window_long_ptr(hwnd, GWL_STYLE)
            .expect("desktop window style must be readable");
        assert_ne!(style, 0);
    }

    #[test]
    fn get_window_long_ptr_clears_stale_last_error_before_call() {
        // 特性化: ラッパー冒頭の SetLastError(ERROR_SUCCESS) により、呼び出し前に
        // スレッドへ残留していた無関係なエラーコードが成功判定を汚染しない。
        // （このクリアがなければ残留エラーが Err として誤報告される）
        let hwnd = unsafe { GetDesktopWindow() };
        unsafe { SetLastError(ERROR_ACCESS_DENIED) };
        let res = get_window_long_ptr(hwnd, GWL_STYLE);
        assert!(res.is_ok());
    }

    // ---- z オーダー／owner ラッパー -------------------------------------------------
    //
    // これらの成功経路は「2 つの実窓の前後関係」そのものが検証対象であり、
    // GetDesktopWindow() のような単独 HWND では前後関係を作れない。したがって
    // 自プロセス所有の実窓を 2 枚生成し、SetWindowPos で既知の重なり順を作ってから
    // 実測する（本リポジトリでは実窓を素の #[test] 内で生成する方式が既定・
    // `ecs/clickthrough/controller_tests.rs` に先例）。
    //
    // ただし `GetWindow(GW_HWNDPREV/GW_HWNDNEXT)` が辿るのは**同じ親を持つ兄弟の列**で
    // あり、トップレベル窓の兄弟列はデスクトップ全体で共有される。同一バイナリの他テスト
    // （clickthrough など）や他プロセスが窓を生成・移動すると 2 枚の間へ割り込みうるため、
    // トップレベル窓では隣接を決定論的に固定できない（実測で再現する不安定を確認済み）。
    // そこで z オーダーの検証はテスト専用の親窓を 1 枚立て、その**子窓 2 枚**で行う。
    // 兄弟列がこの 2 枚だけに閉じるため、隣接も「隣が無いこと」も決定論的に確定する。

    /// 実所有のテスト HWND（非表示ポップアップ "Static"）を生成する。
    /// `ecs/clickthrough/controller_tests.rs` の生成レシピと同一。呼び出し側が破棄する。
    fn create_test_hwnd(title: PCWSTR) -> HWND {
        create_test_window(title, WINDOW_STYLE(WS_POPUP.0), None)
    }

    /// テスト専用親窓の子として非表示の実窓を生成する（兄弟列を閉じるため）。
    fn create_test_child_hwnd(parent: HWND, title: PCWSTR) -> HWND {
        create_test_window(title, WINDOW_STYLE(WS_CHILD.0), Some(parent))
    }

    fn create_test_window(title: PCWSTR, style: WINDOW_STYLE, parent: Option<HWND>) -> HWND {
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ウィンドウを生成する。
        unsafe {
            let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("Static"),
                title,
                style,
                0,
                0,
                0,
                0,
                parent,
                None,
                Some(hinstance.into()),
                None,
            )
            .expect("CreateWindowExW should create a hidden test window")
        }
    }

    /// テスト HWND を破棄する（後始末）。
    fn destroy_test_hwnd(hwnd: HWND) {
        // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する。
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }

    /// `hwnd` を `after` のすぐ背後（1 つ背面側）へ移す。
    ///
    /// `SetWindowPos` の `hWndInsertAfter` は「位置決め対象の 1 つ手前に来る窓」であり、
    /// 対象はその**背面側**へ入る（"insert after" は z オーダー列挙上の後ろ＝背面）。
    fn insert_after(hwnd: HWND, after: HWND) {
        // SAFETY: Win32 境界。自プロセス所有の実 HWND に対する z オーダーのみの変更。
        unsafe {
            SetWindowPos(
                hwnd,
                Some(after),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
            .expect("SetWindowPos should reorder self-owned test windows");
        }
    }

    #[test]
    fn get_window_above_and_below_report_real_two_window_zorder() {
        let parent = create_test_hwnd(w!("wintf-zorder-test-parent"));
        // 兄弟列はこの 2 枚だけ。**生成直後の重なりには依存しない**——配置は直後の
        // `insert_after` で明示的に作る。実測すると子窓は**先に作ったものほど手前**であり
        // （新しい子窓は兄弟列の背面側へ入る）、生成順から前後関係を読むと逆になる。
        let first = create_test_child_hwnd(parent, w!("wintf-zorder-test-first"));
        let second = create_test_child_hwnd(parent, w!("wintf-zorder-test-second"));

        // 配置 A: first を手前・second を背後にする（second を first のすぐ背後へ落とす）。
        insert_after(second, first);
        assert_eq!(
            get_window_above(second).expect("GetWindow(GW_HWNDPREV) must succeed"),
            Some(first),
            "配置 A: second の 1 つ手前は first のはず"
        );
        assert_eq!(
            get_window_below(first).expect("GetWindow(GW_HWNDNEXT) must succeed"),
            Some(second),
            "配置 A: first の 1 つ背後は second のはず"
        );
        // 兄弟が 2 枚しか居ないので、最前面／最背面の外側は「隣が無い」。
        // 失敗ではなく正常な不在なので Ok(None) で返る。
        assert_eq!(
            get_window_above(first).expect("GetWindow(GW_HWNDPREV) must succeed"),
            None,
            "配置 A: first より手前には誰も居ないはず"
        );
        assert_eq!(
            get_window_below(second).expect("GetWindow(GW_HWNDNEXT) must succeed"),
            None,
            "配置 A: second より背後には誰も居ないはず"
        );

        // 配置 B（対照）: 前後を入れ替えると、両ラッパーの答えも入れ替わる。
        // above/below の対応付け（GW_HWNDPREV=手前／GW_HWNDNEXT=背後）が取り違えられて
        // いれば、配置 A と配置 B のどちらかが必ず落ちる。
        insert_after(first, second);
        assert_eq!(
            get_window_above(first).expect("GetWindow(GW_HWNDPREV) must succeed"),
            Some(second),
            "配置 B: first の 1 つ手前は second のはず"
        );
        assert_eq!(
            get_window_below(second).expect("GetWindow(GW_HWNDNEXT) must succeed"),
            Some(first),
            "配置 B: second の 1 つ背後は first のはず"
        );
        assert_eq!(
            get_window_above(second).expect("GetWindow(GW_HWNDPREV) must succeed"),
            None,
            "配置 B: second より手前には誰も居ないはず"
        );
        assert_eq!(
            get_window_below(first).expect("GetWindow(GW_HWNDNEXT) must succeed"),
            None,
            "配置 B: first より背後には誰も居ないはず"
        );

        destroy_test_hwnd(second);
        destroy_test_hwnd(first);
        destroy_test_hwnd(parent);
    }

    #[test]
    fn get_window_above_and_below_report_absence_as_ok_none() {
        // デスクトップウィンドウは兄弟を持たないため、前後いずれの隣接窓も存在しない。
        // 「隣が無い」は失敗ではなく正常な不在であり Ok(None) で返る。
        // 対照は get_window_above_and_below_report_real_two_window_zorder（隣が在れば
        // Some が返る）と null HWND のエラー経路（真の失敗は Err で返る）の 2 本。
        let desktop = unsafe { GetDesktopWindow() };
        assert_eq!(get_window_above(desktop).unwrap(), None);
        assert_eq!(get_window_below(desktop).unwrap(), None);
    }

    #[test]
    fn get_window_above_with_null_hwnd_returns_invalid_window_handle() {
        let err = get_window_above(HWND::default()).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn get_window_below_with_null_hwnd_returns_invalid_window_handle() {
        let err = get_window_below(HWND::default()).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn set_window_owner_with_null_hwnd_returns_invalid_window_handle() {
        let err = set_window_owner(HWND::default(), unsafe { GetDesktopWindow() }).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn clear_window_owner_with_null_hwnd_returns_invalid_window_handle() {
        let err = clear_window_owner(HWND::default()).unwrap_err();
        assert_eq!(err.code(), ERROR_INVALID_WINDOW_HANDLE.to_hresult());
    }

    #[test]
    fn get_foreground_window_returns_absence_or_a_real_window() {
        // 何が前面に居るかはテスト環境で決まらない（他プロセスの窓が前面なのが普通）。
        // 決定論的に主張できるのは「不在（None）か、実在する窓のいずれかである」ことだけで
        // ある——NULL を実在ハンドルとして返してしまう取り違えはここで落ちる。
        // 値そのものを使った判定の網羅は純関数 `is_behind_foreground` 側で行う。
        match get_foreground_window() {
            None => {}
            Some(hwnd) => {
                assert!(
                    !hwnd.is_invalid(),
                    "不在は None であって NULL ハンドルではない"
                );
                assert!(
                    unsafe { IsWindow(Some(hwnd)) }.as_bool(),
                    "返るのは実在する窓のはず"
                );
            }
        }
    }

    #[test]
    fn set_then_clear_window_owner_round_trips_on_real_windows() {
        let owner = create_test_hwnd(w!("wintf-owner-test-owner"));
        let owned = create_test_hwnd(w!("wintf-owner-test-owned"));

        // 初期状態: owner 無し（対照。ここが最初から owner 有りなら以降の assert は無意味）。
        assert_eq!(
            get_window_long_ptr(owned, GWLP_HWNDPARENT).unwrap(),
            0,
            "生成直後の owner は未設定のはず"
        );

        set_window_owner(owned, owner).expect("set_window_owner must succeed");
        assert_eq!(
            get_window_long_ptr(owned, GWLP_HWNDPARENT).unwrap(),
            owner.0 as isize,
            "GWLP_HWNDPARENT に owner の HWND が入るはず"
        );
        assert_eq!(
            unsafe { GetWindow(owned, GW_OWNER) }.ok(),
            Some(owner),
            "GetWindow(GW_OWNER) からも owner が読めるはず"
        );

        clear_window_owner(owned).expect("clear_window_owner must succeed");
        assert_eq!(
            get_window_long_ptr(owned, GWLP_HWNDPARENT).unwrap(),
            0,
            "切離し後の GWLP_HWNDPARENT は 0 のはず"
        );
        assert_eq!(
            unsafe { GetWindow(owned, GW_OWNER) }.ok(),
            None,
            "切離し後は GetWindow(GW_OWNER) も不在を返すはず"
        );

        destroy_test_hwnd(owned);
        destroy_test_hwnd(owner);
    }
}
