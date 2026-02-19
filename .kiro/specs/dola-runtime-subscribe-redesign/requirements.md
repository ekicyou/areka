# 要件定義書: dola-runtime-subscribe-redesign

## プロジェクト概要

DolaRuntime の変数購読・更新 API を再設計する。現行の `subscriber_id` を介した複数購読者モデルを廃止し、ランタイム単位の一元購読管理と `variable_id` による効率的な差分配信モデルへ移行する。

### 背景と動機

現行設計では `DolaRuntime::subscribe(subscriber_id, variable_name)` および `DolaRuntime::update(subscriber_id, current_time)` の双方に `subscriber_id: u64` を要求するが、以下の問題がある:

1. **不要な複雑性**: 実際の使用パターンでは単一の購読者しか存在しない（全テストで `subscriber_id=1` のみ使用）
2. **外部採番の負担**: `subscriber_id` の発行・管理責任が呼び出し側にあり、API 利用の摩擦となる
3. **変数名ベースの非効率**: `changes` が `Vec<(String, EvaluatedValue)>` で、毎フレームの文字列比較・アロケーションが発生しうる

### 設計方針

- `update()` から `subscriber_id` パラメータを除去し、全購読変数の差分を返却する
- 購読管理をランタイム単位に一元化（複数購読者モデルを廃止）
- `subscribe()` は `variable_id: i64` を返却し、変数名→ID のマッピングをランタイムが管理する
- `UpdateResult::changes` を `Vec<(i64, EvaluatedValue)>` に変更し、数値ID による効率的な差分配信を実現する

---

## 要件

### 要件 1: subscribe メソッドの再設計

**目的:** ランタイム利用者として、変数名を購読登録し、対応する数値IDを取得したい。これにより更新通知を数値IDベースで効率的に処理できるようにする。

#### 受入基準

1. When `DolaRuntime::subscribe(variable_name)` が呼び出された場合、DolaRuntime shall 新規の `variable_id: i64` を生成して返却する
2. When 既に購読済みの `variable_name` で `subscribe` が呼び出された場合、DolaRuntime shall 以前に割り当てた同一の `variable_id` を返却する（冪等性）
3. The DolaRuntime shall `variable_id` をランタイム内部で自動採番する（呼び出し側による ID 管理は不要）
4. When 指示書（ドキュメント）がまだ読み込まれていない状態で `subscribe` が呼び出された場合、DolaRuntime shall 正常に `variable_id` を返却する（事前購読対応、現行 Req 6.1 の維持）
5. The DolaRuntime shall `variable_id` を 0 から始まる連番で割り当てる

### 要件 2: ランタイム単位の一元購読管理

**目的:** ランタイム利用者として、購読者IDの管理から解放されたい。ランタイムは単一の購読状態のみ保持すればよい。

#### 受入基準

1. The DolaRuntime shall ランタイムインスタンスごとに単一の購読状態を内部管理する（現行の `HashMap<u64, SubscriberState>` による複数購読者モデルを廃止）
2. The DolaRuntime shall 全ての購読者向けメソッド（`subscribe`, `update`, `unsubscribe`, `unsubscribe_all`）から `subscriber_id` パラメータを除去する

> **注**: 各メソッドのシグネチャ詳細は要件 1（subscribe）、要件 3（update）、要件 5（unsubscribe のライフサイクル）にそれぞれ規定する。

### 要件 3: update メソッドの再設計

**目的:** ランタイム利用者として、購読者IDを指定せずにアニメーション状態を更新し、全購読変数の差分を受信したい。

#### 受入基準

1. When `DolaRuntime::update(current_time)` が呼び出された場合、DolaRuntime shall 全購読変数を評価し、前回値との差分を `UpdateResult` として返却する
2. The DolaRuntime shall `update` 呼び出し時にアニメーションの進行処理（finish deadline チェック、トリガー発火、ループ処理、自然終了検知）を引き続き実行する
3. When 購読変数の値に変化がない場合、DolaRuntime shall `UpdateResult::changes` を空の `Vec` として返却する

### 要件 4: UpdateResult::changes の型変更

**目的:** ランタイム利用者として、変数名の文字列ではなく数値IDで差分を受信し、効率的にUI更新を行いたい。

#### 受入基準

1. The DolaRuntime shall `UpdateResult::changes` の型を `Vec<(i64, EvaluatedValue)>` に変更する（`i64` は `variable_id`）
2. When `update` が差分を返却する場合、DolaRuntime shall 各エントリに `subscribe` 時に割り当てた `variable_id` を使用する
3. The DolaRuntime shall `variable_id` から `variable_name` への逆引き手段を提供する（デバッグ・ログ用途）

### 要件 5: variable_id のライフサイクル管理

**目的:** ランタイム利用者として、variable_id の一貫性と予測可能な振る舞いを期待する。

#### 受入基準

1. The DolaRuntime shall `variable_id` の割り当てをランタイムインスタンスのライフタイム内で一意に保つ
2. When `unsubscribe(variable_id)` が呼び出された場合、DolaRuntime shall その `variable_id` を再利用しない
3. When `unsubscribe` 後に同じ `variable_name` で再度 `subscribe` が呼び出された場合、DolaRuntime shall 新しい `variable_id` を割り当てる
4. When `unsubscribe_all` が呼び出された場合、DolaRuntime shall 全ての購読を解除し、以降の `subscribe` では新しい `variable_id` を割り当てる


### 要件 6: 既存機能の互換性維持

**目的:** ランタイム利用者として、購読モデルの変更後も、既存のアニメーション再生・停止・トリガー機能が正しく動作することを期待する。

#### 受入基準

1. The DolaRuntime shall 指示書読み込み (`load_document`)、開始 (`start`)、一時停止 (`pause`)、再開 (`resume`)、停止 (`cancel`) の各メソッドは既存の動作を維持する
2. When ストーリーボードが終了（Conclude）した場合、DolaRuntime shall 購読変数の最終値を `update` の差分に含める（`force_update_last_values` 相当の動作維持。ただしAPIはvariable_id（i64）ベースのHashMapで値を強制設定する設計判断とする）
3. While アニメーションが再生中、DolaRuntime shall トリガー機能が正しく動作する（`TriggerResult` の既存仕様維持）
4. When 指示書が差し替え (`load_document` 再呼び出し) された場合、DolaRuntime shall 購読状態（`variable_id` マッピング）を維持する

### 要件 7: テストカバレッジ

**目的:** 開発者として、再設計された API の品質を検証可能にする。

#### 受入基準

1. The DolaRuntime shall 全既存テスト（runtime_facade_test, conflict_resolution_test, loop_offset_test 等）を新 API シグネチャに対応させ、パスする
2. The DolaRuntime shall `subscribe` の冪等性（同一変数名→同一ID）を検証するテストを含む
3. The DolaRuntime shall `unsubscribe(variable_id)` 後の再 `subscribe` で新しい `variable_id` が割り当てられることを検証するテストを含む
4. The DolaRuntime shall `variable_id` による差分配信が正しく動作することを検証するテストを含む
5. The DolaRuntime shall `get_variable_name(variable_id)` の逆引きが正しく動作することを検証するテストを含む
