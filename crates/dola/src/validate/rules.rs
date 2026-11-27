//! `DolaDocument::validate` から呼ばれる個々のバリデーションルール。

use std::collections::BTreeSet;

use crate::document::DolaDocument;
use crate::easing::{EasingFunction, ParametricEasing};
use crate::error::DolaError;
use crate::storyboard::{KeyframeNames, KeyframeRef, LoopOffset};
use crate::time::serde_secs::within_document_range;
use crate::transition::{TransitionDef, TransitionRef, TransitionValue};
use crate::variable::AnimationVariableDef;

/// 暗黙のキーフレーム名の接頭辞（名の無いエントリは `__implicit_{idx}`・明示名では使えない）
pub(crate) const IMPLICIT_KEYFRAME_PREFIX: &str = "__implicit_";

/// 期待されるスキーマバージョン
pub(super) const EXPECTED_SCHEMA_VERSION: &str = "1.0";

/// スキーマバージョン検証
pub(super) fn validate_schema_version(doc: &DolaDocument, errors: &mut Vec<DolaError>) {
    if doc.schema_version != EXPECTED_SCHEMA_VERSION {
        errors.push(DolaError::SchemaVersionMismatch {
            expected: EXPECTED_SCHEMA_VERSION.to_string(),
            found: doc.schema_version.clone(),
        });
    }
}

/// 数値の規則: 有限・符号・範囲を外れた実数を全部 `InvalidNumber`、文書の値の上限を越える時間を全部 `TimeOutOfDocumentRange` として集める
///
/// NaN は比べが全部偽になり、ほかの規則（V12・V14〜V16・重なりの検査）を素通りするので、ここで拒む。
pub(super) fn validate_numbers(doc: &DolaDocument, errors: &mut Vec<DolaError>) {
    for (name, var_def) in &doc.variable {
        if let AnimationVariableDef::Float { initial, min, max } = var_def {
            let location = format!("variable '{name}'");
            push_unless(errors, &location, "initial", *initial, initial.is_finite());
            for (field, v) in [("min", min), ("max", max)] {
                if let Some(v) = v {
                    push_unless(errors, &location, field, *v, v.is_finite());
                }
            }
        }
    }
    for (name, def) in &doc.transition {
        validate_transition_numbers(&format!("transition '{name}'"), def, errors);
    }
    for (sb_name, sb) in &doc.storyboard {
        let location = format!("storyboard '{sb_name}'");
        let ts = sb.time_scale;
        push_unless(
            errors,
            &location,
            "time_scale",
            ts,
            ts.is_finite() && ts > 0.0,
        );
        let lc = sb.loop_count;
        push_unless(
            errors,
            &location,
            "loop_count",
            lc.into(),
            lc >= 1 || lc == -1,
        );
        match &sb.loop_offset {
            Some(LoopOffset::Scalar(v)) => {
                push_unless_in_range(errors, &location, "loop_offset", v.as_ticks())
            }
            Some(LoopOffset::Range(r)) => {
                push_unless_in_range(errors, &location, "loop_offset.min", r.min.as_ticks());
                push_unless_in_range(errors, &location, "loop_offset.max", r.max.as_ticks());
                validate_easing_numbers(&location, "loop_offset.easing", &r.easing, errors);
            }
            None => {}
        }
        for (idx, entry) in sb.entry.iter().enumerate() {
            let location = format!("storyboard '{sb_name}' entry {idx}");
            if let Some(KeyframeRef::WithOffset { offset, .. }) = &entry.at {
                push_unless_in_range(errors, &location, "offset", offset.as_ticks());
            }
            if let Some(v) = entry.trigger_start_offset {
                push_unless_in_range(errors, &location, "trigger_start_offset", v.as_ticks());
            }
            if let Some(TransitionRef::Inline(def)) = &entry.transition {
                validate_transition_numbers(&location, def, errors);
            }
        }
    }
}

fn validate_transition_numbers(location: &str, def: &TransitionDef, errors: &mut Vec<DolaError>) {
    push_unless_in_range(errors, location, "delay", def.delay.as_ticks());
    if let Some(d) = def.duration {
        push_unless_in_range(errors, location, "duration", d.as_ticks());
    }
    for (field, value) in [("from", &def.from), ("to", &def.to)] {
        if let Some(TransitionValue::Scalar(v)) = value {
            push_unless(errors, location, field, *v, v.is_finite());
        }
    }
    if let Some(v) = def.relative_to {
        push_unless(errors, location, "relative_to", v, v.is_finite());
    }
    if let Some(easing) = &def.easing {
        validate_easing_numbers(location, "easing", easing, errors);
    }
}

/// ベジェの制御点の大きさの上限。有限でもこれを超えると補間が inf／NaN にあふれうる
/// （CSS の cubic-bezier の y はふつう −1〜2 なので、ゆとりを持たせた値）
const BEZIER_POINT_LIMIT: f64 = 1e3;

