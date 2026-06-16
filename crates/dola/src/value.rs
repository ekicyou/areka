//! フォーマット非依存の動的値型（DynamicValue）。
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// フォーマット非依存の動的値型（JSON/TOML/YAML 共通）
/// バリアント順序: Integer を Float より前に定義し、TOML の整数/浮動小数点区別を保持
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// `DynamicValue` の `Eq` 実装。
///
/// `Float(f64)` は厳密には `Eq` を満たさない（NaN != NaN）が、
/// アニメーション定義値として NaN が入ることは実用上ない前提で実装する。
impl Eq for DynamicValue {}

/// `DynamicValue` の `Hash` 実装。
///
/// `Float(f64)` は `to_bits()` でビットパターンをハッシュする。
impl Hash for DynamicValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            DynamicValue::Null => {}
            DynamicValue::Bool(v) => v.hash(state),
            DynamicValue::Integer(v) => v.hash(state),
            DynamicValue::Float(v) => v.to_bits().hash(state),
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
