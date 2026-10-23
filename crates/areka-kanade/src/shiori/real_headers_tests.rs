//! ヘッダつきの GET／NOTIFY の既定の実装の檻（areka-P0-script-security-level 要件 2.1・2.8・3.1）。
//!
//! 既定のままの受け手（テストの偽物・中身なしの受け手）は、ヘッダを使わず今の `get`／`notify`
//! を呼び、応答のレベルは無し——振る舞いは今と同じであることを固定する。

use super::*;

/// 呼ばれた往復を記録し、GET は id と status をつないだ値を返す偽物（既定の実装のまま）。
#[derive(Default)]
struct DefaultsBackend {
    calls: Vec<String>,
}

impl ShioriBackend for DefaultsBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.calls
            .push(format!("get:{id}:{}:{status:?}", references.join(",")));
        Ok((id != "OnEmpty").then(|| format!("value-of-{id}")))
    }
    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.calls
            .push(format!("notify:{id}:{}:{status:?}", references.join(",")));
        Ok(())
    }
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        Ok(ExitKind::Clean)
    }
    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }
}

fn external_headers() -> RequestHeaders<'static> {
    RequestHeaders {
        security_level: "external",
        security_origin: "null",
        sender: "MCP",
        sender_type: "external",
    }
}

#[test]
fn default_get_with_headers_calls_get_and_has_no_level() {
    let mut backend = DefaultsBackend::default();
    let refs = vec!["a".to_string(), "b".to_string()];
    let reply = backend
        .get_with_headers("OnBoot", &refs, Some("talking"), &external_headers())
        .expect("既定の GET は get の結果をそのまま返す");
    assert_eq!(
        reply,
        Some(ValueReply {
            value: "value-of-OnBoot".to_string(),
            security_level: None,
        })
    );
    assert_eq!(backend.calls, vec!["get:OnBoot:a,b:Some(\"talking\")"]);
}

#[test]
fn default_get_with_headers_keeps_no_content() {
    let mut backend = DefaultsBackend::default();
    let reply = backend
        .get_with_headers("OnEmpty", &[], None, &external_headers())
        .expect("204 は Ok(None)");
    assert_eq!(reply, None);
    assert_eq!(backend.calls, vec!["get:OnEmpty::None"]);
}

#[test]
fn default_get_with_headers_passes_errors_through() {
    struct Failing;
    impl ShioriBackend for Failing {
        fn get(
            &mut self,
            _id: &str,
            _references: &[String],
            _status: Option<&str>,
        ) -> Result<Option<String>, RequestError> {
            Err(RequestError::Timeout)
        }
        fn notify(
            &mut self,
            _id: &str,
            _references: &[String],
            _status: Option<&str>,
        ) -> Result<(), RequestError> {
            Err(RequestError::Timeout)
        }
        fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
            Ok(ExitKind::Clean)
        }
        fn status(&mut self) -> HelperStatus {
            HelperStatus::Running
        }
    }
    let headers = external_headers();
    assert!(matches!(
        Failing.get_with_headers("OnBoot", &[], None, &headers),
        Err(RequestError::Timeout)
    ));
    assert!(matches!(
        Failing.notify_with_headers("OnBoot", &[], None, &headers),
        Err(RequestError::Timeout)
    ));
}

#[test]
fn default_notify_with_headers_calls_notify() {
    let mut backend = DefaultsBackend::default();
    let refs = vec!["x".to_string()];
    backend
        .notify_with_headers("OnNotify", &refs, Some("online"), &external_headers())
        .expect("既定の NOTIFY は notify の結果をそのまま返す");
    assert_eq!(backend.calls, vec!["notify:OnNotify:x:Some(\"online\")"]);
}

// ---- handle_call が要求の出どころから入れ物を作り、応答のレベルを結果へ載せる（タスク 3.4） ----

use crate::msg::EventId;
use crate::request_origin::{RequestOrigin, SenderName};
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use areka_actor::reply_channel;
use areka_talk::{ScriptOrigin, SecurityLevel};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// 渡った入れ物の写し（Sender・SenderType・SecurityLevel・SecurityOrigin の順）。
type Seen = (String, String, String, String);

/// ヘッダつきのメソッドを上書きし、渡った入れ物を記録して決めた生のレベルで答える偽物。
struct HeaderRecorder {
    seen: Arc<Mutex<Vec<Seen>>>,
    reply_level: Option<String>,
}

impl HeaderRecorder {
    fn record(&self, headers: &RequestHeaders<'_>) {
        self.seen.lock().unwrap().push((
            headers.sender.to_string(),
            headers.sender_type.to_string(),
            headers.security_level.to_string(),
            headers.security_origin.to_string(),
        ));
    }
}