/// ベジェの制御点は有限かつ |x| ≤ `BEZIER_POINT_LIMIT` に限る
fn validate_easing_numbers(
    location: &str,
    prefix: &str,
    easing: &EasingFunction,
    errors: &mut Vec<DolaError>,
) {
    let points: &[(&str, f64)] = match easing {
        EasingFunction::Named(_) => &[],
        EasingFunction::Parametric(ParametricEasing::QuadraticBezier { x0, x1, x2 }) => {
            &[("x0", *x0), ("x1", *x1), ("x2", *x2)]
        }
        EasingFunction::Parametric(ParametricEasing::CubicBezier { x0, x1, x2, x3 }) => {
            &[("x0", *x0), ("x1", *x1), ("x2", *x2), ("x3", *x3)]
        }
    };
    for (name, v) in points {
        push_unless(
            errors,
            location,
            &format!("{prefix}.{name}"),
            *v,
            v.is_finite() && v.abs() <= BEZIER_POINT_LIMIT,
        );
    }
}

fn push_unless(errors: &mut Vec<DolaError>, location: &str, field: &str, value: f64, ok: bool) {
    if !ok {
        errors.push(DolaError::InvalidNumber {
            location: location.to_string(),
            field: field.to_string(),
            value,
        });
    }
}

fn push_unless_in_range(errors: &mut Vec<DolaError>, location: &str, field: &str, ticks: i64) {
    if !within_document_range(ticks) {
        errors.push(DolaError::TimeOutOfDocumentRange {
            location: location.to_string(),
            field: field.to_string(),
        });
    }
}

/// 変数初期値の値域検証
pub(super) fn validate_variable_ranges(doc: &DolaDocument, errors: &mut Vec<DolaError>) {
    for (var_name, var_def) in &doc.variable {
        match var_def {
            AnimationVariableDef::Float { initial, min, max } => {
                if let Some(min_val) = min
                    && *initial < *min_val
                {
                    errors.push(DolaError::ValueOutOfRange {
                        variable: var_name.clone(),
                        field: "initial".to_string(),
                        value: *initial,
                        min: *min_val,
                        max: max.unwrap_or(f64::INFINITY),
                    });
                }
                if let Some(max_val) = max
                    && *initial > *max_val
                {
                    errors.push(DolaError::ValueOutOfRange {
                        variable: var_name.clone(),
                        field: "initial".to_string(),
                        value: *initial,
                        min: min.unwrap_or(f64::NEG_INFINITY),
                        max: *max_val,
                    });
                }
            }
            AnimationVariableDef::Integer {
                initial, min, max, ..
            } => {
                if let Some(min_val) = min
                    && *initial < *min_val
                {
                    errors.push(DolaError::ValueOutOfRange {
                        variable: var_name.clone(),
                        field: "initial".to_string(),
                        value: *initial as f64,
                        min: *min_val as f64,
                        max: max.unwrap_or(i64::MAX) as f64,
                    });
                }
                if let Some(max_val) = max
                    && *initial > *max_val
                {
                    errors.push(DolaError::ValueOutOfRange {
                        variable: var_name.clone(),
                        field: "initial".to_string(),
                        value: *initial as f64,
                        min: min.unwrap_or(i64::MIN) as f64,
                        max: *max_val as f64,
                    });
                }
            }
            AnimationVariableDef::Object { .. } => {
                // 値域検証なし
            }
        }
    }
}

/// キーフレーム名重複検出（明示的 keyframe フィールドのみ対象）
pub(super) fn validate_duplicate_keyframes(
    sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    let mut seen = BTreeSet::new();
    for entry in &sb.entry {
        if let Some(ref kf_name) = entry.keyframe
            && !seen.insert(kf_name.clone())
        {
            errors.push(DolaError::DuplicateKeyframe {
                storyboard: sb_name.to_string(),
                name: kf_name.clone(),
            });
        }
    }
}

/// 予約キーフレーム名検証（"start" と暗黙名の接頭辞 `__implicit_` は使用禁止）
pub(super) fn validate_reserved_keyframe_names(
    sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    for (entry_index, entry) in sb.entry.iter().enumerate() {
        if let Some(ref kf_name) = entry.keyframe
            && (kf_name == "start" || kf_name.starts_with(IMPLICIT_KEYFRAME_PREFIX))
        {
            errors.push(DolaError::ReservedKeyframeName {
                storyboard: sb_name.to_string(),
                entry_index,
                name: kf_name.clone(),
            });
        }
    }
}

