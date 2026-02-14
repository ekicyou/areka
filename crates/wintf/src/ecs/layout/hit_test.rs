//! # Hit Test System - ヒットテストAPI
//!
//! このモジュールは、画面座標からエンティティを特定するヒットテストシステムを提供します。
//!
//! ## 主要機能
//!
//! ### HitTestMode / HitTest コンポーネント
//! エンティティのヒットテスト動作を設定します。
//!
//! ### hit_test_entity
//! 単一エンティティのヒットテストを実行します。
//!
//! ### hit_test
//! ルート配下を走査してスクリーン座標でヒットテストを実行します。
//!
//! ### hit_test_in_window
//! ウィンドウクライアント座標でヒットテストを実行します。
//!
//! # 例
//!
//! ```rust,ignore
//! use wintf::ecs::layout::{hit_test, HitTest, HitTestMode};
//!
//! // ヒットテスト対象外として設定
//! commands.spawn((Arrangement::default(), HitTest::none()));
//!
//! // ルートからヒットテスト実行
//! if let Some(entity) = hit_test(world, root, PhysicalPoint::new(100.0, 200.0)) {
//!     println!("Hit: {:?}", entity);
//! }
//! ```

use bevy_ecs::prelude::*;
use tracing::warn;

use super::hit_region::HitRegionMap;
use super::{Arrangement, D2DRectExt, GlobalArrangement};
use crate::ecs::WindowPos;
use crate::ecs::common::DepthFirstReversePostOrder;

// ============================================================================
// PhysicalPoint - 物理ピクセル座標型
// ============================================================================

/// 物理ピクセル座標（スクリーン座標系）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalPoint {
    pub x: f32,
    pub y: f32,
}

impl PhysicalPoint {
    /// 新しい座標を作成
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

// ============================================================================
// HitTestMode - ヒットテストモード
// ============================================================================

/// ヒットテストの動作モード
///
/// エンティティがマウスイベントにどう反応するかを定義します。
///
/// # 例
/// ```rust,ignore
/// use wintf::ecs::layout::{HitTest, HitTestMode};
///
/// // ヒットテスト対象外
/// let hit_test = HitTest { mode: HitTestMode::None };
///
/// // 矩形領域でヒットテスト（デフォルト）
/// let hit_test = HitTest::bounds();
///
/// // αマスクによるピクセル単位ヒットテスト
/// let hit_test = HitTest::alpha_mask();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HitTestMode {
    /// ヒットテスト対象外（マウスイベントを透過）
    None,
    /// バウンディングボックス（GlobalArrangement.bounds）でヒットテスト
    #[default]
    Bounds,
    /// αマスクによるピクセル単位ヒットテスト
    AlphaMask,
    /// 名前付きヒット領域（矩形・多角形・カラーマップ）による部位判定
    NamedRegions,
}

// ============================================================================
// HitTest コンポーネント
// ============================================================================

/// ヒットテスト設定コンポーネント
///
/// エンティティのヒットテスト動作を設定します。
/// このコンポーネントが付与されていない場合、デフォルトで `HitTestMode::Bounds` として扱われます。
///
/// # 例
/// ```rust,ignore
/// use wintf::ecs::layout::HitTest;
///
/// // ヒットテスト対象外として作成
/// commands.spawn((Arrangement::default(), HitTest::none()));
///
/// // 矩形領域でヒットテスト（デフォルト）
/// commands.spawn((Arrangement::default(), HitTest::bounds()));
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitTest {
    pub mode: HitTestMode,
}

impl HitTest {
    /// ヒットテスト対象外として作成
    pub fn none() -> Self {
        Self {
            mode: HitTestMode::None,
        }
    }

    /// バウンディングボックスでヒットテスト（デフォルト）
    pub fn bounds() -> Self {
        Self {
            mode: HitTestMode::Bounds,
        }
    }

    /// αマスクによるピクセル単位ヒットテスト
    pub fn alpha_mask() -> Self {
        Self {
            mode: HitTestMode::AlphaMask,
        }
    }

    /// 名前付きヒット領域によるヒットテスト
    pub fn named_regions() -> Self {
        Self {
            mode: HitTestMode::NamedRegions,
        }
    }
}

// ============================================================================
// hit_test_entity - 単一エンティティヒットテスト
// ============================================================================

