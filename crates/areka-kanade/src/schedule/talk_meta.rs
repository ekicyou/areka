//! 運行表のトークの付帯情報（実行するゴーストの出口の付与）。

use super::Action;
use crate::msg::KanadeConfig;
use crate::talk::TalkRunner;

/// 返す一括の全部の再生の開始に、この kanade の実行するゴーストを押す。
pub(super) fn stamp_runner(config: &KanadeConfig, actions: &mut [Action]) {
    for action in actions {
        if let Action::StartTalk(start) = action {
            start.meta.runner = TalkRunner {
                name: config.ghost_name.clone(),
                place: config.ghost_place.clone(),
            };
        }
    }
}

#[cfg(test)]
#[path = "talk_meta_tests.rs"]
mod tests;
