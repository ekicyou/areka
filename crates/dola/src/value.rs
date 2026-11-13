//! フォーマット非依存の動的値型（DynamicValue）。
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// フォーマット非依存の動的値型（JSON/TOML 共通）
/// バリアント順序: Integer を Float より前に定義し、TOML の整数/浮動小数点区別を保持
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DynamicValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<DynamicValue>),
    Map(BTreeMap<String, DynamicValue>),
}

/// Float の等しさとハッシュの鍵: 0.0 と -0.0 を同じ値、どの NaN も 1 つの値、ほかはビット。
fn float_key(v: f64) -> u64 {
    if v == 0.0 {
        0
    } else if v.is_nan() {
        f64::NAN.to_bits()
    } else {
        v.to_bits()
    }
}

/// `DynamicValue` の等しさ。Float の腕だけ `float_key` で比べ、`Hash` と同じ鍵にする
/// （ObjectInternPool の HashMap は等しさで引く）。ほかの腕は derive と同じ。
impl PartialEq for DynamicValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DynamicValue::Null, DynamicValue::Null) => true,
            (DynamicValue::Bool(a), DynamicValue::Bool(b)) => a == b,
            (DynamicValue::Integer(a), DynamicValue::Integer(b)) => a == b,
            (DynamicValue::Float(a), DynamicValue::Float(b)) => float_key(*a) == float_key(*b),
            (DynamicValue::String(a), DynamicValue::String(b)) => a == b,
            (DynamicValue::Array(a), DynamicValue::Array(b)) => a == b,
            (DynamicValue::Map(a), DynamicValue::Map(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for DynamicValue {}

/// `DynamicValue` の `Hash` 実装。Float は `float_key` をハッシュする。
impl Hash for DynamicValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            DynamicValue::Null => {}
            DynamicValue::Bool(v) => v.hash(state),
            DynamicValue::Integer(v) => v.hash(state),
            DynamicValue::Float(v) => float_key(*v).hash(state),
            DynamicValue::String(v) => v.hash(state),
            DynamicValue::Array(v) => {
                v.len().hash(state);
                for item in v {
                    item.hash(state);
                }
            }
            DynamicValue::Map(v) => {
                v.len().hash(state);
                for (k, val) in v {
                    k.hash(state);
                    val.hash(state);
                }
            }
        }
    }
}
