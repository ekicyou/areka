//! 実プロセス・実ソケットの strict の通し（spec: mcp-strict-errors task 9.4・要件 7.9）。
//!
//! emo2 の検体を起こし、誤りを 2 つ含む台本（無い surface・誰も拾わない `\!`）を `sakurascript` の
//! `strict` 真で流す。答えの `since_id` で `get_log(log_type=error, ghost_name=…)` を読むと、その台本の
//! strict の記録が 2 件、台本の順に読めることを判定する。
//!
//! 子の起こし方・MCP の呼び出し・閉じ方は窓口 `real-run-kit` に任せる（先例
//! `mcp_get_log_real_run.rs`・`script_security_real_run.rs`）。

use real_run_kit::{EndedBy, Launch, McpError, RealRun};

/// 起動の終わりの目印（標準出力からは消え、履歴には残る info）。
const REAL_WINDOWS: &str = "本物のゴースト窓を開きました";
/// 誤りを 2 つ含む台本（JSON の文の中の逆斜線は 2 つ）。`\s[99999]` は 2-11・`\![nosuch]` は 12-22。
const STRICT_SCRIPT_ARGS: &str = r#""script":"\\0\\s[99999]あ\\![nosuch]\\e","strict":true"#;
/// strict 真の答えの `since_id` の前置き。
const SINCE_ID: &str = "since_id=";

/// `#<番号> <yyyy/mm/dd hh:mm> [<種別>] <名> : <本文>` の本文（継続行は持たない記録だけを読む）。
fn body(line: &str) -> Option<&str> {
    line.split_once(" : ").map(|(_, b)| b)
}

#[test]
fn strict_script_records_two_errors_in_script_order_readable_by_since_id() {
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        .launch(Launch::Sample {
            name: "emo2".into(),
            balloon: Some("emo2-kakukaku".into()),
        })
        .spawn();

    // 起動の終わりまで待つ（窓口が呼び直し、子の終了も見る）。
    match run.call_until("get_log", r#"{"log_type":"status"}"#, REAL_WINDOWS) {
        Ok(_) => {}
        Err(McpError::ChildExited(exited)) => {
            let all = run.output();
            if all.contains("起動窓を開けません") && all.contains("モニタ列挙に失敗")
            {
                assert!(
                    !exited.status.success(),
                    "起動窓を開けないのに exit 0。\n{all}"
                );
                eprintln!("note: モニタ 0 台環境のため告知＋非 0 終了で受理");
                return;
            }
            panic!(
                "起動の終わりの前に子が終わりました（status={:?}）。\n{all}",
                exited.status
            );
        }
        Err(e) => panic!("get_log を呼べませんでした: {e}\n{}", run.output()),
    }

    let ghost = run
        .call("get_active_ghost_list", "{}")
        .unwrap_or_else(|e| {
            panic!(
                "get_active_ghost_list を呼べませんでした: {e}\n{}",
                run.output()
            )
        })
        .text;
    assert!(
        !ghost.is_empty(),
        "起動中のゴースト名が空。\n{}",
        run.output()
    );
    let ghost_json = ghost.replace('\\', "\\\\").replace('"', "\\\"");

    // 起動のトークの再生中は断られるので、受理（strict の答えの since_id）まで呼び直す。
    let sent = run
        .call_until(
            "sakurascript",
            &format!(r#"{{{STRICT_SCRIPT_ARGS},"ghost_name":"{ghost_json}"}}"#),
            SINCE_ID,
        )
        .unwrap_or_else(|e| panic!("sakurascript を呼べませんでした: {e}\n{}", run.output()));
    assert!(
        !sent.is_error && sent.text.starts_with("OK:"),
        "sakurascript が受理を答えなかった: {}",
        sent.text
    );
    let since_id: u64 = sent
        .text
        .split_once(SINCE_ID)
        .and_then(|(_, rest)| rest.split(')').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("答えに since_id が無い: {}", sent.text));

    // 2 件目（`\!` の記録）が書かれるまで読み直す。
    let log = run
        .call_until(
            "get_log",
            &format!(r#"{{"log_type":"error","ghost_name":"{ghost_json}","since_id":{since_id}}}"#),
            "unknown_command",
        )
        .unwrap_or_else(|e| panic!("get_log を呼べませんでした: {e}\n{}", run.output()))
        .text;

    let ended = run.close();
    let ctx = || {
        format!(
            "--- error log since {since_id} ---\n{log}\n{}",
            run.output()
        )
    };
    assert_eq!(
        ended.by,
        EndedBy::UserClose,
        "利用者と同じ閉じ方で終わるはず。\n{}",
        ctx()
    );

    // 2 件・台本の順・種類と位置と綴り（要件 7.9・2.3・2.5）。
    let bodies: Vec<&str> = log.split("\r\n").filter_map(body).collect();
    assert_eq!(bodies.len(), 2, "strict の記録が 2 件でない。\n{}", ctx());
    for (got, (kind, range, text)) in bodies.iter().zip([
        ("missing_surface", "2-11", r"\s[99999]"),
        ("unknown_command", "12-22", r"\![nosuch]"),
    ]) {
        assert!(
            got.starts_with(&format!("{kind} {range} ")) && got.contains(&format!(": {text}")),
            "記録が {kind} {range} {text} でない: {got}\n{}",
            ctx()
        );
    }
}
