use super::*;

// バイトは受領 payload と同じだが、意味は駆動すべき request であって echo ではない。
#[test]
fn request_classifies_to_reply_with_request_bytes() {
    let payload = b"round-trip";
    let raw = MsgTag::Request.as_u32() as usize;
    let action = classify_inbound(raw, payload.len(), payload.to_vec());
    assert_eq!(action, InboundAction::Reply(payload.to_vec()));
}

#[test]
fn request_with_empty_payload_carries_empty_request_bytes() {
    let raw = MsgTag::Request.as_u32() as usize;
    let action = classify_inbound(raw, 0, Vec::new());
    assert_eq!(action, InboundAction::Reply(Vec::new()));
}

#[test]
fn known_nonrequest_tags_are_ignored() {
    for tag in [MsgTag::Hello, MsgTag::Response] {
        let raw = tag.as_u32() as usize;
        let action = classify_inbound(raw, 0, Vec::new());
        assert_eq!(action, InboundAction::IgnoreKnown(tag));
    }
}

#[test]
fn unload_classifies_to_trigger_unload() {
    let raw = MsgTag::Unload.as_u32() as usize;

    let action = classify_inbound(raw, 0, Vec::new());
    assert_eq!(action, InboundAction::TriggerUnload);

    // wire で内容を運ばないので、ペイロードは無視される。
    let payload = b"ignored-unload-payload";
    let action = classify_inbound(raw, payload.len(), payload.to_vec());
    assert_eq!(action, InboundAction::TriggerUnload);
}

#[test]
fn new_helper_shared_defaults_quit_requested_false() {
    let s = HelperShared::new(0, PathBuf::new(), String::new());
    assert!(!s.quit_requested.get());
    assert_eq!(s.unloads_handled.get(), 0);
}

#[test]
fn load_classifies_to_trigger_load() {
    let raw = MsgTag::Load.as_u32() as usize;

    let action = classify_inbound(raw, 0, Vec::new());
    assert_eq!(action, InboundAction::TriggerLoad);

    // wire でパスを運ばないので、ペイロードは無視される。
    let payload = b"C:\\ghost\\master\\ignored-path";
    let action = classify_inbound(raw, payload.len(), payload.to_vec());
    assert_eq!(action, InboundAction::TriggerLoad);
}

#[test]
fn unknown_tag_is_ignored_as_bad() {
    let action = classify_inbound(0xFFusize, 0, Vec::new());
    assert!(matches!(action, InboundAction::IgnoreBad(_)));
}

#[test]
fn length_mismatch_is_ignored_as_bad() {
    let raw = MsgTag::Request.as_u32() as usize;
    let action = classify_inbound(raw, 10, b"abc".to_vec());
    assert!(matches!(action, InboundAction::IgnoreBad(_)));
}
