//! バリデーションルールヘルパー関数群
//!
//! validate/mod.rs の `impl Validate for DolaDocument` から呼び出される
//! 個々のバリデーションルール実装。

use std::collections::BTreeSet;

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::storyboard::{KeyframeNames, KeyframeRef, LoopOffset};
use crate::transition::TransitionValue;
use crate::variable::AnimationVariableDef;

/// 期待されるスキーマバージョン
pub(super) const EXPECTED_SCHEMA_VERSION: &str = "1.0";

/// V1: スキーマバージョン検証
pub(super) fn validate_schema_version(doc: &DolaDocument, errors: &mut Vec<DolaError>) {
    if doc.schema_version != EXPECTED_SCHEMA_VERSION {
        errors.push(DolaError::SchemaVersionMismatch {
            expected: EXPECTED_SCHEMA_VERSION.to_string(),
            found: doc.schema_version.clone(),
        });
    }
}

/// V12: 変数初期値の値域検証
///
/// NOTE(D3-V): NaN の initial/min/max は全比較（`<` / `>`）が false となるため
/// 本検査を素通りする（数値フィールドの有限性検証の欠如は P14 参照。
/// tests/validation/transition_test.rs::v12_nan_tests で現行挙動を特性化済み）。
pub(super) fn validate_variable_ranges(doc: &DolaDocument, errors: &mut Vec<DolaError>) {
    for (var_name, var_def) in &doc.variable {
        match var_def {
            AnimationVariableDef::Float { initial, min, max } => {
                if let Some(min_val) = min {
                    if *initial < *min_val {
                        errors.push(DolaError::ValueOutOfRange {
                            variable: var_name.clone(),
                            field: "initial".to_string(),
                            value: *initial,
                            min: *min_val,
                            max: max.unwrap_or(f64::INFINITY),
                        });
                    }
                }
                if let Some(max_val) = max {
                    if *initial > *max_val {
                        errors.push(DolaError::ValueOutOfRange {
                            variable: var_name.clone(),
                            field: "initial".to_string(),
                            value: *initial,
                            min: min.unwrap_or(f64::NEG_INFINITY),
                            max: *max_val,
                        });
                    }
                }
            }
            AnimationVariableDef::Integer {
                initial, min, max, ..
            } => {
                if let Some(min_val) = min {
                    if *initial < *min_val {
                        errors.push(DolaError::ValueOutOfRange {
                            variable: var_name.clone(),
                            field: "initial".to_string(),
                            value: *initial as f64,
                            min: *min_val as f64,
                            max: max.unwrap_or(i64::MAX) as f64,
                        });
                    }
                }
                if let Some(max_val) = max {
                    if *initial > *max_val {
                        errors.push(DolaError::ValueOutOfRange {
                            variable: var_name.clone(),
                            field: "initial".to_string(),
                            value: *initial as f64,
                            min: min.unwrap_or(i64::MIN) as f64,
                            max: *max_val as f64,
                        });
                    }
                }
            }
            AnimationVariableDef::Object { .. } => {
                // Object 型には値域検証なし
            }
        }
    }
}

/// V2: キーフレーム名重複検出（明示的 keyframe フィールドのみ対象）
pub(super) fn validate_duplicate_keyframes(
    sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    let mut seen = BTreeSet::new();
    for entry in &sb.entry {
        if let Some(ref kf_name) = entry.keyframe {
            if !seen.insert(kf_name.clone()) {
                errors.push(DolaError::DuplicateKeyframe {
                    storyboard: sb_name.to_string(),
                    name: kf_name.clone(),
                });
            }
        }
    }
}

/// V3: 予約キーフレーム名検証（"start" 使用禁止）
pub(super) fn validate_reserved_keyframe_names(
    _sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    for entry in &sb.entry {
        if let Some(ref kf_name) = entry.keyframe {
            if kf_name == "start" {
                errors.push(DolaError::ReservedKeyframeName {
                    name: kf_name.clone(),
                });
            }
        }
    }
}

