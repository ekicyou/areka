use super::*;
use crate::value::DynamicValue;
use std::rc::Rc;

// =====================================================================
// 5.1 購読・採番・冪等性
// =====================================================================

#[test]
fn subscribe_returns_sequential_ids() {
    let mut mgr = SubscriptionManager::new();
    let x_id = mgr.subscribe("x");
    let y_id = mgr.subscribe("y");
    assert_eq!(x_id, 0);
    assert_eq!(y_id, 1);
}

#[test]
fn subscribe_idempotent() {
    let mut mgr = SubscriptionManager::new();
    let id1 = mgr.subscribe("x");
    let id2 = mgr.subscribe("x");
    assert_eq!(id1, id2);
}

#[test]
fn subscribe_and_get_variable_names() {
    let mut mgr = SubscriptionManager::new();
    mgr.subscribe("x");
    mgr.subscribe("y");
    let mut vars = mgr.get_subscribed_variable_names();
    vars.sort();
    assert_eq!(vars, vec!["x", "y"]);
}

#[test]
fn subscribe_before_document() {
    let mut mgr = SubscriptionManager::new();
    let id = mgr.subscribe("x");
    assert_eq!(id, 0);
    let vars = mgr.get_subscribed_variable_names();
    assert_eq!(vars.len(), 1);

    // evaluate 結果なし → 変化なし（値が存在しない）
    let changed = mgr.diff_and_update(HashMap::new());
    assert!(changed.is_empty());
}

// =====================================================================
// 5.2 unsubscribe / unsubscribe_all / ライフサイクル
// =====================================================================

#[test]
fn unsubscribe_by_id() {
    let mut mgr = SubscriptionManager::new();
    let x_id = mgr.subscribe("x");
    mgr.subscribe("y");
    mgr.unsubscribe(x_id).unwrap();
    let vars = mgr.get_subscribed_variable_names();
    assert_eq!(vars.len(), 1);
    assert!(vars.contains(&"y".to_string()));
}

#[test]
fn unsubscribe_error_on_invalid_id() {
    let mut mgr = SubscriptionManager::new();
    let result = mgr.unsubscribe(999);
    assert!(matches!(result, Err(RuntimeError::InvalidVariableId(999))));
}

#[test]
fn unsubscribe_error_on_already_unsubscribed() {
    let mut mgr = SubscriptionManager::new();
    let id = mgr.subscribe("x");
    mgr.unsubscribe(id).unwrap();
    let result = mgr.unsubscribe(id);
    assert!(matches!(result, Err(RuntimeError::InvalidVariableId(_))));
}

#[test]
fn unsubscribe_all_clears_subscriptions() {
    let mut mgr = SubscriptionManager::new();
    mgr.subscribe("x");
    mgr.subscribe("y");
    mgr.unsubscribe_all();
    assert!(mgr.get_subscribed_variable_names().is_empty());
}

#[test]
fn resubscribe_after_unsubscribe_gets_new_id() {
    let mut mgr = SubscriptionManager::new();
    let id1 = mgr.subscribe("x");
    assert_eq!(id1, 0);
    mgr.unsubscribe(id1).unwrap();

    let id2 = mgr.subscribe("x");
    assert_eq!(id2, 1); // 新しいID
    assert_ne!(id1, id2);
}

#[test]
fn resubscribe_after_unsubscribe_all_gets_new_id() {
    let mut mgr = SubscriptionManager::new();
    let id1 = mgr.subscribe("x");
    mgr.unsubscribe_all();

    let id2 = mgr.subscribe("x");
    assert_ne!(id1, id2);
}

// =====================================================================
// 5.3 get_variable_name 逆引き
// =====================================================================

#[test]
fn get_variable_name_valid() {
    let mut mgr = SubscriptionManager::new();
    let id = mgr.subscribe("opacity");
    assert_eq!(mgr.get_variable_name(id).unwrap(), "opacity");
}

#[test]
fn get_variable_name_after_unsubscribe_still_works() {
    let mut mgr = SubscriptionManager::new();
    let id = mgr.subscribe("opacity");
    mgr.unsubscribe(id).unwrap();
    // id_to_name は維持されるので逆引き可能
    assert_eq!(mgr.get_variable_name(id).unwrap(), "opacity");
}

#[test]
fn get_variable_name_error_on_invalid_id() {
    let mgr = SubscriptionManager::new();
    let result = mgr.get_variable_name(999);
    assert!(matches!(result, Err(RuntimeError::InvalidVariableId(999))));
}

// =====================================================================
// 5.4 差分検出・凍結値・force_update・convert_names_to_ids
// =====================================================================

#[test]
fn diff_detects_change() {
    let mut mgr = SubscriptionManager::new();
    let x_id = mgr.subscribe("x");

    let mut values = HashMap::new();
    values.insert("x".to_string(), EvaluatedValue::Float(10.0));

    let changed = mgr.diff_and_update(values);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, x_id);
    assert_eq!(changed[0].1, EvaluatedValue::Float(10.0));
}

