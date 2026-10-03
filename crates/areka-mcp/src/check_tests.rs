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

// ---- 10 本の定義に対する検査（要件 2.2〜2.5・2.8）。定義は `tools::TABLE` から読む（保存した JSON は読まない）。

/// 1 本の定義の (ツール名, inputSchema)。
fn table_schemas() -> Vec<(String, Map<String, Value>)> {
    crate::tools::TABLE
        .iter()
        .map(|(definition, _)| {
            let spec = crate::tools::spec_from_definition(definition).expect("定義が読める");
            (spec.name, spec.input_schema)
        })
        .collect()
}

/// 欄の型に合う値。
fn valid_value(prop: &Value) -> Value {
    match prop["type"].as_str() {
        Some("string") => json!("x"),
        Some("boolean") => json!(true),
        Some("integer") => json!(1),
        Some("array") => json!(["a"]),
        other => panic!("定義に想定外の type: {other:?}"),
    }
}

/// 欄の型に合わない値と、そのときの理由の文。
fn wrong_values(name: &str, prop: &Value) -> Vec<(Value, String)> {
    let must = |ty: &str| format!("argument {name} must be {ty}");
    match prop["type"].as_str() {
        Some("string") => vec![(json!(1), must("string")), (json!(["x"]), must("string"))],
        Some("boolean") => vec![
            (json!("true"), must("boolean")),
            (json!(1), must("boolean")),
        ],
        Some("integer") => vec![
            (json!(1.5), must("integer")),
            (json!("1"), must("integer")),
            (json!(u64::MAX), must("integer")),
            (json!(true), must("integer")),
        ],
        Some("array") => vec![
            (json!("a"), must("array")),
            (json!([1]), must("array of string")),
            (json!(["a", null]), must("array of string")),
        ],
        other => panic!("定義に想定外の type: {other:?}"),
    }
}

fn names(list: Option<&Value>) -> Vec<String> {
    list.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|v| v.as_str().expect("required は文字列").to_owned())
        .collect()
}

/// (要件 2.2〜2.5・2.8) 10 本それぞれの定義を読み、欄が全部ある・必須の欄が無い・`null` の必須の欄・
/// 欄ごとの型違い・`null` の任意の欄・余計な欄・`1.0` を検査に通す。理由の文は逐語で比べる。
#[test]
fn ten_definitions_table() {
    let schemas = table_schemas();
    assert_eq!(schemas.len(), 10);
    for (tool, schema) in &schemas {
        let empty = Map::new();
        let props = schema
            .get("properties")
            .and_then(Value::as_object)
            .unwrap_or(&empty);
        let required = names(schema.get("required"));
        let full: Map<String, Value> = props
            .iter()
            .map(|(k, p)| (k.clone(), valid_value(p)))
            .collect();
        let mut cases: Vec<(String, Map<String, Value>, Result<(), String>)> = Vec::new();
        let with = |k: &str, v: Value| {
            let mut a = full.clone();
            a.insert(k.to_owned(), v);
            a
        };

        cases.push(("全部ある".into(), full.clone(), Ok(())));
        cases.push((
            "余計な欄".into(),
            with("no_such_field", json!({"x": 1})),
            Ok(()),
        ));
        for name in required.iter().filter(|n| *n != "ghost_name") {
            let missing = Err(format!("missing required argument: {name}"));
            let mut absent = full.clone();
            absent.remove(name);
            cases.push((format!("{name} が無い"), absent, missing.clone()));
            cases.push((format!("{name} が null"), with(name, Value::Null), missing));
        }
        for (name, prop) in props {
            if !required.contains(name) || name == "ghost_name" {
                cases.push((format!("{name} が null"), with(name, Value::Null), Ok(())));
            }
            for (bad, reason) in wrong_values(name, prop) {
                cases.push((format!("{name} に {bad}"), with(name, bad), Err(reason)));
            }
            if prop["type"] == "integer" {
                cases.push((format!("{name} に 1.0"), with(name, json!(1.0)), Ok(())));
            }
        }

        // 空回りでないこと: 全部ある・余計な欄の 2 つに加え、欄ごとに 1 つ以上の型違いがある。
        assert!(cases.len() >= 2 + props.len(), "{tool}: {}", cases.len());
        for (label, args, want) in cases {
            let got = check_arguments(schema, &args);
            if let Err(reason) = &got {
                assert!(
                    !reason.starts_with("failed to deserialize parameters:"),
                    "{tool} {label}: {reason}"
                );
            }
            assert_eq!(got, want, "{tool} {label}");
        }
    }
}

/// (要件 2.2・2.4・2.8) 表のどの欄を調べたかの釘。必須の欄（`ghost_name` を除く）は要件 2.2 の 4 本だけ、
/// `array` の欄は `raise_event` の `references` だけ、欄の無いツールは `get_active_ghost_list` だけ。
#[test]
fn ten_definitions_cover_the_named_fields() {
    let mut required = Vec::new();
    let mut arrays = Vec::new();
    let mut no_props = Vec::new();
    for (tool, schema) in table_schemas() {
        for name in names(schema.get("required")) {
            if name != "ghost_name" {
                required.push(format!("{tool}.{name}"));
            }
        }
        let props = schema.get("properties").and_then(Value::as_object);
        if props.is_none_or(Map::is_empty) {
            no_props.push(tool.clone());
        }
        for (name, prop) in props.into_iter().flatten() {
            if prop["type"] == "array" {
                arrays.push(format!("{tool}.{name}"));
            }
        }
    }
    assert_eq!(
        required,
        [
            "get_property.property_name",
            "sakurascript.script",
            "raise_event.event",
            "reload.target"
        ]
    );
    assert_eq!(arrays, ["raise_event.references"]);
    assert_eq!(no_props, ["get_active_ghost_list"]);
}

/// (要件 2.3) `get_expression_table` の `ghost_name` は `required` に載っているが、無くても `null` でも通る。
#[test]
fn get_expression_table_without_ghost_name_passes() {
    let (_, schema) = table_schemas()
        .into_iter()
        .find(|(name, _)| name == "get_expression_table")
        .expect("表に在る");
    assert!(names(schema.get("required")).contains(&"ghost_name".to_owned()));
    assert_eq!(check_arguments(&schema, &Map::new()), Ok(()));
    assert_eq!(
        check_arguments(&schema, &obj(json!({"ghost_name": null}))),
        Ok(())
    );
}