/// V6: キーフレーム参照検証（前方参照許可 + 暗黙的KF追跡）
pub(super) fn validate_keyframe_references(
    sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    // 第1パス: 全キーフレーム名を収集（明示的 + 暗黙的 + "start" 予約）
    let mut known_keyframes = BTreeSet::new();
    known_keyframes.insert("start".to_string());

    for (idx, entry) in sb.entry.iter().enumerate() {
        if let Some(ref kf_name) = entry.keyframe {
            known_keyframes.insert(kf_name.clone());
        } else {
            // 暗黙的KF: __implicit_{index}
            known_keyframes.insert(format!("__implicit_{}", idx));
        }
    }

    // 第2パス: at/between の参照先が存在するか検証
    for entry in &sb.entry {
        if let Some(ref kf_ref) = entry.at {
            let names = collect_keyframe_names_from_ref(kf_ref);
            for name in names {
                if !known_keyframes.contains(&name) {
                    errors.push(DolaError::UndefinedKeyframe {
                        storyboard: sb_name.to_string(),
                        name,
                    });
                }
            }
        }
        if let Some(ref between) = entry.between {
            if !known_keyframes.contains(&between.from) {
                errors.push(DolaError::UndefinedKeyframe {
                    storyboard: sb_name.to_string(),
                    name: between.from.clone(),
                });
            }
            if !known_keyframes.contains(&between.to) {
                errors.push(DolaError::UndefinedKeyframe {
                    storyboard: sb_name.to_string(),
                    name: between.to.clone(),
                });
            }
        }
    }
}

/// KeyframeRef からキーフレーム名を収集
pub(crate) fn collect_keyframe_names_from_ref(kf_ref: &KeyframeRef) -> Vec<String> {
    match kf_ref {
        KeyframeRef::Single(name) => vec![name.clone()],
        KeyframeRef::Multiple(names) => names.clone(),
        KeyframeRef::WithOffset { keyframes, .. } => match keyframes {
            KeyframeNames::Single(name) => vec![name.clone()],
            KeyframeNames::Multiple(names) => names.clone(),
        },
    }
}

/// V10, V12(transition), V13: 変数型に基づくトランジション制約
pub(super) fn validate_transition_type_constraints(
    sb_name: &str,
    entry_idx: usize,
    var_name: &str,
    var_def: &AnimationVariableDef,
    trans_def: &crate::transition::TransitionDef,
    errors: &mut Vec<DolaError>,
) {
    match var_def {
        AnimationVariableDef::Object { .. } => {
            // V10: Object 型 → to のみ許可、from/relative_to/easing 不可、to は Dynamic のみ
            for (field, is_present) in [
                ("from", trans_def.from.is_some()),
                ("relative_to", trans_def.relative_to.is_some()),
                ("easing", trans_def.easing.is_some()),
            ] {
                if is_present {
                    errors.push(DolaError::ObjectTransitionViolation {
                        storyboard: sb_name.to_string(),
                        entry_index: entry_idx,
                        field: field.to_string(),
                    });
                }
            }
            if let Some(ref to) = trans_def.to {
                if matches!(to, TransitionValue::Scalar(_)) {
                    errors.push(DolaError::TypeMismatch {
                        storyboard: sb_name.to_string(),
                        entry_index: entry_idx,
                        reason: "Object variable requires Dynamic transition value".to_string(),
                    });
                }
            }
        }
        AnimationVariableDef::Float { .. } | AnimationVariableDef::Integer { .. } => {
            // V13: f64/i64 変数 → from/to は Scalar のみ
            if let Some(ref from) = trans_def.from {
                if matches!(from, TransitionValue::Dynamic(_)) {
                    errors.push(DolaError::TypeMismatch {
                        storyboard: sb_name.to_string(),
                        entry_index: entry_idx,
                        reason: "Numeric variable requires Scalar transition value for 'from'"
                            .to_string(),
                    });
                }
            }
            if let Some(ref to) = trans_def.to {
                if matches!(to, TransitionValue::Dynamic(_)) {
                    errors.push(DolaError::TypeMismatch {
                        storyboard: sb_name.to_string(),
                        entry_index: entry_idx,
                        reason: "Numeric variable requires Scalar transition value for 'to'"
                            .to_string(),
                    });
                }
            }

            // V12: トランジション from/to の値域検証
            let (min_f, max_f) = match var_def {
                AnimationVariableDef::Float { min, max, .. } => (
                    min.unwrap_or(f64::NEG_INFINITY),
                    max.unwrap_or(f64::INFINITY),
                ),
                AnimationVariableDef::Integer { min, max, .. } => (
                    min.map(|v| v as f64).unwrap_or(f64::NEG_INFINITY),
                    max.map(|v| v as f64).unwrap_or(f64::INFINITY),
                ),
                // SAFETY: 外側の match アームが Float | Integer のみを通すため、
                // この分岐は到達不能（panic 不能）。
                _ => unreachable!(),
            };

            for (field, value) in [("from", &trans_def.from), ("to", &trans_def.to)] {
                if let Some(TransitionValue::Scalar(val)) = value {
                    if *val < min_f || *val > max_f {
                        errors.push(DolaError::ValueOutOfRange {
                            variable: var_name.to_string(),
                            field: field.to_string(),
                            value: *val,
                            min: min_f,
                            max: max_f,
                        });
                    }
                }
            }
        }
    }
}

