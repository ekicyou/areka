//! 設定（`config.rs`）のテスト。
//!
//! どれも値だけを受け取る `Config::from_values` の判定で、環境変数・ファイル・机のポートには触らない
//! （パスは組むだけで開かない）。

use std::path::PathBuf;
use std::time::Duration;

use super::*;

fn candidates_of(port_value: Option<&str>) -> Vec<u16> {
    Config::from_values(port_value, None, None, false, None).candidates
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
    profile_dir: Option<&str>,
    exe: Option<&str>,
    areka_beside: bool,
    local_app_data: Option<&str>,
) -> Option<PathBuf> {
    Config::from_values(
        None,
        profile_dir.map(PathBuf::from),
        exe.map(PathBuf::from),
        areka_beside,
        local_app_data.map(PathBuf::from),
    )
    .cache_dir
}

/// `LOCALAPPDATA` の値の見本（組むだけで開かない）。
const LOCAL: &str = r"C:\Users\u\AppData\Local";

/// 控えの置き場の 4 通り（設計 D-7 の 3 段＋なし）。
#[test]
fn cache_dir_follows_the_three_steps() {
    let exe = Some(r"app\areka-mcp-bridge.exe");

    // ⑴ `AREKA_PROFILE_DIR` があれば、ほかが揃っていてもその下。
    assert_eq!(
        cache_dir_of(Some("prof"), exe, true, Some(LOCAL)),
        Some(PathBuf::from(r"prof\mcp-bridge"))
    );
    // ⑵ 無ければ、隣に `areka.exe` が居るときだけ exe の隣の木。
    assert_eq!(
        cache_dir_of(None, exe, true, Some(LOCAL)),
        Some(PathBuf::from(r"app\profile\areka\mcp-bridge"))
    );
    // ⑶ 隣に居なければ `LOCALAPPDATA` の下。
    assert_eq!(
        cache_dir_of(None, exe, false, Some(LOCAL)),
        Some(PathBuf::from(r"C:\Users\u\AppData\Local\areka\mcp-bridge"))
    );
    // どの段も取れなければ控えなし。
    assert_eq!(cache_dir_of(None, exe, false, None), None);
}

/// 隣に居ると言われても exe の場所が取れていなければ、exe の隣の木は組めないので次の段へ。
#[test]
fn cache_dir_without_exe_location_falls_to_local_app_data() {
    assert_eq!(
        cache_dir_of(None, None, true, Some(LOCAL)),
        Some(PathBuf::from(r"C:\Users\u\AppData\Local\areka\mcp-bridge"))
    );
}

/// 空・相対の `LOCALAPPDATA` は使わない（起こされた場所しだいの相対の置き場に黙って書かない）。
#[test]
fn cache_dir_ignores_a_local_app_data_that_is_not_absolute() {
    assert_eq!(cache_dir_of(None, None, false, Some("")), None);
    assert_eq!(cache_dir_of(None, None, false, Some("local")), None);
}

/// 待つ時間の既定は 3 つの定数（設計 D-11）。
#[test]
fn default_timeouts_are_the_three_constants() {
    assert_eq!(CONNECT_WAIT, Duration::from_millis(300));
    assert_eq!(VERIFY_WAIT, Duration::from_secs(3));
    assert_eq!(REPLY_WAIT, Duration::from_secs(30));

    let timeouts = Config::from_values(None, None, None, false, None).timeouts;
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
