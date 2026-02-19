# ギャップ分析: dola-runtime-subscribe-redesign

## 分析サマリー

- **スコープ**: `SubscriptionManager` の構造改修、`DolaRuntime` facade のAPI変更、`conflict_resolver` の引数修正、全5テストファイル（46箇所の `subscribe` + 112箇所の `update`）の移行
- **外部依存**: DolaRuntime を使用する外部クレート（areka, wintf）は現時点で存在しない → **破壊的変更のリスクは低い**
- **最大の変更面**: テストコードの機械的な書き換え（量は多いが複雑性は低い）
- **設計上の要注意点**: `force_update_last_values` は内部的に変数名ベースで動作しており、`variable_id` マッピングとの整合が必要
- **推定規模**: M（3〜7日）、リスク: Medium

---

## 1. 現状の資産マップ

### 変更対象ファイル

| ファイル                          | 役割              | 変更内容                                                                                         |
| --------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------ |
| `runtime/subscription_manager.rs` | 購読管理の中核    | **大幅改修**: `HashMap<u64, SubscriberState>` → フラット構造、`variable_id` 採番・マッピング追加 |
| `runtime/facade.rs`               | 公開API（facade） | **API変更**: `subscribe`, `update`, `unsubscribe`, `unsubscribe_all` のシグネチャ変更            |
| `runtime/types.rs`                | 公開型定義        | **型変更**: `UpdateResult::changes` を `Vec<(i64, EvaluatedValue)>` へ                           |
| `runtime/conflict_resolver.rs`    | 競合解決          | **引数型維持**: `force_update_last_values` の内部I/Fが変わる場合のみ                             |
| `runtime/mod.rs`                  | モジュール公開    | 変更なし or 最小                                                                                 |
| `lib.rs`                          | クレート公開API   | 変更なし（re-exports は型レベルのみ）                                                            |

### テストファイル（影響範囲）

| テストファイル                | `subscribe` 呼出数 | `update` 呼出数 | `changes` アクセスパターン                             |
| ----------------------------- | ------------------ | --------------- | ------------------------------------------------------ |
| `runtime_facade_test.rs`      | 多数               | 多数            | `diff[0].0 == "name"`, `.find(\|(k, _)\| k == "name")` |
| `conflict_resolution_test.rs` | 約20箇所           | 約15箇所        | `.find(\|(name, _)\| name == "x")`                     |
| `loop_integration_test.rs`    | 数箇所             | 数箇所          | `diff[0].0`, `.find()`                                 |
| `loop_offset_test.rs`         | 数箇所             | 数箇所          | 同上                                                   |
| `trigger_test.rs`             | 数箇所             | 数箇所          | `.find(\|(name, _)\| name == "opacity")`               |

### SubscriptionManager ユニットテスト（内部）

`subscription_manager.rs` 内に7つのユニットテストが存在（`#[cfg(test)]` モジュール）。すべて改修が必要。

---

## 2. 要件→資産マッピング & ギャップ

### 要件 1: subscribe の再設計（`variable_name → variable_id`）

| 技術要素             | 現状                                                        | ギャップ                                                    |
| -------------------- | ----------------------------------------------------------- | ----------------------------------------------------------- |
| subscribe シグネチャ | `subscribe(subscriber_id: u64, variable_name: &str)` → `()` | **Missing**: 戻り値 `i64` が必要                            |
| variable_id 採番     | 存在しない                                                  | **Missing**: カウンタ + `HashMap<String, i64>` の追加が必要 |
| 冪等性               | `HashSet::insert` で自然に冪等                              | 既存概念を拡張可能（名前→ID マップの lookup）               |
| 事前購読             | 対応済み（Req 6.1）                                         | 維持可能（ID採番は指示書とは独立）                          |

### 要件 2: subscriber_id の廃止

| 技術要素                 | 現状                                                                    | ギャップ                                                                    |
| ------------------------ | ----------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| SubscriptionManager 構造 | `HashMap<u64, SubscriberState>`                                         | **Constraint**: フラット化が必要（単一の状態に統合）                        |
| facade API               | 3メソッドに `subscriber_id` 引数                                        | **Missing**: パラメータ削除、`unsubscribe` は `variable_id: i64` 引数に変更 |
| conflict_resolver 連携   | `&mut SubscriptionManager` を受け取り `force_update_last_values` を呼ぶ | 内部メソッドI/F変更の伝播が必要                                             |

### 要件 3: update の再設計

| 技術要素          | 現状                                                            | ギャップ                                   |
| ----------------- | --------------------------------------------------------------- | ------------------------------------------ |
| update シグネチャ | `update(subscriber_id: u64, current_time: f64)`                 | **Missing**: `subscriber_id` 除去          |
| Step 3: 変数評価  | `get_subscribed_variables(subscriber_id)` → 名前リスト          | 改修箇所: 単一購読状態からの取得に変更     |
| Step 4: 差分検出  | `diff_and_update(subscriber_id, values)` → `Vec<(String, ...)>` | 改修箇所: `variable_id` ベースの出力に変更 |

### 要件 4: changes の型変更

| 技術要素                | 現状                                                        | ギャップ                                                |
| ----------------------- | ----------------------------------------------------------- | ------------------------------------------------------- |
| `UpdateResult::changes` | `Vec<(String, EvaluatedValue)>`                             | **型変更**: `Vec<(i64, EvaluatedValue)>`                |
| テストのアサーション    | `.find(\|(name, _)\| name == "x")` パターンが全テストで使用 | **テスト移行**: 全箇所を `variable_id` ベースに更新必要 |

### 要件 5: variable_id ライフサイクル

