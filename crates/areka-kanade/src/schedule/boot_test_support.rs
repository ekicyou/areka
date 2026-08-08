use super::*;

pub(super) fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// boot 進行は `State::initial()` を起点に `step()` 経由で駆動する（統合貫通テスト）。
pub(super) fn initial() -> State {
    State::initial()
}

/// Action が期待の GET（id・references が events:: と一致）であることを検証する。
pub(super) fn assert_get(action: &Action, expected: &crate::msg::ShioriCall) {
    match (action, expected) {
        (
            Action::ShioriRequest(crate::msg::ShioriCall::Get { id, references, .. }),
            crate::msg::ShioriCall::Get {
                id: eid,
                references: erefs,
                ..
            },
        ) => {
            assert_eq!(id, eid, "GET id 不一致");
            assert_eq!(references, erefs, "GET references 不一致");
        }
        _ => panic!("expected ShioriRequest(Get) matching events output"),
    }
}

/// Action が期待の NOTIFY（id・references が events:: と一致）であることを検証する。
pub(super) fn assert_notify(action: &Action, expected: &crate::msg::ShioriCall) {
    match (action, expected) {
        (
            Action::ShioriRequest(crate::msg::ShioriCall::Notify { id, references, .. }),
            crate::msg::ShioriCall::Notify {
                id: eid,
                references: erefs,
                ..
            },
        ) => {
            assert_eq!(id, eid, "NOTIFY id 不一致");
            assert_eq!(references, erefs, "NOTIFY references 不一致");
        }
        _ => panic!("expected ShioriRequest(Notify) matching events output"),
    }
}
