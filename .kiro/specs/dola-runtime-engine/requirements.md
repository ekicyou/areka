# Requirements Document — dola-runtime-engine

## Project Description (Input)

親（オーケストレーター）から配信されるアニメーション定義（指示書）を受け取り、子（購読者）が変数の値変更を差分取得するリアクティブなアニメーションランタイムエンジン。WAM（Windows Animation Manager）の設計を参考としつつ、ストーリーボードはコンパイル後に変数ごとのタイムテーブルに低レベル展開する方式を採る。

## Introduction

本ドキュメントは dola ランタイムエンジンの機能要件を定義する。dola クレートが既に持つ宣言的アニメーション定義フォーマット（`DolaDocument`、コンパイラ等）を基盤とし、その上に「指示書の受信 → コンパイル → タイムテーブル管理 → 購読者への差分配信」というランタイム実行レイヤーを構築する。

---

## Requirements

### Requirement 1: 指示書の受信と管理

**Objective:** オーケストレーターとして、TOML形式のアニメーション定義（指示書）をランタイムに配信し、変数・トランジション・ストーリーボードを定義したい。これにより、アニメーション定義をデータ駆動で動的に差し替えられる。

#### Acceptance Criteria

1. When オーケストレーターが指示書（TOML文字列）を配信した場合, the Dola Runtime shall 指示書をパースし、変数定義・トランジション定義・ストーリーボード定義を内部に保持する。
2. When 新しい指示書が配信された場合, the Dola Runtime shall 旧定義を完全に上書きし、新定義で置換する。
3. When 新定義に旧定義と同名の変数が含まれる場合, the Dola Runtime shall 同一対象として引き継ぎ、現在の値を維持する。
4. When 新定義に旧定義の変数が含まれない場合, the Dola Runtime shall 当該変数の指示書定義を削除する。購読中の変数は最後の値で凍結状態となり、購読が継続する限り値を保持する。未購読の変数は即座に破棄される。
5. If 指示書のパースに失敗した場合, then the Dola Runtime shall エラーを返却し、既存の定義を変更しない。
6. The Dola Runtime shall 指示書が上書きされるまでストーリーボード定義を保持し、同じストーリーボード名での再 Start を可能とする。

---

### Requirement 2: ストーリーボード開始（Start コマンド）

**Objective:** オーケストレーターとして、定義済みストーリーボードをコンパイルして再生開始したい。これにより、任意のタイミングでアニメーションを起動できる。

#### Acceptance Criteria

1. When オーケストレーターが Start コマンド（ストーリーボード名 + 開始時刻）を発行した場合, the Dola Runtime shall 該当ストーリーボード定義をコンパイルし、変数ごとのタイムテーブルに展開して再生を開始する。
2. When コンパイルが実行される場合, the Dola Runtime shall 単調増加の連番で一意な `group_id: u64` を採番し、実行インスタンスを識別する。
3. When コンパイル結果が生成された場合, the Dola Runtime shall 各コンパイル済みトランジションに `group_id` および元ストーリーボードの `InterruptionPolicy` をメタデータとして付与する。
4. When 同一ストーリーボードに対して複数回 Start が発行された場合, the Dola Runtime shall それぞれ異なる `group_id` を持つ独立した実行インスタンスを生成する。
5. When Start が正常に完了した場合, the Dola Runtime shall `group_id` と「正常に再生した場合の終了予定時刻（f64秒）」を返却する。これにより、オーケストレーターは連鎖アニメーションのタイミングを事前計算できる。
6. If 存在しないストーリーボード名で Start が発行された場合, then the Dola Runtime shall エラーを返却する。

---

### Requirement 3: ストーリーボード制御コマンド

**Objective:** オーケストレーターとして、再生中のストーリーボードを一時停止・再開・終了・破棄したい。これにより、アニメーションのライフサイクルを細かく制御できる。

#### Acceptance Criteria

1. When Pause コマンドが発行された場合, the Dola Runtime shall 指定 `group_id` の実行インスタンスの経過時刻加算を停止する。
2. When Resume コマンドが発行された場合, the Dola Runtime shall 指定 `group_id` の実行インスタンスの経過時刻加算を再開し、「正常に再生した場合の終了予定時刻（f64秒）」を返却する。一時停止中の経過時間を差し引いて再計算する。
3. When Conclude コマンドが発行された場合, the Dola Runtime shall 指定 `group_id` の現在再生中トランジションを最終値にジャンプさせ、未開始トランジションをスキップして終了する。
4. When Cancel コマンドが発行された場合, the Dola Runtime shall 指定 `group_id` の現在の補間値でそのまま凍結して破棄する（WAM の Abandon 相当）。
5. When Finish(offset) コマンドが発行された場合, the Dola Runtime shall 指定オフセット時間経過後に Conclude と同等の動作を実行する。
6. While ストーリーボードが Paused 状態にある場合, the Dola Runtime shall 当該ストーリーボードの経過時刻の加算のみを停止し、他のストーリーボードの再生に影響を与えない。
7. If 終了状態（Concluded / Cancelled / Trimmed / Compressed）にある実行インスタンスに制御コマンドが発行された場合, then the Dola Runtime shall 当該コマンドを無視するか、エラーを返却する。