/// 単一エンティティのヒットテスト
///
/// 指定エンティティのみを判定対象とします（子孫は走査しません）。
///
/// # Arguments
/// - `world`: ECS World 参照
/// - `entity`: 判定対象エンティティ
/// - `point`: スクリーン座標（物理ピクセル）
///
/// # Returns
/// - `true`: ヒット
/// - `false`: ヒットしない、または GlobalArrangement がない
///
/// # Note
/// `HitTest` コンポーネントがない場合は `HitTestMode::Bounds` として扱います。
///
/// # AlphaMask判定
/// `HitTestMode::AlphaMask` の場合:
/// 1. まず矩形判定（早期リターン）
/// 2. BitmapSourceResource.alpha_mask を取得
/// 3. 座標変換（スクリーン → マスク座標）
/// 4. AlphaMask.is_hit() 呼び出し
/// 5. αマスク未生成時は矩形判定にフォールバック
pub fn hit_test_entity(world: &World, entity: Entity, point: PhysicalPoint) -> bool {
    use crate::ecs::widget::bitmap_source::BitmapSourceResource;

    // HitTest コンポーネントを取得（なければデフォルト = Bounds）
    let mode = world
        .get::<HitTest>(entity)
        .map(|h| h.mode)
        .unwrap_or(HitTestMode::Bounds);

    // HitTestMode::None の場合はヒットしない
    if mode == HitTestMode::None {
        return false;
    }

    // GlobalArrangement を取得（なければヒットしない）
    let Some(global) = world.get::<GlobalArrangement>(entity) else {
        return false;
    };

    // まず矩形判定（全モード共通の早期リターン）
    if !global.bounds.contains(point.x, point.y) {
        return false;
    }

    // HitTestMode::Bounds の場合は矩形判定のみ
    if mode == HitTestMode::Bounds {
        return true;
    }

    // HitTestMode::AlphaMask の場合
    // BitmapSourceResource を取得
    let Some(resource) = world.get::<BitmapSourceResource>(entity) else {
        // BitmapSourceResource がない場合は矩形判定にフォールバック
        return true;
    };

    // αマスクを取得
    let Some(alpha_mask) = resource.alpha_mask() else {
        // αマスク未生成の場合は矩形判定にフォールバック
        return true;
    };

    // スクリーン座標 → マスク座標への変換
    let bounds = &global.bounds;
    let bounds_width = bounds.right - bounds.left;
    let bounds_height = bounds.bottom - bounds.top;

    if bounds_width <= 0.0 || bounds_height <= 0.0 {
        return true; // サイズが0以下の場合はフォールバック
    }

    // 相対座標を計算（0.0〜1.0）
    let rel_x = (point.x - bounds.left) / bounds_width;
    let rel_y = (point.y - bounds.top) / bounds_height;

    // マスク座標に変換（切り捨て、範囲チェックはis_hit内で行う）
    let mask_x = (rel_x * alpha_mask.width() as f32) as u32;
    let mask_y = (rel_y * alpha_mask.height() as f32) as u32;

    // αマスクで判定
    alpha_mask.is_hit(mask_x, mask_y)
}

// ============================================================================
// HitTestResult - 拡張ヒットテスト結果
// ============================================================================

/// 拡張ヒットテスト結果
///
/// ヒットしたエンティティと、名前付きリージョン（NamedRegions モード時）を含む。
#[derive(Debug, Clone)]
pub struct HitTestResult {
    /// ヒットしたエンティティ
    pub entity: Entity,
    /// ヒットした領域名（`NamedRegions` モード時のみ、`None` は無名ヒット）
    pub region: Option<String>,
}

// ============================================================================
// RegionHit - 内部判定結果
// ============================================================================

/// hit_test_entity_ex の内部返値
pub(crate) enum RegionHit {
    /// エンティティにヒットしなかった
    Miss,
    /// エンティティにヒットした（リージョン名は Option）
    Hit(Option<String>),
}

// ============================================================================
// hit_test_entity_ex - 単一エンティティ拡張ヒットテスト
// ============================================================================

