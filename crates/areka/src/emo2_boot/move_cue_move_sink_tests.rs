// =============================================================================
// MoveCueSink 名前選別 sink 檻（task 7.3・R4.5・R8.5）
// =============================================================================

use super::*;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use std::sync::mpsc::{TryRecvError, channel};

/// `\![name,tokens...]` 汎用キャリア cue を組む（正準形＝`Custom` の String Array）。
fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
    let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::command_carrier(name, toks),
        duration: 0.0,
    }
}

/// 名前選別: `"move"` キャリアのみ解釈して MoveDirective を送出する（R4.5・最初の実消費者）。
#[test]
fn move_carrier_sends_directive() {
    let (tx, rx) = channel::<MoveDirective>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(carrier_cue(
        "0",
        "move",
        &["-353", "", "", "0", "base", "base"],
    ));
    let d = rx
        .try_recv()
        .expect("move キャリアは MoveDirective を送出する");
    assert_eq!(d.scope, 0);
    assert_eq!(d.x, AxisSpec::Px(-353));
    assert_eq!(d.base, MoveBase::Scope(0));
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "送出は 1 件のみ");
}

/// 担当外キャリア（`bind`/`raise`/未知名）は良性スキップ＝何も送出しない
/// （高々 1 消費者・自らの名前リテラル `"move"` で自己選別・一意性は areka 消費者台帳が保証・R4.5）。
#[test]
fn non_move_carrier_is_benign_skip() {
    for name in ["bind", "raise", "unknownfoo"] {
        let (tx, rx) = channel::<MoveDirective>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(carrier_cue("0", name, &["1", "2"]));
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "担当外キャリア {name} は何も送出しない（良性スキップ・R8.5）"
        );
    }
}

/// 非キャリア cue（`Text` 等の担当外 broadcast）は良性スキップ＝何も送出しない（R8.5）。
#[test]
fn non_carrier_cue_is_benign_skip() {
    let (tx, rx) = channel::<MoveDirective>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("アヒル".into()),
        duration: 0.0,
    });
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
}

/// scope 抽出: `\1` actor → `MoveDirective.scope == 1`（scope は cue.actor 由来）。
#[test]
fn scope_reflects_actor() {
    let (tx, rx) = channel::<MoveDirective>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(carrier_cue(
        "1",
        "move",
        &["-353", "", "", "0", "base", "base"],
    ));
    let d = rx.try_recv().expect("送出される");
    assert_eq!(d.scope, 1, "scope は cue.actor（\\1）由来");
}

/// 非数値 actor（`sakura` 等）は warn＋スキップ＝非 panic・何も送出しない（design 破損・異常）。
#[test]
fn non_numeric_actor_is_skipped() {
    let (tx, rx) = channel::<MoveDirective>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(carrier_cue("sakura", "move", &["-353"]));
    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Empty),
        "非数値 scope はスキップ（非 panic）"
    );
}

/// 名前付きの形も旧い形と同じく読んで送る（要件 1.1・areka-P0-move-canon）。
#[test]
fn named_form_is_sent() {
    let (tx, rx) = channel::<MoveDirective>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(carrier_cue(
        "1",
        "move",
        &[
            "--X=0",
            "--Y=0",
            "--base=0",
            "--base-offset=left.bottom",
            "--move-offset=right.bottom",
        ],
    ));
    let d = rx.try_recv().expect("名前付きの形も送出する");
    assert_eq!(d.scope, 1);
    assert_eq!((d.x, d.y), (AxisSpec::Px(0), AxisSpec::Px(0)));
    assert_eq!(d.base, MoveBase::Scope(0));
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "送出は 1 件のみ");
}

/// `MoveCueSink` は Clone: clone 側から送出しても同一受信端へ届く
/// （dispatcher が talk ごとに sink を clone する前提・boot 型境界）。
#[test]
fn clone_reaches_same_receiver() {
    let (tx, rx) = channel::<MoveDirective>();
    let sink = MoveCueSink::new(tx);
    let mut clone = sink.clone();
    clone.emit(carrier_cue(
        "0",
        "move",
        &["10", "", "", "0", "base", "base"],
    ));
    let d = rx.try_recv().expect("clone からの送出も同一受信端へ届く");
    assert_eq!(d.x, AxisSpec::Px(10));
    drop(sink); // 元 sink 生存の確認（Clone は独立ハンドル）。
}

/// boot 型境界（`dola::cue::CueSink + Clone + Send + 'static`）をコンパイル時に固定する。
#[test]
fn satisfies_boot_sink_bounds() {
    fn require<T: dola::cue::CueSink + Clone + Send + 'static>() {}
    require::<MoveCueSink>();
}
