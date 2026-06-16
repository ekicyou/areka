//! core_types: DynamicValue serde round-trip / hash / ordering
//! Task 2.5

use super::*;

mod dynamic_value_tests {
    use super::*;

    #[test]
    fn null_roundtrip() {
        let val = DynamicValue::Null;
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, "null");
        let deserialized: DynamicValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn bool_roundtrip() {
        let val = DynamicValue::Bool(true);
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, "true");
        let deserialized: DynamicValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn integer_roundtrip() {
        let val = DynamicValue::Integer(42);
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, "42");
        let deserialized: DynamicValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn float_roundtrip() {
        let val = DynamicValue::Float(3.14);
        let json = serde_json::to_string(&val).unwrap();
        let deserialized: DynamicValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn string_roundtrip() {
        let val = DynamicValue::String("hello".to_string());
        let json = serde_json::to_string(&val).unwrap();
        let deserialized: DynamicValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn array_roundtrip() {
        let val = DynamicValue::Array(vec![
            DynamicValue::Integer(1),
            DynamicValue::String("two".to_string()),
            DynamicValue::Bool(false),
        ]);
        let json = serde_json::to_string(&val).unwrap();
        let deserialized: DynamicValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn map_roundtrip() {
        let mut m = BTreeMap::new();
        m.insert("key".to_string(), DynamicValue::String("value".to_string()));
        m.insert("num".to_string(), DynamicValue::Integer(99));
        let val = DynamicValue::Map(m);
        let json = serde_json::to_string(&val).unwrap();
        let deserialized: DynamicValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deserialized);
    }

    /// D1b-T 追加: Hash 実装の整合性 — 構造的に等しい値は同一ハッシュ
    /// （runtime の ObjectInternPool は DynamicValue を HashMap キーとして使用する）
    #[test]
    fn hash_consistent_for_equal_values() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        fn hash_of(v: &DynamicValue) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        }

        let make = || {
            DynamicValue::Map({
                let mut m = BTreeMap::new();
                m.insert(
                    "arr".to_string(),
                    DynamicValue::Array(vec![
                        DynamicValue::Integer(1),
                        DynamicValue::Float(2.5),
                        DynamicValue::Bool(true),
                        DynamicValue::Null,
                    ]),
                );
                m.insert("s".to_string(), DynamicValue::String("x".to_string()));
                m
            })
        };
        let a = make();
        let b = make();
        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b), "equal values must hash equally");
    }

    /// D1b-T 追加（特性化）: Float は to_bits() でハッシュされるため、
    /// 0.0 と -0.0 は PartialEq では等しいがハッシュは異なる（Hash/Eq 契約違反の縁、P10 参照）。
    #[test]
    fn float_negative_zero_eq_but_hash_differs() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        fn hash_of(v: &DynamicValue) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        }

        let pos = DynamicValue::Float(0.0);
        let neg = DynamicValue::Float(-0.0);
        assert_eq!(pos, neg, "f64 PartialEq: 0.0 == -0.0");
        assert_ne!(
            hash_of(&pos),
            hash_of(&neg),
            "to_bits() hashing distinguishes 0.0 from -0.0 (current behavior)"
        );
    }

    /// D1b-T 追加（特性化）: NaN は自分自身と等しくない（Eq 実装のドキュメント済み前提）
    #[test]
    fn float_nan_not_equal_to_itself() {
        let nan = DynamicValue::Float(f64::NAN);
        assert_ne!(nan, nan.clone());
    }

    #[test]
    fn btreemap_deterministic_order() {
        // Insert keys in reverse order, verify serialized output is alphabetical
        let mut m = BTreeMap::new();
        m.insert("z_key".to_string(), DynamicValue::Integer(1));
        m.insert("a_key".to_string(), DynamicValue::Integer(2));
        m.insert("m_key".to_string(), DynamicValue::Integer(3));
        let val = DynamicValue::Map(m);
        let json = serde_json::to_string(&val).unwrap();
        let a_pos = json.find("a_key").unwrap();
        let m_pos = json.find("m_key").unwrap();
        let z_pos = json.find("z_key").unwrap();
        assert!(a_pos < m_pos);
        assert!(m_pos < z_pos);
    }
}
