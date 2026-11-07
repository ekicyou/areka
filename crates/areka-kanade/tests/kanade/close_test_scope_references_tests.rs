//! 利用者の終了の指示のスコープ番号が `OnClose` の References まで届くことの通しの檻
//! （popup-menu-residue 要件 7.1〜7.4）。
//!
//! kanade の入口へ `CloseRequest { reason: CloseReason::User { scope } }` を入れ、偽の SHIORI の
//! 記録列に現れる `OnClose` の GET の References を、スコープ 0 と 1 で確かめる（挨拶なしの足場・
//! `OnClose` は既定で 204＝無言終了・待ちは全部期限付き）。
//!
//! # 足場の決まりの意図した例外
//! 足場（`common/common_recording.rs`）は「期待する References は `events::*` から組み、書き下さない」
//! と決めている。このテストはその意図した例外で、`events::on_close` そのものの誤りに気付くため
//! 期待値を `["user","0","0"]`・`["user","1","1"]` と書き下す（要件 7.3）。足場の決まりに戻さないこと。
//! 書き下した `["user","1","1"]` が Reference1＝Reference2 も檻に入れる（要件 7.4）。

use super::{
    CallMethod, CloseReason, Driven, Fixture, KanadeConfig, KanadeMsg, QuitPolicy, send,
    spawn_harness,
};

#[test]
fn a_user_close_carries_the_window_scope_into_onclose_references() {
    let cases: [(u32, [&str; 3]); 2] = [(0, ["user", "0", "0"]), (1, ["user", "1", "1"])];

    for (scope, expected) in cases {
        let harness = spawn_harness(
            KanadeConfig::new("master", "1.0.0"),
            Fixture::default().without_boot_greeting(),
            QuitPolicy::PerTalk(vec![]),
        );
        send(&harness, KanadeMsg::Boot);
        send(
            &harness,
            KanadeMsg::CloseRequest {
                reason: CloseReason::User { scope },
            },
        );

        let Driven { recorded, .. } = harness.finish("kanade scope-close join");
        let onclose: Vec<&Vec<String>> = recorded
            .iter()
            .filter(|c| c.method == CallMethod::Get && c.id == "OnClose")
            .map(|c| &c.references)
            .collect();
        assert_eq!(
            onclose,
            vec![&expected.map(String::from).to_vec()],
            "scope {scope}: OnClose の GET はちょうど 1 件で References は書き下した値: {recorded:?}"
        );
    }
}