impl ShioriBackend for HeaderRecorder {
    fn get(
        &mut self,
        _: &str,
        _: &[String],
        _: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        panic!("ヘッダの無い get は呼ばれてはならない")
    }
    fn notify(&mut self, _: &str, _: &[String], _: Option<&str>) -> Result<(), RequestError> {
        panic!("ヘッダの無い notify は呼ばれてはならない")
    }
    fn get_with_headers(
        &mut self,
        _: &str,
        _: &[String],
        _: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<Option<ValueReply>, RequestError> {
        self.record(headers);
        Ok(Some(ValueReply {
            value: "hello".to_string(),
            security_level: self.reply_level.clone(),
        }))
    }
    fn notify_with_headers(
        &mut self,
        _: &str,
        _: &[String],
        _: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<(), RequestError> {
        self.record(headers);
        Ok(())
    }
    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        Ok(ExitKind::Clean)
    }
    fn status(&mut self) -> HelperStatus {
        HelperStatus::Running
    }
}

/// SHIORI の回り役へ偽物を挿して 1 往復させ、渡った入れ物と結果を返す。
fn via_worker(call: ShioriCall, reply_level: Option<&str>) -> (Vec<Seen>, ShioriOutcome) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let backend = HeaderRecorder {
        seen: Arc::clone(&seen),
        reply_level: reply_level.map(str::to_string),
    };
    let (on_down, _on_down_rx) = mpsc::channel::<KanadeMsg>();
    let (tx, handle, _probe) = spawn_shiori_actor(
        move || Ok(Box::new(backend) as Box<dyn ShioriBackend>),
        on_down,
    );
    let (reply, receiver) = reply_channel::<ShioriOutcome>();
    tx.send(ShioriMsg::Request { call, reply })
        .expect("send Request");
    let outcome = receiver.recv().expect("reply received");
    tx.send(ShioriMsg::Close).expect("send Close");
    drop(tx);
    handle.join().expect("worker joins");
    let seen = seen.lock().unwrap().clone();
    (seen, outcome)
}

fn get(id: EventId) -> ShioriCall {
    ShioriCall::Get {
        id,
        references: Vec::new(),
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

fn notify(id: EventId) -> ShioriCall {
    ShioriCall::Notify {
        id,
        references: Vec::new(),
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    }
}

fn seen(sender: &str, sender_type: &str) -> Seen {
    (
        sender.to_string(),
        sender_type.to_string(),
        "local".to_string(),
        "null".to_string(),
    )
}

#[test]
fn schedule_category_passes_areka_internal() {
    // 運行表（要件 2.2）: GET も NOTIFY も同じ入れ物。
    let (got, _) = via_worker(get(EventId::Static("OnBoot")), None);
    assert_eq!(got, vec![seen("areka", "internal")]);
    let (got, outcome) = via_worker(notify(EventId::Static("OnNotifyOSInfo")), None);
    assert_eq!(got, vec![seen("areka", "internal")]);
    assert!(matches!(outcome, ShioriOutcome::Notified));
}

#[test]
fn choice_category_passes_areka_internal() {
    // 選択肢（要件 2.2）。
    let (got, _) = via_worker(get(EventId::Choice("OnChoiceX".to_string())), None);
    assert_eq!(got, vec![seen("areka", "internal")]);
}

#[test]
fn raise_event_category_passes_mcp_external_sstp() {
    // MCP の raise_event（要件 2.3）: GET も NOTIFY も同じ入れ物。
    let (got, _) = via_worker(get(EventId::Sstp("OnMcpEvent".to_string())), None);
    assert_eq!(got, vec![seen("MCP", "external,sstp")]);
    let (got, _) = via_worker(notify(EventId::Sstp("OnMcpEvent".to_string())), None);
    assert_eq!(got, vec![seen("MCP", "external,sstp")]);
}

#[test]
fn translate_category_passes_the_held_origin() {
    // MCP の sakurascript の翻訳（要件 2.4）。
    let (got, _) = via_worker(
        get(EventId::Translate(RequestOrigin::MCP_SEND_TRANSLATE)),
        None,
    );
    assert_eq!(got, vec![seen("MCP", "internal")]);
    // ゴースト自身の応答の翻訳（要件 2.5）。
    let ghost = RequestOrigin {
        sender: SenderName::Areka,
        origin: ScriptOrigin::GHOST,
    };
    let (got, _) = via_worker(get(EventId::Translate(ghost)), None);
    assert_eq!(got, vec![seen("areka", "internal")]);
    // raise_event の応答の翻訳（要件 2.6）。
    let raised = RequestOrigin::for_translated(&EventId::Sstp("OnMcpEvent".to_string()));
    let (got, _) = via_worker(get(EventId::Translate(raised)), None);
    assert_eq!(got, vec![seen("areka", "external,sstp")]);
}

fn level_of(outcome: ShioriOutcome) -> SecurityLevel {
    match outcome {
        ShioriOutcome::Value(value, level) => {
            assert_eq!(value, "hello");
            level
        }
        _ => panic!("Value の結果を期待した"),
    }
}

#[test]
fn external_reply_level_reaches_the_outcome() {
    // 応答の external（大小無視）で結果のレベルが external（要件 3.1）。
    let (_, outcome) = via_worker(get(EventId::Static("OnBoot")), Some("External"));
    assert_eq!(level_of(outcome), SecurityLevel::External);
    let (_, outcome) = via_worker(
        get(EventId::Translate(RequestOrigin::MCP_SEND_TRANSLATE)),
        Some("external"),
    );
    assert_eq!(level_of(outcome), SecurityLevel::External);
}

#[test]
fn absent_or_unreadable_reply_level_is_local() {
    // 無い → local・読めない → local（要件 3.4）。
    let (_, outcome) = via_worker(get(EventId::Static("OnBoot")), None);
    assert_eq!(level_of(outcome), SecurityLevel::Local);
    let (_, outcome) = via_worker(get(EventId::Static("OnBoot")), Some("bogus"));
    assert_eq!(level_of(outcome), SecurityLevel::Local);
}
