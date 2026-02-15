# Requirements Document — dola-runtime-3-facade

## Introduction

本ドキュメントは dola ランタイムエンジンの本体を定義する子仕様 `dola-runtime-facade` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 1（指示書管理）、Req 2（Start）、Req 3（制御コマンド）、Req 4（購読管理）、Req 5（差分配信）、Req 6（タイムテーブル管理）、Req 8（状態遷移の適用）、Req 9（同時再生）を子仕様の粒度に詳細化する。

本子仕様は Tier 2 に位置し、`dola-runtime-core-types`（Tier 1）に依存する。競合解決（Req 7）とループ再生（Req 12）は Tier 3 `dola-runtime-conflict-loop` の責務であり、本仕様では対応しない。Tier 2 単独での暫定動作として、同一変数への複数 group_id エントリは最新 group_id 優先で共存する。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` を参照

---

## Requirements

### Requirement 1: 指示書の受信とバリデーション

_Parent: Req 1.1, 1.5_

**Objective:** オーケストレーターとして、デシリアライズ済みの `DolaDocument` をランタイムに配信し、バリデーション成功時のみ変数・トランジション・ストーリーボードの定義を保持させたい。シリアライズ形式（TOML/JSON/YAML）の選択と変換は呼び出し側の責務であり、dola のスコープ外とする。

#### Acceptance Criteria

1. When `load_document(doc: DolaDocument)` が呼び出された場合, the DocumentStore shall `doc.validate()` でバリデーションを実行する。
2. If バリデーションが成功した場合, then the DocumentStore shall `DolaDocument` を内部に保持する。
3. If バリデーションが失敗した場合, then the DolaRuntime shall `RuntimeError::CompileError(Vec<DolaError>)` を返却し、既存の document を保持する（無効な指示書を受け入れない）。

---

### Requirement 2: 指示書の差し替えと変数引き継ぎ

_Parent: Req 1.2, 1.3, 1.4, 1.6_

**Objective:** オーケストレーターとして、新しい指示書で旧定義を差し替えつつ、同名変数の値を引き継ぎたい。

#### Acceptance Criteria

1. When 新しい指示書が配信された場合, the DocumentStore shall 旧定義を完全に上書きし、新定義で置換する。
2. When 新定義に旧定義と同名の変数が含まれる場合, the DolaRuntime shall 当該変数を同一対象として引き継ぎ、タイムテーブル上の現在値を維持する。
3. When 新定義に旧定義の変数が含まれない場合, the DolaRuntime shall 当該変数を凍結状態とする。購読中の変数は最後の値で凍結し購読が継続する限り値を保持する。未購読の変数は即座に破棄する。
4. The DocumentStore shall 指示書が上書きされるまでストーリーボード定義を保持し、同名による再 Start を可能とする。

---

### Requirement 3: ストーリーボード開始（Start）

_Parent: Req 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

**Objective:** オーケストレーターとして、定義済みストーリーボードをコンパイルして再生を開始し、group_id と終了予定時刻を受け取りたい。

#### Acceptance Criteria

1. When `start(name, start_time)` が呼び出された場合, the DolaRuntime shall 該当ストーリーボード定義をコンパイルし、変数ごとのタイムテーブルに展開して再生を開始する。
2. The DolaRuntime shall 単調増加の連番で一意な `group_id: u64` を採番し、実行インスタンスを識別する。
3. The DolaRuntime shall コンパイル済みトランジションに `group_id` および `InterruptionPolicy` をメタデータとして付与する。
4. When 同一ストーリーボードに対して複数回 Start が発行された場合, the DolaRuntime shall それぞれ独立した実行インスタンスを生成する。
5. When Start が正常に完了した場合, the DolaRuntime shall `StartResult { group_id, end_time }` を返却する。
6. When `loop_count` が `Some(0)` の場合, the DolaRuntime shall `end_time = f64::INFINITY` を返却する。

---

### Requirement 4: Start のエラー条件

_Parent: Req 2.7, 2.8, 2.9_

**Objective:** オーケストレーターとして、不正な Start 操作に対してエラーを受け取りたい。

#### Acceptance Criteria

1. When `calculate_end_time(name, start_time)` が呼び出された場合, the DolaRuntime shall 終了予定時刻のみを返却し、実行インスタンスの生成やタイムテーブルへの追加を行わない。
2. If 存在しないストーリーボード名で Start / CalculateEndTime が発行された場合, then the DolaRuntime shall `RuntimeError::StoryboardNotFound` を返却する。
3. If duration=0 かつ `loop_count` が `None` 以外の場合, then the DolaRuntime shall `RuntimeError::ZeroDurationWithLoop` を返却する。

---

### Requirement 5: ストーリーボード制御コマンド

_Parent: Req 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

**Objective:** オーケストレーターとして、再生中のストーリーボードを group_id で一時停止・再開・終了・破棄したい。

#### Acceptance Criteria

1. When `pause(group_id)` が呼び出された場合, the InstanceManager shall 指定インスタンスの経過時刻加算を停止し、`pause_start` を記録する。
2. When `resume(group_id, current_time)` が呼び出された場合, the InstanceManager shall 経過時刻加算を再開し、一時停止時間を `pause_accumulated` に加算して終了予定時刻を再計算して返却する。
3. When `conclude(group_id)` が呼び出された場合, the InstanceManager shall 現在再生中トランジションを最終値にジャンプさせ、未開始トランジションをスキップして終了する。
4. When `cancel(group_id)` が呼び出された場合, the InstanceManager shall 現在の補間値でそのまま凍結して破棄する。
5. When `finish(group_id, offset)` が呼び出された場合, the InstanceManager shall `finish_deadline` を設定し、オフセット時間経過後に Conclude 相当の動作を実行する。
6. While インスタンスが Paused 状態にある場合, the InstanceManager shall 当該インスタンスの経過時刻の加算のみを停止し、他のインスタンスの再生に影響を与えない。
7. If 存在しないまたは終了済みの `group_id` に対して制御コマンドが発行された場合, then the InstanceManager shall `RuntimeError::InvalidGroupId` を返却する。

---

### Requirement 6: 購読管理

_Parent: Req 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

**Objective:** 購読者として、関心のある変数のみを購読し、不要な評価を回避したい。

#### Acceptance Criteria

1. The SubscriptionManager shall 購読登録を指示書受信より前に受け付け可能とする。
2. When `subscribe(subscriber_id, variable_name)` が呼び出された場合, the SubscriptionManager shall 当該変数を評価対象に追加する。
3. When `unsubscribe(subscriber_id, variable_name)` が呼び出された場合, the SubscriptionManager shall 当該変数を評価対象から除外する。
4. When `unsubscribe_all(subscriber_id)` が呼び出された場合, the SubscriptionManager shall 全購読を解除する（`Drop` トレイトによる自動 Unsubscribe に対応）。
5. The SubscriptionManager shall 購読されていない変数の評価を行わない。
6. If 指示書に存在しない変数を購読している場合, then the SubscriptionManager shall 当該変数を無視する（コンパイル対象にならない）。

---

### Requirement 7: 変数評価と差分配信（Update）

_Parent: Req 5.1, 5.2, 5.3, 5.4, 5.5_

**Objective:** 購読者として、Update 呼び出しで購読中変数の値変化のみを差分で取得したい。

#### Acceptance Criteria

1. When `update(subscriber_id, current_time)` が呼び出された場合, the DolaRuntime shall 購読中の全変数を現在時刻で評価し、前回の `update` 呼び出しから値が変化した変数のみを `Vec<(String, EvaluatedValue)>` として返す。
2. When Update が呼び出された場合, the TimelineManager shall 終了済みトランジションをタイムテーブルから破棄する。
3. The DolaRuntime shall Update を購読者への唯一の値配信経路とする。
4. When 全トランジションが終了し変数が凍結状態にある場合, the DolaRuntime shall 空の結果を返す（値変化なし）。
5. The DolaRuntime shall 現在時刻を OS 起動時からの秒数（f64）として受け取る。

---

### Requirement 8: タイムテーブル管理

_Parent: Req 6.1, 6.2, 6.3, 6.4, 6.5_

**Objective:** ランタイム内部として、購読変数ごとのタイムテーブルを管理し、コンパイル済みトランジションの時系列実行を実現したい。

#### Acceptance Criteria

1. The TimelineManager shall 購読変数の数だけタイムテーブルを保持する。
2. When Start でコンパイル結果が生成された場合, the TimelineManager shall 該当変数のタイムテーブルにコンパイル済みトランジションを追加する。
3. When Pause が適用された場合, the TimelineManager shall 時間オフセットを設定して経過時刻の進行を停止する。
4. When Resume が適用された場合, the TimelineManager shall 時間オフセットを調整して経過時刻の進行を再開する。
5. When Update でトランジションの終了が検出された場合, the TimelineManager shall 当該トランジションをタイムテーブルから破棄する。

---

### Requirement 9: 状態遷移の適用

_Parent: Req 8.1, 8.2, 8.3, 8.4, 8.5_

**Objective:** InstanceManager として、core-types が定義する `InstanceState` の遷移ルールをインスタンスのライフサイクルに適用したい。

#### Acceptance Criteria

1. The InstanceManager shall `InstanceState::try_transition()` を使用して全状態遷移の正当性を検証する。
2. The InstanceManager shall Start 時に `Created → Playing` 遷移を適用する。
3. While インスタンスが Playing 状態にある場合, the InstanceManager shall Pause コマンドで `Paused` 状態への遷移を許可する。
4. While インスタンスが Paused 状態にある場合, the InstanceManager shall Resume コマンドで `Playing` 状態への復帰を許可する。
5. When 実行インスタンスが終了状態（Concluded / Cancelled / Trimmed / Compressed）に入った場合, the InstanceManager shall 当該インスタンスを再利用不可とし、以降の状態遷移を拒否する。
6. The InstanceManager shall 同一ストーリーボード定義から複数の独立した実行インスタンスを同時に管理可能とする。

---

### Requirement 10: 同時再生

_Parent: Req 9.1, 9.2, 9.3_

**Objective:** オーケストレーターとして、異なる変数を操作する複数のストーリーボードを同時に再生したい。

#### Acceptance Criteria

1. The TimelineManager shall 異なる変数を操作するストーリーボードを無制限に並行再生する。
2. The DolaRuntime shall 同時再生数に人為的な上限を設けない。
3. The DolaRuntime shall 実質的な計算コストを購読変数数に比例させる。

---

### Requirement 11: Tier 2 暫定動作（競合未実装時）

_Parent: 統合指針 Section 4.3_

**Objective:** ランタイム実装者として、Tier 3 未実装の状態でもランタイムが動作可能であることを保証したい。

#### Acceptance Criteria

1. When 同一変数に対する複数 group_id のエントリが共存する場合, the TimelineManager shall 最新（最大）group_id の値を採用する。
2. The DolaRuntime shall `loop_count` を無視し、常に1回再生として扱う（ループは Tier 3 で実装）。
3. The DolaRuntime shall ConflictResolver / LoopController の注入ポイントを内部設計に含め、Tier 3 追加を容易にする。