/// 単一エンティティの拡張ヒットテスト
///
/// 既存 `hit_test_entity` と同等だが、`NamedRegions` モード時にリージョン名も返す。
///
/// # Arguments
/// - `world`: ECS World 参照
/// - `entity`: 判定対象エンティティ
/// - `point`: スクリーン座標（物理ピクセル）
///
/// # Returns
/// - `RegionHit::Hit(Some(name))`: 名前付き領域にヒット
/// - `RegionHit::Hit(None)`: エンティティにヒット（無名）
/// - `RegionHit::Miss`: ヒットしない
pub(crate) fn hit_test_entity_ex(world: &World, entity: Entity, point: PhysicalPoint) -> RegionHit {
    use crate::ecs::widget::bitmap_source::BitmapSourceResource;

    // HitTest コンポーネントを取得（なければデフォルト = Bounds）
    let mode = world
        .get::<HitTest>(entity)
        .map(|h| h.mode)
        .unwrap_or(HitTestMode::Bounds);

    // HitTestMode::None の場合はヒットしない
    if mode == HitTestMode::None {
        return RegionHit::Miss;
    }

    // GlobalArrangement を取得（なければヒットしない）
    let Some(global) = world.get::<GlobalArrangement>(entity) else {
        return RegionHit::Miss;
    };

    // まず矩形判定（全モード共通の早期リターン）
    if !global.bounds.contains(point.x, point.y) {
        return RegionHit::Miss;
    }

    // モード別判定
    match mode {
        HitTestMode::None => unreachable!(),
        HitTestMode::Bounds => RegionHit::Hit(None),
        HitTestMode::AlphaMask => {
            // BitmapSourceResource を取得
            let Some(resource) = world.get::<BitmapSourceResource>(entity) else {
                return RegionHit::Hit(None); // フォールバック
            };
            let Some(alpha_mask) = resource.alpha_mask() else {
                return RegionHit::Hit(None); // フォールバック
            };

            let bounds = &global.bounds;
            let bounds_width = bounds.right - bounds.left;
            let bounds_height = bounds.bottom - bounds.top;

            if bounds_width <= 0.0 || bounds_height <= 0.0 {
                return RegionHit::Hit(None);
            }

            let rel_x = (point.x - bounds.left) / bounds_width;
            let rel_y = (point.y - bounds.top) / bounds_height;
            let mask_x = (rel_x * alpha_mask.width() as f32) as u32;
            let mask_y = (rel_y * alpha_mask.height() as f32) as u32;

            if alpha_mask.is_hit(mask_x, mask_y) {
                RegionHit::Hit(None)
            } else {
                RegionHit::Miss
            }
        }
        HitTestMode::NamedRegions => {
            // HitRegionMap を取得
            let Some(region_map) = world.get::<HitRegionMap>(entity) else {
                // HitRegionMap 不在時は Bounds フォールバック（1.3）
                warn!(
                    entity = ?entity,
                    "NamedRegions モードですが HitRegionMap がありません。Bounds にフォールバックします"
                );
                return RegionHit::Hit(None);
            };

            // 正規化座標を計算
            let bounds = &global.bounds;
            let bounds_width = bounds.right - bounds.left;
            let bounds_height = bounds.bottom - bounds.top;

            if bounds_width <= 0.0 || bounds_height <= 0.0 {
                return RegionHit::Hit(None);
            }

            let rel_x = (point.x - bounds.left) / bounds_width;
            let rel_y = (point.y - bounds.top) / bounds_height;

            // Arrangement.size を取得（DIPサイズ）
            // Arrangement がない場合は bounds サイズをフォールバック値として使用
            let entity_size =
                world
                    .get::<Arrangement>(entity)
                    .map(|a| a.size)
                    .unwrap_or(super::Size {
                        width: bounds_width,
                        height: bounds_height,
                    });

            // HitRegionMap で判定
            let region_name = region_map.hit_test_region(rel_x, rel_y, &entity_size);

            RegionHit::Hit(region_name.map(|s| s.to_string()))
        }
    }
}

// ============================================================================
// hit_test - ツリー走査ヒットテスト
// ============================================================================

/// 指定ルートエンティティ配下でスクリーン座標によるヒットテストを実行
///
/// # Arguments
/// - `world`: ECS World 参照
/// - `root`: 検索ルートエンティティ（LayoutRoot, Window, 任意のサブツリー）
/// - `screen_point`: スクリーン座標（物理ピクセル）
///
/// # Returns
/// - `Some(Entity)`: ヒットした最前面エンティティ
/// - `None`: ヒットなし
///
/// # Algorithm
/// 深さ優先・逆順・後順走査で最前面エンティティを優先
pub fn hit_test(world: &World, root: Entity, screen_point: PhysicalPoint) -> Option<Entity> {
    let mut traversal = DepthFirstReversePostOrder::new(root);

    while let Some(entity) = traversal.next(world) {
        if hit_test_entity(world, entity, screen_point) {
            return Some(entity);
        }
    }

    None
}