| 技術要素                   | 現状     | ギャップ                                    |
| -------------------------- | -------- | ------------------------------------------- |
| ID一意性                   | 概念なし | **Missing**: モノトニックカウンタの実装     |
| unsubscribe後の再利用禁止  | 概念なし | **Missing**: 削除されたIDを再利用しない設計 |
| 逆引き（name→id, id→name） | 概念なし | **Missing**: 双方向マッピングの追加         |

### 要件 6: 互換性維持

| 技術要素                       | 現状                                | ギャップ                                                         |
| ------------------------------ | ----------------------------------- | ---------------------------------------------------------------- |
| load/start/pause/resume/cancel | 購読管理に依存しない                | ギャップなし（影響を受けない）                                   |
| `force_update_last_values`     | 変数名ベースで `last_values` を更新 | **注意**: 内部的に変数名で動作するため、name→id 変換が内部で必要 |
| トリガー                       | 購読管理に依存しない                | ギャップなし                                                     |
| 指示書差し替え                 | 購読状態は独立して維持              | ギャップなし（`variable_id` マッピングも指示書とは独立）         |

### 要件 7: テストカバレッジ

| 技術要素       | 現状                           | ギャップ                                                    |
| -------------- | ------------------------------ | ----------------------------------------------------------- |
| 既存テスト移行 | 46 subscribe + 112 update 呼出 | **大量の機械的変更**（パターンは統一的）                    |
| 新規テスト     | なし                           | **Missing**: 冪等性、unsubscribe→再subscribe、逆引き テスト |

---

## 3. 実装アプローチの選択肢

### Option A: SubscriptionManager の段階的改修（推奨）

**概要**: 既存の `SubscriptionManager` を直接改修し、フラット構造に置換する。

**変更対象**:
1. `SubscriptionManager`: `HashMap<u64, SubscriberState>` → フラット `SubscriptionState` + ID管理
2. `facade.rs`: 公開APIシグネチャ変更
3. `types.rs`: `UpdateResult::changes` の型変更
4. `conflict_resolver.rs`: `force_update_last_values` の内部I/F変更
5. テスト: 全5ファイルの機械的書き換え

**フェーズ**:
1. **SubscriptionManager 内部改修**: フラット構造 + variable_id 管理ロジック
2. **facade API 変更**: シグネチャ変更 + 内部委譲の調整
3. **types.rs 型変更**: `UpdateResult::changes` の型更新
4. **テスト移行**: 全テストを新APIに対応

**トレードオフ**:
- ✅ 既存ファイル構成を維持、新規ファイル不要
- ✅ SubscriptionManager の責務境界が明確（ID管理も含む）
- ✅ conflict_resolver への影響が最小（`force_update_last_values` 内部のみ）
- ❌ 一度にコンパイルが通らなくなる（段階的コンパイル確認が困難）

### Option B: 新 SubscriptionManager を並存させる

**概要**: 新しい `VariableSubscriptionManager` を作成し、旧 `SubscriptionManager` と並存させた後、段階的に移行。

**フェーズ**:
1. 新 `VariableSubscriptionManager` を作成（新ロジック）
2. facade に新マネージャを統合、旧マネージャへの委譲を残す
3. テストを新APIに移行
4. 旧マネージャを削除

**トレードオフ**:
- ✅ 段階的に移行でき、常にコンパイル可能
- ✅ ロールバックが容易
- ❌ 一時的に2つの管理コンポーネントが存在する複雑さ
- ❌ 最終的に旧コードを削除する手間

### Option C: SubscriptionManager の責務分割

**概要**: ID管理（`VariableIdRegistry`）と差分検出（`DiffTracker`）を別構造に分離。

**トレードオフ**:
- ✅ 単一責任原則に沿う
- ✅ 各コンポーネントが独立テスト可能
- ❌ ファイル数増加・ナビゲーション負荷
- ❌ 現状の規模感では過剰設計の可能性

---

## 4. 複雑性とリスクの評価

### 実装規模: **M（3〜7日）**

**根拠**:
- 主要な改修対象は3ファイル（subscription_manager, facade, types）
- テスト移行は量が多い（158箇所）が、パターンが統一的で機械的に適用可能
- 新ロジック（ID採番・マッピング）はシンプルなカウンタ + HashMap

### リスク: **Medium**

**根拠**:
- `force_update_last_values` はフラット化後に単一購読状態への適用に単純化されるが、変数名→ID の変換ロジックが正しいことの検証が必要
- テスト移行後のアサーション変更（名前ベース→IDベース）で、テストの可読性が低下するリスク
  - → **対策**: テスト用ヘルパー関数（`find_change_by_name`）の導入を推奨
- `conflict_resolver` の4戦略（Cancel/Conclude/Trim/Compress）すべてが `force_update_last_values` を使用 → 全パスのテスト確認が必要

---

## 5. 設計フェーズへの申し送り事項

### 主要な設計決定が必要な項目

1. **SubscriptionState の内部構造**: `HashMap<String, i64>` (name→id) + `HashMap<i64, String>` (id→name) の双方向マップか、単方向で十分か
2. **`force_update_last_values` のI/F**: 引数はvariable_idベース（`HashMap<i64, EvaluatedValue>`）に統一する（設計判断済み）
3. **テストヘルパーの設計**: `variable_id` ベースのテストで可読性を維持する手法（`get_variable_name` を使った逆引きヘルパー等）
4. **`diff_and_update` 内部の変数名→ID変換タイミング**: evaluate結果（変数名ベース）をID変換するのは diff_and_update 内部か facade か

### Research Needed

- なし（既存技術スタック内で完結するため、外部調査不要）
