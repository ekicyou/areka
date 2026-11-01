//! `PropGetCueSink` の檻（spec `shell-scale-linkage` 要件 7.1・design「Testing Strategy」1）。
//!
//! 偽の読み口＝FakeIo の sylphya に点付きの値を publish したもの、偽の kanade＝`mpsc` の受信端。
//! 受信端に届いた知らせ（`KanadeMsg::PropertyEvent`）と、捕捉した記録の段で振る舞いを固定する。

use super::*;
use crate::test_log_capture::{CapturedEvent, assert_logged_event, capture};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_sylphya::persist::FakePersistIo;
use areka_sylphya::{ScopeRoots, SylphyaInit, SylphyaParts, spawn_sylphya};
use std::sync::mpsc::{self, Receiver};
use tracing::Level;

const TARGET: &str = "areka_ghost::prop_get_sink";
const ASKER: &str = "ghost/a";

/// `\![name,tokens...]` の汎用キャリア cue（正準形）。
fn carrier(name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

/// 点付きの値 2 つ（`baseware.name`・`currentghost.scope(0).scaling`）を持つ読み口と、受け口と受信端。
fn rig() -> (SylphyaParts, PropGetCueSink, Receiver<KanadeMsg>) {
    let parts = spawn_sylphya(SylphyaInit {
        roots: ScopeRoots::default(),
        io: Box::new(FakePersistIo::new()),
        runtime_sink: None,
    });
    parts.publisher.publish_static(
        AskerId::new(ASKER),
        vec![],
        vec![
            ("baseware.name".to_string(), "areka".to_string()),
            (
                "currentghost.scope(0).scaling".to_string(),
                "150,150".to_string(),
            ),
            ("bad.value".to_string(), "a\r\nb".to_string()),
            ("nul.value".to_string(), "a\0b".to_string()),
        ],
    );
    parts
        .publisher
        .barrier()
        .expect("barrier while actor alive");
    let (tx, rx) = mpsc::channel();
    let sink = PropGetCueSink::new(parts.reader.clone(), AskerId::new(ASKER), tx);
    (parts, sink, rx)
}

/// 受信端に届いた知らせを全部取り出す。
fn drained(rx: &Receiver<KanadeMsg>) -> Vec<(String, Vec<String>)> {
    rx.try_iter()
        .map(|m| match m {
            KanadeMsg::PropertyEvent { id, references } => (id, references),
            _ => panic!("PropertyEvent 以外が届いた"),
        })
        .collect()
}

fn warns(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.target == TARGET && e.level == Level::WARN)
        .count()
}

/// 値は名前の順に Reference0 から並び、無い名前は空で番号をずらさない（要件 1.2・1.4）。
/// 受け手は名前を問わない（`baseware.name`・要件 1.5）。送るのは 1 回だけで、成功は debug（要件 2.5）。
#[test]
fn values_in_order_missing_is_empty_and_sent_once() {
    let (_parts, mut sink, rx) = rig();
    let events = capture(|| {
        sink.emit(carrier(
            "get",
            &[
                "property",
                "OnX",
                "currentghost.scope(0).scaling",
                "no.such.name",
                "baseware.name",
            ],
        ))
    });
    assert_eq!(
        drained(&rx),
        vec![(
            "OnX".to_string(),
            vec!["150,150".to_string(), String::new(), "areka".to_string()]
        )]
    );
    assert_logged_event(&events, Level::DEBUG, TARGET, "property_get_sent");
    assert_eq!(warns(&events), 0);
}

/// `On` で始まらない名前もそのまま送る（要件 2.1）。
#[test]
fn event_name_not_starting_with_on_is_sent() {
    let (_parts, mut sink, rx) = rig();
    sink.emit(carrier(
        "get",
        &["property", "pasta_scale", "baseware.name"],
    ));
    assert_eq!(
        drained(&rx),
        vec![("pasta_scale".to_string(), vec!["areka".to_string()])]
    );
}

/// イベント名が空・空白だけ・無いは送らず warn 1 件（要件 2.2）。
#[test]
fn empty_blank_or_missing_event_name_warns_and_does_not_send() {
    for tokens in [
        &["property", "", "baseware.name"][..],
        &["property", "  \t", "baseware.name"][..],
        &["property"][..],
    ] {
        let (_parts, mut sink, rx) = rig();
        let events = capture(|| sink.emit(carrier("get", tokens)));
        assert!(drained(&rx).is_empty(), "送ってはならない: {tokens:?}");
        assert_logged_event(&events, Level::WARN, TARGET, "property_get_empty_event");
        assert_eq!(warns(&events), 1, "{tokens:?}");
    }
}

/// 名前が 1 つも無いときは値を引く前に debug で打ち切り、送らない（要件 2.4）。
#[test]
fn no_names_stops_below_warn() {
    let (_parts, mut sink, rx) = rig();
    let events = capture(|| sink.emit(carrier("get", &["property", "OnX"])));
    assert!(drained(&rx).is_empty());
    assert_logged_event(&events, Level::DEBUG, TARGET, "property_get_no_names");
    assert_eq!(warns(&events), 0);
}

/// 引いた値・イベント名に CR・LF・NUL があれば warn で送らない（要件 2.3）。
#[test]
fn control_characters_warn_and_do_not_send() {
    for tokens in [
        &["property", "OnX", "bad.value"][..],
        &["property", "OnX", "nul.value"][..],
        &["property", "On\nX", "baseware.name"][..],
        &["property", "On\0X", "baseware.name"][..],
    ] {
        let (_parts, mut sink, rx) = rig();
        let events = capture(|| sink.emit(carrier("get", tokens)));
        assert!(drained(&rx).is_empty(), "送ってはならない: {tokens:?}");
        assert_logged_event(&events, Level::WARN, TARGET, "property_get_control_char");
        assert_eq!(warns(&events), 1, "{tokens:?}");
    }
}

/// 担当外（`\![get,other]`・`\![set,property]`・非キャリア）は warn より下で読み飛ばす。
#[test]
fn not_mine_is_skipped_below_warn() {
    let (_parts, mut sink, rx) = rig();
    let events = capture(|| {
        sink.emit(carrier("get", &["other", "OnX", "baseware.name"]));
        sink.emit(carrier("set", &["property", "OnX", "baseware.name"]));
        sink.emit(TalkCue {
            at: 0.0,
            actor: ActorKey::from("0"),
            command: CueCommand::Text("hi".into()),
            duration: 0.0,
        });
    });
    assert!(drained(&rx).is_empty());
    assert_eq!(warns(&events), 0);
}

/// kanade が落ちていても panic せず warn を残す。
#[test]
fn kanade_gone_warns_without_panic() {
    let (_parts, mut sink, rx) = rig();
    drop(rx);
    let events = capture(|| sink.emit(carrier("get", &["property", "OnX", "baseware.name"])));
    assert_logged_event(&events, Level::WARN, TARGET, "property_get_kanade_gone");
}
