//! ウィンドウライフサイクルおよびディスプレイ変更ハンドラ
//!
//! WM_ERASEBKGND, WM_PAINT, WM_CLOSE, WM_DISPLAYCHANGE
//!
//! NOTE: WM_NCCREATE / WM_NCDESTROY（GWLP_USERDATA への Entity 格納・破棄時 despawn）は
//! 旧 `ecs_wndproc` 専用だったため撤去した（task 4.5）。新経路ではウィンドウ生成・破棄を
//! ライブラリ／`WindowRegistry` の drop 駆動が所管する。

#![allow(non_snake_case)]

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::*;

use crate::ecs::world::EcsWorld;

/// メッセージハンドラの戻り値型
type HandlerResult = Option<LRESULT>;

/// WM_ERASEBKGND: 背景消去要求
///
/// ULW が全画面を管理するため、背景消去をスキップする
#[inline]
pub(super) fn WM_ERASEBKGND(
    _world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    Some(LRESULT(1)) // 背景消去をスキップ
}

/// WM_PAINT: 再描画要求
///
/// - `CompositionMode::DComp` → `DefWindowProcW` に委譲（DComp は OS 管理）
/// - `CompositionMode::ULW` またはフォールバック → `BeginPaint`/`EndPaint` 最小ペア
#[inline]
pub(super) fn WM_PAINT(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    // Entity から CompositionMode を判定
    let is_dcomp = if let Ok(world_borrow) = world.try_borrow() {
        world_borrow
            .world()
            .get::<crate::ecs::window::Window>(entity)
            .map(|w| w.composition_mode() == crate::ecs::window::CompositionMode::DComp)
            .unwrap_or(false)
    } else {
        false
    };

    if is_dcomp {
        // DComp モード: DefWindowProcW に委譲
        None
    } else {
        // ULW モード: BeginPaint/EndPaint 最小ペア
        use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
        let mut ps = PAINTSTRUCT::default();
        unsafe {
            let _ = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
        }
        Some(LRESULT(0))
    }
}

/// WM_CLOSE: ウィンドウクローズ要求
///
/// 対象 Entity の除去要求（despawn）として処理する。`DestroyWindow` を直叩きせず、
/// `Window` コンポーネント消失を `RemovedComponents<Window>`（reconcile_window_registry・
/// タスク 3.3）が検知し、レジストリ要素 drop 駆動で `DestroyWindow` させることで
/// ハンドル破棄を Entity ライフサイクルに一致させる（要件 1.3）。
///
/// 同期再入時の二重借用を避けるため `try_borrow_mut` を用い、既に借用中なら
/// safe-skip する（パニックさせない）。`Some(LRESULT(0))` を返して既定手続きの
/// `DestroyWindow` を抑止する（破棄は reconcile 経由・要件 2.3）。
#[inline]
pub(super) fn WM_CLOSE(
    world: &Rc<RefCell<EcsWorld>>,
    entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    if let Ok(mut w) = world.try_borrow_mut() {
        w.world_mut().despawn(entity);
    }
    Some(LRESULT(0))
}

/// WM_DISPLAYCHANGE: ディスプレイ構成変更通知
///
/// Appリソースのmark_display_changeを呼び出す
#[inline]
pub(super) fn WM_DISPLAYCHANGE(
    world: &Rc<RefCell<EcsWorld>>,
    _entity: Entity,
    _hwnd: HWND,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> HandlerResult {
    if let Ok(mut world_borrow) = world.try_borrow_mut() {
        if let Some(mut app) = world_borrow
            .world_mut()
            .get_resource_mut::<crate::ecs::App>()
        {
            app.mark_display_change();
        }
    }
    None // DefWindowProcWに委譲
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::HWND;

    /// テスト用に `Window` コンポーネント付き Entity を spawn する。
    /// ヘッドレス: `on_window_add` フックはコマンドを deferred enqueue するのみで
    /// （実窓を inline 生成しない）、コマンド未フラッシュのため OS リソースに触れない。
    fn spawn_window_entity(world: &Rc<RefCell<EcsWorld>>) -> Entity {
        world
            .borrow_mut()
            .world_mut()
            .spawn(crate::ecs::window::Window::default())
            .id()
    }

    /// WM_CLOSE は対象 Entity を despawn し（`DestroyWindow` 直叩きをしない）、
    /// `Some(LRESULT(0))`（既定破棄抑止）を返す（要件 1.3 / 2.3）。
    /// ヘッドレス: 実 HWND / メッセージループ不要。
    #[test]
    fn wm_close_despawns_target_entity() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = spawn_window_entity(&world);

        // 事前条件: Entity は生存している。
        assert!(
            world.borrow().world().get_entity(entity).is_ok(),
            "spawn 直後は Entity が生存しているべき"
        );

        let ret = WM_CLOSE(
            &world,
            entity,
            HWND(std::ptr::null_mut()),
            WPARAM(0),
            LPARAM(0),
        );

        // 既定 DestroyWindow 抑止のため Some(LRESULT(0))。
        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "WM_CLOSE は既定破棄抑止の Some(LRESULT(0)) を返すべき"
        );

        // 反転: 除去要求として Entity が despawn されている。
        assert!(
            world.borrow().world().get_entity(entity).is_err(),
            "WM_CLOSE 後は対象 Entity が despawn されているべき（除去要求）"
        );
    }

    /// world が既に `borrow_mut` 保持中でも WM_CLOSE はパニックせず safe-skip する
    /// （`try_borrow_mut` による同期再入の二重借用回避・要件 2.3）。
    /// 借用失敗のため Entity は残存する。
    #[test]
    fn wm_close_safe_skips_when_world_already_borrowed() {
        let world = Rc::new(RefCell::new(EcsWorld::new()));
        let entity = spawn_window_entity(&world);

        // world を borrow_mut 保持したまま WM_CLOSE を呼ぶ（再入相当）。
        let guard = world.borrow_mut();
        let ret = WM_CLOSE(
            &world,
            entity,
            HWND(std::ptr::null_mut()),
            WPARAM(0),
            LPARAM(0),
        );
        drop(guard);

        // パニックせず Some(LRESULT(0)) を返す。
        assert_eq!(
            ret,
            Some(LRESULT(0)),
            "再入時も Some(LRESULT(0)) を返す（safe-skip）"
        );

        // try_borrow_mut 失敗のため despawn されず Entity は残存。
        assert!(
            world.borrow().world().get_entity(entity).is_ok(),
            "借用中の再入では despawn されず Entity が残存するべき"
        );
    }
}
