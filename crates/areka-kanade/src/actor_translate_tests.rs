//! 殻が翻訳の依頼を実行する関数（[`run_translate`]）の檻（要件 2.1・2.2・4.1・4.4・4.5・7.3・8.1）。
//!
//! 依頼を直接渡し、1 件の Request を受けて控える偽の SHIORI と、呼ばれた引数を控える MAKOTO の口で
//! 確かめる: 展開の後の台詞が `OnTranslate` の Reference0 に届くこと・Status は依頼に載った値で
//! 届くこと・応答の行列ごとの返り値・MAKOTO の口が応答の種類に依らず（台詞, 元のイベントの ID）で
//! 1 回呼ばれること・欠番の印と同じ値の Reference が空文字に置き換わって警告が 1 件残ること。
//!
//! 往復は呼出スレッドで同期に走るので、記録はスレッドローカルの捕捉窓で拾える。

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use tracing::Level;

use super::*;
use crate::actor::round_trip_request;
use crate::msg::{EventId, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome};
use crate::schedule::events::SourceEvent;
use crate::schedule::log_capture::{CapturedEvent, capture};
use crate::schedule::translate::{TranslateRequest, TranslateResult};
use crate::shiori::real::ABSENT_REFERENCE;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use crate::translate::TranslateSeams;

/// 元のイベントの ID（MAKOTO の口の第 2 引数・`OnTranslate` の Reference2）。
const SOURCE_ID: &str = "OnMouseDoubleClick";

/// 1 件の Request を受けて控え、`reply` を返す偽の SHIORI。join すると受けた呼出が返る。
fn fake_shiori(reply: ShioriOutcome) -> (Sender<ShioriMsg>, JoinHandle<Option<ShioriCall>>) {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    let helper = thread::spawn(move || match shiori_rx.recv() {
        Ok(ShioriMsg::Request { call, reply: tx }) => {
            let _ = tx.send(reply);
            Some(call)
        }
        _ => None,
    });
    (shiori_tx, helper)
}

/// 再生中（`talking`）の Status。依頼に載った値がそのまま届くことを見るため、既定と違う値にする。
fn talking_status() -> ExecutionStatus {
    ExecutionStatus::derive(&ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    })
}

/// MAKOTO の口が受けた（台詞, 元のイベントの ID）の控え。
type MakotoCalls = Arc<Mutex<Vec<(String, String)>>>;

/// 展開は「`%username` → 太郎」、MAKOTO の口は引数を控えて印 `#makoto` を足して返す。
fn spy_seams() -> (TranslateSeams, MakotoCalls) {
    let calls: MakotoCalls = Arc::default();
    let sink = Arc::clone(&calls);
    let seams = TranslateSeams {
        expand: Box::new(|script| script.replace("%username", "太郎")),
        makoto: Box::new(move |script, source_id| {
            sink.lock()
                .unwrap()
                .push((script.to_owned(), source_id.to_owned()));
            format!("{script}#makoto")
        }),
    };
    (seams, calls)
}

/// 1 回の実行の観測（返り値・偽の SHIORI が受けた呼出・MAKOTO の口の控え・記録）。
struct Observed {
    result: TranslateResult,
    received: Option<ShioriCall>,
    makoto: Vec<(String, String)>,
    events: Vec<CapturedEvent>,
}

/// 台詞 `%usernameさん`・元のイベント `OnMouseDoubleClick`（`source_refs`）の依頼を、`reply` を返す
/// 偽の SHIORI へ向けて実行する。
fn run(reply: ShioriOutcome, source_refs: &[&str]) -> Observed {
    let (shiori_tx, helper) = fake_shiori(reply);
    let (seams, calls) = spy_seams();
    let request = TranslateRequest {
        script: "%usernameさん".to_string(),
        source: SourceEvent {
            id: EventId::Static(SOURCE_ID),
            references: source_refs.iter().map(|s| s.to_string()).collect(),
        },
        status: talking_status(),
    };
    let mut result = None;
    let events = capture(|| {
        result = Some(run_translate(request, &shiori_tx, &seams));
    });
    let makoto = calls.lock().unwrap().clone();
    // 送信端を落としてから待つ（要求が届かなかったとき偽の SHIORI が受信で止まらないように）。
    drop(shiori_tx);
    Observed {
        result: result.expect("run_translate returns"),
        received: helper.join().expect("fake shiori thread"),
        makoto,
        events,
    }
}

/// 捕えた記録のうち、レベル・`event` が一致するもの。
fn records<'a>(events: &'a [CapturedEvent], level: Level, event: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == "kanade" && e.level == level && e.event.as_deref() == Some(event))
        .collect()
}

/// 偽の SHIORI が受けた呼出の ID・Reference・Status（GET でなければ panic）。
fn received_get(received: Option<ShioriCall>) -> (String, Vec<String>, ExecutionStatus) {
    match received {
        Some(ShioriCall::Get {
            id,
            references,
            status,
        }) => (id.as_str().to_string(), references, status),
        Some(ShioriCall::Notify { .. }) => panic!("OnTranslate は GET で送るはず"),
        None => panic!("偽の SHIORI に要求が届いていない"),
    }
}

