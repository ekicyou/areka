//! helper の標準エラーを記録へ回すテスト（audit-shiori-host32 U5・指摘 4・要件 5.2・5.4・10.12）。
//!
//! 行の読み（[`forward_helper_stderr`]）と記録の形（[`log_helper_line`]）は同じスレッドで確かめ、
//! 実の i686 helper の行が読み手のスレッドから sink へ届くことを 1 本で縛る。

use std::io::Cursor;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;

// 成果物の探し方とテストの根は結合テストの道具をそのまま引く（写しを作らない）。
#[path = "../tests/common/mod.rs"]
mod common;

fn forwarded(bytes: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();
    forward_helper_stderr(Cursor::new(bytes.to_vec()), |line| {
        lines.push(line.to_owned())
    })
    .expect("Cursor は読みに失敗しない");
    lines
}

/// LF で切り、行末の CR を落とし、UTF-8 でない行は ANSI コードページで読む（要件 5.4）。
#[test]
fn forward_lines_reads_utf8_and_ansi_lines() {
    // SJIS の「あ」（0x82 0xA0）は UTF-8 として読めない。
    let mut input = b"[helper] one\r\n\ntwo \xE3\x81\x82\n".to_vec();
    input.extend_from_slice(&[0x82, 0xA0, b'\r', b'\n']);
    input.extend_from_slice(b"last without newline");
    let lines = forwarded(&input);

    // SAFETY: 引数なし・副作用なし。
    let acp = unsafe { windows::Win32::Globalization::GetACP() };
    let ansi = match acp {
        932 => "あ".to_owned(),
        1252 => "\u{201A}\u{A0}".to_owned(),
        // 他のコードページ: 少なくとも UTF-8 の置き換え読みではないこと。
        _ => {
            assert_ne!(
                lines[3],
                String::from_utf8_lossy(&[0x82, 0xA0]),
                "ACP {acp}"
            );
            lines[3].clone()
        }
    };
    assert_eq!(
        lines,
        [
            "[helper] one",
            "",
            "two あ",
            ansi.as_str(),
            "last without newline"
        ]
    );
}

/// 1 行が `warn`・宛名 `shiori-host32`・`event = "helper_stderr"`・`pid` で、文がそのまま書かれる（要件 10.12）。
#[test]
fn a_helper_line_is_logged_verbatim_at_warn() {
    let line = r#"[helper] 窓生成/HELLO 失敗: "quoted" back\slash {braces}"#;
    let ((), events) = log_capture_kit::capture(|| log_helper_line(4321, line));
    let ours: Vec<_> = events
        .iter()
        .filter(|e| e.target == "shiori-host32")
        .collect();
    assert_eq!(ours.len(), 1, "{events:?}");
    let event = ours[0];
    assert_eq!(event.level, tracing::Level::WARN);
    assert_eq!(event.field_str("event"), Some("helper_stderr"));
    assert_eq!(event.field("pid"), Some("4321"));
    assert_eq!(event.message(), line);
}

/// 実の i686 helper が書く `[helper]` の行が、`helper_command` の管と読み手のスレッドを通って
/// sink へ届く（親の窓 0 で HELLO に失敗して終わる道）。成果物が無ければ panic（要件 4.4）。
#[test]
fn real_helper_stderr_reaches_the_sink() {
    let root = common::TestRoot::new("stderr");
    let helper = common::resolve_helper_exe();
    let (tx, rx) = mpsc::channel::<(u32, String)>();
    let handle = spawn_command_with(
        helper_command(&helper, root.path(), "x.dll", 0),
        move |pid, line| {
            let _ = tx.send((pid, line.to_owned()));
        },
    )
    .expect("i686 helper を起こせる");

    let deadline = Instant::now() + common::HANDSHAKE_TIMEOUT;
    let mut seen = Vec::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left.max(Duration::from_millis(1))) {
            Ok((pid, line)) => {
                assert_eq!(pid, handle.pid());
                if line.contains("[helper] 窓生成/HELLO 失敗") {
                    break;
                }
                seen.push(line);
            }
            Err(e) => panic!("helper の行が届かない（{e:?}）。届いた行: {seen:?}"),
        }
    }
}
