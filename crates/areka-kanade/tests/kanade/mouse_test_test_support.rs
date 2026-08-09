use super::{CallMethod, MouseEventKind, MouseInput, RecordedCall};

/// Move 入力を組む（`region` は不透明転写・`None`＝判定外）。
pub(super) fn move_input(x: i64, y: i64, scope: u32, region: Option<&str>) -> MouseInput {
    MouseInput {
        scope,
        x,
        y,
        region: region.map(str::to_string),
        kind: MouseEventKind::Move,
    }
}

/// 記録がマウス GET（`OnMouseMove`／`OnMouseDoubleClick`）か。
fn is_mouse_get(c: &RecordedCall) -> bool {
    c.method == CallMethod::Get && (c.id == "OnMouseMove" || c.id == "OnMouseDoubleClick")
}

/// 記録列からマウス GET のみを処理順に抽出する。
pub(super) fn mouse_gets(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded.iter().filter(|c| is_mouse_get(c)).collect()
}
