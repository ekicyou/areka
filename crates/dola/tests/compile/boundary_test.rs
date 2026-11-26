//! デシリアライズ境界・数値境界・panic 経路のテスト
//!
//! 外部入力（JSON・機能 `toml` の TOML）由来の DolaDocument が compile へ渡される境界で、
//! 規則を外れた文書が panic でなくエラーで拒まれることと、
//! 検証を通った文書が panic しないことを縛る。

use dola::*;

use super::common::{delta_ms, float_var, make_doc_with_storyboard, ms, transition_entry};

/// 文書の値の上限の中で最も大きい値（100ns の数で 2^51 − 1）
const MAX_DOC_TICKS: i64 = (1 << 51) - 1;

/// compile が数の規則・時刻の範囲で拒み、違反の欄を返す
fn rejected_fields(doc: &DolaDocument, sb_name: &str) -> Vec<String> {
    compile_storyboard(doc, sb_name)
        .unwrap_err()
        .into_iter()
        .map(|e| match e {
            DolaError::InvalidNumber { field, .. }
            | DolaError::TimeOutOfDocumentRange { field, .. } => field,
            DolaError::LoopDurationOutOfRange { .. } => "loop_duration".to_string(),
            other => panic!("unexpected error: {other:?}"),
        })
        .collect()
}

