use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::{CompositionDrawingSurface, CompositionSurfaceBrush, Visual};
use windows::Win32::Graphics::Direct2D::*;

/// GPUリソースを使用するエンティティを宣言するマーカーコンポーネント
///
/// このコンポーネントは以下の役割を持つ：
/// 1. GPUリソースを使用するエンティティを宣言（存在自体がマーカー）
/// 2. `Changed<HasGraphicsResources>` でGPUリソース再初期化をトリガー
///
/// デバイスロスト時は `set_changed()` を呼び出すことで、
/// 各GPUリソースコンポーネント（VisualGraphics, WindowGraphics等）の
/// 再初期化システムをトリガーする。
///
/// # メモリ戦略
/// - **Table戦略**: 多数のエンティティに付与されるGPUリソース宣言マーカー
/// - デバイスロスト時にset_changed()で一括再初期化をトリガー
#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HasGraphicsResources;

#[derive(Debug)]
struct WindowGraphicsInner {
    pub target: DesktopWindowTarget,
    pub device_context: ID2D1DeviceContext,
}

/// ウィンドウごとのグラフィックスリソース
#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct WindowGraphics {
    inner: Option<WindowGraphicsInner>,
    generation: u32,
}

// SAFETY 条件: 保持する COM/WinRT ポインタの跨スレッド移動（Send）・参照共有（Sync）は
// 「同一オブジェクトへの呼び出しが同時に実行されない」ことを前提とする。
// device_context（D2D）は MULTI_THREADED ファクトリ系列のため内部同期されるが、
// target（WUC DesktopWindowTarget）は UI スレッドアフィニティを持ち、ECS スケジュール構成
// （&mut 排他または同一コンポーネントへ並行アクセスしないシステム配置）が安全性条件を担う。
unsafe impl Send for WindowGraphics {}
unsafe impl Sync for WindowGraphics {}

impl WindowGraphics {
    pub fn new(target: DesktopWindowTarget, device_context: ID2D1DeviceContext) -> Self {
        Self {
            inner: Some(WindowGraphicsInner {
                target,
                device_context,
            }),
            generation: 0,
        }
    }

    pub fn invalidate(&mut self) {
        self.inner = None;
    }

    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn increment_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn get_target(&self) -> Option<&DesktopWindowTarget> {
        self.inner.as_ref().map(|i| &i.target)
    }

    pub fn get_dc(&self) -> Option<&ID2D1DeviceContext> {
        self.inner.as_ref().map(|i| &i.device_context)
    }
}

/// ウィンドウのルートビジュアルノード (R9: parent_visualキャッシュ方式)
#[derive(Component)]
#[component(on_remove = on_visual_graphics_remove)]
pub struct VisualGraphics {
    inner: Option<Visual>,
    /// 親Visual参照（子削除用にキャッシュ）
    /// 階層同期時に子挿入と同時に設定される
    parent_visual: Option<Visual>,
}

// on_remove フック: 親Visualから自分を削除
fn on_visual_graphics_remove(world: DeferredWorld, hook: HookContext) {
    // 親Visualから自分を削除
    // エラーは無視（親が先に削除されている場合など）
    // DComp の `parent.remove_visual(visual)` は WUC では `parent.Children()?.Remove(visual)` に対応。
    // 基底 Visual は Children を持たないため ContainerVisual へ cast してから Children を得る。
    if let Some(vg) = world.get::<VisualGraphics>(hook.entity) {
        if let (Some(parent), Some(visual)) = (&vg.parent_visual, &vg.inner) {
            use windows::core::Interface;
            let _ = parent
                .cast::<windows::UI::Composition::ContainerVisual>()
                .and_then(|cv| cv.Children())
                .and_then(|c| c.Remove(visual)); // エラー無視
        }
    }
}

// SAFETY 条件: WindowGraphics と同様。WUC Visual への呼び出しが
// 同時に実行されないことを ECS スケジュール構成が担保する前提（UI スレッドアフィニティ）。
unsafe impl Send for VisualGraphics {}
unsafe impl Sync for VisualGraphics {}

impl std::fmt::Debug for VisualGraphics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisualGraphics")
            .field("inner", &self.inner.is_some())
            .field("parent_visual", &self.parent_visual.is_some())
            .finish()
    }
}

impl Default for VisualGraphics {
    /// 空のVisualGraphicsを作成（GPUリソースなし）
    /// Changed<VisualGraphics> + !is_valid() でシステムが検知してGPUリソースを作成
    fn default() -> Self {
        Self {
            inner: None,
            parent_visual: None,
        }
    }
}

impl VisualGraphics {
    pub fn new(visual: Visual) -> Self {
        Self {
            inner: Some(visual),
            parent_visual: None,
        }
    }

    /// 親Visualを指定してVisualGraphicsを作成
    pub fn new_with_parent(visual: Visual, parent_visual: Option<Visual>) -> Self {
        Self {
            inner: Some(visual),
            parent_visual,
        }
    }

    pub fn invalidate(&mut self) {
        self.inner = None;
        self.parent_visual = None;
    }

    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    /// WUC Visual への参照を取得する
    pub fn visual(&self) -> Option<&Visual> {
        self.inner.as_ref()
    }

