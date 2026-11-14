use super::*;

// ===== unsafe impl Send/Sync の不変条件（W7a-V） =====

/// `ZOrder` / `WindowPos` は HWND（`*mut c_void` newtype・非 Send/Sync）を内包する
/// ため手動 `unsafe impl Send/Sync` で Send+Sync を表明している。本テストはその
/// 不変条件をコンパイル時に固定する: 将来フィールドが追加され（かつ手動 impl が
/// 撤去され）て Send/Sync が壊れた場合に検出する回帰検知器。device 非依存（型のみ）。
#[test]
fn test_window_pos_types_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ZOrder>();
    assert_send_sync::<WindowPos>();
}

// ===== ZOrder =====

#[test]
fn test_zorder_default_is_no_change() {
    assert_eq!(ZOrder::default(), ZOrder::NoChange);
}

// ===== WindowPos::default / new =====

#[test]
fn test_default_uses_cw_usedefault_position_and_size() {
    let pos = WindowPos::default();
    assert_eq!(pos.zorder, ZOrder::NoChange);
    assert_eq!(
        pos.position,
        Some(Point {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT
        })
    );
    assert_eq!(
        pos.size,
        Some(SizeI {
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT
        })
    );
    // 全ての SWP bool フラグは既定で false
    assert!(!pos.no_redraw);
    assert!(!pos.no_activate);
    assert!(!pos.frame_changed);
    assert!(!pos.show_window);
    assert!(!pos.hide_window);
    assert!(!pos.no_copy_bits);
    assert!(!pos.no_owner_zorder);
    assert!(!pos.no_send_changing);
    assert!(!pos.defer_erase);
    assert!(!pos.async_window_pos);
}

#[test]
fn test_new_equals_default() {
    assert_eq!(WindowPos::new(), WindowPos::default());
}

// ===== builder pattern =====

#[test]
fn test_builder_with_position_and_size() {
    let pos = WindowPos::new()
        .with_position(Point { x: 10, y: 20 })
        .with_size(SizeI {
            width: 300,
            height: 400,
        });
    assert_eq!(pos.position, Some(Point { x: 10, y: 20 }));
    assert_eq!(
        pos.size,
        Some(SizeI {
            width: 300,
            height: 400
        })
    );
}

#[test]
fn test_builder_zorder_helpers() {
    assert_eq!(WindowPos::new().zorder_no_change().zorder, ZOrder::NoChange);
    assert_eq!(WindowPos::new().zorder_topmost().zorder, ZOrder::TopMost);
    assert_eq!(
        WindowPos::new().zorder_notopmost().zorder,
        ZOrder::NoTopMost
    );
    assert_eq!(WindowPos::new().zorder_top().zorder, ZOrder::Top);
    assert_eq!(WindowPos::new().zorder_bottom().zorder, ZOrder::Bottom);
    let hwnd = HWND(0x1234 as *mut _);
    assert_eq!(
        WindowPos::new().zorder_insert_after(hwnd).zorder,
        ZOrder::InsertAfter(hwnd)
    );
    assert_eq!(
        WindowPos::new().with_zorder(ZOrder::Top).zorder,
        ZOrder::Top
    );
}

#[test]
fn test_builder_bool_flag_setters() {
    let pos = WindowPos::new()
        .no_redraw(true)
        .no_activate(true)
        .frame_changed(true)
        .show_window(true)
        .hide_window(true)
        .no_copy_bits(true)
        .no_owner_zorder(true)
        .no_send_changing(true)
        .defer_erase(true)
        .async_window_pos(true);
    assert!(pos.no_redraw);
    assert!(pos.no_activate);
    assert!(pos.frame_changed);
    assert!(pos.show_window);
    assert!(pos.hide_window);
    assert!(pos.no_copy_bits);
    assert!(pos.no_owner_zorder);
    assert!(pos.no_send_changing);
    assert!(pos.defer_erase);
    assert!(pos.async_window_pos);
}

// ===== build_flags_for_system (auto-detect NOMOVE/NOSIZE/NOZORDER) =====

