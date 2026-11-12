//! BitmapSource コンポーネント
//!
//! 画像パスを保持する論理コンポーネント。
//! on_add時にVisualとBitmapSourceGraphicsを自動挿入し、非同期読み込みを開始する。

use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::DeferredWorld;
use tracing::debug;

/// 画像表示ウィジェット（WIC BitmapSourceベース）
///
/// # Example
/// ```ignore
/// commands.spawn((
///     BitmapSource::new("assets/logo.png"),
///     BoxStyle {
///         size: Some(BoxSize {
///             width: Some(Dimension::Px(200.0)),
///             height: Some(Dimension::Px(100.0)),
///         }),
///         ..Default::default()
///     },
///     ChildOf(parent), // 親も BoxStyle（根は LayoutRoot）を持つこと
/// ));
/// ```
#[derive(Component, Debug, Clone)]
#[component(on_add = on_bitmap_source_add, on_remove = on_bitmap_source_remove)]
pub struct BitmapSource {
    /// 画像ファイルパス（相対または絶対）
    path: String,
}

impl BitmapSource {
    /// 新しいBitmapSourceコンポーネントを作成
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self { path: path.into() }
    }

    /// 画像ファイルパス
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// BitmapSource追加時のフック
///
/// - Visual + BitmapSourceGraphics + HitTest::alpha_mask()を、それぞれ無ければ入れる
/// - 非同期読み込みタスクを起動
fn on_bitmap_source_add(mut world: DeferredWorld, hook: HookContext) {
    use super::WintfTaskPool;
    use super::resource::BitmapSourceGraphics;
    use super::task_pool::BoxedCommand;
    use super::wic_core::WicCore;
    use crate::ecs::Visual;
    use crate::ecs::layout::HitTest;
    use tracing::warn;

    let entity = hook.entity;

    // 無ければ入れる（Rectangle/Labelパターン踏襲）。HitTest は利用者が先に入れていればそれを使う
    let has_visual = world.get::<Visual>(entity).is_some();
    let has_graphics = world.get::<BitmapSourceGraphics>(entity).is_some();
    let has_hit_test = world.get::<HitTest>(entity).is_some();
    let mut commands = world.commands();
    let mut entity_commands = commands.entity(entity);
    if !has_visual {
        entity_commands.insert(Visual::default());
    }
    if !has_graphics {
        entity_commands.insert(BitmapSourceGraphics::new());
    }
    if !has_hit_test {
        entity_commands.insert(HitTest::alpha_mask());
    }

    // WicCoreをcloneして取得
    let wic_core = match world.get_resource::<WicCore>() {
        Some(wic) => wic.clone(),
        None => {
            warn!("[BitmapSource] WicCore not found");
            return;
        }
    };

    // パスを取得
    let path = match world.get::<BitmapSource>(entity) {
        Some(bs) => bs.path().to_owned(),
        None => return,
    };

    // 非同期読み込みタスクを起動
    if let Some(task_pool) = world.get_resource::<WintfTaskPool>() {
        task_pool.spawn(move |tx| async move {
            // パス解決
            let resolved = match super::systems::resolve_path(&path) {
                Ok(p) => p,
                Err(e) => {
                    warn!("[BitmapSource] Failed to resolve path '{}': {:?}", path, e);
                    return;
                }
            };

            // 画像読み込み（解読はここで 1 回だけ）
            match super::systems::load_decoded_bitmap(wic_core.factory(), &resolved) {
                Ok(source) => {
                    use super::resource::BitmapSourceResource;
                    // BitmapSourceResourceにラップしてからクロージャに渡す
                    // (BitmapSourceResourceはSend実装済み、IWICBitmapSourceは未実装)
                    let resource = BitmapSourceResource::new(source);
                    let cmd: BoxedCommand = Box::new(move |world: &mut bevy_ecs::world::World| {
                        insert_loaded_resource(world, entity, &path, resource);
                    });
                    let _ = tx.send(cmd);
                }
                Err(e) => {
                    warn!(
                        "[BitmapSource] Failed to load '{}': {:?}",
                        resolved.display(),
                        e
                    );
                }
            }
        });
    }
}

/// 裏の作業で読んだ画像を部品へ入れる
///
/// 部品がまだ同じ `path` の `BitmapSource` を持つときだけ入れる。外した直後や
/// 別の画像に付け直した後に前の読み込みが遅れて着いた場合は捨てる。
pub(super) fn insert_loaded_resource(
    world: &mut bevy_ecs::world::World,
    entity: bevy_ecs::entity::Entity,
    path: &str,
    resource: super::resource::BitmapSourceResource,
) {
    let Ok(mut entity_ref) = world.get_entity_mut(entity) else {
        debug!(entity = ?entity, path, "[BitmapSource] Entity gone, dropping loaded image");
        return;
    };
    if entity_ref
        .get::<BitmapSource>()
        .is_some_and(|bs| bs.path() == path)
    {
        entity_ref.insert(resource);
    } else {
        debug!(entity = ?entity, path, "[BitmapSource] Image no longer attached, dropping loaded image");
    }
}

/// BitmapSource削除時のフック
///
/// 命令の列を空にし、D2D の画像を無効にし、画像の資源を外す。
/// `HitTest` は残す（利用者が入れたものかもしれない・マスクが無ければ当たりは矩形に戻る）。
fn on_bitmap_source_remove(mut world: DeferredWorld, hook: HookContext) {
    use super::resource::{BitmapSourceGraphics, BitmapSourceResource};
    use crate::ecs::GraphicsCommandList;
    use bevy_ecs::change_detection::DetectChangesMut;

    let entity = hook.entity;

    // GraphicsCommandListをクリア（Changed検出のため）
    if let Some(mut cmd_list) = world.get_mut::<GraphicsCommandList>(entity) {
        cmd_list.set_if_neq(GraphicsCommandList::empty());
    }
    if let Some(mut graphics) = world.get_mut::<BitmapSourceGraphics>(entity) {
        graphics.invalidate();
    }
    world
        .commands()
        .entity(entity)
        .try_remove::<BitmapSourceResource>();
}
