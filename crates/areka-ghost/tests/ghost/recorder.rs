//! recorder.rs — 駆動対象非依存の交信記録デコレータ `Recorder<B: ShioriBackend>`（テスト支援）。
//!
//! 設計 design.md「Recorder（交信記録デコレータ）」の Service Interface が型定義の正本。
//! 発行イベントの種別・ID・順序・結果を、駆動対象（InProc 実 DLL backend／`ScriptedShioriBackend`
//! ／実 `ShioriConnection`）の種類を問わず同一の手口（`ShioriBackend` seam へのデコレート）で
//! 記録する（要件 1.4「同一手口」）。採取ハーネス（要件 2.6）とも共用する。
//!
//! # 不変条件
//! - 記録は呼出順（shiori actor による直列化）で**追記のみ**（並べ替え・削除をしない）。
//! - `status()` は素通し（sticky 死活検査ノイズを記録に混ぜない・append-only を呼出順で保つ）。
//! - `RequestError` は非 Clone/非 PartialEq のため、結果は要約列挙 [`ExchangeOutcome`]（文字列化）
//!   で保持し assert 可能性（PartialEq）を確保する（design「Data Models → ExchangeRecord」）。
//!
//! 使う pub 項目は子モジュールごとに違い、この test binary で未使用のものもあるため dead-code 警告を抑止する。
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use areka_kanade::{ShioriBackend, ShioriUnblock};
use shiori_host32_host::{
    ExitKind, HelperStatus, RequestError, RequestHeaders, ShutdownError, ValueReply,
};

/// 交信の種別（`status()` は記録対象外ゆえ variant を持たない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeKind {
    /// GET（応答を要するイベント）。
    Get,
    /// NOTIFY（片道イベント）。
    Notify,
    /// unload（正規 clean shutdown）。
    Unload,
}

/// 1 交信の結果（`RequestError` が非 Clone ゆえ結果は要約列挙で保持する）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeOutcome {
    /// get `Ok(Some)`。
    Value(String),
    /// get `Ok(None)`（204 相当）。
    NoContent,
    /// notify `Ok(())`。
    NotifyOk,
    /// `Err`（Display 文字列化）。
    Failed(String),
    /// unload の結果（`ExitKind`／`Err` の Debug 文字列化）。
    Unloaded(String),
}

/// 1 交信の記録。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeRecord {
    /// 交信の種別（Get / Notify / Unload）。
    pub kind: ExchangeKind,
    /// イベント ID（Unload は `None`）。
    pub id: Option<String>,
    /// Reference 群（Unload は空）。
    pub references: Vec<String>,
    /// render 済み wire status（`None`＝ヘッダ行なし・Unload は `None`）。
    pub status: Option<String>,
    /// 結果（要約列挙）。
    pub outcome: ExchangeOutcome,
}

/// `ShioriBackend` seam に噛ませる、駆動対象非依存の交信記録デコレータ。
///
/// `inner`（任意の `B: ShioriBackend`）へ全呼出を委譲しつつ、GET/NOTIFY/unload を共有ログへ
/// 追記する。戻り値は一切改変せず素通しする。
pub struct Recorder<B: ShioriBackend> {
    inner: B,
    log: Arc<Mutex<Vec<ExchangeRecord>>>,
}

/// [`Recorder`] の記録を（別スレッドへ move された後も）読み出すための `Clone` 可能なハンドル。
#[derive(Clone)]
pub struct RecorderHandle {
    log: Arc<Mutex<Vec<ExchangeRecord>>>,
}

impl RecorderHandle {
    /// 記録のスナップショット（クローン）を返す。
    #[must_use]
    pub fn records(&self) -> Vec<ExchangeRecord> {
        self.log
            .lock()
            .expect("recorder log mutex poisoned")
            .clone()
    }
}

impl<B: ShioriBackend> Recorder<B> {
    /// `inner` を包んだ記録デコレータと、記録読出用の [`RecorderHandle`] のペアを返す。
    pub fn new(inner: B) -> (Self, RecorderHandle) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let recorder = Recorder {
            inner,
            log: Arc::clone(&log),
        };
        let handle = RecorderHandle { log };
        (recorder, handle)
    }

    /// 記録を末尾へ追記する（呼出順の直列化は shiori actor 側が保証する）。
    fn append(&self, record: ExchangeRecord) {
        self.log
            .lock()
            .expect("recorder log mutex poisoned")
            .push(record);
    }
}

