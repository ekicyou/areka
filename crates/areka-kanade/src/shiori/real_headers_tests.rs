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
