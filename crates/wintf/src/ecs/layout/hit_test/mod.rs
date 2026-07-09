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
use super::{Arrangement, D2DRect, D2DRectExt, GlobalArrangement};
use crate::ecs::WindowPos;
use crate::ecs::common::DepthFirstReversePostOrder;
use crate::ecs::types::PointF;
use crate::ecs::widget::bitmap_source::AlphaMask;

// ============================================================================
// PhysicalPoint — PointF への後方互換エイリアス
// ============================================================================

/// 後方互換性エイリアス（PhysicalPoint → PointF）
pub type PhysicalPoint = PointF;

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
// AlphaMaskResource - 汎用 αマスク CPU リソース
// ============================================================================

/// hit-test 用 αマスク CPU リソース（汎用供給口）
///
/// `BitmapSourceResource` 以外の供給者（例: emo-present の合成表示）が
/// `HitTestMode::AlphaMask` のヒットテストへ [`AlphaMask`] を渡すための
/// `Component`。`hit_test_entity` / `hit_test_entity_ex` の `AlphaMask` 分岐で
/// **最優先**に読まれ、不在時は従来どおり `BitmapSourceResource` → 矩形
/// フォールバックへ委ねる（既存挙動は完全後方互換）。
///
/// # 座標契約
/// マスク原寸は表示 surface の原寸（物理 px）＝ `GlobalArrangement.bounds` 寸に
/// 一致する運用を想定する。この場合 bounds 相対→マスク座標変換は恒等となり、
/// 任意 DPI でクリック座標が一致する（design DPI 表示契約・R2.5）。
///
/// # 命名
/// wintf の CPU リソース命名規約（`XxxResource`・`BitmapSourceResource` 等）に準拠。
#[derive(Component, Debug, Clone, Default)]
pub struct AlphaMaskResource {
    mask: Option<AlphaMask>,
}

impl AlphaMaskResource {
    /// 空（マスク未設定）の `AlphaMaskResource` を作成
    pub fn new() -> Self {
        Self { mask: None }
    }

    /// αマスクを設定（表示所有者が合成結果から更新）
    pub fn set(&mut self, mask: AlphaMask) {
        self.mask = Some(mask);
    }

    /// αマスクへの参照を取得（未設定なら `None`）
    pub fn mask(&self) -> Option<&AlphaMask> {
        self.mask.as_ref()
    }
}

// ============================================================================
// alpha_mask_hit - AlphaMask 読み取り + 座標変換 + 判定の共有ヘルパ
// ============================================================================

