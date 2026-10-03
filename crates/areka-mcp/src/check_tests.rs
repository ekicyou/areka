//! `check` の決定論テスト（10 本の定義を使う）。

use serde_json::{Map, Value, json};

use super::{as_integer, check_arguments};

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        other => panic!("オブジェクトでない: {other}"),
    }
}

/// 手書きの schema。必須は `name`（string）と `ghost_name`（string）。他の欄は任意。
fn schema() -> Map<String, Value> {
    obj(json!({
        "type": "object",
        "properties": {
            "ghost_name": { "type": "string" },
            "name": { "type": "string" },
            "flag": { "type": "boolean" },
            "count": { "type": "integer" },
            "refs": { "type": "array", "items": { "type": "string" } },
            "loose": { "type": "array" },
            "other": { "type": "number" }
        },
        "required": ["name", "ghost_name"]
    }))
}

fn check(args: Value) -> Result<(), String> {
    check_arguments(&schema(), &obj(args))
}

/// (要件 2.2〜2.5) 規則の表。`Ok` か、`Err` の理由の文そのもの。
#[test]
fn rules_table() {
    let missing_name = Err("missing required argument: name".to_owned());
    let cases: Vec<(&str, Value, Result<(), String>)> = vec![
        // 欄が全部ある
        (
            "全部ある",
            json!({"ghost_name": "a", "name": "b", "flag": true, "count": -3, "refs": ["x", "y"], "loose": [1, "z"]}),
            Ok(()),
        ),
        // 必須（2.2）
        ("必須の欠落", json!({}), missing_name.clone()),
        ("null の必須", json!({"name": null}), missing_name.clone()),
        // ghost_name の欠落・null は通る（2.3）
        ("ghost_name の欠落", json!({"name": "b"}), Ok(())),
        (
            "ghost_name が null",
            json!({"name": "b", "ghost_name": null}),
            Ok(()),
        ),
        // 余計な欄は見ない（2.5）
        ("余計な欄", json!({"name": "b", "extra": {"x": 1}}), Ok(())),
        // null の任意の欄は省略と同じ（2.4）
        (
            "null の任意の欄",
            json!({"name": "b", "flag": null, "count": null, "refs": null}),
            Ok(()),
        ),
        // 型（2.4）
        (
            "string に数",
            json!({"name": 1}),
            Err("argument name must be string".to_owned()),
        ),
        (
            "boolean に文字列",
            json!({"name": "b", "flag": "true"}),
            Err("argument flag must be boolean".to_owned()),
        ),
        (
            "integer に 1.5",
            json!({"name": "b", "count": 1.5}),
            Err("argument count must be integer".to_owned()),
        ),
        (
            "integer に文字列",
            json!({"name": "b", "count": "1"}),
            Err("argument count must be integer".to_owned()),
        ),
        (
            "integer に i64 を超える数",
            json!({"name": "b", "count": u64::MAX}),
            Err("argument count must be integer".to_owned()),
        ),
        (
            "integer に 1.0 は通る",
            json!({"name": "b", "count": 1.0}),
            Ok(()),
        ),
        (
            "array に文字列",
            json!({"name": "b", "refs": "x"}),
            Err("argument refs must be array".to_owned()),
        ),
        (
            "array の要素違反",
            json!({"name": "b", "refs": ["x", 2]}),
            Err("argument refs must be array of string".to_owned()),
        ),
        (
            "array の要素が null",
            json!({"name": "b", "refs": [null]}),
            Err("argument refs must be array of string".to_owned()),
        ),
        (
            "items の無い array は要素を見ない",
            json!({"name": "b", "loose": [1, null]}),
            Ok(()),
        ),
        // 未知の type は見ない
        ("未知の type", json!({"name": "b", "other": "x"}), Ok(())),
        // 必須 → 型 の順（2.7）
        (
            "必須が型より先",
            json!({"count": "x"}),
            missing_name.clone(),
        ),
    ];
    for (label, args, want) in cases {
        assert_eq!(check(args), want, "{label}");
    }
}

/// (要件 2.7) 理由の文は `failed to deserialize parameters:` で始めない（始めると rmcp が `isError` の結果へ変える）。
#[test]
fn reasons_do_not_use_the_deserialize_prefix() {
    for args in [
        json!({}),
        json!({"name": 1}),
        json!({"name": "b", "count": 1.5}),
        json!({"name": "b", "refs": [1]}),
    ] {
        let reason = check(args.clone()).expect_err(&args.to_string());
        assert!(
            !reason.starts_with("failed to deserialize parameters:"),
            "{reason}"
        );
        assert!(
            reason.starts_with("missing required argument: ") || reason.starts_with("argument "),
            "{reason}"
        );
    }
}

/// (要件 2.2) 必須の欄は `required` の並び順で最初の欠落を返す。
#[test]
fn first_missing_follows_required_order() {
    let s = obj(json!({
        "properties": {"a": {"type": "string"}, "b": {"type": "string"}},
        "required": ["b", "a"]
    }));
    assert_eq!(
        check_arguments(&s, &Map::new()),
        Err("missing required argument: b".to_owned())
    );
}

/// `required`・`properties` の無い schema は何でも通す（`properties` に無い欄は見ない）。
#[test]
fn empty_schema_accepts_anything() {
    assert_eq!(
        check_arguments(&Map::new(), &obj(json!({"x": 1, "y": null}))),
        Ok(())
    );
}

/// (要件 2.4) `as_integer` の読み方。
#[test]
fn as_integer_table() {
    let two53 = 9_007_199_254_740_992_i64; // 2^53
    let cases: Vec<(Value, Option<i64>)> = vec![
        (json!(0), Some(0)),
        (json!(-7), Some(-7)),
        (json!(i64::MAX), Some(i64::MAX)),
        (json!(i64::MIN), Some(i64::MIN)),
        (json!(1.0), Some(1)),
        (json!(-2.0), Some(-2)),
        (json!(-0.0), Some(0)),
        (json!(two53 as f64), Some(two53)),
        (json!(-(two53 as f64)), Some(-two53)),
        (json!(2.0 * two53 as f64), None),
        (json!(1.5), None),
        (json!(u64::MAX), None),
        (json!(i64::MAX as u64 + 1), None),
        (json!(1e300), None),
        (json!("1"), None),
        (json!(true), None),
        (Value::Null, None),
        (json!([1]), None),
    ];
    for (v, want) in cases {
        assert_eq!(as_integer(&v), want, "{v}");
    }
}