/// V14-V17: loop_offset バリデーション
///
/// NOTE(D3-V): NaN の min/max（Scalar(NaN) 含む）は V14-V16 の全比較が false と
/// なるため検出されずに通過し、ランタイムのループ遅延サンプリングへ流入する
/// （有限性検証の欠如は P14 参照。tests/runtime/loop_offset_test.rs::
/// nan_boundary_tests で現行挙動を特性化済み）。
pub(super) fn validate_loop_offset(
    sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    let lo = match &sb.loop_offset {
        Some(lo) => lo,
        None => return,
    };

    let (min, max) = match lo {
        LoopOffset::Scalar(v) => (0.0, *v),
        LoopOffset::Range(r) => (r.min, r.max),
    };

    // V14: min < 0
    if min < 0.0 {
        errors.push(DolaError::LoopOffsetNegativeMin {
            storyboard: sb_name.to_string(),
            value: min,
        });
    }

    // V15: max < 0
    if max < 0.0 {
        errors.push(DolaError::LoopOffsetNegativeMax {
            storyboard: sb_name.to_string(),
            value: max,
        });
    }

    // V16: min > max
    if min > max {
        errors.push(DolaError::LoopOffsetRangeInverted {
            storyboard: sb_name.to_string(),
            min,
            max,
        });
    }

    // V17: easing は serde の型システムが保証（追加バリデーション不要）
}

/// V15t: トリガー循環参照検出（ドキュメントレベル DFS）
///
/// ドキュメント内全ストーリーボードのトリガーグラフを構築し、
/// DFS で循環を検出する。O(V+E)。
pub(super) fn validate_trigger_cycles(doc: &DolaDocument, errors: &mut Vec<DolaError>) {
    use std::collections::{HashMap, HashSet};

    // トリガーグラフ構築: SB名 → トリガー先SB名のリスト
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for (sb_name, sb) in &doc.storyboard {
        let targets: Vec<&str> = sb
            .entry
            .iter()
            .filter_map(|e| e.trigger_storyboard.as_deref())
            .collect();
        graph.insert(sb_name.as_str(), targets);
    }

    // DFS による循環検出
    let mut visited: HashSet<&str> = HashSet::new();
    let mut in_stack: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for sb_name in graph.keys() {
        if !visited.contains(sb_name) {
            if let Some(cycle) =
                dfs_detect_cycle(sb_name, &graph, &mut visited, &mut in_stack, &mut path)
            {
                errors.push(DolaError::TriggerCycle { cycle });
            }
        }
    }
}

/// DFS ヘルパー: 循環検出時にパスを返す
///
/// NOTE(D3-V): 本関数はトリガー連鎖の深さに比例してコールスタックを消費する
/// 再帰実装であり、極端に深い trigger_storyboard 連鎖（数万 SB 規模）を持つ
/// 細工文書でスタック枯渇（abort）の可能性がある（D2-V 申し送りの確認結果）。
/// 明示スタックによる反復化は循環報告のメンバー・パス順序の同一性証明を要する
/// 構造的ロジック変更のため本ループでは実施せず、P23 として提案記録済み
/// （現実的な指示書規模では問題にならない。200 段チェーンは
/// tests/trigger/validation_test.rs::v15t_long_chain_200_storyboards_validates_ok
/// で動作をピン留め済み）。
fn dfs_detect_cycle<'a>(
    node: &'a str,
    graph: &std::collections::HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    in_stack: &mut std::collections::HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    visited.insert(node);
    in_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = graph.get(node) {
        for &next in neighbors {
            if !visited.contains(next) {
                if let Some(cycle) = dfs_detect_cycle(next, graph, visited, in_stack, path) {
                    return Some(cycle);
                }
            } else if in_stack.contains(next) {
                // 循環検出: path から next の位置以降を抽出
                // SAFETY: in_stack への挿入/除去は path への push/pop と常に対で
                // 行われるため両者は同一の要素集合を保持する。in_stack に含まれる
                // next は必ず path 内に存在し、position(..).unwrap() は panic 不能。
                let start_pos = path.iter().position(|&n| n == next).unwrap();
                let mut cycle: Vec<String> =
                    path[start_pos..].iter().map(|s| s.to_string()).collect();
                cycle.push(next.to_string()); // 循環を閉じる
                return Some(cycle);
            }
        }
    }

    path.pop();
    in_stack.remove(node);
    None
}