impl<B: ShioriBackend> ShioriBackend for Recorder<B> {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        let result = self.inner.get(id, references, status);
        // 結果は参照で検査して要約列挙を作る（`result` を消費しない）。
        let outcome = match &result {
            Ok(Some(value)) => ExchangeOutcome::Value(value.clone()),
            Ok(None) => ExchangeOutcome::NoContent,
            Err(e) => ExchangeOutcome::Failed(e.to_string()),
        };
        self.append(ExchangeRecord {
            kind: ExchangeKind::Get,
            id: Some(id.to_string()),
            references: references.to_vec(),
            status: status.map(|s| s.to_string()),
            outcome,
        });
        result
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        let result = self.inner.notify(id, references, status);
        let outcome = match &result {
            Ok(()) => ExchangeOutcome::NotifyOk,
            Err(e) => ExchangeOutcome::Failed(e.to_string()),
        };
        self.append(ExchangeRecord {
            kind: ExchangeKind::Notify,
            id: Some(id.to_string()),
            references: references.to_vec(),
            status: status.map(|s| s.to_string()),
            outcome,
        });
        result
    }

    // 新しい 2 メソッドも中身へそのまま渡し、同じ記録を残す（kanade は新しいメソッドだけを呼ぶ）。
    fn get_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<Option<ValueReply>, RequestError> {
        let result = self.inner.get_with_headers(id, references, status, headers);
        let outcome = match &result {
            Ok(Some(reply)) => ExchangeOutcome::Value(reply.value.clone()),
            Ok(None) => ExchangeOutcome::NoContent,
            Err(e) => ExchangeOutcome::Failed(e.to_string()),
        };
        self.append(ExchangeRecord {
            kind: ExchangeKind::Get,
            id: Some(id.to_string()),
            references: references.to_vec(),
            status: status.map(|s| s.to_string()),
            outcome,
        });
        result
    }

    fn notify_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<(), RequestError> {
        let result = self
            .inner
            .notify_with_headers(id, references, status, headers);
        let outcome = match &result {
            Ok(()) => ExchangeOutcome::NotifyOk,
            Err(e) => ExchangeOutcome::Failed(e.to_string()),
        };
        self.append(ExchangeRecord {
            kind: ExchangeKind::Notify,
            id: Some(id.to_string()),
            references: references.to_vec(),
            status: status.map(|s| s.to_string()),
            outcome,
        });
        result
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        let result = self.inner.unload();
        // `ExitKind`／`ShutdownError` の Debug 文字列化（例 "Ok(Clean)" / "Err(ExitTimeout)"）。
        let outcome = ExchangeOutcome::Unloaded(format!("{result:?}"));
        self.append(ExchangeRecord {
            kind: ExchangeKind::Unload,
            id: None,
            references: Vec::new(),
            status: None,
            outcome,
        });
        result
    }

    fn status(&mut self) -> HelperStatus {
        // 素通し: sticky 死活検査ノイズを記録に混ぜない（append-only を呼出順で保つ）。
        self.inner.status()
    }

    // 保守の機会と解く手も素通し（既定の本体のままだと host32 の窓の汲み出しと外からの解除が落ちる）。
    fn on_idle(&mut self) {
        self.inner.on_idle();
    }

    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        self.inner.unblock_handle()
    }
}

/// `Box<dyn ShioriBackend>` を [`Recorder`] に包むための委譲（`Box<dyn ShioriBackend>` は
/// `ShioriBackend` を実装せず、孤児則で包括の実装も足せないので newtype で橋渡しする）。
/// 口を全部中身へ渡す（既定の本体のままだと頭と応答のレベルが黙って落ちる）。
pub struct BoxedBackend(pub Box<dyn ShioriBackend>);