    /// 親Visualへの参照を取得する
    pub fn parent_visual(&self) -> Option<&Visual> {
        self.parent_visual.as_ref()
    }

    /// 親Visualを設定/更新する
    pub fn set_parent_visual(&mut self, parent: Option<Visual>) {
        self.parent_visual = parent;
    }
}

/// ウィンドウの描画サーフェス
///
/// Note: 旧on_add/on_replaceフックは廃止され、
/// mark_dirty_surfacesシステムでAdded<SurfaceGraphics>として検出される
#[derive(Component, Debug, Default)]
pub struct SurfaceGraphics {
    inner: Option<CompositionDrawingSurface>,
    /// surface を束ねる SurfaceBrush（要件 6.3）。
    /// DComp は visual.SetContent(surface) 一段だったが、WUC は brush が一段挟まる。
    /// brush を保持し損ねると surface が解放され黒画像化するため、寿命をここで固定する。
    brush: Option<CompositionSurfaceBrush>,
    pub size: (u32, u32),
}

// SAFETY 条件: WindowGraphics と同様。CompositionDrawingSurface / CompositionSurfaceBrush への
// 呼び出しが同時に実行されないことを ECS スケジュール構成が担保する前提（UI スレッドアフィニティ）。
unsafe impl Send for SurfaceGraphics {}
unsafe impl Sync for SurfaceGraphics {}

impl SurfaceGraphics {
    pub fn new(
        surface: CompositionDrawingSurface,
        brush: CompositionSurfaceBrush,
        size: (u32, u32),
    ) -> Self {
        Self {
            inner: Some(surface),
            brush: Some(brush),
            size,
        }
    }

    pub fn invalidate(&mut self) {
        self.inner = None;
        self.brush = None;
        self.size = (0, 0);
    }

    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    /// CompositionDrawingSurface への参照を取得
    pub fn surface(&self) -> Option<&CompositionDrawingSurface> {
        self.inner.as_ref()
    }

    /// SurfaceBrush への参照を取得
    pub fn brush(&self) -> Option<&CompositionSurfaceBrush> {
        self.brush.as_ref()
    }

    /// Surface と brush を設定（既存のコンポーネントを直接更新）
    ///
    /// commands.insert()の代わりにこのメソッドを使用することで、
    /// 同一フレーム内で変更が即座に反映される。
    /// サイズが異なる場合のみ更新し、Changedフラグを適切に管理。
    pub fn set_surface(
        &mut self,
        surface: CompositionDrawingSurface,
        brush: CompositionSurfaceBrush,
        size: (u32, u32),
    ) {
        self.inner = Some(surface);
        self.brush = Some(brush);
        self.size = size;
    }

    /// Surface と brush をクリア（invalidateと同じだが意図を明確に）
    ///
    /// commands.remove()の代わりにこのメソッドを使用することで、
    /// コンポーネント自体は残したまま内容だけをクリアする。
    pub fn clear(&mut self) {
        self.inner = None;
        self.brush = None;
        self.size = (0, 0);
    }
}

/// SurfaceGraphicsがダーティ（再描画が必要）であることを示すコンポーネント
///
/// マーカーコンポーネント `SurfaceUpdateRequested` の置き換え。
/// `Changed<SurfaceGraphicsDirty>` パターンにより、アーキタイプ変更を排除し、
/// 同一スケジュール内での即時伝搬を実現する。
///
/// # メモリ戦略
/// - **Table戦略**: すべてのVisualに付与され、値更新でChanged検出する設計
/// - Visual.on_addで自動挿入、削除されない永続コンポーネント
/// - アーキタイプ変更を排除するための設計意図により、Table戦略が最適
#[derive(Component, Default, Debug, Clone, PartialEq)]
pub struct SurfaceGraphicsDirty {
    /// 最後に描画をリクエストしたフレーム番号
    pub requested_frame: u64,
}

// ========== Surface Creation Statistics ==========

/// デバッグビルド時のSurface生成統計リソース
///
/// Surface生成の最適化状況を把握するための統計情報を収集する。
/// 開発者がパフォーマンスチューニングやデバッグを行う際に使用。
///
/// # Requirements
/// - Req 5.3: デバッグビルド時のSurface生成統計
///
/// # Usage
/// ```ignore
/// let stats = world.resource::<SurfaceCreationStats>();
/// eprintln!("Created: {}, Skipped: {}", stats.created_count, stats.skipped_count);
/// ```
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct SurfaceCreationStats {
    /// 作成されたSurfaceの累計数
    pub created_count: u64,
    /// スキップされたSurface作成の累計数（CommandList不在等）
    pub skipped_count: u64,
    /// 削除されたSurfaceの累計数
    pub deleted_count: u64,
    /// リサイズされたSurfaceの累計数
    pub resize_count: u64,
}

impl SurfaceCreationStats {
    /// Surface作成を記録
    pub fn record_created(&mut self) {
        self.created_count += 1;
    }

    /// スキップを記録
    pub fn record_skipped(&mut self) {
        self.skipped_count += 1;
    }

    /// 削除を記録
    pub fn record_deleted(&mut self) {
        self.deleted_count += 1;
    }

    /// リサイズを記録
    pub fn record_resized(&mut self) {
        self.resize_count += 1;
    }
}
