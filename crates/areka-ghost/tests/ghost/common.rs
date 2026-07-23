//! common.rs — ghost 統合テスト共有の小道具（テスト支援）。
//!
//! task 8.3: 退役した `areka_ghost::default_system_vars()`（W1 暫定 provider）の忠実な代役
//! スタンドインを一箇所に集約する。8.2 で本番既定は sylphya 読み口由来
//! （[`areka_ghost::SystemVarWiring::FromSylphya`]）へ移行し `default_system_vars` は退役したが、
//! 既存 e2e テストは従来どおり既定 username 前提の凍結像刻印を検証する「生きているテスト」であり、
//! [`areka_ghost::SystemVarWiring::Custom`] による直接注入でその意図を無改変のまま保つ
//! （R7.1・R9.1・design「テスト呼出面の一括更新」）。
//!
//! 消費者は spine_e2e_test／inproc_e2e_test／snapshot_capture_test／real_pasta_test。env ゲート
//! 追験（real_pasta）が無効な構成では未使用になり得るため dead-code 警告を抑止する。
#![allow(dead_code)]

use areka_ghost::SystemVarSource;
use areka_sakura::contract::SystemVarSnapshot;
use areka_sakura::sysvar::DEFAULT_USERNAME;

/// 退役した `default_system_vars()` と同一挙動: `{"username": DEFAULT_USERNAME}` のみを
/// 充填した凍結スナップショットを毎回新規構築して返すクロージャ。`GhostBootOptions.system_vars`
/// へは `SystemVarWiring::Custom(test_system_vars())` として渡す。
pub fn test_system_vars() -> SystemVarSource {
    Box::new(|| {
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", DEFAULT_USERNAME);
        snapshot
    })
}
