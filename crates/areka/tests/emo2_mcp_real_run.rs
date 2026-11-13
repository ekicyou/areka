//! 実プロセス・実ソケットの MCP の試験を、emo2 の起動 1 回の上でまとめて行う。
//!
//! 以前は 3 本の試験が別々に emo2 の新しい写しを起こし、全体テストで初回起動のトークが 3 回流れていた。
//! 起動は 1 回にし、次の 3 つを順に確かめる（確かめごとの中身は前と同じ）。
//!
//! 1. `get_log`（spec: areka-P0-mcp-log-history task 5.1・要件 1.1・1.7・1.8・4.6・4.7・6.3）:
//!    標準出力から消えた info が履歴に残る・履歴の本文が標準出力の同じ行と 1 字も違わない
//!    （逆斜線入りのパスと二重引用符が JSON を通って戻る）・各行の頭の形と番号の昇順。
//! 2. 出どころ（spec: areka-P0-script-security-level task 7.2・要件 1.2・1.3・2.2・2.3・2.4・2.8・4.3）:
//!    `kanade=trace` の `shiori_request` と `areka::input_events::user_break` の debug の記録を標準出力から読む。
//!    ⑴ MCP の `raise_event` は `sender=MCP`・`sender_type=external,sstp`・`security_level=local`
//!    ⑵ MCP の `sakurascript` の `OnTranslate` は `sender=MCP`・`sender_type=internal`
//!    ⑶ 起動の道のゴースト自身のイベントは `sender=areka`・`sender_type=internal`
//!    ⑷ MCP の `sakurascript` で流した `\![enter,nouserbreakmode]` は今どおり区間に入る（MCP は `local`）。
//!    決定論のテスト（偽の SHIORI の檻）の代わりではない。両方を置く。
//! 3. strict（spec: mcp-strict-errors task 9.4・要件 7.9）: 誤りを 2 つ含む台本を `strict` 真で流し、
//!    答えの `since_id` で `get_log(log_type=error, ghost_name=…)` を読むと 2 件が台本の順に読める。
//!
//! 記録の段は 3 つの和にする。`areka::ghost_session` の info（`本物のゴースト窓を開きました`）は
//! どれにも含まれないので、1 の「標準出力から消える」は今のまま成り立つ。
//! 子の起こし方・MCP の呼び出し・閉じ方は窓口 `real-run-kit` に任せる。証拠を集め終えたら利用者と同じ
//! 閉じ方で閉じ（保険の自動終了を待たない）、閉じた後に判定する。

use std::time::Duration;

use real_run_kit::{EndedBy, Launch, McpError, RealRun, Run};

/// 起動の終わりの目印（`RUST_LOG` の外の info＝標準出力からは消え、履歴には残る）。
const REAL_WINDOWS: &str = "本物のゴースト窓を開きました";
/// 本文を比べる出来事の目印。
const GHOST_RESOLVED: &str = "event=\"ghost_resolved\"";
/// 標準出力の行で本文が始まる前置き（`<時刻>  INFO areka::boot_config: <本文>`）。
const BOOT_CONFIG_PREFIX: &str = "areka::boot_config: ";
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
/// 誤りを 2 つ含む台本（JSON の文の中の逆斜線は 2 つ）。`\s[99999]` は 2-11・`\![nosuch]` は 12-22。
const STRICT_SCRIPT_ARGS: &str = r#""script":"\\0\\s[99999]あ\\![nosuch]\\e","strict":true"#;
/// strict 真の答えの `since_id` の前置き。
const SINCE_ID: &str = "since_id=";

/// 行の頭 `#<数字> <yyyy/mm/dd hh:mm> [STAT] STAT : ` を読み、番号と本文を返す。
fn parse_status_line(line: &str) -> Option<(u64, &str)> {
    let rest = line.strip_prefix('#')?;
    let (id, rest) = rest.split_once(' ')?;
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let stamp = rest.get(..16)?;
    let shape_ok = stamp.bytes().enumerate().all(|(i, b)| match i {
        4 | 7 => b == b'/',
        10 => b == b' ',
        13 => b == b':',
        _ => b.is_ascii_digit(),
    });
    if !shape_ok {
        return None;
    }
    Some((
        id.parse().ok()?,
        rest.get(16..)?.strip_prefix(" [STAT] STAT : ")?,
    ))
}

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

