use super::{CloseReason, ExecutionSnapshot, RecordedCall, events, expected_call};

/// 記録列中の OnClose GET（events 表導出・reason 指定）の初出インデックスを返す。
///
/// 通常握手の OnClose は talk 非アクティブ（`begin_close` が INACTIVE スナップショットで発行）ゆえ
/// Status 行なし。events 表から導出して照合する（References/Status をハードコードしない）。
pub(super) fn onclose_get_index(recorded: &[RecordedCall], reason: CloseReason) -> Option<usize> {
    let onclose = expected_call(events::on_close(reason, &ExecutionSnapshot::INACTIVE));
    recorded.iter().position(|c| *c == onclose)
}
