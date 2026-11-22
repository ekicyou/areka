// MoveCueSink の名前選別のテスト。

use super::*;
use crate::consumer_ledger::test_support::actor_carrier_cue;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use std::sync::mpsc::{TryRecvError, channel};

/// 線から届いた指令が「移動」であることを確かめて中身を返す。
fn as_move(cue: PositionCue) -> MoveDirective {
    match cue {
        PositionCue::Move(d) => d,
        other => panic!("移動の指令のはずが {other:?}"),
    }
}

/// 名前選別: `"move"` キャリアのみ解釈して MoveDirective を送出する。
#[test]
fn move_carrier_sends_directive() {
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(actor_carrier_cue(
        "0",
        "move",
        &["-353", "", "", "0", "base", "base"],
    ));
    let d = as_move(
        rx.try_recv()
            .expect("move キャリアは PositionCue::Move を送出する"),
    );
    assert_eq!(d.scope, 0);
    assert_eq!(d.x, AxisSpec::Px(-353));
    assert_eq!(d.base, MoveBase::Scope(0));
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "送出は 1 件のみ");
}

/// 非キャリア cue（`Text` 等の担当外 broadcast）は良性スキップ＝何も送出しない。
#[test]
fn non_carrier_cue_is_benign_skip() {
    let (tx, rx) = channel::<PositionCue>();
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
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(actor_carrier_cue(
        "1",
        "move",
        &["-353", "", "", "0", "base", "base"],
    ));
    let d = as_move(rx.try_recv().expect("送出される"));
    assert_eq!(d.scope, 1, "scope は cue.actor（\\1）由来");
}

/// 非数値 actor（`sakura` 等）は warn＋スキップ＝非 panic・何も送出しない。
#[test]
fn non_numeric_actor_is_skipped() {
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(actor_carrier_cue("sakura", "move", &["-353"]));
    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Empty),
        "非数値 scope はスキップ（非 panic）"
    );
}

/// 名前付きの形も旧い形と同じく読んで送る。
#[test]
fn named_form_is_sent() {
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    sink.emit(actor_carrier_cue(
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
    let d = as_move(rx.try_recv().expect("名前付きの形も送出する"));
    assert_eq!(d.scope, 1);
    assert_eq!((d.x, d.y), (AxisSpec::Px(0), AxisSpec::Px(0)));
    assert_eq!(d.base, MoveBase::Scope(0));
    assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "送出は 1 件のみ");
}

/// `MoveCueSink` は Clone: clone 側から送出しても同一受信端へ届く
/// （dispatcher が talk ごとに sink を clone する前提・boot 型境界）。
#[test]
fn clone_reaches_same_receiver() {
    let (tx, rx) = channel::<PositionCue>();
    let sink = MoveCueSink::new(tx);
    let mut clone = sink.clone();
    clone.emit(actor_carrier_cue(
        "0",
        "move",
        &["10", "", "", "0", "base", "base"],
    ));
    let d = as_move(rx.try_recv().expect("clone からの送出も同一受信端へ届く"));
    assert_eq!(d.x, AxisSpec::Px(10));
}

/// boot 型境界（`dola::cue::CueSink + Clone + Send + 'static`）をコンパイル時に固定する。
#[test]
fn satisfies_boot_sink_bounds() {
    fn require<T: dola::cue::CueSink + Clone + Send + 'static>() {}
    require::<MoveCueSink>();
}

/// `\![execute,install,…]`・引数の無い `execute`・`\![set,position,…]`・`\![reset,position]` は
/// どれも送らない（install は別の受け口・位置の set／reset は SSP 2.9.18 も受けない）。
#[test]
fn other_execute_and_position_tags_are_not_sent() {
    let cases: [(&str, &[&str]); 4] = [
        ("execute", &["install", "url", "x"]),
        ("execute", &[]),
        ("set", &["position", "100", "100", "1"]),
        ("reset", &["position"]),
    ];
    for (name, tokens) in cases {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(actor_carrier_cue("0", name, tokens));
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "{name},{tokens:?} は送らない"
        );
    }
}

