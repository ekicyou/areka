# リサーチ & 設計判断ログ: dola-runtime-subscribe-redesign

## サマリー
- **機能**: dola-runtime-subscribe-redesign
- **ディスカバリー範囲**: Extension（既存システムの改修）
- **主要な発見**:
  - SubscriptionManager は `HashMap<u64, SubscriberState>` による複数購読者モデルだが、全テストで subscriber_id=1 のみ使用
  - `force_update_last_values` は conflict_resolver の4戦略（Cancel/Conclude/Trim/Compress）すべてで使用される
  - テスト移行は158箇所と量が多いが、パターンが統一的で機械的に適用可能

## リサーチログ

### 拡張ポイント分析

- **コンテキスト**: SubscriptionManager のフラット化と variable_id 導入に伴う影響範囲の特定
- **調査対象**: runtime/ 配下の全ファイル、テスト5ファイル
- **発見**:
  - `subscription_manager.rs`: 唯一の改修核。`SubscriberState` 構造体と `SubscriptionManager` 構造体の両方を再設計
  - `facade.rs`: 公開API（subscribe, unsubscribe, unsubscribe_all, update）の4メソッドがシグネチャ変更対象
  - `types.rs`: `UpdateResult::changes` の型変更（`Vec<(String, EvaluatedValue)>` → `Vec<(i64, EvaluatedValue)>`）
  - `conflict_resolver.rs`: 4つの apply 関数すべてが `force_update_last_values` を呼ぶ。引数型変更の影響を受ける
  - `timeline_manager.rs`: evaluate 系メソッドは変数名ベースの `HashMap<String, EvaluatedValue>` を返却。name→id 変換の境界が必要
- **含意**: 変数名→ID変換の責務境界を明確にする必要がある。evaluate結果（変数名ベース）からID変換への変換は SubscriptionManager 内部で行うのが最も自然

### force_update_last_values の呼び出しフロー分析

- **コンテキスト**: idベースAPI統一に伴い、呼び出し元すべての影響を把握
- **調査対象**: facade.rs, conflict_resolver.rs
- **発見**:
  - `facade.rs::conclude_internal()`: `timeline_manager.collect_final_values(group_id)` → `HashMap<String, EvaluatedValue>` を取得し、そのまま `force_update_last_values` に渡す
  - `conflict_resolver.rs::apply_cancel()`: `timeline_manager.evaluate_all_for_group()` → 同上
  - `conflict_resolver.rs::apply_conclude()`: `timeline_manager.collect_current_segment_final_values()` → 同上
  - `conflict_resolver.rs::apply_trim()`: `timeline_manager.evaluate_all_for_group()` → 同上
  - `conflict_resolver.rs::apply_compress()`: `timeline_manager.collect_final_values()` → 同上
  - すべての呼び出し元が `HashMap<String, EvaluatedValue>` を渡している
- **含意**: `force_update_last_values` のI/Fをidベースに変更する場合、呼び出し元で name→id 変換が必要。ただし SubscriptionManager が双方向マップを持つため、name→id 変換メソッドを提供し、呼び出し元（facade/conflict_resolver）で変換するか、SubscriptionManager 内部に name ベースの受付メソッドを残すかの選択肢がある

### 既存パターンの確認

- **コンテキスト**: Rust 2024 Edition、dola クレートの設計パターン確認
- **調査対象**: runtime/mod.rs, facade.rs
- **発見**:
  - Facade パターンで公開APIを一元管理
  - 内部コンポーネント（SubscriptionManager, TimelineManager, InstanceManager）は `pub(crate)` で非公開
  - エラー型は `RuntimeError` enum で統一
  - Result 型は `Result<T, RuntimeError>` パターン
  - 現行 subscribe/unsubscribe は戻り値なし（`()`）
- **含意**: 新APIも Facade パターンを維持。subscribe の戻り値を `i64` に変更、unsubscribe/get_variable_name は `Result` 返却（設計判断済み）

## アーキテクチャパターン評価

| オプション  | 概要                                      | 強み                           | リスク/制約            | 備考              |
| ----------- | ----------------------------------------- | ------------------------------ | ---------------------- | ----------------- |
| A: 直接改修 | 既存 SubscriptionManager を直接フラット化 | ファイル構成維持、責務境界明確 | 一時的にコンパイル不可 | gap-analysis 推奨 |
| B: 並存移行 | 新旧 Manager を並存                       | 段階的移行、常にコンパイル可能 | 一時的な複雑さ         | ロールバック容易  |
| C: 責務分割 | ID管理と差分検出を分離                    | 単一責任原則                   | 過剰設計の可能性       | 現規模では不要    |

## 設計判断

### 判断: SubscriptionState の内部構造

- **コンテキスト**: variable_id の双方向検索が必要
- **検討した選択肢**:
  1. 単方向マップ（name→id のみ）+ 逆引き時に全探索
  2. 双方向マップ（name→id + id→name）
- **選択**: 双方向マップ（`HashMap<String, i64>` + `HashMap<i64, String>`）
- **理由**: subscribe（name→id）と get_variable_name（id→name）の両方が O(1) で必要。ランタイム内部でも force_update_last_values 等で両方向の変換が発生する
- **トレードオフ**: メモリ使用量が若干増加するが、購読変数の数は通常少数（数十程度）のため無視できる
- **フォローアップ**: なし

### 判断: force_update_last_values のI/F

- **コンテキスト**: 内部APIのidベース統一
- **検討した選択肢**:
  1. 変数名ベース維持（呼び出し側は従来通り、内部で name→id 変換）
  2. variable_id ベースに統一（呼び出し側も id で渡す）
- **選択**: variable_id ベースに統一（`HashMap<i64, EvaluatedValue>`）
- **理由**: API一貫性。ただし呼び出し元（facade/conflict_resolver）が timeline_manager から取得する値は変数名ベースのため、変換メソッドが必要
- **トレードオフ**: 呼び出し元に変換コードが増えるが、一貫性が向上
- **フォローアップ**: SubscriptionManager に `convert_name_values_to_id` 等の変換ヘルパーを提供することを設計で検討

### 判断: テストヘルパー関数

- **コンテキスト**: idベーステストの可読性
- **検討した選択肢**:
  1. find_change_by_name 等のヘルパーを導入
  2. idベースでテストを直接記述
- **選択**: ヘルパー関数は不要。idベースでテストを直接記述
- **理由**: subscribe の戻り値を変数に保持すれば、テスト内で `let x_id = rt.subscribe("x"); ... assert!(changes.iter().any(|(id, _)| *id == x_id))` と書ける。十分に可読
- **トレードオフ**: なし
- **フォローアップ**: なし

## リスク & 軽減策
- timeline_manager の evaluate 系メソッドが変数名ベースを返却するため、name→id 変換の境界で不整合が起きる可能性 → SubscriptionManager に変換メソッドを集約し、単一責任で管理
- テスト移行量が多い（158箇所）→ パターンが統一的なため、機械的置換で対応可能
- conflict_resolver の4戦略すべてが force_update_last_values を使用 → 全パスのテスト確認が必要

## 参考資料
- 既存コードベース: `crates/dola/src/runtime/` 配下
- gap-analysis.md: `.kiro/specs/dola-runtime-subscribe-redesign/gap-analysis.md`