#[test]
fn overflowing_sums_of_finite_numbers_are_rejected() {
    // validate は数を 1 つずつしか見ないので、有限どうしの和が inf になった値が
    // そのまま通り、inf の値を配っていた
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(1e308)),
                    relative_to: Some(1e308),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                ..Default::default()
            })
            .build(),
    );
    assert_eq!(rejected_fields(&doc, "test"), ["relative_to"]);

    // 上限の中の長さも小さい time_scale では 1 周の経った長さが型の範囲を越え、インスタンスが終わらない
    let doc = make_doc_with_storyboard(
        vec![("x", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            .time_scale(1e-10)
            .entry(transition_entry(
                "x",
                ms(0),
                Some(TimeSpan::try_from_ticks(MAX_DOC_TICKS).unwrap()),
            ))
            .build(),
    );
    assert_eq!(rejected_fields(&doc, "test"), ["loop_duration"]);
}

#[test]
fn time_sums_beyond_the_type_range_are_rejected() {
    // 上限の中のずらしも積み重ねると型の範囲を越える。4096 個の和 2^63 − 4096 までは収まり、
    // そこへもう 1 つ足す純粋なキーフレーム・トリガー・遷移の終わりが、どれも拒まれる
    let at_plus_max = |name: &str| {
        Some(KeyframeRef::WithOffset {
            keyframes: KeyframeNames::Single(name.to_string()),
            offset: TimeDelta::from_ticks(MAX_DOC_TICKS),
        })
    };
    let mut builder = StoryboardBuilder::new();
    for i in 0..4096 {
        let base = if i == 0 {
            "start".to_string()
        } else {
            format!("k{}", i - 1)
        };
        builder = builder.entry(StoryboardEntry {
            keyframe: Some(format!("k{i}")),
            at: at_plus_max(&base),
            ..Default::default()
        });
    }
    let parent = builder
        .entry(StoryboardEntry {
            keyframe: Some("beyond".to_string()),
            at: at_plus_max("k4095"),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            trigger_storyboard: Some("child".to_string()),
            at: at_plus_max("k4095"),
            ..Default::default()
        })
        .entry(StoryboardEntry {
            at: Some(KeyframeRef::Single("k4095".to_string())),
            ..transition_entry(
                "x",
                ms(0),
                Some(TimeSpan::try_from_ticks(MAX_DOC_TICKS).unwrap()),
            )
        })
        .build();
    let mut doc = make_doc_with_storyboard(vec![("x", float_var())], vec![], "parent", parent);
    doc.storyboard.insert(
        "child".to_string(),
        StoryboardBuilder::new()
            .entry(StoryboardEntry {
                keyframe: Some("c".to_string()),
                at: Some(KeyframeRef::Single("start".to_string())),
                ..Default::default()
            })
            .build(),
    );
    let mut fields = rejected_fields(&doc, "parent");
    fields.sort();
    assert_eq!(fields, ["end_time", "fire_time", "keyframe_time"]);
}

#[test]
fn segment_sort_is_ascending_by_start_time() {
    // delay は 48…2 秒の降順に並べてあるので、並べ替えを外すと昇順の確かめが落ちる
    let mut builder = StoryboardBuilder::new();
    for i in 0..24 {
        let mut entry = transition_entry("x", ms(2000 * (24 - i)), Some(ms(1000)));
        entry.at = Some(KeyframeRef::Single("start".to_string()));
        builder = builder.entry(entry);
    }
    let doc = make_doc_with_storyboard(vec![("x", float_var())], vec![], "test", builder.build());

    let result = compile_storyboard(&doc, "test").unwrap();
    let starts: Vec<TimeDelta> = result.timelines["x"]
        .segments
        .iter()
        .map(|s| s.start_time)
        .collect();
    let expected: Vec<TimeDelta> = (1..=24).map(|k| delta_ms(2000 * k)).collect();
    assert_eq!(starts, expected);
}

// ---------------------------------------------------------------
// デシリアライズ境界（JSON は NaN を拒否する）
// ---------------------------------------------------------------

#[test]
fn json_rejects_nan_literal_at_deserialization_boundary() {
    // 標準 JSON 経由では NaN/inf リテラルを注入できない（serde_json がパース時に
    // 拒否する）。NaN/inf の流入経路は機能 `toml` の TOML（nan/inf/-inf）で、時間の欄は
    // 読み込みで落ち、ほかの数値の欄に来た値は validate の数値の規則が拒む。Object の値の
    // 中の数は補間しない中身として運ぶだけで、数値の規則の外（下の TOML の境界のテスト）。
    assert!(serde_json::from_str::<TransitionDef>(r#"{ "delay": NaN }"#).is_err());
    assert!(serde_json::from_str::<TransitionDef>(r#"{ "delay": Infinity }"#).is_err());
}

#[test]
fn keyframe_name_with_implicit_prefix_is_rejected() {
    // 明示の "__implicit_1" はエントリ 1（keyframe 省略）の暗黙名とぶつかり、
    // 後勝ちで時刻が黙って取り違えられる。暗黙名の接頭辞は予約として拒む
    let doc = make_doc_with_storyboard(
        vec![("x", float_var()), ("y", float_var())],
        vec![],
        "test",
        StoryboardBuilder::new()
            // entry 0: 明示 keyframe "__implicit_1" at start+1
            .entry(StoryboardEntry {
                keyframe: Some("__implicit_1".to_string()),
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("start".to_string()),
                    offset: delta_ms(1000),
                }),
                ..Default::default()
            })
            // entry 1: keyframe 省略の遷移エントリ → 暗黙名 "__implicit_1"、
            // at start+5・duration 0 → kf_time = 5.0
            .entry(StoryboardEntry {
                variable: Some("y".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    to: Some(TransitionValue::Scalar(1.0)),
                    ..Default::default()
                })),
                keyframe: None,
                at: Some(KeyframeRef::WithOffset {
                    keyframes: KeyframeNames::Single("start".to_string()),
                    offset: delta_ms(5000),
                }),
                ..Default::default()
            })
            // entry 2: at "__implicit_1" を参照する遷移
            .entry(StoryboardEntry {
                variable: Some("x".to_string()),
                transition: Some(TransitionRef::Inline(TransitionDef {
                    from: Some(TransitionValue::Scalar(0.0)),
                    to: Some(TransitionValue::Scalar(1.0)),
                    duration: Some(ms(1000)),
                    ..Default::default()
                })),
                at: Some(KeyframeRef::Single("__implicit_1".to_string())),
                ..Default::default()
            })
            .build(),
    );

    let errors = compile_storyboard(&doc, "test").unwrap_err();
    assert!(errors.iter().any(|e| matches!(
        e,
        DolaError::ReservedKeyframeName { storyboard, entry_index: 0, name }
            if storyboard == "test" && name == "__implicit_1"
    )));
}

#[test]
fn long_dependency_chain_compiles_without_stack_exhaustion() {
    // 10,000 エントリの at 依存連鎖（kf0 ← kf1 ← ... ← kf9999）を 64 KiB の
    // スタックのスレッドでコンパイルする。トポロジカルソート・時刻解決が
    // 連鎖長に比例したスタックを使う再帰なら、このスレッドは落ちる。
    const N: usize = 10_000;
    let mut builder = StoryboardBuilder::new();
    for i in 0..N {
        let at_ref = if i == 0 {
            KeyframeRef::Single("start".to_string())
        } else {
            KeyframeRef::WithOffset {
                keyframes: KeyframeNames::Single(format!("kf{}", i - 1)),
                offset: delta_ms(1000),
            }
        };
        builder = builder.entry(StoryboardEntry {
            keyframe: Some(format!("kf{}", i)),
            at: Some(at_ref),
            ..Default::default()
        });
    }
    let doc = make_doc_with_storyboard(vec![], vec![], "chain", builder.build());

    let result = std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || compile_storyboard(&doc, "chain").unwrap())
        .unwrap()
        .join()
        .expect("compile must not exhaust a 64 KiB stack");
    assert_eq!(result.timelines.len(), 0);
    assert_eq!(result.total_base_duration, ms(0));
}

use std::collections::BTreeMap;

