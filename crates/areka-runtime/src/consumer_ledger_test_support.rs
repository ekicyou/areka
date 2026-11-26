//! `consumer_ledger_agreement_tests.rs` の檻が `areka-runtime` と本体の両方で使う見本の表と台帳の鍵。
//!
//! 受け口のテストが使う cue の作り手（汎用キャリアの `carrier_cue`・演者つきの `actor_carrier_cue`・
//! 開けない荷物の `broken_custom_cue`）もここに置く。受け口ごとに写しを持たない。

use super::{ConsumerLedger, LedgerKey};
use areka_ghost::prop_sink::PROP_SET_CUE_NAME;
use areka_parsers::sakura::{JUMP_TAG_CARRIER, MARKER_COMMAND, RAISE_TAG_CARRIER};
use areka_sakura::contract::FONT_TAG_CARRIER;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, TalkCue};
use std::collections::BTreeSet;

// ---------------------------------------------------------------- 見本

/// 絶対パスの書庫（`install_cue_tests.rs` の見本と同じ形。fs は読まない）。
const ABSOLUTE: &str = r"C:\areka-install-cue-test\in\hana.nar";

/// 表の各行の見本＝（名前, 選別子, 引数の列）。引数の列は第 1 引数から書く。
///
/// どれも担当の受け口（またはクレート）の既存のテストが受理を確かめている形から取った。
pub fn samples() -> Vec<(&'static str, Option<&'static str>, Vec<&'static str>)> {
    vec![
        // move_cue_move_sink_tests.rs の move_carrier_sends_directive
        ("move", None, vec!["-353", "", "", "0", "base", "base"]),
        // areka-seriko の actor_bind_loop_tests.rs
        ("bind", None, vec!["腕", "伸び", "1"]),
        // zorder_cue_tests.rs の set_zorder_becomes_a_set_directive_carrying_the_element_tokens／reset_zorder_becomes_a_reset_directive
        ("set", Some("zorder"), vec!["zorder", "1", "0"]),
        ("reset", Some("zorder"), vec!["zorder"]),
        // readme_cue_tests.rs の bare_open_readme_sends_exactly_one_request
        ("open", Some("readme"), vec!["readme"]),
        // readme_cue_tests.rs の open_forms_and_jump_send_classified_destination
        // （`open` の 5 組と `\j`）
        ("open", Some("file"), vec!["file", "notepad.exe"]),
        (
            "open",
            Some("browser"),
            vec!["browser", "https://example.com/"],
        ),
        ("open", Some("explorer"), vec!["explorer", r"C:\x"]),
        ("open", Some("editor"), vec!["editor", "a.txt", "3"]),
        ("open", Some("mailer"), vec!["mailer", "a@b.c"]),
        (JUMP_TAG_CARRIER, None, vec!["http://example.com/"]),
        // user_break_cue_tests.rs の enter／leave
        ("enter", Some("nouserbreakmode"), vec!["nouserbreakmode"]),
        ("leave", Some("nouserbreakmode"), vec!["nouserbreakmode"]),
        // areka-emo-text の actor_choice_contract_tests.rs
        (FONT_TAG_CARRIER, None, vec!["height", "40"]),
        // change_cue_tests.rs／switch_cue_tests.rs
        ("change", Some("ghost"), vec!["ghost", "B"]),
        ("change", Some("shell"), vec!["shell", "S"]),
        ("change", Some("balloon"), vec!["balloon", "Y"]),
        // install_cue_tests.rs の script_string_becomes_one_script_request_with_the_path
        (
            "execute",
            Some("install"),
            vec!["install", "path", ABSOLUTE],
        ),
        // move_cue_move_sink_tests.rs の execute_resetwindowpos_sends_reset
        ("execute", Some("resetwindowpos"), vec!["resetwindowpos"]),
        // update_cue_tests.rs の four_entry_forms_become_the_expected_request
        ("updatebymyself", None, vec![]),
        ("update", None, vec!["all"]),
        ("updateother", None, vec!["--balloon=B", "--shell=S"]),
        // areka-sakura の compile_choice_timeout_tests.rs
        ("set", Some("choicetimeout"), vec!["choicetimeout", "5000"]),
        // areka-ghost の prop_sink.rs の boot_count_carrier_is_persisted
        (PROP_SET_CUE_NAME, None, vec!["areka.boot.count", "1"]),
        // talk_lifecycle_signals_tests.rs の待ち時間の指定
        (
            "set",
            Some("balloontimeout"),
            vec!["balloontimeout", "3000"],
        ),
        // reload_cue_tests.rs の picked_spellings_send_one_request_each
        ("reload", Some("ghost"), vec!["ghost"]),
        ("reload", Some("shiori"), vec!["shiori"]),
        ("reload", Some("shell"), vec!["shell"]),
        ("reload", Some("balloon"), vec!["balloon"]),
        ("reload", Some("descript"), vec!["descript", "ghost shell"]),
        ("reloadsurface", None, vec![]),
        // move_cue_move_sink_tests.rs の窓の組・吸着・バルーン位置初期化
        (
            "set",
            Some("alignmenttodesktop"),
            vec!["alignmenttodesktop", "top"],
        ),
        (
            "set",
            Some("alignmentondesktop"),
            vec!["alignmentondesktop", "free"],
        ),
        (
            "set",
            Some("sticky-window"),
            vec!["sticky-window", "0", "1"],
        ),
        ("reset", Some("sticky-window"), vec!["sticky-window"]),
        ("execute", Some("resetballoonpos"), vec!["resetballoonpos"]),
        // move_cue_balloon_lock_tests.rs
        ("lock", Some("balloonmove"), vec!["balloonmove"]),
        ("unlock", Some("balloonmove"), vec!["balloonmove"]),
        // window_state_cue_tests.rs の常に手前の指令と `\v`
        ("set", Some("windowstate"), vec!["windowstate", "stayontop"]),
        (RAISE_TAG_CARRIER, None, vec![]),
        // areka-seriko の着せ替えの別名
        ("bind-noevent", None, vec!["腕", "伸び", "1"]),
        // 文字の層の印の合図
        (MARKER_COMMAND, None, vec![]),
        // areka-ghost の prop_get_sink.rs のプロパティの読み
        (
            "get",
            Some("property"),
            vec!["property", "OnX", "currentghost.scope(0).scaling"],
        ),
    ]
}

