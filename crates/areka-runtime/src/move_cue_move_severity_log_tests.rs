// MoveCueSink の severity 宛名規律のログ捕捉テスト。
//
// 非正準 Custom params の枝は、宛名（`Custom{command}`）で severity を分岐する
// （自分宛（move・lock・unlock）＝WARN／他人宛・未知＝DEBUG）。
//
// 捕捉は `log-capture-kit` の捕捉窓へ委譲する。`with_default` はスレッドローカルだが、
// callsite の interest キャッシュはプロセス全体で 1 つなので、捕捉窓なしで先に踏まれると
// 取りこぼす（機序は `log_capture_kit` の crate doc）。

use super::*;
use crate::consumer_ledger::test_support::{actor_carrier_cue, broken_custom_cue};
use crate::frame::test_support::{capture_logs, count_level};
use dola::cue::CueSink;
use std::sync::mpsc::{TryRecvError, channel};

#[test]
fn broken_custom_addressed_to_this_sink_warns_once() {
    for command in ["move", "lock", "unlock"] {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        let logs = capture_logs(|| sink.emit(broken_custom_cue("0", command)));

        assert_eq!(
            count_level(&logs, "WARN"),
            1,
            "自分宛（{command}）の非正準 Custom は WARN=1（自分宛の壊れ物）: {logs:?}"
        );
        assert!(
            logs.iter()
                .any(|l| l.contains("level=WARN") && l.contains("move_cue_unopenable")),
            "{logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            0,
            "自分宛（{command}）の壊れ物は DEBUG 良性 skip ではない（WARN 一択）: {logs:?}"
        );
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "壊れ Custom（{command}）は指令を送出しない（skip）"
        );
    }
}

/// 宛名が他人/未知で params 非正準（担当外）→ WARN=0 かつ DEBUG 良性 skip・送出なし。
#[test]
fn broken_other_addressed_custom_debug_not_warn() {
    for command in ["noexist", "bind", "raise"] {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        let logs = capture_logs(|| sink.emit(broken_custom_cue("0", command)));

        assert_eq!(
            count_level(&logs, "WARN"),
            0,
            "他人宛/未知（{command}）の非正準 Custom は WARN を出さない（担当外）: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            1,
            "他人宛/未知（{command}）は DEBUG で良性 skip（報告責任は宛名の担当者）: {logs:?}"
        );
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "担当外の壊れ Custom は送出しない（skip）"
        );
    }
}

/// 名前ゲート: 正準キャリアだが `name != "move"`（担当外）→ DEBUG 良性 skip・送出なし。
/// （params は正準ゆえ `as_command_carrier()` は Some で開封成功——分岐は名前ゲート側。）
#[test]
fn canonical_non_move_carrier_debug_skips() {
    for name in ["bind", "raise", "unknownfoo"] {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        let cue = actor_carrier_cue("0", name, &["1", "2"]);
        let logs = capture_logs(|| sink.emit(cue));

        assert_eq!(
            count_level(&logs, "WARN"),
            0,
            "正準・担当外キャリア（{name}）は WARN を出さない（名前ゲート）: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            1,
            "正準・担当外キャリア（{name}）は DEBUG で良性 skip（名前自己選別）: {logs:?}"
        );
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "担当外キャリア {name} は何も送出しない"
        );
    }
}

/// 読みの置き換えの記録は `move_arg_substituted` の WARN として 1 件ずつ出て、指令は送られる。
/// 台本どおりの読みは WARN を出さない。
#[test]
fn parse_notes_warn_once_each_and_directive_is_sent() {
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    let cue = |tokens: &[&str]| actor_carrier_cue("1", "move", tokens);

    let logs = capture_logs(|| sink.emit(cue(&["--X=abc", "--bogus=1", "--time=500"])));
    let substituted: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("level=WARN") && l.contains("move_arg_substituted"))
        .collect();
    assert_eq!(substituted.len(), 3, "記録 3 件 = WARN 3 行: {logs:?}");
    assert_eq!(count_level(&logs, "WARN"), 3, "{logs:?}");
    assert!(rx.try_recv().is_ok(), "置き換えがあっても指令は送られる");

    let clean = capture_logs(|| sink.emit(cue(&["-353", "", "", "0", "base", "base"])));
    assert_eq!(count_level(&clean, "WARN"), 0, "{clean:?}");
    assert!(rx.try_recv().is_ok());
}

/// 宛名が `execute` の開けない荷物は `warn!` にしない（`warn!` は install の受け口が持つ・同じ荷物に
/// 2 行出さない）。DEBUG で読み飛ばし、何も送らない。
#[test]
fn broken_execute_addressed_custom_is_debug_not_warn() {
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    let logs = capture_logs(|| sink.emit(broken_custom_cue("0", "execute")));
    assert_eq!(count_level(&logs, "WARN"), 0, "{logs:?}");
    assert_eq!(count_level(&logs, "DEBUG"), 1, "{logs:?}");
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
}

/// `\![execute,resetwindowpos]` は「並べ直し」を 1 件送る。残りの引数は読まず、WARN を出さない。
#[test]
fn execute_resetwindowpos_sends_without_warn() {
    for tokens in [
        &["resetwindowpos"][..],
        &["resetwindowpos", "junk", "1"][..],
    ] {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        let cue = actor_carrier_cue("0", "execute", tokens);
        let logs = capture_logs(|| sink.emit(cue));
        assert_eq!(count_level(&logs, "WARN"), 0, "{tokens:?}: {logs:?}");
        assert_eq!(rx.try_recv(), Ok(PositionCue::ResetWindowPos), "{tokens:?}");
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "送出は 1 件のみ");
    }
}