/// `#<番号> <yyyy/mm/dd hh:mm> [<種別>] <名> : <本文>` の本文（継続行は持たない記録だけを読む）。
fn body(line: &str) -> Option<&str> {
    line.split_once(" : ").map(|(_, b)| b)
}

/// 子が終わったときの文（`wait_for` の失敗）。
fn exited(run: &Run, status: impl std::fmt::Debug) -> String {
    format!("子が終わりました（{status:?}）。\n{}", run.output())
}

#[test]
fn emo2_mcp_get_log_origin_and_strict_over_one_real_boot() {
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        .launch(Launch::Sample {
            name: "emo2".into(),
            balloon: Some("emo2-kakukaku".into()),
        })
        .rust_log("warn,areka::boot_config=info,kanade=trace,areka::input_events::user_break=debug")
        // 3 つを 1 回で行うので、初回の起動のトーク（約 17 秒）の後まで保険の自動終了（既定 45 秒）に掛からない幅を取る。
        .safety_exit(Duration::from_secs(120))
        .spawn();
    let start = std::time::Instant::now();

    // 1 の答え: 起動の終わりの行が履歴に出るまで問い合わせる（窓口が呼び直し、子の終了も見る）。
    let status_answer = match run.call_until("get_log", r#"{"log_type":"status"}"#, REAL_WINDOWS) {
        Ok(reply) => reply.text,
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
    };

    // 2 の証拠: 起動のトーク（初回は約 17 秒）が終わるまで待つ。再生中の MCP の台本・イベントは
    // 「the ghost cannot accept a script now」で断られる。毎秒の OnSecondChange の要求の
    // 実行状態から talking が消え、バルーンだけが残った行を終わりの証拠にする。
    run.wait_for(BOOT_TALK_ENDED)
        .unwrap_or_else(|e| panic!("{}", exited(&run, e.status)));
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
        .unwrap_or_else(|e| panic!("{}", exited(&run, e.status)));

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
        .unwrap_or_else(|e| panic!("{}", exited(&run, e.status)));
    // 2 の切り分けは 3 の台本を流す前で閉じる（3 の台本の行を混ぜない）。
    let after_send = run
        .stdout()
        .get(before_send..)
        .expect("流す前の位置が標準出力の字の境目に無い")
        .to_owned();

    // 3 の証拠: 起動中のゴースト名を取り、strict の台本を流す。
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
    // 2 の台本（3 秒待つ）の再生中は断られるので、受理（strict の答えの since_id）まで呼び直す。
    let strict_sent = run
        .call_until(
            "sakurascript",
            &format!(r#"{{{STRICT_SCRIPT_ARGS},"ghost_name":"{ghost_json}"}}"#),
            SINCE_ID,
        )
        .unwrap_or_else(|e| panic!("sakurascript を呼べませんでした: {e}\n{}", run.output()));
    assert!(
        !strict_sent.is_error && strict_sent.text.starts_with("OK:"),
        "sakurascript が受理を答えなかった: {}",
        strict_sent.text
    );
    let since_id: u64 = strict_sent
        .text
        .split_once(SINCE_ID)
        .and_then(|(_, rest)| rest.split(')').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("答えに since_id が無い: {}", strict_sent.text));
    // 2 件目（`\!` の記録）が書かれるまで読み直す。
    let error_log = run
        .call_until(
            "get_log",
            &format!(r#"{{"log_type":"error","ghost_name":"{ghost_json}","since_id":{since_id}}}"#),
            "unknown_command",
        )
        .unwrap_or_else(|e| panic!("get_log を呼べませんでした: {e}\n{}", run.output()))
        .text;

    // 利用者と同じ閉じ方で閉じてから標準出力を読む（保険の自動終了を待たない）。
    let collected = start.elapsed();
    let ended = run.close();
    eprintln!(
        "note: 証拠 {collected:?}・終了 {:?}（{:?}・{:?}）",
        start.elapsed(),
        ended.status,
        ended.by
    );
    let stdout = run.stdout();
    let ctx = || {
        format!(
            "--- status answer ---\n{status_answer}\n--- error log since {since_id} ---\n{error_log}\n{}",
            run.output()
        )
    };
    // 利用者と同じ閉じ方で終わった（窓口の綴りの本物の檻・D-19）。
    assert_eq!(
        ended.by,
        EndedBy::UserClose,
        "利用者と同じ閉じ方で終わるはず。\n{}",
        ctx()
    );

    // 1-1: 標準出力から消えた info が履歴に残る（要件 1.7・6.3）。
    assert!(
        !stdout.contains(REAL_WINDOWS),
        "RUST_LOG に areka::ghost_session が無いのに標準出力に「{REAL_WINDOWS}」がある。\n{}",
        ctx()
    );
    // 1-3: 各行の頭の形と番号の昇順（継続行はタブで始まり、直前の行に属する）。
    let mut records: Vec<(u64, &str)> = Vec::new();
    for line in status_answer.split("\r\n") {
        if line.starts_with('\t') {
            assert!(!records.is_empty(), "先頭が継続行。\n{}", ctx());
            continue;
        }
        let parsed = parse_status_line(line)
            .unwrap_or_else(|| panic!("行の頭の形が違う: {line:?}\n{}", ctx()));
        if let Some((prev, _)) = records.last() {
            assert!(parsed.0 > *prev, "番号が昇順でない: {line:?}\n{}", ctx());
        }
        records.push(parsed);
    }
    // 1-2: ghost_resolved の本文が標準出力の同じ行と 1 字も違わない（要件 1.8・4.6・4.7）。
    let in_history: Vec<&str> = records
        .iter()
        .map(|(_, body)| *body)
        .filter(|b| b.contains(GHOST_RESOLVED))
        .collect();
    let in_stdout: Vec<&str> = stdout
        .lines()
        .filter(|l| l.contains(GHOST_RESOLVED))
        .filter_map(|l| {
            l.split_once(BOOT_CONFIG_PREFIX)
                .map(|(_, b)| b.trim_end_matches('\r'))
        })
        .collect();
    assert_eq!(
        in_history.len(),
        1,
        "履歴の ghost_resolved が 1 行でない。\n{}",
        ctx()
    );
    assert!(
        in_history[0].contains('\\') && in_history[0].contains('"'),
        "ghost_resolved の本文に逆斜線と引用符が無い（往復を踏んでいない）。\n{}",
        ctx()
    );
    assert_eq!(
        in_history,
        in_stdout,
        "履歴と標準出力の本文が違う。\n{}",
        ctx()
    );

    // 2⑴ raise_event の要求（要件 1.3・2.3）。
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
    // 2⑵ sakurascript の OnTranslate（要件 1.2・2.4・2.8）。
    let mcp_translate = (Some("\"MCP\""), Some("internal"), Some("\"local\""));
    assert!(
        requests(&after_send, "OnTranslate")
            .iter()
            .any(|l| origin(l) == mcp_translate),
        "sakurascript の OnTranslate が sender=MCP・sender_type=internal・security_level=local で\
         送られていない。\n{}",
        ctx()
    );
    // 2⑶ 起動の道の要求は全部ゴースト自身（要件 2.2）。起動のイベントが 1 本は在ること。
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
    // 2⑷ MCP の台本は local なので止めない（要件 4.3）。区間に入った記録は上で待った。
    assert!(
        !after_send.contains(REFUSED),
        "MCP の台本でタグが止められた。\n{}",
        ctx()
    );

    // 3: 2 件・台本の順・種類と位置と綴り（要件 7.9・2.3・2.5）。
    let bodies: Vec<&str> = error_log.split("\r\n").filter_map(body).collect();
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
