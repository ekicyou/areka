//! `mount.rs` テストの共有フィクスチャ（純 ECS の装着・GPU 不要）。
//!
//! 装着（[`VisualMount::attach`]）は COM を呼ばない純 ECS の spawn になったため、構造アサートを
//! 共有する複数のテストモジュール（`tests`／`visibility_tests`）へ GPU 無しの World を配る。
//! 表示記録には [`GraphicsCommandList::empty`] を渡す（値として扱えるため装着の構造は成立する）。

use super::*;

/// `w×h` 原寸・k=1 の窓へ `VisualMount::attach`（可視構築）した状態を組む共通フィクスチャ。
///
/// 返り値: (world, window entity, mount)。window は実 `Window` ではない素の entity
/// （純 ECS 構造アサートのため。owner Window 不在でも surface/slot の構造は成立する）。
pub(super) fn attach_fixture(w: u32, h: u32) -> (World, Entity, VisualMount) {
    attach_fixture_with_visibility(w, h, true, |_| {})
}

/// `attach_fixture` の初期可視性指定版。
///
/// `before_attach` は `VisualMount::attach` の**直前**に World へ触れる差し込み口
/// （observer 登録など。装着中の component 挿入を観測する用途）。
pub(super) fn attach_fixture_with_visibility(
    w: u32,
    h: u32,
    initially_visible: bool,
    before_attach: impl FnOnce(&mut World),
) -> (World, Entity, VisualMount) {
    let mut world = World::new();
    let window = world.spawn_empty().id();
    before_attach(&mut world);
    let mount = VisualMount::attach(
        &mut world,
        window,
        (w, h),
        ScaleRatio::ONE,
        &GraphicsCommandList::empty(),
        initially_visible,
    );

    (world, window, mount)
}

/// 親を**実 `Window`** にして `w×h` 原寸・拡大率 `k` で装着した状態を組む（GPU 不要）。
///
/// [`attach_fixture`] の親は素の entity ゆえ wintf の `Visual::on_add` は owner Window 判定で
/// 落ちる。本フィクスチャは親へ `Window::default()` を置き、同フックの連鎖挿入
/// （`VisualGraphics`／`SurfaceGraphics`／`SurfaceGraphicsDirty`）が**実際に起きる**側を組む。
/// `Window` の `on_add` は `LayoutRoot` 不在なら親付けを諦めるだけで、COM も HWND も作らない。
///
/// 返り値: (world, window entity, mount)。
pub(super) fn attach_fixture_under_window(
    w: u32,
    h: u32,
    k: ScaleRatio,
) -> (World, Entity, VisualMount) {
    let mut world = World::new();
    let window = world.spawn(wintf::ecs::Window::default()).id();
    world.flush();
    let mount = VisualMount::attach(
        &mut world,
        window,
        (w, h),
        k,
        &GraphicsCommandList::empty(),
        true,
    );

    (world, window, mount)
}
