//! デシリアライズ境界・数値境界・panic 経路のテスト
//!
//! 外部入力（JSON/TOML/YAML）由来の DolaDocument が compile へ渡される境界で、
//! 数値の規則（#29）を外れた文書が panic でなくエラーで拒まれることと、
//! 検証を通った文書が panic しないことを縛る。

use dola::*;

use super::common::make_doc_with_storyboard;

/// f64 変数 1 個（制約なし）の variables リスト
fn float_var(name: &str) -> Vec<(&str, AnimationVariableDef)> {
    vec![(
        name,
        AnimationVariableDef::Float {
            initial: 0.0,
            min: None,
            max: None,
        },
    )]
}

/// インライン遷移エントリ（delay/duration 指定）
fn transition_entry(var: &str, delay: f64, duration: Option<f64>) -> StoryboardEntry {
    StoryboardEntry {
        variable: Some(var.to_string()),
        transition: Some(TransitionRef::Inline(TransitionDef {
            from: Some(TransitionValue::Scalar(0.0)),
            to: Some(TransitionValue::Scalar(1.0)),
            relative_to: None,
            easing: None,
            delay,
            duration,
        })),
        ..Default::default()
    }
}

#[cfg(test)]
mod numeric_boundary_tests {
    use super::*;

    /// compile が数値の規則で拒み、違反の欄を返す
    fn rejected_fields(doc: &DolaDocument, sb_name: &str) -> Vec<String> {
        compile_storyboard(doc, sb_name, 0.0)
            .unwrap_err()
            .into_iter()
            .map(|e| match e {
                DolaError::InvalidNumber { field, .. } => field,
                other => panic!("unexpected error: {other:?}"),
            })
            .collect()
    }

