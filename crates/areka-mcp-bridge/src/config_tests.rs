//! 設定（`config.rs`）のテスト。
//!
//! どれも値だけを受け取る `Config::from_values` の判定で、環境変数・ファイル・机のポートには触らない
//! （パスは組むだけで開かない）。

use std::path::PathBuf;
use std::time::Duration;

use areka_user_data::AppProfile;

use super::*;

fn candidates_of(port_value: Option<&str>) -> Vec<u16> {
    Config::from_values(port_value, None, None, None).candidates
}

/// `AREKA_MCP_PORT` の 4 通り（要件 2.1・2.4・2.5・2.6）。並びは areka 本体と同じ関数の結果（8.2）。
#[test]
fn port_value_maps_to_candidates_in_four_ways() {
    let defaults = areka_mcp::candidates_from_env_value(None);
    assert_eq!(defaults.len(), 20);

    assert_eq!(candidates_of(None), defaults, "未設定は既定の 20 個");
    assert_eq!(
        candidates_of(Some("9000")),
        vec![9000],
        "番号はその 1 つだけ"
    );
    assert_eq!(
        candidates_of(Some("0")),
        Vec::<u16>::new(),
        "0 は空（待ち受けない設定）"
    );
    assert_eq!(
        candidates_of(Some("abc")),
        defaults,
        "読めない値は既定の 20 個"
    );
    // UTF-8 でない値は、`from_env` が読めない字を U+FFFD に替えて渡す。数字に挟まっていても読めない値。
    assert_eq!(
        candidates_of(Some("90\u{FFFD}00")),
        defaults,
        "U+FFFD の入った値は既定の 20 個"
    );
}

fn cache_dir_of(
    profile_dir: Option<PathBuf>,
    beside: Option<AppProfile>,
    local_app_data: Option<PathBuf>,
) -> Option<PathBuf> {
    Config::from_values(None, profile_dir, beside, local_app_data).cache_dir
}

/// ワークツリーの `target\` の下の見本のパス（組むだけで開かない）。
fn under_target(rel: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(r"..\..\target\config-tests")
        .join(rel)
}

/// 控えの置き場の表の 5 行（bridge-store-user-root 要件 2.4〜2.10・6.4・設計「`from_values` の控えの置き場」）。
#[test]
fn cache_dir_follows_the_table() {
    let local = under_target("Local");
    let ready = under_target(r"Local\areka\profile\areka");
    let root = under_target(r"Local\areka");

    // ⑴ `AREKA_PROFILE_DIR` があれば、隣の答えがどれでもその下（2.9）。
    for beside in [
        None,
        Some(AppProfile::Ready(ready.clone())),
        Some(AppProfile::Unlocatable),
    ] {
        assert_eq!(
            cache_dir_of(Some(under_target("prof")), beside, Some(local.clone())),
            Some(under_target(r"prof\mcp-bridge"))
        );
    }
    // ⑵ 使える形なら、その下の `mcp-bridge`（2.6・2.8）。
    assert_eq!(
        cache_dir_of(
            None,
            Some(AppProfile::Ready(ready.clone())),
            Some(local.clone())
        ),
        Some(ready.join("mcp-bridge"))
    );
    // ⑶ 使える形以外の 3 つは、`LOCALAPPDATA` が在っても控え無し（そこへ落ちない・2.4・2.5・2.7）。
    for beside in [
        AppProfile::RootMissing { root: root.clone() },
        AppProfile::MigrationPending { root: root.clone() },
        AppProfile::Unlocatable,
    ] {
        assert_eq!(
            cache_dir_of(None, Some(beside.clone()), Some(local.clone())),
            None,
            "{beside:?}"
        );
    }
    // ⑷ 隣に areka が居ないなら `LOCALAPPDATA\areka\mcp-bridge`（今どおり・2.10）。
    assert_eq!(
        cache_dir_of(None, None, Some(local.clone())),
        Some(local.join(r"areka\mcp-bridge"))
    );
    // ⑸ 無い・空・相対の `LOCALAPPDATA` は使わない（起こされた場所しだいの相対の置き場に黙って書かない）。
    assert_eq!(cache_dir_of(None, None, None), None);
    assert_eq!(cache_dir_of(None, None, Some(PathBuf::new())), None);
    assert_eq!(cache_dir_of(None, None, Some(PathBuf::from("local"))), None);
}

/// 待つ時間の既定は 3 つの定数（設計 D-11）。
#[test]
fn default_timeouts_are_the_three_constants() {
    assert_eq!(CONNECT_WAIT, Duration::from_millis(300));
    assert_eq!(VERIFY_WAIT, Duration::from_secs(3));
    assert_eq!(REPLY_WAIT, Duration::from_secs(30));

    let timeouts = Config::from_values(None, None, None, None).timeouts;
    assert_eq!(timeouts.connect, CONNECT_WAIT);
    assert_eq!(timeouts.verify, VERIFY_WAIT);
    assert_eq!(timeouts.reply, REPLY_WAIT);
}

/// 応答の上限は areka のツールが答えるまでの上限より長い（要件 4.4 の見張り）。
#[test]
fn reply_wait_is_longer_than_the_areka_tool_limit() {
    assert!(REPLY_WAIT > areka_mcp::tools::REPLY_WAIT);
}

/// テストは時間を差し替えられる（`from_parts`）。
#[test]
fn from_parts_keeps_the_given_values() {
    let short = Timeouts {
        connect: Duration::from_millis(1),
        verify: Duration::from_millis(2),
        reply: Duration::from_millis(3),
    };
    let config = Config::from_parts(vec![1234], Some(PathBuf::from("dir")), short.clone());
    assert_eq!(config.candidates, vec![1234]);
    assert_eq!(config.cache_dir, Some(PathBuf::from("dir")));
    assert_eq!(config.timeouts.connect, short.connect);
    assert_eq!(config.timeouts.verify, short.verify);
    assert_eq!(config.timeouts.reply, short.reply);
}
