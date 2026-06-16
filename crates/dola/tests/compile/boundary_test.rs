//! D2-V: デシリアライズ境界・数値境界・panic 経路の特性化テスト
//!
//! 外部入力（JSON/TOML/YAML）由来の DolaDocument が compile へ渡される境界における
//! 現行挙動を固定する。検証（有限性・符号・名前予約）の追加は外部観測可能な
//! 挙動変更となるため本ループでは行わず（P14/P20/P21 参照）、ここでは
//! 「panic しないこと」と「静かな縮退の具体的な形」を回帰検知器として固定する。

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

    // ---------------------------------------------------------------
    // 負値（符号未検証: P20）
    // ---------------------------------------------------------------

    #[test]
    fn negative_duration_produces_inverted_segment() {
        // duration の負値は検証されず、end_time < start_time の反転セグメントが
        // 生成され base_duration も負となる。total_base_duration は
        // fold(0.0, f64::max) の初期値 0.0 でマスクされる（P20 で特性化）。
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(transition_entry("x", 0.0, Some(-3.0)))
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments.len(), 1);
        assert_eq!(tl.segments[0].start_time, 0.0);
        assert_eq!(tl.segments[0].end_time, -3.0); // 反転（end < start）
        assert_eq!(tl.base_duration, -3.0); // 負の base_duration
        assert_eq!(result.total_base_duration, 0.0); // fold 初期値 0.0 でマスク
    }

    #[test]
    fn negative_delay_shifts_segment_before_start_time() {
        // 負の delay は start_time より過去のセグメント開始を生成する（検証なし）
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(transition_entry("x", -2.0, Some(1.0)))
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 10.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].start_time, 8.0); // 10.0 + (-2.0)
        assert_eq!(tl.segments[0].end_time, 9.0);
    }

    // ---------------------------------------------------------------
    // NaN / inf（有限性未検証: P14）
    // ---------------------------------------------------------------

    #[test]
    fn nan_duration_propagates_without_panic() {
        // NaN duration は panic せずセグメント時刻へ伝播する。
        // base_duration は NaN、total_base_duration は f64::max の
        // NaN 無視特性により 0.0 へ黙って脱落する。
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(transition_entry("x", 0.0, Some(f64::NAN)))
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert!(tl.segments[0].end_time.is_nan());
        assert!(tl.base_duration.is_nan());
        assert_eq!(result.total_base_duration, 0.0); // NaN は max で脱落
    }

    #[test]
    fn nan_times_bypass_overlap_detection() {
        // 1 本目のセグメントが NaN 時刻となる場合、重複検査の比較
        // （prev.end > next.start）は NaN で常に false → 重複は検出されず
        // コンパイルは成功する（現行挙動の固定。P14）。
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(transition_entry("x", f64::NAN, Some(10.0)))
                .entry(transition_entry("x", 0.0, Some(1.0)))
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments.len(), 2); // エラーなく 2 セグメント出力
    }

    #[test]
    fn infinite_delay_yields_infinite_segment_times() {
        // +inf delay は inf 時刻として panic せず伝播する
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                .entry(transition_entry("x", f64::INFINITY, Some(1.0)))
                .build(),
        );

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert_eq!(tl.segments[0].start_time, f64::INFINITY);
        assert_eq!(tl.segments[0].end_time, f64::INFINITY);
    }

    #[test]
    fn between_nan_delay_bypasses_inversion_check() {
        // between 配置の反転検査（segment_start >= segment_end）は NaN 比較が
        // 常に false となるため素通りし、NaN 開始時刻のセグメントが出力される
        let doc = make_doc_with_storyboard(
            float_var("x"),
            vec![],
            "test",
            StoryboardBuilder::new()
                // kf "a" at start
                .entry(StoryboardEntry {
                    keyframe: Some("a".to_string()),
                    at: Some(KeyframeRef::Single("start".to_string())),
                    ..Default::default()
                })
                // kf "b" at start+5
                .entry(StoryboardEntry {
                    keyframe: Some("b".to_string()),
                    at: Some(KeyframeRef::WithOffset {
                        keyframes: KeyframeNames::Single("start".to_string()),
                        offset: 5.0,
                    }),
                    ..Default::default()
                })
                // between a..b, delay = NaN
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

        let result = compile_storyboard(&doc, "test", 0.0).unwrap();
        let tl = result.timelines.get("x").unwrap();
        assert!(tl.segments[0].start_time.is_nan()); // 反転検査を素通り
        assert_eq!(tl.segments[0].end_time, 5.0);
    }

    #[test]
    fn nan_trigger_start_offset_passes_through() {
        // trigger_start_offset は検証なしで CompiledTrigger へそのまま伝播する（P14）
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

        let result = compile_storyboard(&doc, "parent", 0.0).unwrap();
        assert_eq!(result.triggers.len(), 1);
        assert!(result.triggers[0].start_offset.unwrap().is_nan());
    }

    // ---------------------------------------------------------------
    // デシリアライズ境界（JSON は NaN を拒否する）
    // ---------------------------------------------------------------

    #[test]
    fn json_rejects_nan_literal_at_deserialization_boundary() {
        // 標準 JSON 経由では NaN/inf リテラルを注入できない（serde_json がパース時に
        // 拒否する）。NaN/inf の流入経路は TOML（nan/inf）・YAML（.nan/.inf）のみ
        // （feature ゲートにより既定ビルド外。P14 参照）。
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
        // 1000 エントリの at 依存連鎖（kf0 ← kf1 ← ... ← kf999）。
        // トポロジカルソート・時刻解決は反復実装のため、連鎖長に比例した
        // スタックを消費しない（再帰なしの実証）。割り当てはエントリ数に
        // 線形比例し、入力サイズを超える増幅（capacity DoS）はない。
        const N: usize = 1000;
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

        let result = compile_storyboard(&doc, "chain", 0.0).unwrap();
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
        assert!(
            errs.iter()
                .any(|e| matches!(e, DolaError::UndefinedVariable { name, .. } if name == "undefined_var"))
        );
    }
}
