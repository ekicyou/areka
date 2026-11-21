//! `areka_runtime::balloon_visibility::phase` の代わりの親（子のテストの宣言だけ）。

use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_runtime::balloon_visibility::BalloonVisibilityState;
use areka_runtime::balloon_visibility::MeasurementOrigin;
use areka_runtime::balloon_visibility::VisibilityAction;
use areka_runtime::balloon_visibility::VisibilityLogEvent;
use areka_runtime::balloon_visibility::VisibilityObservations;
use areka_runtime::balloon_visibility::VisibilityTrigger;
use areka_runtime::balloon_visibility::decide;
use areka_runtime::balloon_visibility::phase::*;
use areka_runtime::frame::RuntimeWiring;
use areka_runtime::talk_lifecycle::TalkLifecycleSignal;
use areka_sakura::ActorKey;
use bevy_ecs::world::World;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
// 中で記録を出す私有の関数は渡し口から同じ名前で引く。
use areka_runtime::balloon_visibility::phase::test_support::{
    collect_observations, emit_visibility_logs, notify_timeout, observe_scope_boxes,
};

#[cfg(test)]
#[path = "balloon_visibility_phase_tests.rs"]
mod tests;
// 隠れたバルーンが次の台詞の最初の文字とともに現れること。
// 本物の文字の層と本物の観測の収集を通し、場面ごとの各フレームの行動と記録を見る決定論テスト。
#[cfg(test)]
#[path = "balloon_visibility_phase_reappear_tests.rs"]
mod reappear_tests;
// 観測と発行に箱をつなぐ配線。
#[cfg(test)]
#[path = "balloon_visibility_phase_box_tests.rs"]
mod box_tests;
// 時間切れの知らせを置き場のゴーストの kanade へ送る配線（送った・送れなかったの記録と切替の直後）。
#[cfg(test)]
#[path = "balloon_visibility_phase_notice_tests.rs"]
mod notice_tests;
