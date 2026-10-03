//! `registry` の決定論テスト（同じ名前の 2 度目の登録は後勝ち・定義が逐語で取り出せる）。

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use log_capture_kit::capture;
use serde_json::json;
use tracing::Level;

use super::{ToolContent, ToolHandler, ToolOutcome, ToolRegistry, ToolSpec};

/// 即値を返す実装（`Box::pin(async { … })`）を 1 度だけ poll して結果を取る。
/// 実行器は要らない（待つ所が無いので最初の poll で Ready になる）。
fn run_now(fut: impl Future<Output = ToolOutcome>) -> ToolOutcome {
    let mut cx = Context::from_waker(Waker::noop());
    match pin!(fut).poll(&mut cx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("即値の実装が Pending を返した"),
    }
}

/// 決まった文字列 1 つを返す実装。
fn text_handler(text: &'static str) -> ToolHandler {
    Arc::new(move |_args| {
        Box::pin(async move {
            ToolOutcome {
                content: vec![ToolContent::Text(text.to_owned())],
                is_error: false,
            }
        })
    })
}

fn spec(name: &str, description: &str) -> ToolSpec {
    ToolSpec {
        name: name.to_owned(),
        title: None,
        description: Some(description.to_owned()),
        input_schema: json!({ "type": "object" }).as_object().unwrap().clone(),
    }
}

/// (要件 7.1) 同じ名前を 2 度登録すると後勝ち（定義も実装も 2 度目）・件数は 1・`warn!` 1 件。
#[test]
fn same_name_registered_twice_last_wins_with_one_warn() {
    let mut registry = ToolRegistry::default();
    assert!(registry.is_empty());

    let ((), first_events) =
        capture(|| registry.register(spec("echo", "1 度目"), text_handler("1")));
    assert!(
        first_events.is_empty(),
        "初回は記録しない: {first_events:?}"
    );

    let ((), events) = capture(|| registry.register(spec("echo", "2 度目"), text_handler("2")));
    assert_eq!(registry.len(), 1);
    assert!(!registry.is_empty());
    assert_eq!(events.len(), 1, "記録は warn 1 件だけ: {events:?}");
    assert_eq!(events[0].level, Level::WARN);
    assert_eq!(events[0].field("name"), Some("echo"), "{events:?}");

    let (got_spec, got_handler) = &registry.entries()[0];
    assert_eq!(got_spec, &spec("echo", "2 度目"));
    assert_eq!(
        run_now(got_handler(json!({}))),
        ToolOutcome {
            content: vec![ToolContent::Text("2".to_owned())],
            is_error: false,
        }
    );
}

/// 別の名前は並べて持つ（後勝ちは同じ名前のときだけ）。
#[test]
fn different_names_are_kept_side_by_side() {
    let mut registry = ToolRegistry::default();
    let ((), events) = capture(|| {
        registry.register(spec("a", "a"), text_handler("a"));
        registry.register(spec("b", "b"), text_handler("b"));
    });
    assert_eq!(registry.len(), 2);
    assert!(events.is_empty(), "{events:?}");
}

/// (要件 7.4) name・title・description・inputSchema を逐語で詰め、そのまま取り出せる。
#[test]
fn registered_spec_comes_back_verbatim() {
    let schema = json!({
        "type": "object",
        "properties": {
            "script": { "type": "string", "description": "さくらスクリプト（\\h\\s[0]）" },
            "wait": { "type": "integer", "minimum": 0 }
        },
        "required": ["script"],
        "additionalProperties": false
    });
    let want = ToolSpec {
        name: "sakurascript".to_owned(),
        title: Some("Sakura Script".to_owned()),
        description: Some("  前後の空白も\n改行も そのまま  ".to_owned()),
        input_schema: schema.as_object().unwrap().clone(),
    };

    let mut registry = ToolRegistry::default();
    registry.register(want.clone(), text_handler("ok"));

    let entries = registry.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, want);
    assert_eq!(
        serde_json::Value::Object(entries[0].0.input_schema.clone()),
        schema
    );
}

/// 結果は text と image（base64 の文字列と MIME 型）と is_error を持つ。
#[test]
fn outcome_carries_text_image_and_is_error() {
    let handler: ToolHandler = Arc::new(|args| {
        Box::pin(async move {
            ToolOutcome {
                content: vec![
                    ToolContent::Text(args.to_string()),
                    ToolContent::Image {
                        data: "iVBORw0KGgo=".to_owned(),
                        mime_type: "image/png".to_owned(),
                    },
                ],
                is_error: true,
            }
        })
    });
    assert_eq!(
        run_now(handler(json!({ "x": 1 }))),
        ToolOutcome {
            content: vec![
                ToolContent::Text(r#"{"x":1}"#.to_owned()),
                ToolContent::Image {
                    data: "iVBORw0KGgo=".to_owned(),
                    mime_type: "image/png".to_owned(),
                },
            ],
            is_error: true,
        }
    );
}
