use super::{CallMethod, MouseEventKind, MouseInput, RecordedCall};

/// マウスの入力を組む（`region` は不透明転写・`None`＝判定外）。翻訳の檻（`translate_test.rs`）も使う。
pub(crate) fn mouse_input(
    kind: MouseEventKind,
    x: i64,
    y: i64,
    scope: u32,
    region: Option<&str>,
) -> MouseInput {
    MouseInput {
        scope,
        x,
        y,
        region: region.map(str::to_string),
        kind,
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

/// 記録のどの呼び出しの ID も `OnMouse` で始まらないか（「送らない」の確かめ）。GET に限らないので、
/// 送らないはずのマウスが NOTIFY で出るように壊れても拾う。
pub(super) fn no_mouse_call(recorded: &[RecordedCall]) -> bool {
    !recorded.iter().any(|c| c.id.starts_with("OnMouse"))
}