#[test]
fn diff_no_change_returns_empty() {
    let mut mgr = SubscriptionManager::new();
    mgr.subscribe("x");

    let mut values = HashMap::new();
    values.insert("x".to_string(), EvaluatedValue::Float(10.0));

    // 初回: 変化あり
    mgr.diff_and_update(values.clone());

    // 2回目: 同じ値 → 変化なし
    let changed = mgr.diff_and_update(values);
    assert!(changed.is_empty());
}

#[test]
fn force_update_last_values_for_conclude() {
    let mut mgr = SubscriptionManager::new();
    let x_id = mgr.subscribe("x");

    // 初回配信: x=50.0
    let mut values = HashMap::new();
    values.insert("x".to_string(), EvaluatedValue::Float(50.0));
    mgr.diff_and_update(values);

    // Conclude: 最終値 x=100.0 で force_update
    let mut final_values = HashMap::new();
    final_values.insert(x_id, EvaluatedValue::Float(100.0));
    mgr.force_update_last_values(&final_values);

    // 次回 update: evaluate 結果なし → 凍結値(100.0) vs 前回配信(50.0) → 差分あり
    let changed = mgr.diff_and_update(HashMap::new());
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, x_id);
    assert_eq!(changed[0].1, EvaluatedValue::Float(100.0));
}

#[test]
fn frozen_variable_persists() {
    let mut mgr = SubscriptionManager::new();
    mgr.subscribe("x");

    // 1回目: x=50.0
    let mut values = HashMap::new();
    values.insert("x".to_string(), EvaluatedValue::Float(50.0));
    mgr.diff_and_update(values);

    // 2回目: evaluate 結果なし → 凍結値(50.0) vs 前回配信(50.0) → 変化なし
    let changed = mgr.diff_and_update(HashMap::new());
    assert!(changed.is_empty());
}

#[test]
fn frozen_value_preserved_after_unrelated_update() {
    let mut mgr = SubscriptionManager::new();
    let x_id = mgr.subscribe("x");
    let y_id = mgr.subscribe("y");

    // x=10.0, y=20.0
    let mut values = HashMap::new();
    values.insert("x".to_string(), EvaluatedValue::Float(10.0));
    values.insert("y".to_string(), EvaluatedValue::Float(20.0));
    mgr.diff_and_update(values);

    // y のみ変更、x は evaluate 結果なし → x の凍結値が消えないこと
    let mut values = HashMap::new();
    values.insert("y".to_string(), EvaluatedValue::Float(30.0));
    let changed = mgr.diff_and_update(values);

    // y のみ差分
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, y_id);
    assert_eq!(changed[0].1, EvaluatedValue::Float(30.0));

    // x の凍結値は保持されている（force_update で確認）
    let mut force_vals = HashMap::new();
    force_vals.insert(x_id, EvaluatedValue::Float(99.0));
    mgr.force_update_last_values(&force_vals);

    let changed = mgr.diff_and_update(HashMap::new());
    let x_change = changed.iter().find(|(id, _)| *id == x_id);
    assert!(x_change.is_some());
    assert_eq!(x_change.unwrap().1, EvaluatedValue::Float(99.0));
}

#[test]
fn convert_names_to_ids_basic() {
    let mut mgr = SubscriptionManager::new();
    let x_id = mgr.subscribe("x");
    let _y_id = mgr.subscribe("y");

    let mut name_values = HashMap::new();
    name_values.insert("x".to_string(), EvaluatedValue::Float(10.0));
    name_values.insert("z".to_string(), EvaluatedValue::Float(30.0)); // 未購読

    let id_values = mgr.convert_names_to_ids(&name_values);
    assert_eq!(id_values.len(), 1);
    assert_eq!(id_values[&x_id], EvaluatedValue::Float(10.0));
}

#[test]
fn object_ptr_eq_comparison() {
    let mut mgr = SubscriptionManager::new();
    let obj_id = mgr.subscribe("obj");

    let rc1 = Rc::new(DynamicValue::String("hello".to_string()));

    // 1回目: obj=rc1
    let mut values = HashMap::new();
    values.insert("obj".to_string(), EvaluatedValue::Object(rc1.clone()));
    let changed = mgr.diff_and_update(values);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, obj_id);

    // 2回目: 同じ Rc → ptr_eq → 変化なし
    let mut values = HashMap::new();
    values.insert("obj".to_string(), EvaluatedValue::Object(rc1.clone()));
    let changed = mgr.diff_and_update(values);
    assert!(changed.is_empty());

    // 3回目: 異なる Rc（内容同一）→ ptr_eq false → 変化あり
    let rc2 = Rc::new(DynamicValue::String("hello".to_string()));
    let mut values = HashMap::new();
    values.insert("obj".to_string(), EvaluatedValue::Object(rc2));
    let changed = mgr.diff_and_update(values);
    assert_eq!(changed.len(), 1);
}