/// `HitTestMode::AlphaMask` 分岐の αマスク読み取り・座標変換・判定を
/// `hit_test_entity` と `hit_test_entity_ex` で共有する純粋ヘルパ。
///
/// # 読み取り優先順位
/// 1. [`AlphaMaskResource`]（最優先・汎用供給口）
/// 2. `BitmapSourceResource::alpha_mask()`（既存経路・後方互換）
/// 3. どちらも供給しない／マスク未生成 → 矩形フォールバック（bounds 内は常にヒット）
///
/// # 前提
/// 呼び出し側で bounds 内判定（早期リターン）済みであること。本ヘルパは bounds 相対
/// 比例→マスク座標変換のみを行い、変換ロジックは従来と同一（emo-present では
/// bounds==マスク原寸で恒等写像＝R2.5 の成立根拠）。
///
/// # Returns
/// - `true`: 当該座標のマスクピクセルがヒット（α ≧ 128）／フォールバック（供給者不在）
/// - `false`: 当該座標のマスクピクセルが透明（α < 128）
fn alpha_mask_hit(world: &World, entity: Entity, bounds: &D2DRect, point: PhysicalPoint) -> bool {
    use crate::ecs::widget::bitmap_source::BitmapSourceResource;

    // AlphaMaskResource を最優先で読み、無ければ BitmapSourceResource の αマスクへ委ねる。
    // どちらも供給しない場合は矩形フォールバック（従来挙動）。
    let mask = world
        .get::<AlphaMaskResource>(entity)
        .and_then(|r| r.mask())
        .or_else(|| {
            world
                .get::<BitmapSourceResource>(entity)
                .and_then(|r| r.alpha_mask())
        });

    let Some(alpha_mask) = mask else {
        // 供給者不在／マスク未生成 → 矩形判定にフォールバック
        return true;
    };

    // スクリーン座標 → マスク座標への変換（従来ロジックと同一）
    let bounds_width = bounds.right - bounds.left;
    let bounds_height = bounds.bottom - bounds.top;

    if bounds_width <= 0.0 || bounds_height <= 0.0 {
        return true; // サイズが0以下の場合はフォールバック
    }

    // 相対座標を計算（0.0〜1.0）
    let rel_x = (point.x - bounds.left) / bounds_width;
    let rel_y = (point.y - bounds.top) / bounds_height;

    // マスク座標に変換（切り捨て、範囲チェックは is_hit 内で行う）
    let mask_x = (rel_x * alpha_mask.width() as f32) as u32;
    let mask_y = (rel_y * alpha_mask.height() as f32) as u32;

    // αマスクで判定
    alpha_mask.is_hit(mask_x, mask_y)
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

    // HitTestMode::Bounds の場合は矩形判定 + α値判定
    if mode == HitTestMode::Bounds {
        // Opacity * Brushes.foreground.a の合成α値が ALPHA_THRESHOLD 未満なら透明と判定
        // AlphaMask の ALPHA_THRESHOLD (128/255 ≈ 0.502) と同一基準
        const ALPHA_THRESHOLD: f32 = 128.0 / 255.0;

        let opacity = world
            .get::<crate::ecs::graphics::Visual>(entity)
            .map(|v| v.clamped_opacity())
            .unwrap_or(1.0);

        let foreground_a = world
            .get::<crate::ecs::widget::brushes::Brushes>(entity)
            .and_then(|b| b.foreground.as_color())
            .map(|c| c.a)
            .unwrap_or_else(|| {
                crate::ecs::widget::brushes::DEFAULT_FOREGROUND
                    .as_color()
                    .map(|c| c.a)
                    .unwrap_or(1.0)
            });

        let composite_alpha = opacity * foreground_a;
        if composite_alpha < ALPHA_THRESHOLD {
            return false;
        }

        return true;
    }

    // HitTestMode::AlphaMask の場合
    // 共有ヘルパで αマスクを読み取り（AlphaMaskResource 最優先 → BitmapSourceResource → 矩形）
    alpha_mask_hit(world, entity, &global.bounds, point)
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
        HitTestMode::Bounds => {
            // Opacity * Brushes.foreground.a の合成α値が ALPHA_THRESHOLD 未満なら透明と判定
            const ALPHA_THRESHOLD: f32 = 128.0 / 255.0;

            let opacity = world
                .get::<crate::ecs::graphics::Visual>(entity)
                .map(|v| v.clamped_opacity())
                .unwrap_or(1.0);

            let foreground_a = world
                .get::<crate::ecs::widget::brushes::Brushes>(entity)
                .and_then(|b| b.foreground.as_color())
                .map(|c| c.a)
                .unwrap_or_else(|| {
                    crate::ecs::widget::brushes::DEFAULT_FOREGROUND
                        .as_color()
                        .map(|c| c.a)
                        .unwrap_or(1.0)
                });

            let composite_alpha = opacity * foreground_a;
            if composite_alpha < ALPHA_THRESHOLD {
                RegionHit::Miss
            } else {
                RegionHit::Hit(None)
            }
        }
        HitTestMode::AlphaMask => {
            // 共有ヘルパで αマスクを読み取り（AlphaMaskResource 最優先 → BitmapSourceResource → 矩形）。
            // hit_test_entity と同一挙動を保証（両エントリポイントの一致）。
            if alpha_mask_hit(world, entity, &global.bounds, point) {
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
        let hit = hit_test_entity(world, entity, screen_point);
        if hit {
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
mod tests;

#[cfg(test)]
mod tests_ex;
