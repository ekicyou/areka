//! `tools` の決定論テスト（定義の読み込み・引数の詰め替え）。

use serde_json::{Map, Value, json};

use super::*;

/// 保存した JSON（テストだけが読む。本番はクレートの外のファイルを読まない＝要件 7.6）。
const SAVED_LIST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../doc/ssp-mcp/tools-list-ssp-2.9.05.json"
);

fn saved_tools() -> Vec<Value> {
    let text = std::fs::read_to_string(SAVED_LIST).expect("保存した JSON が読めない");
    let list: Value = serde_json::from_str(&text).expect("保存した JSON が JSON でない");
    list["tools"]
        .as_array()
        .expect("tools が配列でない")
        .clone()
}

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        other => panic!("オブジェクトでない: {other}"),
    }
}

fn parse_by_name(name: &str, args: Value) -> ToolCall {
    let (_, parse) = TABLE
        .iter()
        .find(|(def, _)| spec_from_definition(def).unwrap().name == name)
        .unwrap_or_else(|| panic!("表に {name} が無い"));
    parse(&obj(args))
}

/// (要件 1.1・1.3) 表が 10 行・先頭が `get_active_ghost_list`。
#[test]
fn table_has_ten_rows_in_ssp_order() {
    assert_eq!(TABLE.len(), 10);
    let first = spec_from_definition(TABLE[0].0).unwrap();
    assert_eq!(first.name, "get_active_ghost_list");
}

/// (要件 1.2・1.3) 10 本の定義が読めて `ToolSpec` になり、並びと中身が保存した JSON と一致する。
#[test]
fn definitions_are_verbatim_copies_of_saved_list() {
    let saved = saved_tools();
    assert_eq!(saved.len(), TABLE.len());
    for ((def, _), want) in TABLE.iter().zip(&saved) {
        let got: Value = serde_json::from_str(def).expect("DEFINITION が JSON でない");
        assert_eq!(&got, want, "DEFINITION が保存した JSON と違う");

        let spec = spec_from_definition(def).expect("ToolSpec にならない");
        assert_eq!(spec.name, want["name"].as_str().unwrap());
        assert_eq!(spec.title.as_deref(), want["title"].as_str());
        assert_eq!(spec.description.as_deref(), want["description"].as_str());
        assert_eq!(&Value::Object(spec.input_schema), &want["inputSchema"]);
    }
}