#[test]
fn on_translate_receives_expanded_script_and_request_status() {
    let observed = run(ShioriOutcome::NoContent, &["10", "20"]);
    let (id, references, status) = received_get(observed.received);
    assert_eq!(id, "OnTranslate");
    assert_eq!(
        references,
        vec![
            "太郎さん".to_string(),
            ABSENT_REFERENCE.to_string(),
            SOURCE_ID.to_string(),
            "10\u{1}20".to_string(),
        ],
        "Reference0 は展開の後の台詞・添字 1 は欠番の印のまま・2 と 3 は元のイベント"
    );
    assert_eq!(status, talking_status(), "Status は依頼に載った値を使う");
    assert!(
        records(
            &observed.events,
            Level::WARN,
            "reference_absent_marker_replaced"
        )
        .is_empty(),
        "OnTranslate の添字 1 の印は置き換えない"
    );
}

/// 応答の行列の 1 行（応答, MAKOTO の口が受ける台詞, translate_reply の kind, そのレベル）。
type ReplyCase = (fn() -> ShioriOutcome, &'static str, &'static str, Level);

#[test]
fn every_reply_kind_passes_through_makoto_once_with_source_id() {
    let cases: [ReplyCase; 4] = [
        (
            || ShioriOutcome::Value("太郎くん".into()),
            "太郎くん",
            "replaced",
            Level::INFO,
        ),
        (
            || ShioriOutcome::Value(String::new()),
            "",
            "empty",
            Level::INFO,
        ),
        (
            || ShioriOutcome::NoContent,
            "太郎さん",
            "no_content",
            Level::INFO,
        ),
        (
            || ShioriOutcome::Failed(ShioriFailure::Shiori("500 Internal Server Error".into())),
            "太郎さん",
            "error_response",
            Level::WARN,
        ),
    ];
    for (reply, text, kind, level) in cases {
        let observed = run(reply(), &[]);
        match &observed.result {
            Ok(script) => assert_eq!(script, &format!("{text}#makoto"), "{kind}: 口の結果を返す"),
            Err(failure) => panic!("{kind}: 輸送路の失敗でないのに Err（{failure}）"),
        }
        assert_eq!(
            observed.makoto,
            vec![(text.to_string(), SOURCE_ID.to_string())],
            "{kind}: MAKOTO の口は（台詞, 元のイベントの ID）で 1 回だけ呼ばれる"
        );
        let replies = records(&observed.events, level, "translate_reply");
        assert_eq!(replies.len(), 1, "{kind}: 応答の記録は 1 件");
        assert_eq!(
            replies[0].fields.get("kind").map(String::as_str),
            Some(kind)
        );
    }
}

#[test]
fn error_response_warns_once_through_reply_reading_only() {
    let observed = run(
        ShioriOutcome::Failed(ShioriFailure::Shiori("400 Bad Request".into())),
        &[],
    );
    let warns: Vec<_> = observed
        .events
        .iter()
        .filter(|e| e.target == "kanade" && e.level == Level::WARN)
        .collect();
    assert_eq!(
        warns.len(),
        1,
        "エラー応答の警告は応答の読みの 1 件だけ（shiori_error_response を重ねない）: {warns:?}"
    );
    assert_eq!(warns[0].event.as_deref(), Some("translate_reply"));
}

#[test]
fn transport_failure_returns_err_without_calling_makoto() {
    let observed = run(
        ShioriOutcome::Failed(ShioriFailure::Ipc("helper gone".into())),
        &[],
    );
    assert!(
        matches!(observed.result, Err(ShioriFailure::Ipc(_))),
        "輸送路の失敗はそのまま返す（故障の判断は運行表）"
    );
    assert!(observed.makoto.is_empty(), "輸送路の失敗では口を呼ばない");
}

#[test]
fn absent_marker_in_on_translate_reference3_is_replaced_with_one_warning() {
    // 元の Reference が印 1 個だけのとき、Reference3 は印と同じ値になる。
    let observed = run(ShioriOutcome::NoContent, &[ABSENT_REFERENCE]);
    let (_, references, _) = received_get(observed.received);
    assert_eq!(references[1], ABSENT_REFERENCE, "添字 1 は欠番の印のまま");
    assert_eq!(references[3], "", "添字 3 の印は空文字に置き換えて送る");
    let replaced = records(
        &observed.events,
        Level::WARN,
        "reference_absent_marker_replaced",
    );
    assert_eq!(replaced.len(), 1, "置き換えた位置ごとに 1 件");
    assert_eq!(
        replaced[0].fields.get("id").map(String::as_str),
        Some("OnTranslate")
    );
    assert_eq!(
        replaced[0].fields.get("index").map(String::as_str),
        Some("3")
    );
}

#[test]
fn absent_marker_from_outside_is_replaced_on_existing_send_path() {
    // 汎用の通知の入口などから入る Reference も、送る 1 か所で同じに置き換わる。
    let (shiori_tx, helper) = fake_shiori(ShioriOutcome::NoContent);
    let call = ShioriCall::Get {
        id: EventId::Static("OnBoot"),
        references: vec![ABSENT_REFERENCE.to_string(), "master".to_string()],
        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
    };
    let events = capture(|| {
        let _ = round_trip_request(&shiori_tx, call);
    });
    drop(shiori_tx);
    let (_, references, _) = received_get(helper.join().expect("fake shiori thread"));
    assert_eq!(references, vec![String::new(), "master".to_string()]);
    let replaced = records(&events, Level::WARN, "reference_absent_marker_replaced");
    assert_eq!(replaced.len(), 1);
    assert_eq!(
        replaced[0].fields.get("id").map(String::as_str),
        Some("OnBoot")
    );
    assert_eq!(
        replaced[0].fields.get("index").map(String::as_str),
        Some("0")
    );
}