impl ShioriBackend for BoxedBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.0.get(id, references, status)
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.0.notify(id, references, status)
    }

    fn get_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<Option<ValueReply>, RequestError> {
        self.0.get_with_headers(id, references, status, headers)
    }

    fn notify_with_headers(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
        headers: &RequestHeaders<'_>,
    ) -> Result<(), RequestError> {
        self.0.notify_with_headers(id, references, status, headers)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.0.unload()
    }

    fn status(&mut self) -> HelperStatus {
        self.0.status()
    }

    fn on_idle(&mut self) {
        self.0.on_idle();
    }

    fn unblock_handle(&self) -> Option<ShioriUnblock> {
        self.0.unblock_handle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use areka_kanade::ShioriBackend;
    use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

    /// 応答を事前登録して呼出順に払い出す最小 fake backend（駆動対象の一例）。
    struct FakeBackend {
        get_responses: VecDeque<Result<Option<String>, RequestError>>,
        notify_responses: VecDeque<Result<(), RequestError>>,
        unload_response: Option<Result<ExitKind, ShutdownError>>,
        status_value: HelperStatus,
        idles: Arc<AtomicUsize>,
    }

    impl FakeBackend {
        fn new() -> Self {
            Self {
                get_responses: VecDeque::new(),
                notify_responses: VecDeque::new(),
                unload_response: None,
                status_value: HelperStatus::Running,
                idles: Arc::new(AtomicUsize::new(0)),
            }
        }
        fn with_get(mut self, response: Result<Option<String>, RequestError>) -> Self {
            self.get_responses.push_back(response);
            self
        }
        fn with_notify(mut self, response: Result<(), RequestError>) -> Self {
            self.notify_responses.push_back(response);
            self
        }
        fn with_unload(mut self, response: Result<ExitKind, ShutdownError>) -> Self {
            self.unload_response = Some(response);
            self
        }
    }

    impl ShioriBackend for FakeBackend {
        fn get(
            &mut self,
            _id: &str,
            _references: &[String],
            _status: Option<&str>,
        ) -> Result<Option<String>, RequestError> {
            self.get_responses
                .pop_front()
                .expect("FakeBackend::get: no scripted response left")
        }
        fn notify(
            &mut self,
            _id: &str,
            _references: &[String],
            _status: Option<&str>,
        ) -> Result<(), RequestError> {
            self.notify_responses
                .pop_front()
                .expect("FakeBackend::notify: no scripted response left")
        }
        fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
            self.unload_response
                .take()
                .expect("FakeBackend::unload: no scripted response configured")
        }
        fn status(&mut self) -> HelperStatus {
            self.status_value
        }
        fn on_idle(&mut self) {
            self.idles.fetch_add(1, Ordering::Relaxed);
        }
        fn unblock_handle(&self) -> Option<ShioriUnblock> {
            Some(std::sync::Arc::new(|| Ok(())))
        }
    }

    #[test]
    fn on_idle_and_unblock_handle_reach_the_inner_backend_through_both_wrappers() {
        let (mut recorder, handle) = Recorder::new(FakeBackend::new());
        recorder.on_idle();
        assert_eq!(
            recorder.inner.idles.load(Ordering::Relaxed),
            1,
            "Recorder は on_idle を中身へ渡す"
        );
        assert!(
            recorder.unblock_handle().is_some(),
            "Recorder は解く手を中身から取る"
        );
        assert!(
            handle.records().is_empty(),
            "保守の機会は交信として記録しない"
        );

        let fake = FakeBackend::new();
        let idles = Arc::clone(&fake.idles);
        let mut boxed = BoxedBackend(Box::new(fake));
        assert!(
            boxed.unblock_handle().is_some(),
            "BoxedBackend は解く手を中身から取る"
        );
        boxed.on_idle();
        assert_eq!(
            idles.load(Ordering::Relaxed),
            1,
            "BoxedBackend は on_idle を中身へ渡す"
        );
    }

    #[test]
    fn records_full_exchange_sequence_in_call_order() {
        let fake = FakeBackend::new()
            .with_get(Ok(None))
            .with_get(Ok(Some("greeting".to_string())))
            .with_notify(Ok(()))
            .with_unload(Ok(ExitKind::Clean));
        let (mut recorder, handle) = Recorder::new(fake);

        let r1 = recorder.get("OnFirstBoot", &[], None);
        assert!(
            matches!(r1, Ok(None)),
            "駆動対象の戻り値は素通し（改変しない）"
        );
        let r2 = recorder.get("OnBoot", &["master".to_string()], Some("talking"));
        assert!(matches!(r2, Ok(Some(ref v)) if v == "greeting"));
        let r3 = recorder.notify("OnInitialize", &[], None);
        assert!(r3.is_ok());
        let r4 = recorder.unload();
        assert_eq!(r4.expect("scripted unload Ok"), ExitKind::Clean);

        let expected = vec![
            ExchangeRecord {
                kind: ExchangeKind::Get,
                id: Some("OnFirstBoot".to_string()),
                references: vec![],
                status: None,
                outcome: ExchangeOutcome::NoContent,
            },
            ExchangeRecord {
                kind: ExchangeKind::Get,
                id: Some("OnBoot".to_string()),
                references: vec!["master".to_string()],
                status: Some("talking".to_string()),
                outcome: ExchangeOutcome::Value("greeting".to_string()),
            },
            ExchangeRecord {
                kind: ExchangeKind::Notify,
                id: Some("OnInitialize".to_string()),
                references: vec![],
                status: None,
                outcome: ExchangeOutcome::NotifyOk,
            },
            ExchangeRecord {
                kind: ExchangeKind::Unload,
                id: None,
                references: vec![],
                status: None,
                outcome: ExchangeOutcome::Unloaded("Ok(Clean)".to_string()),
            },
        ];
        assert_eq!(
            handle.records(),
            expected,
            "記録は呼出順の追記のみ・kind/id/references/status/outcome まで一致"
        );
    }

    #[test]
    fn get_err_is_recorded_as_failed_with_display() {
        let fake = FakeBackend::new().with_get(Err(RequestError::Timeout));
        let (mut recorder, handle) = Recorder::new(fake);

        let _ = recorder.get("OnBoot", &[], None);

        let records = handle.records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].outcome,
            ExchangeOutcome::Failed(RequestError::Timeout.to_string()),
            "Err は Display 文字列化した Failed として記録される"
        );
    }

    #[test]
    fn status_calls_are_not_recorded() {
        let fake = FakeBackend::new().with_get(Ok(None)).with_notify(Ok(()));
        let (mut recorder, handle) = Recorder::new(fake);

        // 交信の合間に status() を挟んでも記録には現れない（sticky 検査ノイズ除外）。
        let _ = recorder.status();
        let _ = recorder.get("OnFirstBoot", &[], None);
        let _ = recorder.status();
        let _ = recorder.notify("OnInitialize", &[], None);
        let _ = recorder.status();

        let records = handle.records();
        assert_eq!(records.len(), 2, "status() は記録されない（2 交信のみ）");
        assert_eq!(records[0].kind, ExchangeKind::Get);
        assert_eq!(records[1].kind, ExchangeKind::Notify);
    }

    #[test]
    fn records_preserve_strict_call_order_across_interleaving() {
        let fake = FakeBackend::new()
            .with_notify(Ok(()))
            .with_get(Ok(Some("a".to_string())))
            .with_notify(Ok(()))
            .with_get(Ok(None));
        let (mut recorder, handle) = Recorder::new(fake);

        let _ = recorder.notify("N1", &[], None);
        let _ = recorder.get("G1", &[], None);
        let _ = recorder.notify("N2", &[], None);
        let _ = recorder.get("G2", &[], None);

        let kinds_ids: Vec<(ExchangeKind, Option<String>)> = handle
            .records()
            .into_iter()
            .map(|r| (r.kind, r.id))
            .collect();
        assert_eq!(
            kinds_ids,
            vec![
                (ExchangeKind::Notify, Some("N1".to_string())),
                (ExchangeKind::Get, Some("G1".to_string())),
                (ExchangeKind::Notify, Some("N2".to_string())),
                (ExchangeKind::Get, Some("G2".to_string())),
            ],
            "記録は呼出順どおり（追記のみ・並べ替えなし）"
        );
    }

    #[test]
    fn handle_is_clone_and_shares_the_same_log() {
        let fake = FakeBackend::new().with_get(Ok(None));
        let (mut recorder, handle) = Recorder::new(fake);
        let handle2 = handle.clone();

        let _ = recorder.get("OnFirstBoot", &[], None);

        assert_eq!(handle.records().len(), 1);
        assert_eq!(
            handle2.records().len(),
            1,
            "clone した handle も同一の記録を読める"
        );
    }
}
