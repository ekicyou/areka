//! ウィンドウプロシージャモジュール
//!
//! Windowsメッセージのディスパッチとハンドラ管理

mod dpi_helpers;
mod keyboard;
mod lifecycle;
mod mouse_click;
mod mouse_dblclick_wheel;
mod mouse_move;
mod window_pos;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::executor::util::WindowMessage;

use std::cell::RefCell;
use std::rc::Rc;

use crate::ecs::world::EcsWorld;

/// Windows メッセージを Entity 配送する純関数（要件 2.4・設計 EntityWndprocBridge）。
///
/// `world`/`entity` を引数で受け取り、各ハンドラへ統一シグネチャ
/// `(world, entity, hwnd, wparam, lparam)` で配送する。`None` 返却時は呼び出し側
/// （`runtime::wndproc_bridge` のライブラリ wndproc クロージャ）が `DefWindowProcW` へ委譲する。
///
/// 本表は `WM_NCCREATE`／`WM_NCDESTROY`（エンティティ確立／後始末のライフサイクル特例）を
/// 含まない。新経路ではウィンドウ生成・破棄をライブラリ／`WindowRegistry` の drop 駆動が
/// 所管する（NCCREATE/NCDESTROY の自前処理は撤去済み・task 4.5）。
pub(crate) fn dispatch_window_message(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    msg: &WindowMessage,
) -> Option<LRESULT> {
    let hwnd = msg.hwnd;
    let wparam = msg.wparam;
    let lparam = msg.lparam;

    match msg.msg {
        WM_ERASEBKGND => lifecycle::WM_ERASEBKGND(world, entity, hwnd, wparam, lparam),
        WM_PAINT => lifecycle::WM_PAINT(world, entity, hwnd, wparam, lparam),
        WM_CLOSE => lifecycle::WM_CLOSE(world, entity, hwnd, wparam, lparam),
        WM_WINDOWPOSCHANGED => window_pos::WM_WINDOWPOSCHANGED(world, entity, hwnd, wparam, lparam),
        WM_DISPLAYCHANGE => lifecycle::WM_DISPLAYCHANGE(world, entity, hwnd, wparam, lparam),
        WM_DPICHANGED => window_pos::WM_DPICHANGED(world, entity, hwnd, wparam, lparam),
        // マウスメッセージ
        WM_SETCURSOR => mouse_move::WM_SETCURSOR(world, entity, hwnd, wparam, lparam),
        WM_NCHITTEST => mouse_move::WM_NCHITTEST(world, entity, hwnd, wparam, lparam),
        WM_MOUSEMOVE => mouse_move::WM_MOUSEMOVE(world, entity, hwnd, wparam, lparam),
        WM_MOUSELEAVE => mouse_move::WM_MOUSELEAVE(world, entity, hwnd, wparam, lparam),
        WM_LBUTTONDOWN => mouse_click::WM_LBUTTONDOWN(world, entity, hwnd, wparam, lparam),
        WM_LBUTTONUP => mouse_click::WM_LBUTTONUP(world, entity, hwnd, wparam, lparam),
        WM_RBUTTONDOWN => mouse_click::WM_RBUTTONDOWN(world, entity, hwnd, wparam, lparam),
        WM_RBUTTONUP => mouse_click::WM_RBUTTONUP(world, entity, hwnd, wparam, lparam),
        WM_MBUTTONDOWN => mouse_click::WM_MBUTTONDOWN(world, entity, hwnd, wparam, lparam),
        WM_MBUTTONUP => mouse_click::WM_MBUTTONUP(world, entity, hwnd, wparam, lparam),
        WM_XBUTTONDOWN => mouse_click::WM_XBUTTONDOWN(world, entity, hwnd, wparam, lparam),
        WM_XBUTTONUP => mouse_click::WM_XBUTTONUP(world, entity, hwnd, wparam, lparam),
        WM_LBUTTONDBLCLK => mouse_dblclick_wheel::WM_LBUTTONDBLCLK(world, entity, hwnd, wparam, lparam),
        WM_RBUTTONDBLCLK => mouse_dblclick_wheel::WM_RBUTTONDBLCLK(world, entity, hwnd, wparam, lparam),
        WM_MBUTTONDBLCLK => mouse_dblclick_wheel::WM_MBUTTONDBLCLK(world, entity, hwnd, wparam, lparam),
        WM_XBUTTONDBLCLK => mouse_dblclick_wheel::WM_XBUTTONDBLCLK(world, entity, hwnd, wparam, lparam),
        WM_MOUSEWHEEL => mouse_dblclick_wheel::WM_MOUSEWHEEL(world, entity, hwnd, wparam, lparam),
        WM_MOUSEHWHEEL => mouse_dblclick_wheel::WM_MOUSEHWHEEL(world, entity, hwnd, wparam, lparam),
        WM_KEYDOWN => keyboard::WM_KEYDOWN(world, entity, hwnd, wparam, lparam),
        WM_CANCELMODE => keyboard::WM_CANCELMODE(world, entity, hwnd, wparam, lparam),
        WM_ACTIVATE => keyboard::WM_ACTIVATE(world, entity, hwnd, wparam, lparam),
        WM_CAPTURECHANGED => keyboard::WM_CAPTURECHANGED(world, entity, hwnd, wparam, lparam),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::App;
    use crate::ecs::world::EcsWorld;

    /// テスト用 `WindowMessage` を組む（hwnd 等は配送に無関係なメッセージ向け）。
    fn msg(message: u32) -> WindowMessage {
        WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: message,
            wparam: WPARAM(0),
            lparam: LPARAM(0),
        }
    }

    /// `dispatch_window_message` は代表メッセージ `WM_ERASEBKGND` を `Some(LRESULT(1))` に
    /// ルーティングする（移行後も `lifecycle::WM_ERASEBKGND` の挙動が不変・要件 2.4）。
    /// ヘッドレス: 実 HWND / メッセージループ不要。
    #[test]
    fn dispatch_routes_erasebkgnd_to_some_lresult_1() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = world.borrow_mut().world_mut().spawn(()).id();

        let ret = dispatch_window_message(&world, entity, &msg(WM_ERASEBKGND));
        assert_eq!(
            ret,
            Some(LRESULT(1)),
            "WM_ERASEBKGND は背景消去抑止の Some(LRESULT(1)) を返すべき"
        );
    }

    /// `dispatch_window_message` は配送表に無いメッセージ（`WM_USER`）を `None` にする
    /// （既定手続き委譲・要件 2.4）。
    #[test]
    fn dispatch_returns_none_for_unhandled_message() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = world.borrow_mut().world_mut().spawn(()).id();

        let ret = dispatch_window_message(&world, entity, &msg(WM_USER));
        assert!(
            ret.is_none(),
            "配送表に無い WM_USER は None（DefWindowProcW 委譲）であるべき"
        );
    }

    /// `dispatch_window_message` は `WM_DISPLAYCHANGE` を `lifecycle::WM_DISPLAYCHANGE` に
    /// 配送し、`App` リソースの display change フラグを立てる（業務ロジック不変・要件 2.4/5.2）。
    /// `None` 返却（DefWindowProcW 委譲）も併せて確認する。ヘッドレス: World への直接操作のみ。
    #[test]
    fn dispatch_displaychange_marks_app_display_change() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = {
            let mut w = world.borrow_mut();
            w.world_mut().insert_resource(App::new());
            w.world_mut().spawn(()).id()
        };

        // 事前条件: フラグ未設定。
        assert!(
            !world
                .borrow()
                .world()
                .resource::<App>()
                .display_configuration_changed(),
            "初期状態は display change 未設定"
        );

        let ret = dispatch_window_message(&world, entity, &msg(WM_DISPLAYCHANGE));

        // WM_DISPLAYCHANGE は DefWindowProcW 委譲（None）。
        assert!(ret.is_none(), "WM_DISPLAYCHANGE は None を返すべき");

        // 業務ロジック: App の display change フラグが立つ。
        assert!(
            world
                .borrow()
                .world()
                .resource::<App>()
                .display_configuration_changed(),
            "WM_DISPLAYCHANGE 配送後は display change が設定されるべき"
        );
    }

    /// 代表メッセージ ① ダブルクリック `WM_LBUTTONDBLCLK`（要件 2.4・設計 Testing Strategy）。
    /// `dispatch_window_message` は `mouse_dblclick_wheel::WM_LBUTTONDBLCLK` へ配送する。
    /// ヘッドレス: hit 領域を持たない最小 World（bare entity）では hit_test がターゲット
    /// 無しを返し、何も挿入せず `Some(LRESULT(0))` を返す（旧 `ecs_wndproc` 同等）。
    /// 実 HWND / hit 領域を要する深い検証（PointerState 挿入）は 5.2/5.3 へ委譲する。
    #[test]
    fn dispatch_routes_lbuttondblclk_to_some_lresult_0() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = world.borrow_mut().world_mut().spawn(()).id();

        // lparam=0（クリック座標 (0,0)）でも hit 領域なし → ターゲット無し → 無操作。
        let ret = dispatch_window_message(&world, entity, &msg(WM_LBUTTONDBLCLK));
        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "WM_LBUTTONDBLCLK はダブルクリック処理後 Some(LRESULT(0)) を返すべき（panic 無し）"
        );
    }

    /// 代表メッセージ ② `WM_WINDOWPOSCHANGED`（要件 2.4・設計 Testing Strategy）。
    /// `dispatch_window_message` は `window_pos::WM_WINDOWPOSCHANGED` へ配送する。
    /// 非 null の `WINDOWPOS` ポインタを `lparam` に設定するが、`WindowHandle` コンポーネント
    /// 不在のため `client_coords` が `None` となり座標更新はスキップされる。最終的に
    /// `None`（DefWindowProcW 委譲）を返す（旧 `ecs_wndproc` 同等）。ヘッドレス: 実 HWND 不要。
    #[test]
    fn dispatch_routes_windowposchanged_to_none() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = world.borrow_mut().world_mut().spawn(()).id();

        // スタックに実体のある WINDOWPOS を用意し、その生ポインタを lparam に渡す
        // （ハンドラは null 判定後に dereference するため、非 null かつ有効な番地が必要）。
        let windowpos = WINDOWPOS {
            hwnd: HWND(std::ptr::null_mut()),
            hwndInsertAfter: HWND(std::ptr::null_mut()),
            x: 100,
            y: 200,
            cx: 640,
            cy: 480,
            flags: SET_WINDOW_POS_FLAGS(0),
        };
        let m = WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: WM_WINDOWPOSCHANGED,
            wparam: WPARAM(0),
            lparam: LPARAM(&windowpos as *const WINDOWPOS as isize),
        };

        let ret = dispatch_window_message(&world, entity, &m);
        assert!(
            ret.is_none(),
            "WM_WINDOWPOSCHANGED は None（DefWindowProcW 委譲）を返すべき"
        );
    }

    /// 代表メッセージ ③ `WM_DPICHANGED`（要件 2.4・設計 Testing Strategy）。
    /// `dispatch_window_message` は `window_pos::WM_DPICHANGED` へ配送する。
    /// `DPI` コンポーネント不在のため DPI 更新はスキップされ、`guarded_set_window_pos` は
    /// null HWND で `Err`（warn ログのみ・panic 無し）となるが、ハンドラは常に
    /// `Some(LRESULT(0))` を返す（旧 `ecs_wndproc` 同等）。ヘッドレス: 実 HWND 不要。
    #[test]
    fn dispatch_routes_dpichanged_to_some_lresult_0() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = world.borrow_mut().world_mut().spawn(()).id();

        // wparam HIWORD/LOWORD = 新 DPI（96）, lparam=0 → suggested_rect は default。
        let m = WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: WM_DPICHANGED,
            wparam: WPARAM(((96u32) << 16 | 96u32) as usize),
            lparam: LPARAM(0),
        };

        let ret = dispatch_window_message(&world, entity, &m);
        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "WM_DPICHANGED は処理後 Some(LRESULT(0)) を返すべき（panic 無し）"
        );
    }

    /// 代表メッセージ ④ 非活性化 `WM_ACTIVATE`（`WA_INACTIVE`）。
    /// `dispatch_window_message` は `keyboard::WM_ACTIVATE` へ配送し、ゴースト窓ペアの
    /// 当事者には沈降観測の目印が付く（要件 4.4／7.5）。返り値は `None`（DefWindowProcW 委譲）
    /// のままであり、**窓の挙動は変わらない**。重なりの実測と記録はこの巡では行わない
    /// （遅延 1 巡の分業は `zorder_pair_sink` の兄弟テストが固定する）。
    /// ヘッドレス: 実 HWND / メッセージループ不要。
    #[test]
    fn dispatch_activate_inactive_marks_the_pair_for_sink_observation() {
        use crate::ecs::window::{KeepDirectlyAbove, SinkObservationPending};

        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let (character, balloon) = {
            let mut w = world.borrow_mut();
            let w = w.world_mut();
            let character = w.spawn(()).id();
            let balloon = w.spawn(KeepDirectlyAbove { peer: character }).id();
            (character, balloon)
        };

        let m = WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: WM_ACTIVATE,
            wparam: WPARAM(0), // WA_INACTIVE
            lparam: LPARAM(0),
        };
        let ret = dispatch_window_message(&world, character, &m);

        assert!(
            ret.is_none(),
            "WM_ACTIVATE は None（DefWindowProcW 委譲）を返すべき"
        );
        assert!(
            world
                .borrow()
                .world()
                .entity(balloon)
                .contains::<SinkObservationPending>(),
            "非活性化の受理でペアの宣言側へ目印が付くべき"
        );
        // 対照: 活性化（WA_ACTIVE）は目印を付けない（付く側の主張が恒真にならない）。
        let stray_world = Rc::new(RefCell::new(EcsWorld::new()));
        let (stray_char, stray_balloon) = {
            let mut w = stray_world.borrow_mut();
            let w = w.world_mut();
            let character = w.spawn(()).id();
            let balloon = w.spawn(KeepDirectlyAbove { peer: character }).id();
            (character, balloon)
        };
        let active = WindowMessage {
            wparam: WPARAM(1), // WA_ACTIVE
            ..m
        };
        dispatch_window_message(&stray_world, stray_char, &active);
        assert!(
            !stray_world
                .borrow()
                .world()
                .entity(stray_balloon)
                .contains::<SinkObservationPending>(),
            "活性化では目印を付けないべき"
        );
    }

    /// 代表メッセージ ⑤ 非クライアント破棄 `WM_NCDESTROY`（要件 2.4・設計 Testing Strategy）。
    /// `WM_NCDESTROY` は配送表からライフサイクル特例として除外されており（タスク 3.2／旧
    /// `ecs_wndproc` シムが直接処理）、新経路 `dispatch_window_message` では表に無いため
    /// `None`（DefWindowProcW 委譲）を返すのが設計どおりの「旧手続きと同等」結果である。
    /// 新経路でのウィンドウ破棄は WindowRegistry の drop 駆動（タスク 4.1）であり、本配送
    /// 純関数の所管外であることをここで固定する。ヘッドレス: 実 HWND 不要。
    #[test]
    fn dispatch_returns_none_for_ncdestroy_excluded_from_table() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = world.borrow_mut().world_mut().spawn(()).id();

        let ret = dispatch_window_message(&world, entity, &msg(WM_NCDESTROY));
        assert!(
            ret.is_none(),
            "WM_NCDESTROY は配送表に無く None（DefWindowProcW 委譲）であるべき（破棄は registry drop 駆動）"
        );
    }
}
