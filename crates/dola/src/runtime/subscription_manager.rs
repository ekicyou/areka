//! 購読者ごとの変数購読状態と差分検出。

use std::collections::{HashMap, HashSet};

use super::types::EvaluatedValue;

/// 購読者ごとの状態。
pub(crate) struct SubscriberState {
    /// 購読中の変数名セット
    pub variables: HashSet<String>,
    /// 凍結値（タイムテーブルにエントリがない変数の現在値保持用）
    pub last_values: HashMap<String, EvaluatedValue>,
    /// 前回配信値（差分比較用）
    pub last_sent_values: HashMap<String, EvaluatedValue>,
}

impl SubscriberState {
    fn new() -> Self {
        Self {
            variables: HashSet::new(),
            last_values: HashMap::new(),
            last_sent_values: HashMap::new(),
        }
    }
}

/// 購読者ごとの変数購読状態と差分検出を行う内部コンポーネント。
///
/// 購読登録は指示書受信前でも受付可能（Req 6.1）。
pub(crate) struct SubscriptionManager {
    subscribers: HashMap<u64, SubscriberState>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }

    /// 購読登録（指示書受信前でも可）。
    pub fn subscribe(&mut self, subscriber_id: u64, variable_name: &str) {
        self.subscribers
            .entry(subscriber_id)
            .or_insert_with(SubscriberState::new)
            .variables
            .insert(variable_name.to_string());
    }

    /// 購読解除。
    pub fn unsubscribe(&mut self, subscriber_id: u64, variable_name: &str) {
        if let Some(state) = self.subscribers.get_mut(&subscriber_id) {
            state.variables.remove(variable_name);
        }
    }

    /// 全購読解除（Drop 対応）。
    pub fn unsubscribe_all(&mut self, subscriber_id: u64) {
        self.subscribers.remove(&subscriber_id);
    }

    /// 購読中変数名のリストを取得。
    pub fn get_subscribed_variables(&self, subscriber_id: u64) -> Vec<String> {
        match self.subscribers.get(&subscriber_id) {
            Some(state) => state.variables.iter().cloned().collect(),
            None => Vec::new(),
        }
    }

    /// 値を比較し、変化した変数のみを返す。同時に last_values を更新。
    ///
    /// `values` は evaluate 結果（タイムテーブルにエントリがある変数のみ含む）。
    /// タイムテーブルにエントリがない購読変数は凍結値（last_values）を現在値とする。
    pub fn diff_and_update(
        &mut self,
        subscriber_id: u64,
        values: HashMap<String, EvaluatedValue>,
    ) -> Vec<(String, EvaluatedValue)> {
        let state = match self.subscribers.get_mut(&subscriber_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut changed = Vec::new();

        for var_name in state.variables.clone() {
            // 現在値 = evaluate 結果 or 凍結値
            let current = values
                .get(&var_name)
                .or_else(|| state.last_values.get(&var_name));

            let current = match current {
                Some(v) => v.clone(),
                None => continue, // 値が存在しない（未定義変数の購読 → 無視）
            };

            // 前回配信値と比較
            let is_changed = match state.last_sent_values.get(&var_name) {
                Some(last_sent) => last_sent != &current,
                None => true, // 初回は常に変化あり
            };

            if is_changed {
                changed.push((var_name.clone(), current.clone()));
                state
                    .last_sent_values
                    .insert(var_name.clone(), current.clone());
            }

            // evaluate 結果があれば凍結値も更新
            if values.contains_key(&var_name) {
                state
                    .last_values
                    .insert(var_name.clone(), current);
            }
        }

        changed
    }

    /// Conclude 用: 最終値で last_values を強制更新。
    ///
    /// Conclude 後の次回 update で diff_and_update が最終値と
    /// 前回配信値を比較し、差分として配信する。
    pub fn force_update_last_values(&mut self, values: &HashMap<String, EvaluatedValue>) {
        for state in self.subscribers.values_mut() {
            for (var_name, val) in values {
                if state.variables.contains(var_name) {
                    state.last_values.insert(var_name.clone(), val.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use crate::value::DynamicValue;

    #[test]
    fn subscribe_and_get_variables() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");
        mgr.subscribe(1, "y");
        let vars = mgr.get_subscribed_variables(1);
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&"x".to_string()));
        assert!(vars.contains(&"y".to_string()));
    }

    #[test]
    fn unsubscribe() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");
        mgr.subscribe(1, "y");
        mgr.unsubscribe(1, "x");
        let vars = mgr.get_subscribed_variables(1);
        assert_eq!(vars.len(), 1);
        assert!(vars.contains(&"y".to_string()));
    }

    #[test]
    fn unsubscribe_all() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");
        mgr.subscribe(1, "y");
        mgr.unsubscribe_all(1);
        assert!(mgr.get_subscribed_variables(1).is_empty());
    }

    #[test]
    fn diff_detects_change() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");

        let mut values = HashMap::new();
        values.insert("x".to_string(), EvaluatedValue::Float(10.0));

        let changed = mgr.diff_and_update(1, values);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].0, "x");
        assert_eq!(changed[0].1, EvaluatedValue::Float(10.0));
    }

    #[test]
    fn diff_no_change_returns_empty() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");

        let mut values = HashMap::new();
        values.insert("x".to_string(), EvaluatedValue::Float(10.0));

        // 初回: 変化あり
        mgr.diff_and_update(1, values.clone());

        // 2回目: 同じ値 → 変化なし
        let changed = mgr.diff_and_update(1, values);
        assert!(changed.is_empty());
    }

    #[test]
    fn force_update_last_values_for_conclude() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");

        // 初回配信: x=50.0
        let mut values = HashMap::new();
        values.insert("x".to_string(), EvaluatedValue::Float(50.0));
        mgr.diff_and_update(1, values);

        // Conclude: 最終値 x=100.0 で force_update
        let mut final_values = HashMap::new();
        final_values.insert("x".to_string(), EvaluatedValue::Float(100.0));
        mgr.force_update_last_values(&final_values);

        // 次回 update: evaluate 結果なし → 凍結値(100.0) vs 前回配信(50.0) → 差分あり
        let changed = mgr.diff_and_update(1, HashMap::new());
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].1, EvaluatedValue::Float(100.0));
    }

    #[test]
    fn subscribe_before_document() {
        // 指示書受信前の購読登録は受け付けられる
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");
        let vars = mgr.get_subscribed_variables(1);
        assert_eq!(vars.len(), 1);

        // evaluate 結果なし → 変化なし（値が存在しない）
        let changed = mgr.diff_and_update(1, HashMap::new());
        assert!(changed.is_empty());
    }

    #[test]
    fn frozen_variable_persists() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "x");

        // 1回目: x=50.0
        let mut values = HashMap::new();
        values.insert("x".to_string(), EvaluatedValue::Float(50.0));
        mgr.diff_and_update(1, values);

        // 2回目: evaluate 結果なし（タイムテーブルからエントリが消えた）
        // → 凍結値(50.0) vs 前回配信(50.0) → 変化なし
        let changed = mgr.diff_and_update(1, HashMap::new());
        assert!(changed.is_empty());
    }

    #[test]
    fn object_ptr_eq_comparison() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(1, "obj");

        let rc1 = Rc::new(DynamicValue::String("hello".to_string()));

        // 1回目: obj=rc1
        let mut values = HashMap::new();
        values.insert("obj".to_string(), EvaluatedValue::Object(rc1.clone()));
        let changed = mgr.diff_and_update(1, values);
        assert_eq!(changed.len(), 1);

        // 2回目: 同じ Rc → ptr_eq → 変化なし
        let mut values = HashMap::new();
        values.insert("obj".to_string(), EvaluatedValue::Object(rc1.clone()));
        let changed = mgr.diff_and_update(1, values);
        assert!(changed.is_empty());

        // 3回目: 異なる Rc（内容同一）→ ptr_eq false → 変化あり
        let rc2 = Rc::new(DynamicValue::String("hello".to_string()));
        let mut values = HashMap::new();
        values.insert("obj".to_string(), EvaluatedValue::Object(rc2));
        let changed = mgr.diff_and_update(1, values);
        assert_eq!(changed.len(), 1);
    }
}