/// キーフレーム参照検証（前方参照を許可し、暗黙のキーフレームも参照先に含める）
pub(super) fn validate_keyframe_references(
    sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    // 全キーフレーム名を収集する（明示的 + 暗黙的 + "start" 予約）
    let mut known_keyframes = BTreeSet::new();
    known_keyframes.insert("start".to_string());

    for (idx, entry) in sb.entry.iter().enumerate() {
        if let Some(ref kf_name) = entry.keyframe {
            known_keyframes.insert(kf_name.clone());
        } else {
            known_keyframes.insert(format!("{IMPLICIT_KEYFRAME_PREFIX}{idx}"));
        }
    }

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

/// 変数型に基づくトランジション制約
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
            // Object 型は to のみ許可（from/relative_to/easing 不可、to は Dynamic のみ）
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
            if let Some(ref to) = trans_def.to
                && matches!(to, TransitionValue::Scalar(_))
            {
                errors.push(DolaError::TypeMismatch {
                    storyboard: sb_name.to_string(),
                    entry_index: entry_idx,
                    reason: "Object variable requires Dynamic transition value".to_string(),
                });
            }
        }
        AnimationVariableDef::Float { .. } | AnimationVariableDef::Integer { .. } => {
            // f64/i64 変数の from/to は Scalar のみ
            if let Some(ref from) = trans_def.from
                && matches!(from, TransitionValue::Dynamic(_))
            {
                errors.push(DolaError::TypeMismatch {
                    storyboard: sb_name.to_string(),
                    entry_index: entry_idx,
                    reason: "Numeric variable requires Scalar transition value for 'from'"
                        .to_string(),
                });
            }
            if let Some(ref to) = trans_def.to
                && matches!(to, TransitionValue::Dynamic(_))
            {
                errors.push(DolaError::TypeMismatch {
                    storyboard: sb_name.to_string(),
                    entry_index: entry_idx,
                    reason: "Numeric variable requires Scalar transition value for 'to'"
                        .to_string(),
                });
            }

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
                if let Some(TransitionValue::Scalar(val)) = value
                    && (*val < min_f || *val > max_f)
                {
                    errors.push(DolaError::TransitionValueOutOfRange {
                        storyboard: sb_name.to_string(),
                        entry_index: entry_idx,
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

/// loop_offset の範囲の逆転の検証
pub(super) fn validate_loop_offset(
    sb_name: &str,
    sb: &crate::storyboard::Storyboard,
    errors: &mut Vec<DolaError>,
) {
    if let Some(LoopOffset::Range(r)) = &sb.loop_offset
        && r.min > r.max
    {
        errors.push(DolaError::LoopOffsetRangeInverted {
            storyboard: sb_name.to_string(),
            min: r.min,
            max: r.max,
        });
    }
}

/// トリガー循環参照検出（ドキュメント内全ストーリーボードのトリガーグラフを DFS する）。
// shortcut: トリガーの枝分かれの数は数えない。各段が次を 2 回起こす n 段の鎖は、1 回の tick に 2^n − 2 個の子を起こす
// （16 段で約 6.5 万件・約 0.17 秒、24 段で約 45 秒の見込み、1 段ごとに倍）。作者の書いた文書を読み込むようになったら、
// 循環の無いことを確かめた後に、1 回の start が起こす子の数（1 ＋ 起こす先の数の和・飽和で足す）に上限を置く。
pub(super) fn validate_trigger_cycles(doc: &DolaDocument, errors: &mut Vec<DolaError>) {
    use std::collections::{HashMap, HashSet};

    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for (sb_name, sb) in &doc.storyboard {
        let targets: Vec<&str> = sb
            .entry
            .iter()
            .filter_map(|e| e.trigger_storyboard.as_deref())
            .collect();
        graph.insert(sb_name.as_str(), targets);
    }

    let mut visited: HashSet<&str> = HashSet::new();
    let mut in_stack: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    // 根は名前の順（BTreeMap）で回し、報告を決定的にする
    for sb_name in doc.storyboard.keys() {
        let sb_name = sb_name.as_str();
        if !visited.contains(sb_name) {
            if let Some(cycle) =
                dfs_detect_cycle(sb_name, &graph, &mut visited, &mut in_stack, &mut path)
            {
                errors.push(DolaError::TriggerCycle { cycle });
            }
            // 循環を見つけて途中で抜けた根の道と印を、次の根へ持ち越さない
            path.clear();
            in_stack.clear();
        }
    }
}

/// DFS ヘルパー: 循環を見つけたらそのパスを返す
///
/// 細工した深い鎖でコールスタックを使い切らないよう、再帰でなく明示のスタックで回す。
/// `path` が節のスタックで、`next_idx` は同じ深さの節が次に見る隣の位置。
fn dfs_detect_cycle<'a>(
    root: &'a str,
    graph: &std::collections::HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    in_stack: &mut std::collections::HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    let mut next_idx: Vec<usize> = vec![0];
    visited.insert(root);
    in_stack.insert(root);
    path.push(root);

    while let (Some(&node), Some(idx)) = (path.last(), next_idx.last_mut()) {
        let Some(&next) = graph.get(node).and_then(|neighbors| neighbors.get(*idx)) else {
            path.pop();
            next_idx.pop();
            in_stack.remove(node);
            continue;
        };
        *idx += 1;
        if !visited.contains(next) {
            visited.insert(next);
            in_stack.insert(next);
            path.push(next);
            next_idx.push(0);
        } else if in_stack.contains(next) {
            // SAFETY: in_stack への挿入/除去は path への push/pop と常に対で
            // 行われるため両者は同一の要素集合を保持する。in_stack に含まれる
            // next は必ず path 内に存在し、position(..).unwrap() は panic 不能。
            let start_pos = path.iter().position(|&n| n == next).unwrap();
            let mut cycle: Vec<String> = path[start_pos..].iter().map(|s| s.to_string()).collect();
            cycle.push(next.to_string());
            return Some(cycle);
        }
    }
    None
}
