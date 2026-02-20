//! ランタイム単位の変数購読管理と差分検出。
//!
//! variable_id ベースの一元購読管理を提供する。

use std::collections::{HashMap, HashSet};

use super::types::{EvaluatedValue, RuntimeError};

/// ランタイム単位の変数購読管理と差分検出を行う内部コンポーネント。
///
/// 購読登録は指示書受信前でも受付可能（Req 1.4）。
/// variable_id は 0-origin のモノトニック増加カウンタで採番し、再利用しない。
pub(crate) struct SubscriptionManager {
    // --- ID 管理 ---
    /// 次に割り当てる variable_id（0-origin、モノトニック増加）
    next_id: i64,
    /// 変数名 → variable_id（購読中のエントリのみ保持。unsubscribe 時に削除）
    name_to_id: HashMap<String, i64>,
    /// variable_id → 変数名（全割り当て済みIDを含む、逆引き用。削除しない）
    id_to_name: HashMap<i64, String>,

    // --- 購読状態 ---
    /// 現在購読中の variable_id セット
    subscribed_ids: HashSet<i64>,
    /// 凍結値（id ベース）
    last_values: HashMap<i64, EvaluatedValue>,
    /// 前回配信値（id ベース、差分比較用）
    last_sent_values: HashMap<i64, EvaluatedValue>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            name_to_id: HashMap::new(),
            id_to_name: HashMap::new(),
            subscribed_ids: HashSet::new(),
            last_values: HashMap::new(),
            last_sent_values: HashMap::new(),
        }
    }

    /// 変数購読登録。冪等（同一名→同一ID返却）。
    /// unsubscribe 済みの名前は新規IDを割り当てる。
    pub fn subscribe(&mut self, variable_name: &str) -> i64 {
        // 既に購読中なら同一IDを返却（冪等）
        if let Some(&id) = self.name_to_id.get(variable_name) {
            return id;
        }

        // 新規ID割り当て
        let id = self.next_id;
        self.next_id += 1;
        self.name_to_id.insert(variable_name.to_string(), id);
        self.id_to_name.insert(id, variable_name.to_string());
        self.subscribed_ids.insert(id);
        id
    }

    /// 変数購読解除（ID指定）。
    /// 存在しない or 既に解除済みの ID → Err(RuntimeError::InvalidVariableId)。
    pub fn unsubscribe(&mut self, variable_id: i64) -> Result<(), RuntimeError> {
        if !self.subscribed_ids.remove(&variable_id) {
            return Err(RuntimeError::InvalidVariableId(variable_id));
        }

        // name_to_id から削除（再 subscribe 時に新IDを割り当てるため）
        if let Some(name) = self.id_to_name.get(&variable_id) {
            self.name_to_id.remove(name);
        }

        // last_values, last_sent_values から削除
        self.last_values.remove(&variable_id);
        self.last_sent_values.remove(&variable_id);

        // id_to_name は維持（逆引き可能性を残す）

        Ok(())
    }

    /// 全購読解除。
    pub fn unsubscribe_all(&mut self) {
        // name_to_id を全削除
        self.name_to_id.clear();
        // subscribed_ids を全削除
        self.subscribed_ids.clear();
        // last_values, last_sent_values を全削除
        self.last_values.clear();
        self.last_sent_values.clear();
        // id_to_name は維持（逆引き用）
    }

    /// 購読中変数名のリストを取得（evaluate 用）。
    pub fn get_subscribed_variable_names(&self) -> Vec<String> {
        self.subscribed_ids
            .iter()
            .filter_map(|id| self.id_to_name.get(id).cloned())
            .collect()
    }

    /// variable_id → variable_name 逆引き。
    /// 存在しない ID → Err(RuntimeError::InvalidVariableId)。
    #[cfg(test)]
    pub fn get_variable_name(&self, variable_id: i64) -> Result<&str, RuntimeError> {
        self.id_to_name
            .get(&variable_id)
            .map(|s| s.as_str())
            .ok_or(RuntimeError::InvalidVariableId(variable_id))
    }

    /// 値を比較し、変化した変数のみを返す（id ベース）。同時に last_values を更新。
    ///
    /// `values` は evaluate 結果（変数名ベース）。
    /// 内部で name→id 変換を行い、id ベースの差分を返却する。
    /// タイムテーブルにエントリがない購読変数は凍結値（last_values）を現在値とする。
    pub fn diff_and_update(
        &mut self,
        values: HashMap<String, EvaluatedValue>,
    ) -> Vec<(i64, EvaluatedValue)> {
        let mut changed = Vec::new();

        let subscribed_ids: Vec<i64> = self.subscribed_ids.iter().copied().collect();

        for id in subscribed_ids {
            let var_name = match self.id_to_name.get(&id) {
                Some(name) => name.clone(),
                None => continue,
            };

            // 現在値 = evaluate 結果 or 凍結値
            let current = values.get(&var_name).or_else(|| self.last_values.get(&id));

            let current = match current {
                Some(v) => v.clone(),
                None => continue, // 値が存在しない（未定義変数の購読 → 無視）
            };

            // 前回配信値と比較
            let is_changed = match self.last_sent_values.get(&id) {
                Some(last_sent) => last_sent != &current,
                None => true, // 初回は常に変化あり
            };

            if is_changed {
                changed.push((id, current.clone()));
                self.last_sent_values.insert(id, current.clone());
            }

            // evaluate 結果があれば凍結値も更新
            if values.contains_key(&var_name) {
                self.last_values.insert(id, current);
            }
        }

        changed
    }

    /// 変数名ベースの値マップを id ベースに変換。
    /// 購読中の変数のみ変換（未購読の名前は無視）。
    pub fn convert_names_to_ids(
        &self,
        name_values: &HashMap<String, EvaluatedValue>,
    ) -> HashMap<i64, EvaluatedValue> {
        let mut result = HashMap::new();
        for (name, val) in name_values {
            if let Some(&id) = self.name_to_id.get(name) {
                if self.subscribed_ids.contains(&id) {
                    result.insert(id, val.clone());
                }
            }
        }
        result
    }

    /// Conclude 用: 最終値で last_values を強制更新（id ベース）。
    /// 購読中の id のみ更新（未購読の id は無視）。
    ///
    /// Conclude 後の次回 update で diff_and_update が最終値と
    /// 前回配信値を比較し、差分として配信する。
    pub fn force_update_last_values(&mut self, values: &HashMap<i64, EvaluatedValue>) {
        for (id, val) in values {
            if self.subscribed_ids.contains(id) {
                self.last_values.insert(*id, val.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
