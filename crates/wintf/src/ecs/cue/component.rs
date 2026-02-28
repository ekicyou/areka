//! PendingCueSheet — 配送待ち CueSheet コンポーネント。

use bevy_ecs::component::Component;

use super::CueSheet;

/// 配送待ち CueSheet を保持する短命コンポーネント。
/// dispatch_pending_cue_sheets システムが消費し、同一エンティティに CueSheetTracker を付与する。
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PendingCueSheet {
    /// 配送する CueSheet
    pub sheet: CueSheet,
    /// 配送時のシート開始時刻（世界絶対時刻）
    pub start_time: f64,
}