---

### Requirement 4: 購読管理

**Objective:** 購読者として、関心のある変数のみを購読し、リソースを効率的に使用したい。これにより、不要な変数評価を避けてパフォーマンスを最適化できる。購読する変数名の決定は購読者（子）の責務・権利であり、指示書受信より前に登録される。

#### Acceptance Criteria

1. The Dola Runtime shall 購読登録を指示書受信より前に受け付け可能とする。購読する変数名の決定は購読者（子）側の責務である。
2. When 購読者が Subscribe コマンドで変数名を登録した場合, the Dola Runtime shall 当該変数をその購読者の評価対象に追加する。
3. When 購読者が Unsubscribe コマンドを発行した場合, the Dola Runtime shall 当該変数をその購読者の評価対象から除外する。
4. When 購読者が Drop された場合, the Dola Runtime shall 自動的に全購読を解除する（`Drop` トレイトによる自動 Unsubscribe）。
5. The Dola Runtime shall 購読されていない変数の評価を行わない。ランタイムが評価対象として保持する変数は、購読登録された変数名のみである。
6. If 指示書に存在しない変数を購読している場合, then the Dola Runtime shall 当該変数を無視する（コンパイル対象にならない）。

---

### Requirement 5: 変数評価と差分配信（Update）

**Objective:** 購読者として、Update 呼び出しで購読中変数の値変化を差分取得したい。これにより、pull 型で必要なタイミングにのみ値を取得できる。

#### Acceptance Criteria

1. When 購読者が Update(現在時刻) を呼び出した場合, the Dola Runtime shall 購読中の全変数を現在時刻で評価し、前回呼び出しから値が変化した変数のみを `Vec<(変数名, 値)>` として返す。
2. When Update が呼び出された場合, the Dola Runtime shall 終了済みトランジションをタイムテーブルから破棄する。
3. The Dola Runtime shall Update を購読者への唯一の値配信経路とする。オーケストレーターへのストーリーボード完了検知は、Start / Resume が返却する終了予定時刻によって実現する（イベント通知やコールバックは提供しない）。
4. When 全トランジションが終了し変数が凍結状態にある場合, the Dola Runtime shall 当該変数に対して値の変化がないものとして空の結果を返す。
5. The Dola Runtime shall 現在時刻を OS 起動時からの秒数（f64）として受け取る。

---

### Requirement 6: タイムテーブル管理

**Objective:** ランタイム内部として、購読変数ごとのタイムテーブルを管理し、コンパイル済みトランジションの時系列実行を実現したい。

#### Acceptance Criteria

1. The Dola Runtime shall 購読変数の数だけタイムテーブルを保持する。
2. When Start コマンドでコンパイル結果が生成された場合, the Dola Runtime shall 該当変数のタイムテーブルにコンパイル済みセグメントを追加する。
3. When Pause が適用された場合, the Dola Runtime shall タイムテーブルへの時間オフセットを設定して経過時刻の進行を停止する。
4. When Resume が適用された場合, the Dola Runtime shall 時間オフセットを調整して経過時刻の進行を再開する。
5. When Update でトランジションの終了が検出された場合, the Dola Runtime shall 当該トランジションをタイムテーブルから破棄する。

---

### Requirement 7: 競合検出と終了戦略

**Objective:** ランタイムとして、同一変数に対する時間的に重複するトランジション（競合）を検出し、ストーリーボード定義に基づく終了戦略を適用したい。これにより、複数ストーリーボードの自然な遷移を実現できる。

#### Acceptance Criteria

