//! [`StartTalk::new`] の既定（後書きが空・出どころがゴースト・厳しくない）を見るテスト。

use super::*;

// ── StartTalk::new / epilogue（Requirements 3.4・design C11） ──

/// 従来形コンストラクタ `StartTalk::new` は epilogue を空で構築する（従来挙動）。
#[test]
fn start_talk_new_has_empty_epilogue() {
    let msg = StartTalk::new(TalkId(1), r"\0hi\e");
    assert_eq!(msg.talk_id, TalkId(1));
    assert_eq!(msg.script, r"\0hi\e");
    assert!(msg.epilogue.is_empty());
}

/// 従来形コンストラクタ `StartTalk::new` は出どころにゴースト自身を入れる
/// （製品の呼び手の振る舞いを据え置く・areka-P0-script-security-level 1.4）。
#[test]
fn start_talk_new_origin_is_ghost() {
    let msg = StartTalk::new(TalkId(1), "hi");
    assert_eq!(msg.origin, ScriptOrigin::GHOST);
}

/// 従来形コンストラクタ `StartTalk::new` は strict の印を立てない
/// （kanade が自分で始める台本へ strict は及ばない・mcp-strict-errors 3.8）。
#[test]
fn start_talk_new_is_not_strict() {
    let msg = StartTalk::new(TalkId(1), "hi");
    assert!(!msg.strict);
}
