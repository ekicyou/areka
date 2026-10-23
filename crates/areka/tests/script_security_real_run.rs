//! 本物の起動の道での出どころの確かめ（spec: areka-P0-script-security-level task 7.2・
//! 要件 1.2・1.3・2.2・2.3・2.4・2.8・4.3）。
//!
//! emo2 の検体を本物の pasta.dll（同じプロセスの経路）で起こし、`kanade=trace` の
//! `shiori_request` の記録と `areka::input_events::user_break` の debug の記録を子の標準出力から
//! 読んで、出どころの 3 項目が既定の値に落ちていないことを確かめる。
//! ⑴ MCP の `raise_event` の要求は `sender=MCP`・`sender_type=external,sstp`・`security_level=local`
//! ⑵ MCP の `sakurascript` の `OnTranslate` は `sender=MCP`・`sender_type=internal`
//! ⑶ 起動の道のゴースト自身のイベントは `sender=areka`・`sender_type=internal`
//! ⑷ MCP の `sakurascript` で流した `\![enter,nouserbreakmode]` は今どおり区間に入る（MCP は `local`）。
//! 決定論のテスト（偽の SHIORI の檻）の代わりではない。両方を置く。
//!
//! 子の起こし方・MCP の呼び出し・閉じ方は窓口 `real-run-kit` に任せる（先例
//! `mcp_get_log_real_run.rs`）。記録は `get_log` が debug の行を残さないことがあるので標準出力で読む。

use real_run_kit::{EndedBy, Launch, McpError, RealRun};

/// 起動の終わりの目印（標準出力からは消え、履歴には残る info）。
const REAL_WINDOWS: &str = "本物のゴースト窓を開きました";
/// 起動のトークの後の実行状態（talking が無くバルーンが在る）。トークの前はバルーンが無い。
const BOOT_TALK_ENDED: &str = "status=Some(\"balloon(";
/// SHIORI へ送る直前の記録の目印。
const SHIORI_REQUEST: &str = "event=\"shiori_request\"";
/// `raise_event` で送るイベント（記録の行を 1 つに絞るための、ほかで使わない名前）。
const PROBE_EVENT: &str = "OnScriptSecurityRealRunProbe";
/// 区間に入った記録（`crates/areka/src/input_events/user_break.rs`）。
const ENTERED: &str = "event=\"no_user_break_changed\" value=true";
/// 外部扱いの台本でタグを止めた warn（出てはならない）。
const REFUSED: &str = "script_tag_refused";
/// 区間に入り、旗が畳まれる巡を跨ぐよう 3 秒待ってから出る台本（JSON の文の中の逆斜線は 2 つ）。
const ENTER_SCRIPT_ARGS: &str =
    r#"{"script":"\\0\\![enter,nouserbreakmode]\\_w[3000]\\![leave,nouserbreakmode]\\e"}"#;

/// 記録の行の ` <key>=<値>` の値（次の空白まで）。
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let at = line.find(&format!(" {key}="))? + key.len() + 2;
    let rest = &line[at..];
    Some(rest.split(' ').next().unwrap_or(rest))
}

/// `shiori_request` の行のうち、ID が `id` のもの。
fn requests<'a>(text: &'a str, id: &str) -> Vec<&'a str> {
    text.lines()
        .filter(|l| l.contains(SHIORI_REQUEST) && field(l, "id") == Some(id))
        .collect()
}

/// 行の出どころの 3 項目（`sender`・`sender_type`・`security_level`）。
fn origin(line: &str) -> (Option<&str>, Option<&str>, Option<&str>) {
    (
        field(line, "sender"),
        field(line, "sender_type"),
        field(line, "security_level"),
    )
}

