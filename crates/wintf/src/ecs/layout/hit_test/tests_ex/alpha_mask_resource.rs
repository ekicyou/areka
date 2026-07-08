//! `AlphaMaskResource` 優先読みのヒットテスト（design Testing Strategy Unit #5・R2.2/2.3/2.5）
//!
//! - `AlphaMaskResource` あり → 最優先で使用（`BitmapSourceResource`・矩形フォールバックより優先）
//! - `AlphaMaskResource` なし → 従来 `BitmapSourceResource` → 矩形フォールバック（後方互換）
//! - 両エントリポイントを檻に含める: `hit_test_entity`（直接）／`hit_test_in_window`（`hit_test_entity_ex` 経由）

use super::super::*;
use super::make_global_arrangement;
use crate::ecs::WindowPos;
use crate::ecs::types::Point;
use crate::ecs::widget::bitmap_source::AlphaMask;
use bevy_ecs::world::World;

/// 2x2 PBGRA32 マスク（(0,0) のみ不透明、他は透明）を生成
fn mask_opaque_at_origin() -> AlphaMask {
    let pixels = vec![
        0, 0, 0, 255, // (0,0) 不透明
        0, 0, 0, 0, // (1,0) 透明
        0, 0, 0, 0, // (0,1) 透明
        0, 0, 0, 0, // (1,1) 透明
    ];
    AlphaMask::from_pbgra32(&pixels, 2, 2, 8)
}

/// 2x2 PBGRA32 マスク（全透明＝いかなる座標もミス）を生成
fn mask_fully_transparent() -> AlphaMask {
    let pixels = vec![0u8; 2 * 2 * 4];
    AlphaMask::from_pbgra32(&pixels, 2, 2, 8)
}

/// 2x2 PBGRA32 マスク（全不透明＝いかなる座標もヒット）を生成
fn mask_fully_opaque() -> AlphaMask {
    let pixels = vec![
        0, 0, 0, 255, //
        0, 0, 0, 255, //
        0, 0, 0, 255, //
        0, 0, 0, 255, //
    ];
    AlphaMask::from_pbgra32(&pixels, 2, 2, 8)
}

/// `AlphaMaskResource` を指定マスクで作成するヘルパ
fn alpha_mask_resource(mask: AlphaMask) -> AlphaMaskResource {
    let mut r = AlphaMaskResource::new();
    r.set(mask);
    r
}

// ========================================================================
// hit_test_entity（直接呼び）経路
// ========================================================================

/// `AlphaMaskResource` あり → 使用される（不透明座標でヒット）
#[test]
fn test_alpha_mask_resource_present_hit() {
    let mut world = World::new();
    // bounds 0..2 × 0..2、マスク 2x2（恒等マッピング）
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 2.0, 2.0),
            HitTest::alpha_mask(),
            alpha_mask_resource(mask_opaque_at_origin()),
        ))
        .id();

    // point (0.5,0.5) → rel(0.25,0.25) → mask(0,0)＝不透明 → ヒット
    let point = PhysicalPoint::new(0.5, 0.5);
    assert!(hit_test_entity(&world, entity, point));
}

/// `AlphaMaskResource` あり → 使用される（透明座標でミス）。
///
/// bounds 内なので `AlphaMaskResource` が無ければ矩形フォールバックで **ヒット** するはず。
/// 全透明マスクで **ミス** になることで、`AlphaMaskResource` が矩形フォールバックより
/// 優先して実際に読まれていることを証明する。
#[test]
fn test_alpha_mask_resource_present_miss_overrides_rect_fallback() {
    let mut world = World::new();
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 2.0, 2.0),
            HitTest::alpha_mask(),
            alpha_mask_resource(mask_fully_transparent()),
        ))
        .id();

    // bounds 内（矩形なら hit）だが全透明マスク → ミス
    let point = PhysicalPoint::new(0.5, 0.5);
    assert!(!hit_test_entity(&world, entity, point));
}

/// `AlphaMaskResource` なし → 従来経路（供給者不在なら矩形フォールバックでヒット・後方互換）
#[test]
fn test_alpha_mask_resource_absent_falls_back_to_rect() {
    let mut world = World::new();
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 2.0, 2.0),
            HitTest::alpha_mask(),
            // AlphaMaskResource / BitmapSourceResource いずれもなし
        ))
        .id();

    let point = PhysicalPoint::new(0.5, 0.5);
    // 供給者不在 → 矩形フォールバックでヒット（従来挙動不変）
    assert!(hit_test_entity(&world, entity, point));
}

/// bounds 外は `AlphaMaskResource` の有無に関わらずミス（早期リターン不変）
#[test]
fn test_alpha_mask_resource_outside_bounds_miss() {
    let mut world = World::new();
    let entity = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 2.0, 2.0),
            HitTest::alpha_mask(),
            alpha_mask_resource(mask_fully_opaque()),
        ))
        .id();

    // bounds 外
    let point = PhysicalPoint::new(100.0, 100.0);
    assert!(!hit_test_entity(&world, entity, point));
}

// ========================================================================
// hit_test_in_window（hit_test_entity_ex 経由）経路
//   clickthrough evaluate_targets → hit_test_in_window → hit_test_entity_ex
//   と同一経路を檻に含める（design Validation 明示要求）
// ========================================================================