/// 名前・inputSchema の無い定義は `ToolSpec` にしない。
#[test]
fn spec_from_definition_rejects_broken_definition() {
    assert!(spec_from_definition("not json").is_err());
    assert!(spec_from_definition(r#"{"inputSchema":{}}"#).is_err());
    assert!(spec_from_definition(r#"{"name":"x"}"#).is_err());
    let spec = spec_from_definition(r#"{"name":"x","inputSchema":{"type":"object"}}"#).unwrap();
    assert_eq!(spec.title, None);
    assert_eq!(spec.description, None);
}

/// (要件 5.3) 10 本の詰め替え: 欄が全部ある・省略・`null`・`references` の省略・空の `ghost_name`。
#[test]
fn parse_fills_typed_args() {
    let g = |s: &str| Some(s.to_owned());
    let cases: Vec<(&str, Value, ToolCall)> = vec![
        (
            "get_active_ghost_list",
            json!({ "extra": 1 }),
            ToolCall::GetActiveGhostList,
        ),
        // get_status
        (
            "get_status",
            json!({ "ghost_name": "emo2" }),
            ToolCall::GetStatus(get_status::Args {
                ghost_name: g("emo2"),
            }),
        ),
        (
            "get_status",
            json!({}),
            ToolCall::GetStatus(get_status::Args { ghost_name: None }),
        ),
        (
            "get_status",
            json!({ "ghost_name": null }),
            ToolCall::GetStatus(get_status::Args { ghost_name: None }),
        ),
        (
            "get_status",
            json!({ "ghost_name": "" }),
            ToolCall::GetStatus(get_status::Args { ghost_name: g("") }),
        ),
        // get_expression_table
        (
            "get_expression_table",
            json!({ "ghost_name": "emo2" }),
            ToolCall::GetExpressionTable(get_expression_table::Args {
                ghost_name: g("emo2"),
            }),
        ),
        (
            "get_expression_table",
            json!({}),
            ToolCall::GetExpressionTable(get_expression_table::Args { ghost_name: None }),
        ),
        // get_property
        (
            "get_property",
            json!({ "property_name": "currentghost.name", "ghost_name": "emo2" }),
            ToolCall::GetProperty(get_property::Args {
                property_name: "currentghost.name".to_owned(),
                ghost_name: g("emo2"),
            }),
        ),
        (
            "get_property",
            json!({ "property_name": "p", "ghost_name": null }),
            ToolCall::GetProperty(get_property::Args {
                property_name: "p".to_owned(),
                ghost_name: None,
            }),
        ),
        // get_log
        (
            "get_log",
            json!({ "log_type": "script", "ghost_name": "", "since_id": 3, "max_count": 5.0 }),
            ToolCall::GetLog(get_log::Args {
                log_type: g("script"),
                ghost_name: g(""),
                since_id: Some(3),
                max_count: Some(5),
            }),
        ),
        (
            "get_log",
            json!({ "log_type": null, "since_id": null }),
            ToolCall::GetLog(get_log::Args {
                log_type: None,
                ghost_name: None,
                since_id: None,
                max_count: None,
            }),
        ),
        // sakurascript
        (
            "sakurascript",
            json!({ "script": "\\0\\s[0]hi\\e", "ghost_name": "emo2", "strict": true }),
            ToolCall::Sakurascript(sakurascript::Args {
                script: "\\0\\s[0]hi\\e".to_owned(),
                ghost_name: g("emo2"),
                strict: Some(true),
            }),
        ),
        (
            "sakurascript",
            json!({ "script": "x", "strict": null }),
            ToolCall::Sakurascript(sakurascript::Args {
                script: "x".to_owned(),
                ghost_name: None,
                strict: None,
            }),
        ),
        // raise_event
        (
            "raise_event",
            json!({ "event": "OnAiTalk", "references": ["a", "b"], "ghost_name": "emo2", "strict": false }),
            ToolCall::RaiseEvent(raise_event::Args {
                event: "OnAiTalk".to_owned(),
                references: vec!["a".to_owned(), "b".to_owned()],
                ghost_name: g("emo2"),
                strict: Some(false),
            }),
        ),
        (
            "raise_event",
            json!({ "event": "OnAiTalk" }),
            ToolCall::RaiseEvent(raise_event::Args {
                event: "OnAiTalk".to_owned(),
                references: vec![],
                ghost_name: None,
                strict: None,
            }),
        ),
        (
            "raise_event",
            json!({ "event": "OnAiTalk", "references": null }),
            ToolCall::RaiseEvent(raise_event::Args {
                event: "OnAiTalk".to_owned(),
                references: vec![],
                ghost_name: None,
                strict: None,
            }),
        ),
        // reload
        (
            "reload",
            json!({ "target": "shell", "ghost_name": "emo2" }),
            ToolCall::Reload(reload::Args {
                target: "shell".to_owned(),
                ghost_name: g("emo2"),
            }),
        ),
        (
            "reload",
            json!({ "target": "ghost" }),
            ToolCall::Reload(reload::Args {
                target: "ghost".to_owned(),
                ghost_name: None,
            }),
        ),
        // dump_surface
        (
            "dump_surface",
            json!({ "scope": 1, "surface": 10, "ghost_name": "emo2" }),
            ToolCall::DumpSurface(dump_surface::Args {
                scope: Some(1),
                surface: Some(10),
                ghost_name: g("emo2"),
            }),
        ),
        (
            "dump_surface",
            json!({ "surface": null }),
            ToolCall::DumpSurface(dump_surface::Args {
                scope: None,
                surface: None,
                ghost_name: None,
            }),
        ),
        // dump_balloon
        (
            "dump_balloon",
            json!({ "scope": 0, "ghost_name": "emo2" }),
            ToolCall::DumpBalloon(dump_balloon::Args {
                scope: Some(0),
                ghost_name: g("emo2"),
            }),
        ),
        (
            "dump_balloon",
            json!({}),
            ToolCall::DumpBalloon(dump_balloon::Args {
                scope: None,
                ghost_name: None,
            }),
        ),
    ];
    for (name, args, want) in cases {
        assert_eq!(parse_by_name(name, args.clone()), want, "{name} {args}");
    }
}

/// 10 本の詰め替えの結果の `ToolCall::name` が、表のその行の定義の名前と一致する。
#[test]
fn tool_call_name_matches_definition_for_all_ten() {
    let full = json!({
        "property_name": "p", "script": "s", "event": "e", "target": "t",
    });
    for (def, parse) in TABLE.iter() {
        let spec = spec_from_definition(def).unwrap();
        assert_eq!(parse(&obj(full.clone())).name(), spec.name);
    }
}

/// (要件 6.2 の上限の値) 本番の上限は 10 秒。
#[test]
fn reply_wait_is_ten_seconds() {
    assert_eq!(REPLY_WAIT, std::time::Duration::from_secs(10));
}

// ---- 登録表と受け口（`entrances`） ----

use std::time::Duration;

use log_capture_kit::capture;
use tracing::Level;

use crate::{ToolHandler, ToolOutcome, ToolRegistry};

/// 処理のフューチャをテストのスレッドの current_thread で回す。
fn run_handler(handler: &ToolHandler, args: Value) -> ToolOutcome {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("tokio")
        .block_on(handler(args))
}

fn names(registry: &ToolRegistry) -> Vec<String> {
    registry
        .entries()
        .iter()
        .map(|(spec, _)| spec.name.clone())
        .collect()
}

/// (要件 1.1・1.3) 組んだ登録表は 10 本を SSP の並びで持ち、定義は表の行と同じ。
#[test]
fn entrances_registers_ten_in_ssp_order() {
    let (registry, _rx) = entrances(REPLY_WAIT);
    assert_eq!(registry.len(), 10);
    let want: Vec<String> = TABLE
        .iter()
        .map(|(def, _)| spec_from_definition(def).unwrap().name)
        .collect();
    assert_eq!(names(&registry), want);
    assert_eq!(want[0], "get_active_ghost_list");
    for ((spec, _), (def, _)) in registry.entries().iter().zip(TABLE.iter()) {
        assert_eq!(spec, &spec_from_definition(def).unwrap());
    }
}

/// 10 本の処理を呼ぶと、受け口に型の付いた `ToolCall` が届き、受け手の返事が処理の結果になる。
#[test]
fn every_handler_delivers_typed_call_and_returns_the_answer() {
    let (registry, rx) = entrances(REPLY_WAIT);
    // 偽のアプリ本体: 届いた呼び出しを書き写して返す。
    let answerer = std::thread::spawn(move || {
        for request in rx {
            let text = format!("{:?}", request.call);
            request.reply.send(outcome::value(text));
        }
    });
    let full = json!({
        "property_name": "p", "script": "s", "event": "e", "target": "t",
        "ghost_name": "emo2", "references": ["a"], "scope": 1,
    });
    for ((spec, handler), (_, parse)) in registry.entries().iter().zip(TABLE.iter()) {
        let want = parse(&obj(full.clone()));
        assert_eq!(
            run_handler(handler, full.clone()),
            outcome::value(format!("{want:?}")),
            "{}",
            spec.name
        );
    }
    drop(registry);
    answerer.join().unwrap();
}

/// 引数が object でない（handler の検査より前には起きない）ときは空の arguments として詰め替える。
#[test]
fn non_object_arguments_are_read_as_empty() {
    let (registry, rx) = entrances(Duration::from_millis(1));
    let (_, handler) = &registry.entries()[1];
    let _ = run_handler(handler, Value::Null);
    let request = rx.try_recv().expect("要求が届かない");
    assert_eq!(
        request.call,
        ToolCall::GetStatus(get_status::Args { ghost_name: None })
    );
}

/// (要件 6.5) 誰も汲まない間に届いた要求は受け口に溜まり、後から汲める。
#[test]
fn requests_queue_until_someone_drains() {
    let (registry, rx) = entrances(Duration::from_millis(1));
    let (_, handler) = &registry.entries()[0];
    let got = run_handler(handler, json!({}));
    assert!(got.is_error, "誰も答えないので上限で返る: {got:?}");
    let request = rx.try_recv().expect("要求が溜まっていない");
    assert_eq!(request.call, ToolCall::GetActiveGhostList);
}

/// 定義が読めない行は `error!` 1 件で登録しない（残りの行は登録する）。
#[test]
fn broken_definition_row_is_skipped_with_one_error() {
    let rows: [(&str, Parse); 2] = [("not json", get_status::parse), TABLE[0]];
    let ((registry, _rx), events) = capture(|| register_rows(&rows, REPLY_WAIT));
    assert_eq!(names(&registry), vec!["get_active_ghost_list".to_owned()]);
    assert_eq!(events.len(), 1, "記録は error 1 件だけ: {events:?}");
    assert_eq!(events[0].level, Level::ERROR);
}