// ============================================================================
// hit_test_in_window - ウィンドウ座標ヒットテスト
// ============================================================================

/// 指定Window内でクライアント座標によるヒットテストを実行
///
/// # Arguments
/// - `world`: ECS World 参照
/// - `window`: Window エンティティ（WindowPos を持つ）
/// - `client_point`: ウィンドウクライアント座標（物理ピクセル）
///
/// # Returns
/// - `Some(Entity)`: ヒットした最前面エンティティ
/// - `None`: ヒットなしまたは WindowPos が存在しない
pub fn hit_test_in_window(
    world: &World,
    window: Entity,
    client_point: PhysicalPoint,
) -> Option<Entity> {
    // WindowPos を取得（なければ None）
    let window_pos = world.get::<WindowPos>(window)?;

    // position が None の場合も None を返す
    let position = window_pos.position?;

    // クライアント座標 + ウィンドウ位置 = スクリーン座標
    let screen_point = PhysicalPoint::new(
        client_point.x + position.x as f32,
        client_point.y + position.y as f32,
    );

    // hit_test に委譲
    hit_test(world, window, screen_point)
}

// ============================================================================
// hit_test_ex - ツリー走査拡張ヒットテスト
// ============================================================================

/// 指定ルートエンティティ配下でスクリーン座標による拡張ヒットテストを実行
///
/// 既存 `hit_test` と同じ走査だが、`HitTestResult`（エンティティ＋リージョン名）を返す。
///
/// # Arguments
/// - `world`: ECS World 参照
/// - `root`: 検索ルートエンティティ
/// - `screen_point`: スクリーン座標（物理ピクセル）
///
/// # Returns
/// - `Some(HitTestResult)`: ヒットした最前面エンティティと領域名
/// - `None`: ヒットなし
pub fn hit_test_ex(
    world: &World,
    root: Entity,
    screen_point: PhysicalPoint,
) -> Option<HitTestResult> {
    let mut traversal = DepthFirstReversePostOrder::new(root);

    while let Some(entity) = traversal.next(world) {
        match hit_test_entity_ex(world, entity, screen_point) {
            RegionHit::Hit(region) => {
                return Some(HitTestResult { entity, region });
            }
            RegionHit::Miss => continue,
        }
    }

    None
}

// ============================================================================
// hit_test_in_window_ex - ウィンドウ座標拡張ヒットテスト
// ============================================================================

