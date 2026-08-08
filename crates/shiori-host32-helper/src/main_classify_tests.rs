use super::*;

// 要件 3.1 / 4.7: REQUEST は受領 request バイト列（echo ではなく proxy 駆動対象）を伴う Reply
// へ分類される（純関数ゆえ proxy へ到達しない・実駆動は handle_message の Reply アーム）。
// バイト内容は受領 payload と同一だが、意味は「駆動すべき request」であって「応答（echo）」ではない。
#[test]
fn request_classifies_to_reply_with_request_bytes() {
    let payload = b"round-trip";
    let raw = MsgTag::Request.as_u32() as usize;
    let action = classify_inbound(raw, payload.len(), payload);
    assert_eq!(action, InboundAction::Reply(payload.to_vec()));
}

#[test]
fn request_with_empty_payload_carries_empty_request_bytes() {
    let raw = MsgTag::Request.as_u32() as usize;
    let action = classify_inbound(raw, 0, b"");
    assert_eq!(action, InboundAction::Reply(Vec::new()));
}

// 要件 4.2: 応答対象外の既知タグは IgnoreKnown（無応答・crash なし）。
// 注: Load は TriggerLoad へ、Unload は TriggerUnload（R5.6）へ分離済みゆえ、ここから除外する。
#[test]
fn known_nonrequest_tags_are_ignored() {
    for tag in [MsgTag::Hello, MsgTag::Response] {
        let raw = tag.as_u32() as usize;
        let action = classify_inbound(raw, 0, b"");
        assert_eq!(action, InboundAction::IgnoreKnown(tag));
    }
}

// R5.6: Unload 受領は正規正常終了経路のトリガ（TriggerUnload）へ分類される。従来の
// 「既知だが無視（IgnoreKnown）」を置換する。ペイロードにパスや理由を期待しない
// ゆえ、ペイロード有無を問わず TriggerUnload であること（`TriggerLoad` と同型）。
#[test]
fn unload_classifies_to_trigger_unload() {
    let raw = MsgTag::Unload.as_u32() as usize;

    // ペイロード無し。
    let action = classify_inbound(raw, 0, b"");
    assert_eq!(action, InboundAction::TriggerUnload);

    // ペイロード有り（wire で内容を運ばないため、無視され同じく TriggerUnload）。
    let payload = b"ignored-unload-payload";
    let action = classify_inbound(raw, payload.len(), payload);
    assert_eq!(action, InboundAction::TriggerUnload);
}

// R5.6: 終了要求フラグ `quit_requested` は新規 HelperShared で既定 false（分類のみの本タスクで
// は set しない＝task 3.2 が結線する）。フィールド追加とその既定値を単体で確認する。
#[test]
fn new_helper_shared_defaults_quit_requested_false() {
    let s = HelperShared::new(0, PathBuf::new(), String::new());
    assert!(!s.quit_requested.get());
    assert_eq!(s.unloads_handled.get(), 0);
}

// 要件 4.1: Load 受領はロード実行トリガ（TriggerLoad）へ分類される。従来の
// 「既知だが無視（IgnoreKnown）」を置換する。ペイロードにパスを期待しない
// ゆえ、ペイロード有無を問わず TriggerLoad であること。
#[test]
fn load_classifies_to_trigger_load() {
    let raw = MsgTag::Load.as_u32() as usize;

    // ペイロード無し。
    let action = classify_inbound(raw, 0, b"");
    assert_eq!(action, InboundAction::TriggerLoad);

    // ペイロード有り（wire でパスを運ばないため、内容は無視され同じく TriggerLoad）。
    let payload = b"C:\\ghost\\master\\ignored-path";
    let action = classify_inbound(raw, payload.len(), payload);
    assert_eq!(action, InboundAction::TriggerLoad);
}

// 要件 2.5: 未知タグは crash させず IgnoreBad（記録のみ）。
#[test]
fn unknown_tag_is_ignored_as_bad() {
    let action = classify_inbound(0xFFusize, 0, b"");
    assert!(matches!(action, InboundAction::IgnoreBad(_)));
}

// 要件 2.5: cbData と実長の不整合は crash させず IgnoreBad。
#[test]
fn length_mismatch_is_ignored_as_bad() {
    let raw = MsgTag::Request.as_u32() as usize;
    let action = classify_inbound(raw, 10, b"abc");
    assert!(matches!(action, InboundAction::IgnoreBad(_)));
}
