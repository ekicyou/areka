//! ランタイム単位の変数購読管理と差分検出。
//!
//! variable_id ベースの一元購読管理を提供する。

use std::collections::{BTreeMap, HashMap};

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
    /// 購読中の variable_id → 変数名（unsubscribe 時に削除。id の昇順で回すので差分の並びが決まる）
    id_to_name: BTreeMap<i64, String>,

    // --- 購読状態 ---
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
            id_to_name: BTreeMap::new(),
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
        id
    }

    /// 変数購読解除（ID指定）。
    /// 存在しない or 既に解除済みの ID → Err(RuntimeError::InvalidVariableId)。
    pub fn unsubscribe(&mut self, variable_id: i64) -> Result<(), RuntimeError> {
        let Some(name) = self.id_to_name.remove(&variable_id) else {
            return Err(RuntimeError::InvalidVariableId(variable_id));
        };

        // name_to_id から削除（再 subscribe 時に新IDを割り当てるため）
        self.name_to_id.remove(&name);

        // last_values, last_sent_values から削除
        self.last_values.remove(&variable_id);
        self.last_sent_values.remove(&variable_id);

        Ok(())
    }

    /// 購読中変数名のリストを取得。
    #[cfg(test)]
    pub fn get_subscribed_variable_names(&self) -> Vec<String> {
        self.id_to_name.values().cloned().collect()
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

    /// 購読中の変数を id の昇順に `evaluate` で評価し、変化した変数のみを返す（id ベース）。
    /// 同時に last_values を更新する。
    ///
    /// `evaluate` が `None` の購読変数は凍結値（last_values）を現在値とする。
    pub fn diff_and_update(
        &mut self,
        mut evaluate: impl FnMut(&str) -> Option<EvaluatedValue>,
    ) -> Vec<(i64, EvaluatedValue)> {
        let mut changed = Vec::new();

        for (&id, name) in &self.id_to_name {
            // evaluate 結果があれば凍結値を更新し、現在値 = 凍結値
            if let Some(v) = evaluate(name) {
                self.last_values.insert(id, v);
            }
            // 値が存在しない（未定義変数の購読）→ 無視
            let Some(current) = self.last_values.get(&id) else {
                continue;
            };

            // 前回配信値と比較（初回は常に変化あり）
            if self.last_sent_values.get(&id) != Some(current) {
                changed.push((id, current.clone()));
                self.last_sent_values.insert(id, current.clone());
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
            // name_to_id は購読中の名前だけを持つ
            if let Some(&id) = self.name_to_id.get(name) {
                result.insert(id, val.clone());
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
            if self.id_to_name.contains_key(id) {
                self.last_values.insert(*id, val.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests;