/// window 経由でも `AlphaMaskResource` が使用される（不透明 → Some(surface)）
#[test]
fn test_alpha_mask_resource_via_window_hit() {
    let mut world = World::new();

    // 窓（position (0,0) → client==screen）・自身はヒット対象外
    let window = world
        .spawn(WindowPos {
            position: Some(Point { x: 0, y: 0 }),
            ..Default::default()
        })
        .id();

    let surface = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 2.0, 2.0),
            HitTest::alpha_mask(),
            alpha_mask_resource(mask_opaque_at_origin()),
        ))
        .id();
    world.entity_mut(window).add_children(&[surface]);

    // client (0.5,0.5) → screen (0.5,0.5) → mask(0,0)＝不透明
    let client = PhysicalPoint::new(0.5, 0.5);
    assert_eq!(hit_test_in_window(&world, window, client), Some(surface));
}

/// window 経由でも `AlphaMaskResource` が優先される（全透明 → None）。
///
/// `AlphaMaskResource` が無ければ矩形フォールバックで surface がヒットし `Some(surface)` に
/// なるはず。全透明マスクで `None` になることで `hit_test_entity_ex` 側でも優先読みが
/// 効いていることを証明する。
#[test]
fn test_alpha_mask_resource_via_window_miss_overrides_rect_fallback() {
    let mut world = World::new();

    let window = world
        .spawn(WindowPos {
            position: Some(Point { x: 0, y: 0 }),
            ..Default::default()
        })
        .id();

    let surface = world
        .spawn((
            make_global_arrangement(0.0, 0.0, 2.0, 2.0),
            HitTest::alpha_mask(),
            alpha_mask_resource(mask_fully_transparent()),
        ))
        .id();
    world.entity_mut(window).add_children(&[surface]);

    let client = PhysicalPoint::new(0.5, 0.5);
    // 全透明 → surface はミス、他にヒット対象なし → None
    assert_eq!(hit_test_in_window(&world, window, client), None);
}

// ========================================================================
// 優先度の一意証明: AlphaMaskResource vs BitmapSourceResource（両者不一致）
//   BitmapSourceResource.alpha_mask=全不透明（hit）／AlphaMaskResource=全透明（miss）
//   結果が miss なら AlphaMaskResource が BitmapSourceResource より優先と確定。
//   BitmapSourceResource は実 IWICBitmapSource を要するため COM 初期化下で構築。
// ========================================================================

/// COM を初期化して closure を実行するヘルパ（integration テストと同一パターン）
fn with_com_initialized<T, F: FnOnce() -> T>(f: F) -> T {
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    // World（BitmapSourceResource が保持する IWICBitmapSource を含む）は
    // closure 内で drop され、CoUninitialize より前に解放される。
    let result = f();
    unsafe {
        CoUninitialize();
    }
    result
}

/// 最小の 2x2 PBGRA32 `IWICBitmapSource` を生成（source フィールドの充足用・
/// hit-test は alpha_mask のみ参照するため中身は不問）
fn make_dummy_wic_source() -> windows::Win32::Graphics::Imaging::IWICBitmapSource {
    use crate::com::wic::wic_factory;
    use windows::Win32::Graphics::Imaging::{
        GUID_WICPixelFormat32bppPBGRA, IWICBitmapSource, WICBitmapCacheOnLoad,
    };
    use windows_core::Interface;

    let factory = wic_factory().expect("wic factory");
    let bitmap = unsafe {
        factory
            .CreateBitmap(2, 2, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
            .expect("create wic bitmap")
    };
    bitmap.cast::<IWICBitmapSource>().expect("cast to source")
}

/// `AlphaMaskResource`（全透明＝miss）と `BitmapSourceResource`（全不透明＝hit）が
/// 不一致のとき、`hit_test_entity` の結果は `AlphaMaskResource` に従う（miss）。
#[test]
fn test_alpha_mask_resource_priority_over_bitmap_source_direct() {
    use crate::ecs::widget::bitmap_source::BitmapSourceResource;

    let result = with_com_initialized(|| {
        let mut world = World::new();

        // BitmapSourceResource: source=dummy, alpha_mask=全不透明（この座標で hit）
        let mut bsr = BitmapSourceResource::new(make_dummy_wic_source());
        bsr.set_alpha_mask(mask_fully_opaque());

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 2.0, 2.0),
                HitTest::alpha_mask(),
                bsr,
                // AlphaMaskResource=全透明（この座標で miss）→ 優先されれば結果は miss
                alpha_mask_resource(mask_fully_transparent()),
            ))
            .id();

        let point = PhysicalPoint::new(0.5, 0.5);
        hit_test_entity(&world, entity, point)
    });

    // AlphaMaskResource（miss）が BitmapSourceResource（hit）より優先 → false
    assert!(
        !result,
        "AlphaMaskResource(miss) must take priority over BitmapSourceResource(hit)"
    );
}

/// 同じ不一致を `hit_test_in_window`（`hit_test_entity_ex` 経由）でも証明する。
#[test]
fn test_alpha_mask_resource_priority_over_bitmap_source_via_window() {
    use crate::ecs::widget::bitmap_source::BitmapSourceResource;

    let result = with_com_initialized(|| {
        let mut world = World::new();

        let window = world
            .spawn(WindowPos {
                position: Some(Point { x: 0, y: 0 }),
                ..Default::default()
            })
            .id();

        let mut bsr = BitmapSourceResource::new(make_dummy_wic_source());
        bsr.set_alpha_mask(mask_fully_opaque());

        let surface = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 2.0, 2.0),
                HitTest::alpha_mask(),
                bsr,
                alpha_mask_resource(mask_fully_transparent()),
            ))
            .id();
        world.entity_mut(window).add_children(&[surface]);

        let client = PhysicalPoint::new(0.5, 0.5);
        hit_test_in_window(&world, window, client)
    });

    // AlphaMaskResource(miss) 優先 → surface はヒットせず None
    assert_eq!(
        result, None,
        "AlphaMaskResource(miss) must take priority over BitmapSourceResource(hit) via _ex path"
    );
}