/// 指定Window内でクライアント座標による拡張ヒットテストを実行
///
/// # Arguments
/// - `world`: ECS World 参照
/// - `window`: Window エンティティ（WindowPos を持つ）
/// - `client_point`: ウィンドウクライアント座標（物理ピクセル）
///
/// # Returns
/// - `Some(HitTestResult)`: ヒットした最前面エンティティと領域名
/// - `None`: ヒットなしまたは WindowPos が存在しない
pub fn hit_test_in_window_ex(
    world: &World,
    window: Entity,
    client_point: PhysicalPoint,
) -> Option<HitTestResult> {
    let window_pos = world.get::<WindowPos>(window)?;
    let position = window_pos.position?;

    let screen_point = PhysicalPoint::new(
        client_point.x + position.x as f32,
        client_point.y + position.y as f32,
    );

    hit_test_ex(world, window, screen_point)
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;
    use windows::Win32::Foundation::{POINT, SIZE};
    use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
    use windows_numerics::Matrix3x2;

    /// テスト用ヘルパー: 指定した bounds を持つ GlobalArrangement を作成
    fn make_global_arrangement(left: f32, top: f32, right: f32, bottom: f32) -> GlobalArrangement {
        GlobalArrangement {
            transform: Matrix3x2::translation(left, top),
            bounds: D2D_RECT_F {
                left,
                top,
                right,
                bottom,
            },
        }
    }

    // ========================================================================
    // HitTestMode / HitTest テスト
    // ========================================================================

    #[test]
    fn test_hit_test_mode_default() {
        assert_eq!(HitTestMode::default(), HitTestMode::Bounds);
    }

    #[test]
    fn test_hit_test_none_constructor() {
        let hit_test = HitTest::none();
        assert_eq!(hit_test.mode, HitTestMode::None);
    }

    #[test]
    fn test_hit_test_bounds_constructor() {
        let hit_test = HitTest::bounds();
        assert_eq!(hit_test.mode, HitTestMode::Bounds);
    }

    #[test]
    fn test_hit_test_default() {
        let hit_test = HitTest::default();
        assert_eq!(hit_test.mode, HitTestMode::Bounds);
    }

    // ========================================================================
    // hit_test_entity テスト
    // ========================================================================

    /// ヒットあり（bounds 内）
    #[test]
    fn test_hit_test_entity_hit_inside() {
        let mut world = World::new();

        // GlobalArrangement を直接設定（伝播システムは使用しない）
        let entity = world
            .spawn((
                make_global_arrangement(10.0, 20.0, 110.0, 70.0), // 100x50
                HitTest::bounds(),
            ))
            .id();

        let point = PhysicalPoint::new(50.0, 40.0);
        assert!(hit_test_entity(&world, entity, point));
    }

    /// ヒットなし（bounds 外）
    #[test]
    fn test_hit_test_entity_miss_outside() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(10.0, 20.0, 110.0, 70.0),
                HitTest::bounds(),
            ))
            .id();

        // bounds外の座標
        let point = PhysicalPoint::new(200.0, 200.0);
        assert!(!hit_test_entity(&world, entity, point));
    }

    /// HitTestMode::None のスキップ
    #[test]
    fn test_hit_test_entity_skip_none_mode() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(10.0, 20.0, 110.0, 70.0),
                HitTest::none(), // ヒットテスト対象外
            ))
            .id();

        // bounds内だがHitTestMode::Noneなのでヒットしない
        let point = PhysicalPoint::new(50.0, 40.0);
        assert!(!hit_test_entity(&world, entity, point));
    }

    /// HitTest コンポーネントなし（デフォルト動作 = Bounds）
    #[test]
    fn test_hit_test_entity_no_component_defaults_to_bounds() {
        let mut world = World::new();

        let entity = world
            .spawn(make_global_arrangement(10.0, 20.0, 110.0, 70.0))
            .id();

        // HitTestコンポーネントなしでもBoundsとして扱う
        let point = PhysicalPoint::new(50.0, 40.0);
        assert!(hit_test_entity(&world, entity, point));
    }

    /// GlobalArrangement なしの場合
    #[test]
    fn test_hit_test_entity_no_global_arrangement() {
        let mut world = World::new();

        // GlobalArrangement なしのエンティティ
        let entity = world.spawn(HitTest::bounds()).id();

        let point = PhysicalPoint::new(50.0, 40.0);
        assert!(!hit_test_entity(&world, entity, point));
    }

    // ========================================================================
    // hit_test テスト
    // ========================================================================

    /// ヒットあり（最前面優先）
    #[test]
    fn test_hit_test_front_priority() {
        let mut world = World::new();

        // 背面エンティティ
        let back = world
            .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
            .id();

        // 前面エンティティ
        let front = world
            .spawn(make_global_arrangement(20.0, 20.0, 80.0, 80.0))
            .id();

        // ルートエンティティ
        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[back, front]);

        // 両方の bounds 内の座標
        let point = PhysicalPoint::new(50.0, 50.0);
        let result = hit_test(&world, root, point);

        // 最前面の front がヒットする
        assert_eq!(result, Some(front));
    }

    /// ヒットあり（背面のみ）
    #[test]
    fn test_hit_test_back_only() {
        let mut world = World::new();

        // 背面エンティティ
        let back = world
            .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
            .id();

        // 前面エンティティ
        let front = world
            .spawn(make_global_arrangement(50.0, 50.0, 100.0, 100.0))
            .id();

        // ルートエンティティ
        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[back, front]);

        // back のみの bounds 内の座標
        let point = PhysicalPoint::new(10.0, 10.0);
        let result = hit_test(&world, root, point);

        // back がヒットする
        assert_eq!(result, Some(back));
    }

    /// ヒットなし
    #[test]
    fn test_hit_test_none() {
        let mut world = World::new();

        let entity = world
            .spawn(make_global_arrangement(10.0, 10.0, 60.0, 60.0))
            .id();

        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[entity]);

        // 全ての bounds 外の座標
        let point = PhysicalPoint::new(200.0, 200.0);
        let result = hit_test(&world, root, point);

        assert_eq!(result, None);
    }

    /// HitTestMode::None のスキップ（子は引き続き調査）
    #[test]
    fn test_hit_test_skip_none_mode() {
        let mut world = World::new();

        // 背面エンティティ
        let back = world
            .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
            .id();

        // オーバーレイ（HitTestMode::None）
        let overlay = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::none(),
            ))
            .id();

        // ルートエンティティ（overlay が最前面）
        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[back, overlay]);

        let point = PhysicalPoint::new(50.0, 50.0);
        let result = hit_test(&world, root, point);

        // overlay はスキップされ、back がヒットする
        assert_eq!(result, Some(back));
    }

    /// 親子関係での優先度確認（子が親より優先）
    #[test]
    fn test_hit_test_child_priority_over_parent() {
        let mut world = World::new();

        // 親エンティティ
        let parent = world
            .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
            .id();

        // 子エンティティ
        let child = world
            .spawn(make_global_arrangement(10.0, 10.0, 90.0, 90.0))
            .id();

        world.entity_mut(parent).add_children(&[child]);

        // 両方の bounds 内の座標
        let point = PhysicalPoint::new(50.0, 50.0);
        let result = hit_test(&world, parent, point);

        // 子が親より優先
        assert_eq!(result, Some(child));
    }

    // ========================================================================
    // hit_test_in_window テスト
    // ========================================================================

    /// クライアント座標からスクリーン座標への変換確認
    #[test]
    fn test_hit_test_in_window_coordinate_conversion() {
        let mut world = World::new();

        // ウィンドウエンティティ
        let window = world
            .spawn((
                make_global_arrangement(100.0, 200.0, 500.0, 500.0),
                WindowPos {
                    position: Some(POINT { x: 100, y: 200 }),
                    size: Some(SIZE { cx: 400, cy: 300 }),
                    ..Default::default()
                },
            ))
            .id();

        // ウィンドウ内のウィジェット（スクリーン座標）
        let widget = world
            .spawn(make_global_arrangement(150.0, 250.0, 250.0, 300.0)) // 100x50
            .id();

        world.entity_mut(window).add_children(&[widget]);

        // クライアント座標 (50, 50) → スクリーン座標 (150, 250)
        let client_point = PhysicalPoint::new(50.0, 50.0);
        let result = hit_test_in_window(&world, window, client_point);

        assert_eq!(result, Some(widget));
    }

    /// WindowPos なしの場合の None 返却
    #[test]
    fn test_hit_test_in_window_no_window_pos() {
        let mut world = World::new();

        // WindowPos なしのエンティティ
        let window = world
            .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
            .id();

        let client_point = PhysicalPoint::new(50.0, 50.0);
        let result = hit_test_in_window(&world, window, client_point);

        assert_eq!(result, None);
    }

    /// WindowPos.position が None の場合
    #[test]
    fn test_hit_test_in_window_position_none() {
        let mut world = World::new();

        let window = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                WindowPos {
                    position: None, // position が None
                    ..Default::default()
                },
            ))
            .id();

        let client_point = PhysicalPoint::new(50.0, 50.0);
        let result = hit_test_in_window(&world, window, client_point);

        assert_eq!(result, None);
    }

    // ========================================================================
    // HitTestMode::AlphaMask テスト
    // ========================================================================

    #[test]
    fn test_hit_test_alpha_mask_constructor() {
        let hit_test = HitTest::alpha_mask();
        assert_eq!(hit_test.mode, HitTestMode::AlphaMask);
    }

    /// αマスク未設定時は矩形判定にフォールバック
    #[test]
    fn test_hit_test_alpha_mask_fallback_no_bitmap_source() {
        let mut world = World::new();

        // BitmapSourceResourceなし、αマスクモード
        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::alpha_mask(),
            ))
            .id();

        // BitmapSourceResourceがないので矩形判定にフォールバック
        let point = PhysicalPoint::new(50.0, 50.0);
        assert!(hit_test_entity(&world, entity, point));
    }

    /// αマスク未生成時は矩形判定にフォールバック
    #[test]
    fn test_hit_test_alpha_mask_fallback_no_mask() {
        let mut world = World::new();

        // BitmapSourceResourceはあるがαマスク未生成
        // Note: 実際のIWICBitmapSourceを作成するのは困難なため、
        // ここではBitmapSourceResourceなしの場合と同じくフォールバックを確認

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::alpha_mask(),
            ))
            .id();

        let point = PhysicalPoint::new(50.0, 50.0);
        // フォールバックでヒット
        assert!(hit_test_entity(&world, entity, point));
    }

    /// αマスクモードでも矩形外はヒットしない
    #[test]
    fn test_hit_test_alpha_mask_outside_bounds() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::alpha_mask(),
            ))
            .id();

        // bounds外の座標
        let point = PhysicalPoint::new(200.0, 200.0);
        assert!(!hit_test_entity(&world, entity, point));
    }

    // ========================================================================
    // HitTestMode::NamedRegions テスト
    // ========================================================================

    #[test]
    fn test_hit_test_named_regions_constructor() {
        let hit_test = HitTest::named_regions();
        assert_eq!(hit_test.mode, HitTestMode::NamedRegions);
    }

    // ========================================================================
    // hit_test_entity_ex テスト
    // ========================================================================

    /// NamedRegions + HitRegionMap 矩形領域ヒット
    #[test]
    fn test_hit_test_entity_ex_named_regions_rect_hit() {
        let mut world = World::new();

        let region_map = HitRegionMap::builder()
            .rect("head", 20.0, 0.0, 60.0, 40.0) // DIP: (20,0)-(80,40) on 100x100
            .build()
            .unwrap();

        // Arrangement の on_add hookが GlobalArrangement を上書きするため、
        // GlobalArrangement のみをセットし、Arrangement.size は bounds からフォールバック
        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::named_regions(),
                region_map,
            ))
            .id();

        // local (50, 20) → screen (50, 20) — "head" 領域内
        let point = PhysicalPoint::new(50.0, 20.0);
        match hit_test_entity_ex(&world, entity, point) {
            RegionHit::Hit(Some(name)) => assert_eq!(name, "head"),
            other => panic!(
                "Expected Hit(Some('head')), got {:?}",
                match other {
                    RegionHit::Miss => "Miss",
                    RegionHit::Hit(None) => "Hit(None)",
                    RegionHit::Hit(Some(_)) => "Hit(Some(...))",
                }
            ),
        }
    }

    /// NamedRegions + HitRegionMap 領域外（bounds内、領域外 → 無名ヒット）
    #[test]
    fn test_hit_test_entity_ex_named_regions_no_region() {
        let mut world = World::new();

        let region_map = HitRegionMap::builder()
            .rect("head", 20.0, 0.0, 60.0, 40.0)
            .build()
            .unwrap();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::named_regions(),
                region_map,
            ))
            .id();

        // local (5, 80) — bounds内だが領域外
        let point = PhysicalPoint::new(5.0, 80.0);
        match hit_test_entity_ex(&world, entity, point) {
            RegionHit::Hit(None) => {} // 期待通り: 無名ヒット
            other => panic!(
                "Expected Hit(None), got {:?}",
                match other {
                    RegionHit::Miss => "Miss",
                    RegionHit::Hit(None) => "Hit(None)",
                    RegionHit::Hit(Some(ref s)) => s.as_str(),
                }
            ),
        }
    }

    /// NamedRegions + HitRegionMap 不在 → Bounds フォールバック (1.3)
    #[test]
    fn test_hit_test_entity_ex_named_regions_no_region_map_fallback() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::named_regions(),
                // HitRegionMap なし
            ))
            .id();

        let point = PhysicalPoint::new(50.0, 50.0);
        match hit_test_entity_ex(&world, entity, point) {
            RegionHit::Hit(None) => {} // Bounds フォールバック
            other => panic!(
                "Expected Hit(None) (fallback), got {:?}",
                match other {
                    RegionHit::Miss => "Miss",
                    RegionHit::Hit(None) => "Hit(None)",
                    RegionHit::Hit(Some(ref s)) => s.as_str(),
                }
            ),
        }
    }

    /// hit_test_entity_ex: Bounds モードでは region: None
    #[test]
    fn test_hit_test_entity_ex_bounds_mode_region_none() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::bounds(),
            ))
            .id();

        let point = PhysicalPoint::new(50.0, 50.0);
        match hit_test_entity_ex(&world, entity, point) {
            RegionHit::Hit(None) => {}
            _ => panic!("Expected Hit(None) for Bounds mode"),
        }
    }

    /// hit_test_entity_ex: bounds外は Miss
    #[test]
    fn test_hit_test_entity_ex_outside_bounds() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::named_regions(),
            ))
            .id();

        let point = PhysicalPoint::new(200.0, 200.0);
        match hit_test_entity_ex(&world, entity, point) {
            RegionHit::Miss => {}
            _ => panic!("Expected Miss for outside bounds"),
        }
    }

    // ========================================================================
    // hit_test_ex テスト
    // ========================================================================

    /// hit_test_ex: ツリー走査で最前面エンティティとリージョン名を返す
    #[test]
    fn test_hit_test_ex_returns_region_name() {
        let mut world = World::new();

        let region_map = HitRegionMap::builder()
            .rect("head", 0.0, 0.0, 100.0, 50.0)
            .rect("body", 0.0, 50.0, 100.0, 50.0)
            .build()
            .unwrap();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::named_regions(),
                region_map,
            ))
            .id();

        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[entity]);

        // head 領域
        let result = hit_test_ex(&world, root, PhysicalPoint::new(50.0, 25.0));
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.entity, entity);
        assert_eq!(result.region.as_deref(), Some("head"));

        // body 領域
        let result = hit_test_ex(&world, root, PhysicalPoint::new(50.0, 75.0));
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.entity, entity);
        assert_eq!(result.region.as_deref(), Some("body"));
    }

    /// hit_test_ex: Bounds モードでは region: None
    #[test]
    fn test_hit_test_ex_bounds_mode() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::bounds(),
            ))
            .id();

        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[entity]);

        let result = hit_test_ex(&world, root, PhysicalPoint::new(50.0, 50.0));
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.entity, entity);
        assert_eq!(result.region, None);
    }

    /// hit_test_ex: 後方互換 — 既存 hit_test と同じエンティティを返す
    #[test]
    fn test_hit_test_ex_backward_compat_same_entity() {
        let mut world = World::new();

        let back = world
            .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
            .id();
        let front = world
            .spawn(make_global_arrangement(20.0, 20.0, 80.0, 80.0))
            .id();

        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[back, front]);

        let point = PhysicalPoint::new(50.0, 50.0);

        let old_result = hit_test(&world, root, point);
        let new_result = hit_test_ex(&world, root, point);

        assert_eq!(old_result.unwrap(), new_result.unwrap().entity);
    }

    // ========================================================================
    // hit_test_in_window_ex テスト
    // ========================================================================

    /// hit_test_in_window_ex: クライアント座標変換 + リージョン名
    #[test]
    fn test_hit_test_in_window_ex_with_region() {
        let mut world = World::new();

        let region_map = HitRegionMap::builder()
            .rect("button", 0.0, 0.0, 100.0, 50.0)
            .build()
            .unwrap();

        let window = world
            .spawn((
                make_global_arrangement(100.0, 200.0, 500.0, 500.0),
                WindowPos {
                    position: Some(POINT { x: 100, y: 200 }),
                    size: Some(SIZE { cx: 400, cy: 300 }),
                    ..Default::default()
                },
            ))
            .id();

        let widget = world
            .spawn((
                make_global_arrangement(150.0, 250.0, 250.0, 300.0), // 100x50
                HitTest::named_regions(),
                region_map,
            ))
            .id();

        world.entity_mut(window).add_children(&[widget]);

        // クライアント (50, 50) → スクリーン (150, 250)
        let result = hit_test_in_window_ex(&world, window, PhysicalPoint::new(50.0, 50.0));
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.entity, widget);
        assert_eq!(result.region.as_deref(), Some("button"));
    }

    /// hit_test_in_window_ex: WindowPos なし → None
    #[test]
    fn test_hit_test_in_window_ex_no_window_pos() {
        let mut world = World::new();
        let window = world
            .spawn(make_global_arrangement(0.0, 0.0, 100.0, 100.0))
            .id();

        let result = hit_test_in_window_ex(&world, window, PhysicalPoint::new(50.0, 50.0));
        assert!(result.is_none());
    }

    /// 既存 hit_test / hit_test_in_window の後方互換性（NamedRegions以外の動作不変）
    #[test]
    fn test_existing_api_backward_compat() {
        let mut world = World::new();

        let entity = world
            .spawn((
                make_global_arrangement(0.0, 0.0, 100.0, 100.0),
                HitTest::bounds(),
            ))
            .id();

        let root = world.spawn_empty().id();
        world.entity_mut(root).add_children(&[entity]);

        // 既存 API は変更なく動作
        let result = hit_test(&world, root, PhysicalPoint::new(50.0, 50.0));
        assert_eq!(result, Some(entity));

        // bounds外
        let result = hit_test(&world, root, PhysicalPoint::new(200.0, 200.0));
        assert_eq!(result, None);
    }
}