/// 窓の組・吸着・バルーン位置初期化の 5 組は、それぞれの種類で 1 件だけ送る。
/// 吸着は実行者のスコープを持ち、組の ID は読まずにそのまま運ぶ。
#[test]
fn window_link_pairs_send_their_cues() {
    use areka_placement::resolver::Anchor;
    use areka_placement::script_alignment::AlignmentWord;
    let cases: [(&str, &str, &[&str], PositionCue); 6] = [
        (
            "1",
            "set",
            &["alignmenttodesktop", "top"],
            PositionCue::SetAlignment {
                scope: 1,
                word: AlignmentWord::Anchor(Anchor::Top),
            },
        ),
        (
            "0",
            "set",
            &["alignmentondesktop", "Free", "junk"],
            PositionCue::SetAlignment {
                scope: 0,
                word: AlignmentWord::Anchor(Anchor::Free),
            },
        ),
        (
            "0",
            "set",
            &["alignmenttodesktop"],
            PositionCue::SetAlignment {
                scope: 0,
                word: AlignmentWord::Missing,
            },
        ),
        (
            "sakura",
            "set",
            &["sticky-window", "0", "x", "0"],
            PositionCue::SetSticky {
                tokens: vec!["0".into(), "x".into(), "0".into()],
            },
        ),
        (
            "sakura",
            "reset",
            &["sticky-window", "junk"],
            PositionCue::ResetSticky,
        ),
        (
            "sakura",
            "execute",
            &["resetballoonpos", "junk"],
            PositionCue::ResetBalloonPos,
        ),
    ];
    for (actor, name, tokens, expected) in cases {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(actor_carrier_cue(actor, name, tokens));
        assert_eq!(rx.try_recv(), Ok(expected), "{name},{tokens:?}");
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty), "送出は 1 件のみ");
    }
}

/// 吸着の実行者が数でなければ `warn!` で捨てる（`move` と同じ読み）。
#[test]
fn alignment_with_non_numeric_actor_is_dropped_with_warn() {
    // 記録の `tag` は実際に届いた第 1 引数の綴り（`alignmentondesktop` も取り違えない）。
    for selector in ["alignmenttodesktop", "alignmentondesktop"] {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        let (_, events) = log_capture_kit::capture(|| {
            sink.emit(actor_carrier_cue("sakura", "set", &[selector, "top"]))
        });
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
        let expected = format!("set,{selector}");
        assert!(
            events.iter().any(|e| e.level == tracing::Level::WARN
                && e.field_str("tag") == Some(expected.as_str())),
            "tag={expected} の warn! が出ていない: {events:?}"
        );
    }
}

/// 掛け合わせの組（名前と第 1 引数の取り違え）は送らない。
#[test]
fn crossed_window_link_pairs_are_not_sent() {
    let cases: [(&str, &[&str]); 5] = [
        ("reset", &["alignmenttodesktop", "top"]),
        ("reset", &["alignmentondesktop"]),
        ("execute", &["sticky-window", "0", "1"]),
        ("set", &["resetballoonpos"]),
        ("execute", &["alignmenttodesktop", "top"]),
    ];
    for (name, tokens) in cases {
        let (tx, rx) = channel::<PositionCue>();
        let mut sink = MoveCueSink::new(tx);
        sink.emit(actor_carrier_cue("0", name, tokens));
        assert_eq!(
            rx.try_recv(),
            Err(TryRecvError::Empty),
            "{name},{tokens:?} は送らない"
        );
    }
}

/// 受信端が落ちていても落ちず、送れなかったことを `warn!` 1 行で記録する。
#[test]
fn dropped_receiver_is_logged_and_does_not_panic() {
    let (tx, rx) = channel::<PositionCue>();
    let mut sink = MoveCueSink::new(tx);
    drop(rx);
    let ((), events) = log_capture_kit::capture(|| {
        sink.emit(actor_carrier_cue(
            "0",
            "move",
            &["-353", "", "", "0", "base", "base"],
        ))
    });
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "{events:?}");
    assert!(warns[0].message().contains("送出に失敗"), "{events:?}");
}