#[test]
fn storyboard_builder_cannot_bypass_compile_validation() {
    // StoryboardBuilder は検証なしで任意の Storyboard を構築できるが、
    // DolaDocumentBuilder を経由せず DolaDocument を直接組んでも、
    // compile_storyboard が冒頭で必ず validate() を実行するため
    // 不正文書（未定義変数参照）は panic ではなくエラーで拒否される。
    let sb = StoryboardBuilder::new()
        .entry(StoryboardEntry {
            variable: Some("undefined_var".to_string()),
            transition: Some(TransitionRef::Inline(TransitionDef {
                to: Some(TransitionValue::Scalar(1.0)),
                ..Default::default()
            })),
            ..Default::default()
        })
        .build();

    let mut storyboard = BTreeMap::new();
    storyboard.insert("test".to_string(), sb);
    let doc = DolaDocument {
        schema_version: "1.0".to_string(),
        variable: BTreeMap::new(),
        transition: BTreeMap::new(),
        storyboard,
    };

    let errs = compile_storyboard(&doc, "test").unwrap_err();
    assert!(errs.iter().any(
        |e| matches!(e, DolaError::UndefinedVariable { name, .. } if name == "undefined_var")
    ));
}

// ---------------------------------------------------------------
// デシリアライズ境界（TOML は nan/inf/-inf を読める・機能 `toml`）
// ---------------------------------------------------------------

#[cfg(feature = "toml")]
#[test]
fn toml_non_finite_numbers_are_rejected_by_validate() {
    // 型の決まった欄（initial・time_scale）と untagged の欄（to・ベジェの点）に同じ値を入れる。
    // 時間の欄は読み込みで落ちる（下の toml_non_finite_times_fail_to_load）
    const TEMPLATE: &str = r#"
schema_version = "1.0"

[variable.x]
type = "f64"
initial = V

[transition.t]
to = V
easing = { type = "cubic_bezier", x0 = 0.0, x1 = V, x2 = 0.0, x3 = 1.0 }

[storyboard.s]
time_scale = V

[[storyboard.s.entry]]
variable = "x"
transition = "t"
at = { keyframes = "start", offset = 0.0 }

[[storyboard.s.entry]]
variable = "x"
transition = { to = V }
"#;
    let mut expected = vec![
        ("storyboard 's'", "time_scale"),
        ("storyboard 's' entry 1", "to"),
        ("transition 't'", "easing.x1"),
        ("transition 't'", "to"),
        ("variable 'x'", "initial"),
    ];
    expected.sort();
    for value in ["nan", "inf", "-inf"] {
        let doc = DolaDocument::from_toml_str(&TEMPLATE.replace('V', value))
            .unwrap_or_else(|e| panic!("{value}: read refused: {e}"));
        let mut fields: Vec<(String, String)> = doc
            .validate()
            .unwrap_err()
            .into_iter()
            .filter_map(|e| match e {
                DolaError::InvalidNumber {
                    location, field, ..
                } => Some((location, field)),
                _ => None,
            })
            .collect();
        fields.sort();
        let fields: Vec<(&str, &str)> = fields
            .iter()
            .map(|(l, f)| (l.as_str(), f.as_str()))
            .collect();
        assert_eq!(fields, expected, "{value}");
    }
}

#[cfg(feature = "toml")]
#[test]
fn toml_non_finite_times_fail_to_load() {
    // 0.5 は読めるので、落ちるのは値のせいで、文書の形のせいではない
    for value in ["0.5", "nan", "inf", "-inf"] {
        for part in [
            format!("[transition.t]\nto = 1.0\ndelay = {value}\n"),
            format!(
                "[storyboard.s]\nloop_offset = {value}\n[[storyboard.s.entry]]\nkeyframe = \"k\"\n"
            ),
            format!(
                "[[storyboard.s.entry]]\nkeyframe = \"k\"\nat = {{ keyframes = \"start\", offset = {value} }}\n"
            ),
        ] {
            let text = format!("schema_version = \"1.0\"\n{part}");
            let read = DolaDocument::from_toml_str(&text);
            assert_eq!(read.is_ok(), value == "0.5", "{text}");
        }
    }
}

#[cfg(feature = "toml")]
#[test]
fn toml_non_finite_in_object_value_does_not_panic() {
    // Object の値は補間しない中身として運ぶだけで、数値の規則の外（通る）
    let doc = DolaDocument::from_toml_str(
        r#"
schema_version = "1.0"

[variable.o]
type = "object"
initial = { a = nan, b = [inf, -inf] }

[[storyboard.s.entry]]
variable = "o"
transition = { to = { a = inf, b = nan } }
"#,
    )
    .unwrap();
    assert!(doc.validate().is_ok());
    assert!(compile_storyboard(&doc, "s").is_ok());
}
