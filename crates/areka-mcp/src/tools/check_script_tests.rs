//! `check_script` の入口の決定論テスト（定義・結果の形・種類の名前）。

use serde_json::Value;

use super::*;
use crate::ToolContent;
use crate::tools::spec_from_definition;

fn diag(kind: Kind, start: usize, end: usize, text: &str) -> Diagnostic {
    Diagnostic {
        kind,
        start,
        end,
        text: text.to_string(),
        message: "areka does not know this tag; playback drops it",
    }
}

/// 本文（テキスト 1 つ）を取り出す。
fn body(outcome: &ToolOutcome) -> &str {
    match outcome.content.as_slice() {
        [ToolContent::Text(text)] => text,
        other => panic!("本文はテキスト 1 つのはず: {other:?}"),
    }
}

#[test]
fn definition_has_name_title_description_and_required_script() {
    let spec = spec_from_definition(DEFINITION).expect("定義が読める");
    assert_eq!(spec.name, "check_script");
    assert_eq!(
        spec.title.as_deref(),
        Some("Check SakuraScript Without Playing")
    );
    assert!(spec.description.is_some_and(|d| d.contains("sakurascript")));
    assert_eq!(spec.input_schema["required"], serde_json::json!(["script"]));
    assert_eq!(
        spec.input_schema["properties"]["ghost_name"]["type"],
        "string"
    );
}

#[test]
fn summary_ja_has_no_html_special_characters() {
    assert!(!SUMMARY_JA.contains(['<', '>', '&']));
}

#[test]
fn zero_diagnostics_is_one_line_and_not_error() {
    let outcome = render(&[], None);
    assert!(!outcome.is_error);
    assert_eq!(body(&outcome), "OK:0 diagnostics");
}

#[test]
fn n_diagnostics_give_count_then_one_json_line_each() {
    let diags = [
        diag(Kind::UnknownTag, 0, 2, r"\x"),
        diag(Kind::MissingSurface, 2, 11, r"\s[99999]"),
        diag(Kind::Ignored, 11, 20, r"\f[sub,1]"),
    ];
    let outcome = render(&diags, None);
    assert!(!outcome.is_error);
    let lines: Vec<&str> = body(&outcome).lines().collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "OK:3 diagnostics");
    for (line, d) in lines[1..].iter().zip(&diags) {
        let v: Value = serde_json::from_str(line).expect("1 行が JSON 1 つ");
        assert_eq!(v.as_object().map(|o| o.len()), Some(5), "欄は 5 つだけ");
        assert_eq!(v["kind"], d.kind.as_str());
        assert_eq!(v["start"], d.start);
        assert_eq!(v["end"], d.end);
        assert_eq!(v["text"], d.text);
        assert_eq!(v["message"], d.message);
    }
}

#[test]
fn text_with_newline_quote_and_japanese_stays_on_one_line() {
    let text = "\\s[0\n「\"こんにちは\"」\r\n";
    let outcome = render(&[diag(Kind::UnreadableArgument, 3, 20, text)], None);
    let lines: Vec<&str> = body(&outcome).split('\n').collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "OK:1 diagnostics");
    let v: Value = serde_json::from_str(lines[1]).expect("1 行が JSON 1 つ");
    assert_eq!(v["text"], text);
}

#[test]
fn unchecked_note_is_appended_to_first_line_in_parentheses() {
    let outcome = render(&[], Some(NO_WINDOW_NOTE));
    assert!(!outcome.is_error);
    assert_eq!(
        body(&outcome),
        "OK:0 diagnostics (surface and balloon IDs were not checked: this ghost has no window)"
    );

    let outcome = render(&[diag(Kind::UnknownTag, 0, 2, r"\x")], Some(NO_WINDOW_NOTE));
    assert!(!outcome.is_error);
    let lines: Vec<&str> = body(&outcome).lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], format!("OK:1 diagnostics ({NO_WINDOW_NOTE})"));
}

#[test]
fn kind_names_are_spelled_as_documented() {
    let names = [
        (Kind::UnknownTag, "unknown_tag"),
        (Kind::UnknownCommand, "unknown_command"),
        (Kind::MissingSurface, "missing_surface"),
        (Kind::MissingBalloon, "missing_balloon"),
        (Kind::UnreadableArgument, "unreadable_argument"),
        (Kind::Ignored, "ignored"),
        (Kind::UnpairedTag, "unpaired_tag"),
    ];
    for (kind, name) in names {
        assert_eq!(kind.as_str(), name);
    }
}

/// (要件 1.7・2.1) 詰め替え: `script`・`ghost_name` を写す。`ghost_name` の省略と `null` は無し。
#[test]
fn parse_copies_script_and_ghost_name() {
    let call = |v: Value| match v {
        Value::Object(map) => parse(&map),
        other => panic!("オブジェクトでない: {other}"),
    };
    let want = |ghost: Option<&str>| {
        ToolCall::CheckScript(Args {
            script: r"\h\s[0]こんにちは\e".to_owned(),
            ghost_name: ghost.map(str::to_owned),
        })
    };
    assert_eq!(
        call(serde_json::json!({ "script": r"\h\s[0]こんにちは\e", "ghost_name": "emo2" })),
        want(Some("emo2"))
    );
    assert_eq!(
        call(serde_json::json!({ "script": r"\h\s[0]こんにちは\e" })),
        want(None)
    );
    assert_eq!(
        call(serde_json::json!({ "script": r"\h\s[0]こんにちは\e", "ghost_name": null })),
        want(None)
    );
}
