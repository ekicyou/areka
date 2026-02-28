//! DolaRuntime — dola アニメーションランタイムの bevy_ecs リソース。
//!
//! dola は必須依存。物理エンティティ（Spot、Balloon）がアニメーション制御に直接使用する。
//! FrameTime リソースの値（.0）を `update_dola_runtime` システムで渡して更新する。

use bevy_ecs::resource::Resource;
use dola::runtime::DolaRuntime as DolaRuntimeInner;

/// dola アニメーションランタイムをラップする bevy_ecs リソース。
///
/// 物理エンティティ（Spot、Balloon）がアニメーション制御に直接使用する。
/// FrameTime リソースの値（.0）を `update_dola_runtime` システムで渡して更新する。
#[derive(Resource)]
pub struct DolaRuntime {
    facade: DolaRuntimeInner,
}

// Safety: wintf は単一スレッド（Windows UI スレッド）でのみ動作する。
// dola::runtime::DolaRuntime 内部の Rc<DynamicValue> は
// 同一スレッドからしかアクセスされない。
unsafe impl Send for DolaRuntime {}
unsafe impl Sync for DolaRuntime {}

impl DolaRuntime {
    /// 新しい DolaRuntime を生成する。
    pub fn new() -> Self {
        Self {
            facade: DolaRuntimeInner::new(),
        }
    }

    /// dola ランタイムへの参照を取得
    pub fn facade(&self) -> &DolaRuntimeInner {
        &self.facade
    }

    /// dola ランタイムへの可変参照を取得
    pub fn facade_mut(&mut self) -> &mut DolaRuntimeInner {
        &mut self.facade
    }
}

impl Default for DolaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DolaRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DolaRuntime").finish_non_exhaustive()
    }
}