#[test]
fn test_build_flags_default_sets_nozorder_only_when_pos_size_present() {
    // 既定は position/size とも Some なので NOMOVE/NOSIZE は立たず、NoChange により NOZORDER のみ
    let flags = WindowPos::default().build_flags_for_system();
    assert_eq!(flags & SWP_NOMOVE, SET_WINDOW_POS_FLAGS(0));
    assert_eq!(flags & SWP_NOSIZE, SET_WINDOW_POS_FLAGS(0));
    assert_eq!(flags & SWP_NOZORDER, SWP_NOZORDER);
}

#[test]
fn test_build_flags_position_none_sets_nomove() {
    let mut pos = WindowPos::new();
    pos.position = None;
    let flags = pos.build_flags_for_system();
    assert_eq!(flags & SWP_NOMOVE, SWP_NOMOVE);
}

#[test]
fn test_build_flags_size_none_sets_nosize() {
    let mut pos = WindowPos::new();
    pos.size = None;
    let flags = pos.build_flags_for_system();
    assert_eq!(flags & SWP_NOSIZE, SWP_NOSIZE);
}

#[test]
fn test_build_flags_nonzero_zorder_clears_nozorder() {
    let flags = WindowPos::new().zorder_top().build_flags_for_system();
    // NoChange 以外では NOZORDER は立たない
    assert_eq!(flags & SWP_NOZORDER, SET_WINDOW_POS_FLAGS(0));
}

#[test]
fn test_build_flags_maps_each_bool_to_swp_flag() {
    let pos = WindowPos::new()
        .no_redraw(true)
        .no_activate(true)
        .frame_changed(true)
        .show_window(true)
        .hide_window(true)
        .no_copy_bits(true)
        .no_owner_zorder(true)
        .no_send_changing(true)
        .defer_erase(true)
        .async_window_pos(true);
    let flags = pos.build_flags_for_system();
    assert_eq!(flags & SWP_NOREDRAW, SWP_NOREDRAW);
    assert_eq!(flags & SWP_NOACTIVATE, SWP_NOACTIVATE);
    assert_eq!(flags & SWP_FRAMECHANGED, SWP_FRAMECHANGED);
    assert_eq!(flags & SWP_SHOWWINDOW, SWP_SHOWWINDOW);
    assert_eq!(flags & SWP_HIDEWINDOW, SWP_HIDEWINDOW);
    assert_eq!(flags & SWP_NOCOPYBITS, SWP_NOCOPYBITS);
    assert_eq!(flags & SWP_NOOWNERZORDER, SWP_NOOWNERZORDER);
    assert_eq!(flags & SWP_NOSENDCHANGING, SWP_NOSENDCHANGING);
    assert_eq!(flags & SWP_DEFERERASE, SWP_DEFERERASE);
    assert_eq!(flags & SWP_ASYNCWINDOWPOS, SWP_ASYNCWINDOWPOS);
}

// ===== get_hwnd_insert_after (ZOrder enum mapping) =====

#[test]
fn test_get_hwnd_insert_after_maps_each_zorder() {
    assert_eq!(
        WindowPos::new().zorder_no_change().get_hwnd_insert_after(),
        None
    );
    assert_eq!(
        WindowPos::new().zorder_topmost().get_hwnd_insert_after(),
        Some(HWND_TOPMOST)
    );
    assert_eq!(
        WindowPos::new().zorder_notopmost().get_hwnd_insert_after(),
        Some(HWND_NOTOPMOST)
    );
    assert_eq!(
        WindowPos::new().zorder_top().get_hwnd_insert_after(),
        Some(HWND_TOP)
    );
    assert_eq!(
        WindowPos::new().zorder_bottom().get_hwnd_insert_after(),
        Some(HWND_BOTTOM)
    );
    let hwnd = HWND(0xABCD as *mut _);
    assert_eq!(
        WindowPos::new()
            .zorder_insert_after(hwnd)
            .get_hwnd_insert_after(),
        Some(hwnd)
    );
}

// ===== to_window_coords_for_creation =====

