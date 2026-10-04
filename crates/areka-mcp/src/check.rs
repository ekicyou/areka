//! 引数の検査（spec: areka-P0-mcp-tool-entrances）。
//!
//! 登録した `inputSchema` に照らして `tools/call` の `arguments` を確かめる純粋な関数を置く。

use serde_json::{Map, Value};

/// 小数で書かれた整数を受ける上限（2^53。f64 が整数を正確に表せる範囲）。
const MAX_EXACT_FLOAT_INTEGER: f64 = 9_007_199_254_740_992.0;

/// 必須の欄 → 型 の順に調べ、最初に見つけた誤りの理由（英文）を返す。
///
/// - 必須: `schema["required"]` の並び順に、`args` に無い・`null` の欄を `missing required argument: <名前>` で拒む。
///   `ghost_name` は調べない（要件 2.3。欠落は名前の解決へ回す）。
/// - 型: `schema["properties"]` の各欄のうち `null` でない値だけを調べ、違えば `argument <名前> must be <型>`。
///   `array` の `items.type` に要素が合わないときは `argument <名前> must be array of <型>`。
///   未知の `type` と `properties` に無い欄は見ない（要件 2.5）。
/// - 理由の文は `failed to deserialize parameters:` で始めない（始めると rmcp が `isError` の結果へ変える）。
pub(crate) fn check_arguments(
    schema: &Map<String, Value>,
    args: &Map<String, Value>,
) -> Result<(), String> {
    let required = schema.get("required").and_then(Value::as_array);
    for name in required.into_iter().flatten().filter_map(Value::as_str) {
        if name == "ghost_name" {
            continue;
        }
        if args.get(name).is_none_or(Value::is_null) {
            return Err(format!("missing required argument: {name}"));
        }
    }

    let properties = schema.get("properties").and_then(Value::as_object);
    for (name, prop) in properties.into_iter().flatten() {
        let Some(value) = args.get(name).filter(|v| !v.is_null()) else {
            continue;
        };
        let Some(ty) = prop.get("type").and_then(Value::as_str) else {
            continue;
        };
        if matches_type(ty, value) == Some(false) {
            return Err(format!("argument {name} must be {ty}"));
        }
        let item_ty = prop
            .get("items")
            .and_then(|i| i.get("type"))
            .and_then(Value::as_str);
        if let (Some(items), Some(item_ty)) = (value.as_array(), item_ty)
            && items
                .iter()
                .any(|v| matches_type(item_ty, v) == Some(false))
        {
            return Err(format!("argument {name} must be array of {item_ty}"));
        }
    }
    Ok(())
}

/// 値が `type` に合うか。未知の `type` は `None`（調べない）。
fn matches_type(ty: &str, value: &Value) -> Option<bool> {
    Some(match ty {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "integer" => as_integer(value).is_some(),
        "array" => value.is_array(),
        _ => return None,
    })
}

/// "integer" の欄の読み方（検査と各ツールの詰め替えが同じ関数を使う）。
///
/// `as_i64` が取れればその値。取れないとき、小数部が 0 で絶対値が 2^53 以下の数（`1.0` など）は整数として受ける。
/// `1.5`・`i64` に収まらない数・数でない値は `None`。
pub(crate) fn as_integer(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    let f = value.as_f64().filter(|_| !value.is_u64())?;
    (f.fract() == 0.0 && f.abs() <= MAX_EXACT_FLOAT_INTEGER).then_some(f as i64)
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