/// 表の行の組（名前, 選別子）の全部。
pub fn ledger_keys(ledger: &ConsumerLedger) -> BTreeSet<LedgerKey> {
    ledger.table.keys().cloned().collect()
}

/// `\![name,args...]` の汎用キャリア cue を `\0` の演者で組む（各受け口のテストと同じ正準形）。
pub fn carrier_cue(name: &str, args: &[&str]) -> TalkCue {
    actor_carrier_cue("0", name, args)
}

pub use dola::cue::test_support::carrier_cue as actor_carrier_cue;

/// 開けない `Custom`（`params` が String 配列でない）を宛名 `command` と演者 `actor` で組む。
///
/// `DynamicValue::Null` は配列ではないので `as_command_carrier()` は `None` を返す。
/// それでも宛名（`Custom{command}`）は読めるため、受け口の宛名で分ける枝へ入る。
pub fn broken_custom_cue(actor: &str, command: &str) -> TalkCue {
    let cue = TalkCue::scheduled(
        dola::TimePoint::ZERO,
        dola::TimeSpan::ZERO,
        ActorKey::from(actor),
        CueCommand::Custom {
            command: command.to_string(),
            params: DynamicValue::Null,
        },
        dola::TimeSpan::ZERO,
    );
    assert_eq!(
        cue.command.as_command_carrier(),
        None,
        "params=Null は開封できない形である（宛名で分ける枝に入る前提）"
    );
    cue
}