#[test]
fn test_to_window_coords_for_creation_passes_through_cw_usedefault() {
    // CW_USEDEFAULT を含む既定 WindowPos は AdjustWindowRectExForDpi を呼ばず素通し
    let pos = WindowPos::default();
    let (x, y, w, h) = pos.to_window_coords_for_creation(WINDOW_STYLE(0), WINDOW_EX_STYLE(0), 96);
    assert_eq!(x, CW_USEDEFAULT);
    assert_eq!(y, CW_USEDEFAULT);
    assert_eq!(w, CW_USEDEFAULT);
    assert_eq!(h, CW_USEDEFAULT);
}

/// 枠つきの窓（`WS_OVERLAPPEDWINDOW`）でクライアント矩形 {500,50,800,250} の外枠の値。
/// 調べる関数を通さず、`AdjustWindowRectExForDpi` を矩形へ直に当てて出す。枠が 0 でないことも確かめる。
fn overlapped_expected() -> (i32, i32, i32, i32) {
    let mut rect = RECT {
        left: 500,
        top: 50,
        right: 800,
        bottom: 250,
    };
    unsafe {
        AdjustWindowRectExForDpi(
            &mut rect,
            WS_OVERLAPPEDWINDOW,
            false,
            WINDOW_EX_STYLE(0),
            96,
        )
    }
    .expect("AdjustWindowRectExForDpi");
    let expected = (
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    );
    let (x, y, w, h) = expected;
    assert!(
        x < 500 && y < 50 && w > 300 && h > 200,
        "枠が 0: {expected:?}"
    );
    expected
}

#[test]
fn test_to_window_coords_for_creation_both_given_adds_frame() {
    // 位置と大きさを両方与えると、外枠の矩形をそのまま返す
    let mut pos = WindowPos::new();
    pos.position = Some(Point { x: 500, y: 50 });
    pos.size = Some(SizeI {
        width: 300,
        height: 200,
    });
    let got = pos.to_window_coords_for_creation(WS_OVERLAPPEDWINDOW, WINDOW_EX_STYLE(0), 96);
    assert_eq!(got, overlapped_expected());
}

#[test]
fn test_to_window_coords_for_creation_position_only_adds_frame_to_position() {
    // 位置だけ与えると、位置に枠の分を足し、大きさは既定のまま返す
    let (bx, by, _, _) = overlapped_expected();
    let mut pos = WindowPos::new();
    pos.position = Some(Point { x: 500, y: 50 });
    let got = pos.to_window_coords_for_creation(WS_OVERLAPPEDWINDOW, WINDOW_EX_STYLE(0), 96);
    assert_eq!(got, (bx, by, CW_USEDEFAULT, CW_USEDEFAULT));
}

#[test]
fn test_to_window_coords_for_creation_size_only_adds_frame_to_size() {
    // 大きさだけ与えると、位置は既定のまま、大きさに枠の分を足して返す
    let (_, _, bw, bh) = overlapped_expected();
    let mut pos = WindowPos::new();
    pos.size = Some(SizeI {
        width: 300,
        height: 200,
    });
    let got = pos.to_window_coords_for_creation(WS_OVERLAPPEDWINDOW, WINDOW_EX_STYLE(0), 96);
    assert_eq!(got, (CW_USEDEFAULT, CW_USEDEFAULT, bw, bh));
}

#[test]
fn test_to_window_coords_for_creation_no_frame_style_is_identity() {
    // WS_POPUP（フレームなし）+ ex_style 0 では AdjustWindowRectExForDpi の調整量が 0 となり、
    // クライアント矩形＝ウィンドウ矩形（座標素通し）。実 API を呼ぶが結果は決定的。
    let mut pos = WindowPos::new();
    pos.position = Some(Point { x: 50, y: 60 });
    pos.size = Some(SizeI {
        width: 200,
        height: 150,
    });
    let (x, y, w, h) = pos.to_window_coords_for_creation(WS_POPUP, WINDOW_EX_STYLE(0), 96);
    assert_eq!((x, y, w, h), (50, 60, 200, 150));
}