    fn single_transition_doc(delay: f64, duration: Option<f64>) -> DolaDocument {
        make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(transition_entry("x", delay, duration))
                .build(),
        )
    }

    #[test]
    fn invalid_delay_and_duration_are_rejected_before_compile() {
        // 拒まないと、負の duration は逆向きのセグメント、負の delay は start より前の
        // セグメント、NaN／inf は NaN／inf 時刻のセグメントになっていた（#29）
        for (delay, duration, field) in [
            (0.0, Some(-3.0), "duration"),
            (0.0, Some(f64::NAN), "duration"),
            (-2.0, Some(1.0), "delay"),
            (f64::INFINITY, Some(1.0), "delay"),
        ] {
            let doc = single_transition_doc(delay, duration);
            assert_eq!(
                rejected_fields(&doc, "test"),
                [field],
                "{delay} {duration:?}"
            );
        }
    }

    #[test]
    fn nan_delay_that_bypassed_overlap_check_is_rejected() {
        // NaN 時刻は重なりの比べ（prev.end > next.start）を素通りしていた
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(transition_entry("x", f64::NAN, Some(10.0)))
                .entry(transition_entry("x", 0.0, Some(1.0)))
                .build(),
        );
        assert_eq!(rejected_fields(&doc, "test"), ["delay"]);
    }

    #[test]
    fn between_nan_delay_is_rejected() {
        // between の逆転の検査（segment_start >= segment_end）を NaN が素通りしていた
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    keyframe: Some("a".to_string()),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    ..Default::default()
                })
                .entry(StoryboardEntry {
                    keyframe: Some("b".to_string()),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("start".to_string()),
                        offset: 5.0,
                    }),
                    ..Default::default()
                })
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: f64::NAN,
                        duration: None,
                    })),
                    between: Some(BetweenKeyframes {
                        from: "a".to_string(),
                        to: "b".to_string(),
                    }),
                    ..Default::default()
                })
                .build(),
        );
        assert_eq!(rejected_fields(&doc, "test"), ["delay"]);
    }

    #[test]
    fn nan_trigger_start_offset_is_rejected() {
        // 拒まないと CompiledTrigger へ NaN のまま渡っていた
        let mut doc = make_doc_with_storyboard(
            vec![],
            vec![],
            "parent",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    trigger_storyboard: Some("child".to_string()),
                    trigger_start_offset: Some(f64::NAN),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    ..Default::default()
                })
                .build(),
        );
        doc.storyboard.insert(
            "child".to_string(),
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    keyframe: Some("k".to_string()),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    ..Default::default()
                })
                .build(),
        );
        assert_eq!(rejected_fields(&doc, "parent"), ["trigger_start_offset"]);
    }

    #[test]
    fn segment_sort_with_many_nan_follows_total_order() {
        // 並べ替えは −0.0 を +0.0 にそろえた全順序（#10）。delay は 48…2 の降順に並べてあるので、
        // 有限の start_time では並べ替えを外すと昇順の確かめが落ちる。
        // #29 の後、公開の口からは NaN と有限の値が混ざった並びを作れない（文書の NaN は検証が拒み、
        // start_time の NaN は全部のセグメントを同じ符号の NaN にする）ので、全順序と partial_cmp の
        // 違いはここでは観測できない。NaN の回は panic しないことだけを見る。
        let mut builder = StoryboardBuilder::new();
        for i in 0..24 {
            let mut entry = transition_entry("x", f64::from(2 * (24 - i)), Some(1.0));
            entry.at = Some(KeyframeRef::Single("start".to_string()));
            builder = builder.entry(entry);
        }
        let doc = make_doc_with_storyboard(float_var("x"), vec![], "test", builder.build());

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let starts: Vec<f64> = result.timelines["x"]
            .segments
            .iter()
            .map(|s| s.start_time)
            .collect();
        let expected: Vec<f64> = (1..=24).map(|k| f64::from(2 * k)).collect();
        assert_eq!(starts, expected);

        let computed_nan = std::hint::black_box(f64::INFINITY) - f64::INFINITY;
        for start_time in [f64::NAN, computed_nan] {
            let result = compile_storyboard(&doc, "test", start_time).unwrap();
            assert_eq!(result.timelines["x"].segments.len(), 24);
        }
    }

    // ---------------------------------------------------------------
    // デシリアライズ境界（JSON は NaN を拒否する）
    // ---------------------------------------------------------------

    #[test]
    fn json_rejects_nan_literal_at_deserialization_boundary() {
        // 標準 JSON 経由では NaN/inf リテラルを注入できない（serde_json がパース時に
        // 拒否する）。NaN/inf の流入経路は TOML（nan/inf）・YAML（.nan/.inf）のみ
        // （feature ゲートにより既定ビルド外。そこから来た値は validate の数値の規則が拒む）。
        assert!(serde_json::from_str::<TransitionDef>(r#"{ "delay": NaN }"#).is_err());
        assert!(serde_json::from_str::<TransitionDef>(r#"{ "delay": Infinity }"#).is_err());
    }
}

#[cfg(test)]
mod keyframe_name_collision_tests {
    use super::*;

    #[test]
    fn explicit_implicit_name_collision_silently_shadows_user_keyframe() {
        // ユーザーが明示的に "__implicit_1" を指定すると、エントリ1（keyframe 省略の
        // 遷移エントリ）の暗黙名と衝突する。バリデーション（V2: 明示名同士の重複、
        // V3: "start" 予約）はこれを検出せず、keyframe_times の後勝ち上書きにより
        // 明示名の時刻（1.0）が暗黙名の時刻（5.0）でシャドウされる（黙った誤解決。
        // P21 で特性化）。
        let mut vars = float_var("x");
        vars.extend(float_var("y"));
        let doc = make_doc_with_storyboard(
            vars,
            vec![],
            "test",
            StoryboardBuilder::new()
                // entry 0: 明示 keyframe "__implicit_1" at start+1
                .entry(StoryboardEntry {
                    keyframe: Some("__implicit_1".to_string()),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("start".to_string()),
                        offset: 1.0,
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
                        offset: 5.0,
                    }),
                    ..Default::default()
                })
                // entry 2: at "__implicit_1" を参照する遷移
                .entry(StoryboardEntry {
                    variable: Some("x".to_string()),
                    transition: Some(TransitionRef::Inline(TransitionDef {
                        from: Some(TransitionValue::Scalar(0.0)),
                        to: Some(TransitionValue::Scalar(1.0)),
                        relative_to: None,
                        easing: None,
                        delay: 0.0,
                        duration: Some(1.0),
                    })),
                    at: Some(KeyframeRef::Single("__implicit_1".to_string())),
                    ..Default::default()
                })
                .build(),
        );

        // バリデーションは通過し（panic なし）、明示名 1.0 ではなく
        // 暗黙名 5.0 が参照解決される（後勝ち上書き）
        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].start_time, 5.0);
    }
}

#[cfg(test)]
mod stack_and_capacity_tests {
    use super::*;

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
                    offset: 1.0,
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
            .spawn(move || compile_storyboard(&doc, "chain", 0.0).unwrap())
            .unwrap()
            .join()
            .expect("compile must not exhaust a 64 KiB stack");
        assert_eq!(result.timelines.len(), 0); // 純粋KFのみ → タイムラインなし
        assert_eq!(result.total_base_duration, 0.0);
    }
}

#[cfg(test)]
mod builder_boundary_tests {
    use super::*;
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

        let errs = compile_storyboard(&doc, "test", 0.0).unwrap_err();
        assert!(errs.iter().any(
            |e| matches!(e, DolaError::UndefinedVariable { name, .. } if name == "undefined_var")
        ));
    }
}
