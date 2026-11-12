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
        // 任意の float 値でよい（円周率の意図はない）。`3.14` は clippy::approx_constant が
        // 円周率の近似と誤検知するため、意味を持たない値を使う。
        let val = DynamicValue::Float(1.25);
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

    /// Hash 実装の整合性 — 構造的に等しい値は同一ハッシュ
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

    /// 等しい値は同じハッシュ（#28）。0.0 と -0.0、どの NaN どうしも等しく、ハッシュも同じ
    /// （runtime の ObjectInternPool は HashMap を等しさで引く）。
    #[test]
    fn float_equal_values_hash_equally() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        fn hash_of(v: &DynamicValue) -> u64 {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        }

        let pairs = [
            (0.0, -0.0),
            (f64::NAN, f64::NAN),
            (f64::NAN, -f64::NAN),
            (f64::NAN, f64::from_bits(0x7ff8_0000_0000_0001)),
        ];
        for (a, b) in pairs {
            let (a, b) = (DynamicValue::Float(a), DynamicValue::Float(b));
            assert_eq!(a, b, "{a:?} と {b:?} は等しい");
            assert_eq!(hash_of(&a), hash_of(&b), "{a:?} と {b:?} は同じハッシュ");
        }
        assert_ne!(DynamicValue::Float(1.0), DynamicValue::Float(f64::NAN));
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