1. When 同一変数に対して時間的に重複するトランジションが発生した場合, the Dola Runtime shall 競合を検出する。
2. When 競合が検出された場合, the Dola Runtime shall 既存ストーリーボード実行インスタンス（`group_id` 単位）に対して終了戦略を一括適用する。
3. When 1つの変数で競合が検出された場合, the Dola Runtime shall 同じ `group_id` を持つ全変数のタイムテーブルに対して終了戦略を一括適用する。
4. When 終了戦略が Cancel の場合, the Dola Runtime shall 既存インスタンスの現在の補間値でそのまま凍結して破棄する（WAM の Abandon 相当）。
5. When 終了戦略が Conclude の場合, the Dola Runtime shall 既存インスタンスの現在再生中トランジションの最終値にジャンプし、未開始トランジションをスキップする。
6. When 終了戦略が Trim の場合, the Dola Runtime shall 既存インスタンスを割り込み開始時点まで再生して切断する。
7. When 終了戦略が Compress の場合, the Dola Runtime shall 既存インスタンスのストーリーボード全体の最終値にジャンプし、全トランジションを完走扱いとする。
8. When 終了戦略が Never の場合, the Dola Runtime shall 既存インスタンスの中断を拒否し、新ストーリーボードの当該変数へのセグメント追加を既存インスタンス完了後まで延期する。
9. If ストーリーボード定義に終了戦略が未指定の場合, then the Dola Runtime shall デフォルトとして Conclude を適用する（既存 `InterruptionPolicy` enum のデフォルトと一致）。

---

### Requirement 8: ストーリーボード状態遷移

**Objective:** ランタイムとして、実行インスタンスごとのライフサイクルを正しく管理したい。これにより、各状態での振る舞いが一貫して予測可能になる。

#### Acceptance Criteria

1. The Dola Runtime shall ストーリーボード実行インスタンスの状態を Created → Playing → {Concluded / Cancelled / Trimmed / Compressed} の遷移で管理する。各終了状態は `InterruptionPolicy` の同名戦略に対応する（Cancel→Cancelled, Conclude→Concluded, Trim→Trimmed, Compress→Compressed）。
2. While ストーリーボードが Playing 状態にある場合, the Dola Runtime shall Pause コマンドで Paused 状態に遷移可能とする。
3. While ストーリーボードが Paused 状態にある場合, the Dola Runtime shall Resume コマンドで Playing 状態に復帰可能とする。
4. When 実行インスタンスが終了状態（Concluded / Cancelled / Trimmed / Compressed）に入った場合, the Dola Runtime shall 当該インスタンスを再利用不可とする。
5. The Dola Runtime shall 同一ストーリーボード定義から複数の独立した実行インスタンスを同時に実行可能とする。

---

### Requirement 9: 同時再生

**Objective:** オーケストレーターとして、異なる変数を操作する複数のストーリーボードを同時に再生したい。これにより、複雑なアニメーション演出を構成できる。

#### Acceptance Criteria

1. The Dola Runtime shall 異なる変数を操作するストーリーボードを無制限に並行再生する。
2. The Dola Runtime shall 同時再生数に人為的な上限を設けない。
3. The Dola Runtime shall 実質的な計算コストを購読変数数に比例させ、非購読変数の評価コストを発生させない。

---

### Requirement 10: イージング関数

**Objective:** アニメーション定義者として、CSS 仕様準拠のイージング関数を使用したい。これにより、多彩な補間カーブを実現できる。

#### Acceptance Criteria

1. The Dola Runtime shall dola クレート既存の `EasingFunction` / `EasingName` 定義に基づいてイージングを適用する。`EasingName` は `interpolation::EaseFunction` 準拠（+ `Linear`）で設計済みであり、1対1マッピングが可能である。
2. The Dola Runtime shall `interpolation` クレート (0.3.0) の `Ease` trait（`impl Ease for f64`）および `EaseFunction` enum を使用して名前付きイージングを評価する。パラメトリックイージング（`ParametricEasing::QuadraticBezier` / `CubicBezier`）には同クレートの `quad_bez` / `cub_bez` 関数を使用する。
3. When トランジションにイージング関数が指定されている場合, the Dola Runtime shall 補間計算時に当該イージング関数を適用する。
4. If トランジションにイージング関数が未指定の場合, then the Dola Runtime shall 線形補間（linear）をデフォルトとして適用する。

---

### Requirement 11: 時刻ユーティリティ

**Objective:** ランタイム利用者として、OS 起動時からの高精度な現在時刻を取得したい。これにより、Update 呼び出し時の時刻指定が簡便になる。

#### Acceptance Criteria

1. The Dola Runtime shall 現在時刻（OS 起動時からの f64 秒数）を取得するユーティリティ関数を提供する。
2. The Dola Runtime shall 時刻取得に適切な既存クレートが利用可能であればそれを使用する。
3. If 適切なクレートが存在しない場合, then the Dola Runtime shall Windows パフォーマンスタイマー（`QueryPerformanceCounter` / `QueryPerformanceFrequency`）を使用して時刻を生成する。
