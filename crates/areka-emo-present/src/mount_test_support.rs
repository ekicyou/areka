//! `mount.rs` テストの共有フィクスチャ（WUC apartment・GraphicsCore・供給面・装着）。
//!
//! 実 GPU を用いる構造アサートを複数のテストモジュール（`tests`／`visibility_tests`）が
//! 共有するため、生成手順をここへ集約する（新規 dev-dep を持ち込まない）。

use super::*;

use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};
use wintf::com::wuc::create_dispatcher_queue_controller;
use wintf::ecs::GraphicsCore;

use crate::chain::SwapChainPresenter;

/// テスト用 WUC apartment / dispatcher（chain.rs / spike と同一方針）。
///
/// cargo test の各テストは専用スレッドで COM 未初期化ゆえ ASTA を第一候補・NONE を保険にする。
/// controller は Compositor より長寿命を要するため呼び出し側で保持する。
pub(super) fn make_dispatcher_and_compositor()
-> (windows::System::DispatcherQueueController, Compositor) {
    let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
        .or_else(|e_asta| create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta))
        .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
    let compositor = Compositor::new().expect("Compositor::new 失敗");
    (dq, compositor)
}

/// 生存させておくべき WUC/GPU リソース群（drop 順の都合でまとめて保持する）。
///
/// `ICompositionSurface` はスワップチェーンを内包する `SwapChainPresenter` に裏打ちされるため、
/// 装着後も presenter を保持する（構造アサートには描画不要だが破棄を防ぐ）。
#[allow(dead_code)]
pub(super) struct Guards {
    pub(super) dq: windows::System::DispatcherQueueController,
    pub(super) compositor: Compositor,
    pub(super) core: GraphicsCore,
    pub(super) presenter: SwapChainPresenter,
}

/// `w×h` の供給面を持つ窓へ `VisualMount::attach`（可視構築）した状態を組む共通フィクスチャ。
///
/// 返り値: (world, window entity, mount, 生存ガード)。window は実 `Window` ではない素の entity
/// （純 ECS 構造アサートのため。owner Window 不在でも surface/slot の構造は成立する）。
pub(super) fn attach_fixture(w: u32, h: u32) -> (World, Entity, VisualMount, Guards) {
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
) -> (World, Entity, VisualMount, Guards) {
    let (dq, compositor) = make_dispatcher_and_compositor();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
    let (presenter, surface) =
        SwapChainPresenter::new(&core, &compositor, w, h).expect("SwapChainPresenter::new 失敗");

    let mut world = World::new();
    let window = world.spawn_empty().id();
    before_attach(&mut world);
    let mount = VisualMount::attach(
        &mut world,
        window,
        &surface,
        &compositor,
        (w, h),
        initially_visible,
    )
    .expect("VisualMount::attach 失敗");

    (
        world,
        window,
        mount,
        Guards {
            dq,
            compositor,
            core,
            presenter,
        },
    )
}
