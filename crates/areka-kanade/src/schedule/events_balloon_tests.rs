//! バルーンの寿命の 3 イベント（`OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout`）の組み立てと
//! 送ってよいイベントの表の 3 行のテスト（areka-P0-balloon-lifecycle-events 要件 1.4・2.3・3.5・4.1・6.4）。
//!
//! どれも GET で、Reference の個数と値を列ごと突き合わせる。

use super::*;

/// GET を分解して (id の綴り, references, Status の wire 値) を取り出す（NOTIFY なら panic）。
fn expect_get(call: ShioriCall) -> (String, Vec<String>, Option<String>) {
    match call {
        ShioriCall::Get {
            id,
            references,
            status,
        } => (id.as_str().to_string(), references, status.render()),
        ShioriCall::Notify { .. } => panic!("GET のはずが NOTIFY"),
    }
}

const SCRIPT: &str = "\\0\\s[0]長い台詞\\w9\\w9\\e";

/// 送る時点のトークを持たない状態（定常で再生中のトークが無い）。
fn idle() -> ExecutionSnapshot {
    ExecutionSnapshot::INACTIVE
}

#[test]
fn on_balloon_break_is_get_with_script_scope_and_empty_position() {
    let (id, refs, status) = expect_get(on_balloon_break(SCRIPT, 1, &idle()));
    assert_eq!(id, "OnBalloonBreak");
    assert_eq!(
        refs.len(),
        3,
        "Reference0〜2 の 3 個（中断位置の位置を保つ）"
    );
    assert_eq!(
        refs[0], SCRIPT,
        "Reference0＝止めたトークの台本（要件 1.2）"
    );
    assert_eq!(refs[1], "1", "Reference1＝スコープ番号の十進（要件 1.3）");
    assert_eq!(refs[2], "", "Reference2（中断位置）は空（要件 1.4）");
    assert_eq!(status, None, "Status は送る時点の状態から導く");
}

#[test]
fn on_balloon_break_writes_scope_in_decimal_for_any_number() {
    let (_, refs, _) = expect_get(on_balloon_break(SCRIPT, 12, &idle()));
    assert_eq!(
        refs,
        vec![SCRIPT.to_string(), "12".to_string(), String::new()]
    );
}

#[test]
fn on_balloon_close_is_get_with_script_only() {
    let (id, refs, status) = expect_get(on_balloon_close(SCRIPT, &idle()));
    assert_eq!(id, "OnBalloonClose");
    assert_eq!(
        refs,
        vec![SCRIPT.to_string()],
        "Reference0＝台本の 1 個だけ（要件 3.2）"
    );
    assert_eq!(status, None);
}

#[test]
fn on_balloon_timeout_is_get_with_script_and_zero() {
    let (id, refs, status) = expect_get(on_balloon_timeout(SCRIPT, &idle()));
    assert_eq!(id, "OnBalloonTimeout");
    assert_eq!(
        refs,
        vec![SCRIPT.to_string(), "0".to_string()],
        "Reference0＝台本・Reference1＝`0`（要件 2.2・2.3）"
    );
    assert_eq!(status, None);
}

/// Status は呼び手が渡した状態から導く（他の組み立てと同じ・`ExecutionStatus::derive`）。
#[test]
fn balloon_builders_derive_status_from_the_snapshot() {
    let active = ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    };
    for call in [
        on_balloon_break(SCRIPT, 0, &active),
        on_balloon_close(SCRIPT, &active),
        on_balloon_timeout(SCRIPT, &active),
    ] {
        let (id, _, status) = expect_get(call);
        assert_eq!(status, Some("talking".to_string()), "{id} の Status");
    }
}

/// 表に正典の 3 語が載り、組み立ての綴りと一致する（要件 3.5・6.4）。
#[test]
fn the_three_balloon_events_are_in_the_allowed_table() {
    for id in ["OnBalloonBreak", "OnBalloonClose", "OnBalloonTimeout"] {
        assert_eq!(allowed_static(id), Some(id), "{id} が表に無い");
    }
    // 正典に無い独自のイベントは足さない（要件 3.5）。
    assert!(!is_allowed_event_id("OnBalloonClick"));
}
