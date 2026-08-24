//! ウィンドウ手続きブリッジ。
//!
//! ライブラリ（`wintf-winmsg-executor`）の `Window<S>` は wndproc を
//! `FnMut(Pin<&S>, WindowMessage) -> Option<LRESULT>` クロージャとして受け取る。
//! 本ユニットはその状態 `S = WndState` を定め、クロージャを ECS 層の配送純関数
//! `crate::ecs::dispatch_window_message` へ橋渡しする「メカニズム」を提供する
//! （配送表本体は ECS 層 `dispatch_window_message` が単一の真実の源として保持する）。
//!
//! # 共有状態アクセス（要件 2.3）
//! 旧経路は `GWLP_USERDATA` への Entity 手詰めとグローバルな弱参照、HWND からの
//! 自己解決で World/Entity を取得していた（撤去済み・task 4.5）。本ブリッジでは生成時に
//! 確定した `WndState{ world: Weak<RefCell<EcsWorld>>, entity }` をクロージャが直接
//! capture する（`GWLP_USERDATA` 手詰めなし・グローバル状態なし）。`Entity` は `Copy`
//! のため pinned state から直接読める。
//!
//! # Entity 配送（要件 2.4）
//! クロージャは `WindowMessage` を `dispatch_window_message(world, entity, msg)` へ
//! 橋渡しする。`None` 返却時はライブラリ既定手続き（`DefWindowProcW`、`Window` 内部の
//! `unwrap_or_else(DefWindowProcW)`）へ委譲される。
//!
//! # 安全スキップ規律（要件 4.3）
//! クロージャは `world.upgrade()`→`None`（破棄中）と World の `try_borrow` 失敗
//! （再入）をいずれも `None`（既定手続き委譲）で安全スキップする。これは現行ハンドラの
//! `try_borrow` 規律を踏襲し、ライブラリ側 `RefCell` 再入防止と二重防御になる。

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Weak;

use crate::executor::util::WindowMessage;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::LRESULT;

use crate::ecs::dispatch_window_message;
use crate::ecs::world::EcsWorld;

/// wndproc クロージャが生成時に capture する共有状態。
///
/// `world`／`entity` はウィンドウ生成時に確定し、以後不変（`create_windows` が entity を
/// 知る）。`Weak` で掴むことで registry↔World の自己循環リークを避け、破棄中は
/// `upgrade()`→`None` で安全にスキップできる（設計 State Management）。
pub(crate) struct WndState {
    /// 共有 World への弱参照（strong 所有者は `WinApp`）。
    pub world: Weak<RefCell<EcsWorld>>,
    /// このウィンドウに対応する ECS Entity（`Copy`・生成時確定）。
    pub entity: Entity,
}

