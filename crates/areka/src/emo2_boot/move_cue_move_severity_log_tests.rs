// =============================================================================
// MoveCueSink severity 宛名規律 ログ捕捉檻（D8④・R2.4/R8.5）
//
// `MoveCueSink::emit` の「非正準 Custom params（as_command_carrier()==None）」枝が、
// 宛名（`Custom{command}` フィールド）で severity を分岐する D8④ 宛名規律
// （自分宛 move＝WARN／他人宛・未知＝DEBUG）を、決定論的なログ捕捉で回帰檻に入れる
// （目視でなく実行テストで網羅する必達方針）。入力依存の判断分岐ゆえ檻が要る。
//
// 定石: 硬化機構の唯一の定義元 `log-capture-kit` の捕捉窓へ委譲する（行の形も呼出側の判定内容も
// 移行前と 1 バイト変わらない）。
//
// 「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」は**誤り**である。差し替わる
// のはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
// interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点を最初に踏んだスレッドの判定が
// 焼き付く（先着が勝つ）。捕捉窓を持たないスレッドの既定は `NoSubscriber` で判定は「不要」ゆえ、
// 先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。
// 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 捕捉窓の内側での interest 再計算 ⑶ 番兵イベントに
// よる空振り検出 の 3 点でこれを塞ぐ（機序の逐条解説は `log_capture_kit` の crate doc と
// 同 crate の `src/probe.rs`）。
// =============================================================================

use super::*;
use dola::DynamicValue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{TryRecvError, channel};

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち指定 level（例 `"WARN"`／`"DEBUG"`）の件数を数える。
fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|l| l.contains(&needle)).count()
}

/// 非正準 params（`as_command_carrier()==None`）で宛名だけを差し替えた壊れ Custom cue を組む。
/// `params: DynamicValue::Null` は String Array でないため `as_command_carrier()` が None を返す
/// （テスト前提: 正準条件を満たさない実データ）——だが `Custom{command}` フィールドは読める。
fn broken_custom_cue(actor: &str, command: &str) -> TalkCue {
    let cue = TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::Custom {
            command: command.to_string(),
            params: DynamicValue::Null,
        },
        duration: 0.0,
    };
    // テスト前提の明示: この params 形は確かに非正準（開封が None）である。
    assert_eq!(
        cue.command.as_command_carrier(),
        None,
        "params=Null は非正準ゆえ as_command_carrier()==None（宛名分岐枝に入る前提）"
    );
    cue
}

/// D8④ アーム①: 宛名 `move` で params 非正準（自分宛の壊れ物）→ **WARN=1**・送出なし。
#[test]
fn broken_move_addressed_custom_warns_once() {
    let (tx, rx) = channel::<MoveDirective>();
    let mut sink = MoveCueSink::new(tx);
    let logs = capture_logs(|| sink.emit(broken_custom_cue("0", "move")));

    assert_eq!(
        count_level(&logs, "WARN"),
        1,
        "自分宛（move）の非正準 Custom は WARN=1（D8④・自分宛の壊れ物）: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "DEBUG"),
        0,
        "自分宛の壊れ物は DEBUG 良性 skip ではない（WARN 一択）: {logs:?}"
    );
    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Empty),
        "壊れ Custom は MoveDirective を送出しない（skip）"
    );
}

/// D8④ アーム②: 宛名が他人/未知で params 非正準（担当外）→ **WARN=0 かつ DEBUG 良性 skip**・送出なし。
#[test]
fn broken_other_addressed_custom_debug_not_warn() {
    for command in ["noexist", "bind", "raise"] {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        let logs = capture_logs(|| sink.emit(broken_custom_cue("0", command)));

        assert_eq!(
            count_level(&logs, "WARN"),
            0,
            "他人宛/未知（{command}）の非正準 Custom は WARN を出さない（担当外・D8④）: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            1,
            "他人宛/未知（{command}）は DEBUG で良性 skip（報告責任は宛名の担当者・R2.5）: {logs:?}"
        );
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "担当外の壊れ Custom は送出しない（skip）"
        );
    }
}

/// D8④ の非空虚化（WARN が blanket でない証）: **同一の非正準 params 形**で宛名だけを
/// `move`↔`noexist` に振り替えると、severity が WARN↔DEBUG に**分岐**する（level だけが変わる）。
/// 「壊れ Custom は一律 warn」という退行を 1 テスト内の対比で捕捉する。
#[test]
fn addressee_discriminates_warn_vs_debug_for_same_broken_shape() {
    // 自分宛（move）: WARN=1・DEBUG=0。
    let move_logs = capture_logs(|| {
        let (tx, _rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(broken_custom_cue("0", "move"));
    });
    // 他人宛（noexist）: 全く同じ params 形（Null）で宛名だけ差し替え。
    let other_logs = capture_logs(|| {
        let (tx, _rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(broken_custom_cue("0", "noexist"));
    });

    assert_eq!(
        (
            count_level(&move_logs, "WARN"),
            count_level(&move_logs, "DEBUG")
        ),
        (1, 0),
        "自分宛 move は WARN=1/DEBUG=0: {move_logs:?}"
    );
    assert_eq!(
        (
            count_level(&other_logs, "WARN"),
            count_level(&other_logs, "DEBUG")
        ),
        (0, 1),
        "他人宛 noexist は WARN=0/DEBUG=1: {other_logs:?}"
    );
    // 対比の要: 入力の params 形は同一・宛名のみ相違で severity が分岐する（blanket warn でない）。
    assert_ne!(
        count_level(&move_logs, "WARN"),
        count_level(&other_logs, "WARN"),
        "同一壊れ形でも宛名で WARN 件数が分岐する（宛名規律 D8④ の非空虚化）"
    );
}

/// 名前ゲート: 正準キャリアだが `name != "move"`（担当外）→ **DEBUG 良性 skip**・送出なし。
/// （params は正準ゆえ `as_command_carrier()` は Some で開封成功——分岐は名前ゲート側。）
#[test]
fn canonical_non_move_carrier_debug_skips() {
    for name in ["bind", "raise", "unknownfoo"] {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        let cue = TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::command_carrier(name, vec!["1".into(), "2".into()]),
            duration: 0.0,
        };
        let logs = capture_logs(|| sink.emit(cue));

        assert_eq!(
            count_level(&logs, "WARN"),
            0,
            "正準・担当外キャリア（{name}）は WARN を出さない（名前ゲート・R4.5）: {logs:?}"
        );
        assert_eq!(
            count_level(&logs, "DEBUG"),
            1,
            "正準・担当外キャリア（{name}）は DEBUG で良性 skip（名前自己選別・R8.5）: {logs:?}"
        );
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "担当外キャリア {name} は何も送出しない"
        );
    }
}