#[test]
fn real_boot_path_carries_request_origin_and_mcp_script_enters_no_user_break() {
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        .launch(Launch::Sample {
            name: "emo2".into(),
            balloon: Some("emo2-kakukaku".into()),
        })
        .rust_log("warn,kanade=trace,areka::input_events::user_break=debug")
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
    // 起動のトーク（初回は約 17 秒）が終わるまで待つ。再生中の MCP の台本・イベントは
    // 「the ghost cannot accept a script now」で断られる。毎秒の OnSecondChange の要求の
    // 実行状態から talking が消え、バルーンだけが残った行を終わりの証拠にする。
    run.wait_for(BOOT_TALK_ENDED)
        .unwrap_or_else(|e| panic!("子が終わりました（{:?}）。\n{}", e.status, run.output()));
    let boot = run.stdout();

    // ⑴ raise_event（SSTP NOTIFY 相当）。
    let raised = run
        .call("raise_event", &format!(r#"{{"event":"{PROBE_EVENT}"}}"#))
        .unwrap_or_else(|e| panic!("raise_event を呼べませんでした: {e}\n{}", run.output()));
    assert!(
        !raised.is_error,
        "raise_event が失敗を答えた: {}",
        raised.text
    );
    run.wait_for(&format!(" id={PROBE_EVENT} "))
        .unwrap_or_else(|e| panic!("子が終わりました（{:?}）。\n{}", e.status, run.output()));

    // ⑵⑷ sakurascript（SSTP SEND 相当）。流す前の標準出力の長さで、後から出た行を切り分ける。
    let before_send = run.stdout().len();
    assert!(
        !run.stdout().contains(ENTERED),
        "MCP で流す前に区間へ入っていた（⑷ の証拠にならない）。\n{}",
        run.output()
    );
    // probe へゴーストが台本を返していれば、その再生が終わるまで断られるので受理（"OK"）まで呼び直す。
    let sent = run
        .call_until("sakurascript", ENTER_SCRIPT_ARGS, "OK")
        .unwrap_or_else(|e| panic!("sakurascript を呼べませんでした: {e}\n{}", run.output()));
    assert!(!sent.is_error, "sakurascript が失敗を答えた: {}", sent.text);
    run.wait_for(ENTERED)
        .unwrap_or_else(|e| panic!("子が終わりました（{:?}）。\n{}", e.status, run.output()));

    // 利用者と同じ閉じ方で閉じてから標準出力を読む（保険の自動終了を待たない）。
    let ended = run.close();
    let stdout = run.stdout();
    let after_send = stdout.get(before_send..).unwrap_or("");
    let ctx = || run.output();
    assert_eq!(
        ended.by,
        EndedBy::UserClose,
        "利用者と同じ閉じ方で終わるはず。\n{}",
        ctx()
    );

    // ⑴ raise_event の要求（要件 1.3・2.3）。
    let probe = requests(&stdout, PROBE_EVENT);
    assert_eq!(
        probe.len(),
        1,
        "raise_event の要求の記録が 1 行でない。\n{}",
        ctx()
    );
    assert_eq!(
        origin(probe[0]),
        (Some("\"MCP\""), Some("external,sstp"), Some("\"local\"")),
        "raise_event の出どころが違う: {}\n{}",
        probe[0],
        ctx()
    );

    // ⑵ sakurascript の OnTranslate（要件 1.2・2.4・2.8）。
    let mcp_translate = (Some("\"MCP\""), Some("internal"), Some("\"local\""));
    assert!(
        requests(after_send, "OnTranslate")
            .iter()
            .any(|l| origin(l) == mcp_translate),
        "sakurascript の OnTranslate が sender=MCP・sender_type=internal・security_level=local で\
         送られていない。\n{}",
        ctx()
    );

    // ⑶ 起動の道の要求は全部ゴースト自身（要件 2.2）。起動のイベントが 1 本は在ること。
    let boot_requests: Vec<&str> = boot
        .lines()
        .filter(|l| l.contains(SHIORI_REQUEST))
        .collect();
    assert!(
        boot_requests
            .iter()
            .any(|l| matches!(field(l, "id"), Some("OnBoot" | "OnFirstBoot"))),
        "起動のイベントの要求の記録が無い（kanade=trace が開いていない？）。\n{}",
        ctx()
    );
    for l in &boot_requests {
        assert_eq!(
            origin(l),
            (Some("\"areka\""), Some("internal"), Some("\"local\"")),
            "起動の道の要求の出どころが違う: {l}\n{}",
            ctx()
        );
    }

    // ⑷ MCP の台本は local なので止めない（要件 4.3）。区間に入った記録は上で待った。
    assert!(
        !after_send.contains(REFUSED),
        "MCP の台本でタグが止められた。\n{}",
        ctx()
    );
}
