//! 実プロセス・実ソケットの `get_log` の試験（spec: areka-P0-mcp-log-history task 5.1・
//! 要件 1.1・1.7・1.8・4.6・4.7・6.3）。
//!
//! emo2 の検体を argv に渡して areka を起こし、`get_log`（`log_type=status`）を呼ぶ。
//! `RUST_LOG=warn,areka::boot_config=info` で標準出力から消えた info が履歴に残ること・
//! 履歴の本文が標準出力の同じ行と 1 字も違わないこと（逆斜線入りのパスと二重引用符が JSON を
//! 通って戻る）・各行の頭の形と番号の昇順を判定する。
//!
//! 子の起こし方・MCP の呼び出し・閉じ方・環境の整え方は実機の試験の窓口 `real-run-kit` に任せる
//! （areka-P0-real-run-mcp-kit 要件 4.2・4.9・5.1・5.3・7.1〜7.3・7.5）。答えを得たら利用者と同じ
//! 閉じ方で閉じ、保険の自動終了を待たない。閉じた後に利用者と同じ閉じ方で終わったことも見る（D-19）。

use real_run_kit::{EndedBy, Launch, McpError, RealRun};

/// 標準出力から消え（`RUST_LOG=warn` の外の info）、履歴には残るはずの文。
const REAL_WINDOWS: &str = "本物のゴースト窓を開きました";
/// 本文を比べる出来事の目印。
const GHOST_RESOLVED: &str = "event=\"ghost_resolved\"";
/// 標準出力の行で本文が始まる前置き（`<時刻>  INFO areka::boot_config: <本文>`）。
const BOOT_CONFIG_PREFIX: &str = "areka::boot_config: ";

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

#[test]
fn get_log_status_over_real_socket_keeps_lines_hidden_from_stdout_and_bodies_intact() {
    let mut run = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        .launch(Launch::Sample {
            name: "emo2".into(),
            balloon: Some("emo2-kakukaku".into()),
        })
        .rust_log("warn,areka::boot_config=info")
        .spawn();
    let start = std::time::Instant::now();

    // 目当ての行が出るまで問い合わせる（窓口が呼び直し、子の終了も見る）。
    let answer = match run.call_until("get_log", r#"{"log_type":"status"}"#, REAL_WINDOWS) {
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
                "答えを得る前に子が終わりました（status={:?}）。\n{all}",
                exited.status
            );
        }
        Err(e) => panic!("get_log を呼べませんでした: {e}\n{}", run.output()),
    };

    // 利用者と同じ閉じ方で閉じてから標準出力を読む（保険の自動終了を待たない）。
    let answered = start.elapsed();
    let ended = run.close();
    eprintln!(
        "note: 答え {answered:?}・終了 {:?}（{:?}・{:?}）",
        start.elapsed(),
        ended.status,
        ended.by
    );
    let stdout = run.stdout();
    let ctx = || format!("--- status answer ---\n{answer}\n{}", run.output());

    // 利用者と同じ閉じ方で終わった（窓口の綴りの本物の檻・D-19）。
    assert_eq!(
        ended.by,
        EndedBy::UserClose,
        "利用者と同じ閉じ方で終わるはず。\n{}",
        ctx()
    );

    // 判定 1: 標準出力から消えた info が履歴に残る（要件 1.7・6.3）。
    assert!(
        !stdout.contains(REAL_WINDOWS),
        "RUST_LOG=warn なのに標準出力に「{REAL_WINDOWS}」がある。\n{}",
        ctx()
    );

    // 判定 3: 各行の頭の形と番号の昇順（継続行はタブで始まり、直前の行に属する）。
    let mut records: Vec<(u64, &str)> = Vec::new();
    for line in answer.split("\r\n") {
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

    // 判定 2: ghost_resolved の本文が標準出力の同じ行と 1 字も違わない（要件 1.8・4.6・4.7）。
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
}