/// `Window::new_ex` 互換の wndproc クロージャを構築する。
///
/// 返すクロージャのシグネチャは `Fn(Pin<&WndState>, WindowMessage) -> Option<LRESULT>`
/// で、ライブラリの `new_ex`（RefCell ラップなし＝**wndproc 再入可**）へそのまま渡せる。
/// `new_checked_ex`（内部 `RefCell` で再入阻止）は採用しない: wintf のドラッグが同期発火する
/// WM_WINDOWPOSCHANGED への wndproc 再入で WindowPos を echo-bypass 更新する設計に依存する
/// ため（`window_factory` §再入方針・5.3 実機回帰）。再入安全性は手順 3 の World `try_borrow`
/// 安全スキップ＋ECS 側 tick ガード（`IS_TICK_FLUSH_IN_PROGRESS`）で担保する。
/// クロージャは以下の規律で配送する:
///
/// 1. pinned state から `entity` を直接読む（`Entity: Copy`）。
/// 2. `world.upgrade()`→`None` なら `None`（破棄中・安全スキップ）。
/// 3. World を `try_borrow`。失敗（再入）なら `None`（安全スキップ）。
/// 4. 成功時のみ `dispatch_window_message(&world, entity, &msg)` を呼ぶ。
//
// `EcsWindowFactory::create_window`（`create_windows` 経由）がウィンドウ生成時に呼び、
// 返したクロージャをライブラリの `Window::new_ex` へ渡す（task 4.3 結線済み）。
pub(crate) fn make_wndproc() -> impl Fn(Pin<&WndState>, WindowMessage) -> Option<LRESULT> {
    move |state: Pin<&WndState>, msg: WindowMessage| -> Option<LRESULT> {
        // Entity は Copy のため pinned state から直接読める（手詰めなし・要件 2.3）。
        let entity = state.entity;

        // World への弱参照を upgrade。破棄中（strong 所有者 drop 済み）なら安全スキップ。
        let world = state.world.upgrade()?;

        // World 借用を試みる。借用失敗（再入・別経路が借用中）は安全スキップ（要件 4.3）。
        // 借用ガード自体は dispatch に渡さない（dispatch は `Rc` を受け取り内部で借用規律を
        // 適用する純関数）。ここでは「借用可能か」だけを試して即座に解放する。
        if world.try_borrow().is_err() {
            return None;
        }

        // 配送表本体は ECS 層の `dispatch_window_message`（単一の真実の源）が担う（要件 2.4）。
        dispatch_window_message(&world, entity, &msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{WM_ERASEBKGND, WM_USER};

    /// テスト用 `WndState` を組み立てる。
    fn make_state(world: &Rc<RefCell<EcsWorld>>) -> WndState {
        let entity = world.borrow_mut().world_mut().spawn(()).id();
        WndState {
            world: Rc::downgrade(world),
            entity,
        }
    }

    /// 代表メッセージ用の `WindowMessage` を組む（hwnd 等は配送に無関係）。
    fn msg(message: u32) -> WindowMessage {
        WindowMessage {
            hwnd: HWND(std::ptr::null_mut()),
            msg: message,
            wparam: WPARAM(0),
            lparam: LPARAM(0),
        }
    }

    /// World が drop 済み（`upgrade()`→`None`）のときクロージャは `None` を返し panic しない
    /// （破棄中の安全スキップ・要件 4.3）。
    #[test]
    fn closure_safe_skips_when_world_dropped() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();

        // strong 所有者を drop → Weak::upgrade が None になる。
        drop(world);

        let boxed = Box::pin(state);
        let ret = wndproc(boxed.as_ref(), msg(WM_ERASEBKGND));
        assert!(
            ret.is_none(),
            "World drop 後は upgrade→None で安全スキップ（None）すべき"
        );
    }

    /// World が借用中（`borrow_mut` 保持）のときクロージャは `None` を返し panic しない
    /// （再入の安全スキップ・要件 4.3）。
    #[test]
    fn closure_safe_skips_when_world_borrowed() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();

        // World を借用したままクロージャを呼ぶ → try_borrow 失敗で安全スキップ。
        let _held = world.borrow_mut();

        let boxed = Box::pin(state);
        let ret = wndproc(boxed.as_ref(), msg(WM_ERASEBKGND));
        assert!(
            ret.is_none(),
            "World 借用中は try_borrow 失敗で安全スキップ（None）すべき"
        );
    }

    /// 生存・未借用 World で代表メッセージは `Some(LRESULT)`、非対応メッセージは `None`
    /// （配送純関数の代表ルーティング・要件 2.4）。
    #[test]
    fn closure_routes_representative_message() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let state = make_state(&world);
        let wndproc = make_wndproc();

        let boxed = Box::pin(state);

        // 代表メッセージ → Some。
        let handled = wndproc(boxed.as_ref(), msg(WM_ERASEBKGND));
        assert_eq!(
            handled,
            Some(LRESULT(1)),
            "代表メッセージ WM_ERASEBKGND は Some(LRESULT(1)) を返すべき"
        );

        // 非対応メッセージ → None（DefWindowProcW 委譲）。
        let unhandled = wndproc(boxed.as_ref(), msg(WM_USER));
        assert!(
            unhandled.is_none(),
            "非対応メッセージは None（既定手続き委譲）であるべき"
        );
    }
}
