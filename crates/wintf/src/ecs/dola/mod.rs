//! DolaAnimator — dola ランタイムの ECS Component ラッパー。
//!
//! `DolaRuntime` をエンティティごとに所有し、フレーム単位で一括 tick する。
//! `tick_dola_animators` System により全エンティティを `Res<FrameTime>` で駆動する。
//!
//! # 安全性
//!
//! `DolaRuntime` 内部に `Rc` を含むため `Send + Sync` を手動実装する。
//! wintf は単一スレッド（Windows UI スレッド）で動作し、
//! `tick_dola_animators` の `Query<&mut DolaAnimator>` が 1 tick 1 回・
//! 単一スレッドでの排他アクセスを型レベルで保証する。
//!
//! # balloon06 との関係
//!
//! balloon06 の `DolaBridgeResource` 設計を本 `DolaAnimator` Component 設計で上書きする。
//! `DolaBridgeResource` は `Resource`（ワールドに一つ）であったのに対し、
//! `DolaAnimator` は `Component`（エンティティごと）で、複数アニメーションの独立管理が可能。

use bevy_ecs::component::Component;
use bevy_ecs::system::{Query, Res};
use dola::runtime::{DolaRuntime, UpdateResult};

use super::graphics::FrameTime;

/// DolaRuntime の ECS Component ラッパー。
///
/// エンティティごとに独立した `DolaRuntime` を保持し、
/// `tick_dola_animators` System で一括更新される。
///
/// # 消費者パターン
///
/// ```rust,ignore
/// fn my_consumer(query: Query<&DolaAnimator>) {
///     for animator in query.iter() {
///         let result = animator.last_result();
///         for change in &result.changes {
///             // 変化した変数に応じて ECS Component を更新
///         }
///     }
/// }
/// ```
#[derive(Component)]
pub struct DolaAnimator {
    runtime: DolaRuntime,
}

// Safety: wintf は単一スレッド（Windows UI スレッド）でのみ動作する。
// DolaRuntime 内部の Rc は単一スレッド内でのみアクセスされる。
// tick_dola_animators の Query<&mut DolaAnimator> が 1 tick 1 回・
// 単一スレッドの排他アクセスを型レベルで保証する。
unsafe impl Send for DolaAnimator {}
unsafe impl Sync for DolaAnimator {}

impl DolaAnimator {
    /// 新しい DolaAnimator を生成する。
    pub fn new() -> Self {
        Self {
            runtime: DolaRuntime::new(),
        }
    }

    /// 既存の DolaRuntime で DolaAnimator を生成する。
    pub fn with_runtime(runtime: DolaRuntime) -> Self {
        Self { runtime }
    }

    /// 現在時刻まで内部状態を進行する。
    ///
    /// 通常は `tick_dola_animators` System から呼び出される。
    /// 消費者が直接呼び出す必要はない。
    pub fn tick(&mut self, current_time: f64) {
        self.runtime.tick(current_time);
    }

    /// 直前の `tick()` 結果を読み取り専用で返す。
    ///
    /// 次の `tick()` まで何度でも参照可能（冪等）。
    pub fn last_result(&self) -> &UpdateResult {
        self.runtime.last_result()
    }

    /// 内部 DolaRuntime への読み取り参照。
    ///
    /// ドキュメントロード状態やサブスクリプション確認に使用する。
    pub fn runtime(&self) -> &DolaRuntime {
        &self.runtime
    }
}

impl Default for DolaAnimator {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DolaAnimator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DolaAnimator")
            .field("has_runtime", &true)
            .finish()
    }
}

/// 全 DolaAnimator を一括 tick する System。
///
/// Update スケジュール先頭に配置し、消費者システムは
/// `.after(tick_dola_animators)` で順序依存を宣言する。
///
/// # スケジュール配置
///
/// ```rust,ignore
/// app.add_systems(Update, tick_dola_animators);
/// app.add_systems(Update, my_consumer.after(tick_dola_animators));
/// ```
pub fn tick_dola_animators(
    mut query: Query<&mut DolaAnimator>,
    frame_time: Res<FrameTime>,
) {
    for mut animator in query.iter_mut() {
        animator.tick(frame_time.0);
    }
}
