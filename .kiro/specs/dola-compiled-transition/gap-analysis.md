# Gap Analysis: dola-compiled-transition

## 1. 現状調査（Current State Investigation）

### 1.1 既存アセットマップ

| ファイル | 責務 | コンパイラとの関連度 |
|---------|------|---------------------|
| `document.rs` | `DolaDocument` ルートコンテナ（variables/transitions/storyboards） | **高** — コンパイル入力の起点 |
| `storyboard.rs` | `Storyboard`, `StoryboardEntry`, `KeyframeRef`, `BetweenKeyframes`, `InterruptionPolicy` | **高** — コンパイル対象の中核 |
| `transition.rs` | `TransitionDef`, `TransitionRef`, `TransitionValue` | **高** — 解決・平坦化の対象 |
| `easing.rs` | `EasingFunction`, `EasingName`, `ParametricEasing` | **中** — セグメントにそのまま転写 |
| `variable.rs` | `AnimationVariableDef` (Float/Integer/Object) | **中** — 型ヒント・初期値の参照元 |
| `value.rs` | `DynamicValue` | **中** — Object型セグメントの値型 |
| `validate.rs` | `Validate` トレイト、V1-V13 バリデーションルール | **高** — コンパイル前提条件 + 参照解決パターンの先行実装 |
| `error.rs` | `DolaError` enum（13バリアント） | **高** — エラー型の拡張先 |
| `builder.rs` | `DolaDocumentBuilder`, `StoryboardBuilder` | **低** — 直接は不要だがテスト構築で使用 |
| `playback.rs` | `PlaybackState`, `ScheduleRequest` | **低** — ランタイム寄り、スコープ外だが `ScheduleRequest.start_time` はコンパイルAPI引数のヒント |

### 1.2 既存コードのパターン・規約

- **BTreeMap 一貫使用**: `DolaDocument` の variable/transition/storyboard すべてが `BTreeMap<String, T>`
- **serde derive**: 全公開型が `#[derive(Serialize, Deserialize)]` を持つ
- **`#[serde(untagged)]` パターン**: `TransitionRef`, `TransitionValue`, `EasingFunction`, `KeyframeRef` 等で多用
- **`#[serde(tag = "type")]` パターン**: `AnimationVariableDef`, `ParametricEasing`
- **エラー収集パターン**: `validate.rs` は `Vec<DolaError>` にエラーを蓄積し、空なら `Ok(())`, 非空なら `Err(errors)` を返す
- **テストファイル配置**: `tests/` 下に専用ファイル（`core_types_test.rs`, `validation_test.rs`, `builder_test.rs`, `integration_test.rs`）
- **依存関係**: dola は純粋なデータ定義クレート。bevy_ecs/windows 非依存。serde + フォーマットfeatureのみ

### 1.3 統合サーフェス

- **Validate との関係**: `validate.rs` 内の参照解決ロジック（Named→定義、キーフレーム名収集）はコンパイラでも類似ロジックが必要。ただし validate は「検証」、compile は「変換」という異なる責務
- **lib.rs エクスポート**: 現在 pub use で全型をフラットにエクスポート。新型も同パターンで追加
- **DolaError 拡張**: 既存13バリアント + `Display` 実装 + `std::error::Error` 実装。新バリアント追加は自然

---

## 2. 要件実現可能性分析（Requirements Feasibility Analysis）

### Requirement 1: コンパイル済み構造体定義

| 技術ニーズ | 既存 | ギャップ |
|-----------|------|---------|
| `CompiledStoryboard` 型 | ❌ | **Missing** — 新規定義が必要 |
| `CompiledVariableTimeline` 型 | ❌ | **Missing** — 新規定義が必要 |
| `CompiledSegment` 型 | ❌ | **Missing** — 新規定義が必要 |
| f64/i64 用セグメント値 | `TransitionValue::Scalar(f64)` あり | 流用可能、ただし i64 の扱いを検討 |
| Object 用セグメント値 | `DynamicValue` あり | 直接流用可能 |
| Serialize/Deserialize | serde 基盤あり | derive 追加のみ |

**複雑度**: 低（型定義のみ）

### Requirement 2: 時刻解決

| 技術ニーズ | 既存 | ギャップ |
|-----------|------|---------|
| キーフレーム名→時刻のマッピング | ❌（validate.rs で名前収集のみ） | **Missing** — 時刻解決アルゴリズムが必要 |
| 暗黙的キーフレーム時刻算出 | ❌ | **Missing** — 各エントリ終了時刻の追跡 |
| at/between配置ロジック | ❌（バリデーションのみ） | **Missing** — 時間計算ロジック |
| delay加算 | ❌ | **Missing** — 単純加算 |
| 前エントリ連結（変数別追跡） | ❌ | **Missing** — 変数別の最終時刻・最終値を追跡するステート管理 |

**複雑度**: **高** — キーフレームDAGの時刻解決はコンパイラの核心。以下の難点:
- エントリ順序 ≠ 時系列順序（at/between で任意のキーフレームにジャンプ可能）
- 複数キーフレーム待ち（`KeyframeRef::Multiple`）→ 最遅時刻の選択
- 暗黙的キーフレーム（`__implicit_{idx}`）の時刻 ＝ その entry が配置する最後のセグメント終了時刻
- 潜在的な**前方参照・相互依存**（entry A が entry B の keyframe を参照し、entry B はまだ処理されていない）

**Research Needed**: キーフレーム時刻解決においてトポロジカルソートが必要かどうか。現行 `validate.rs` は「全KF名を先に2パスで収集」する戦略だが、時刻解決では順序依存がある。

### Requirement 3: トランジション解決・平坦化

| 技術ニーズ | 既存 | ギャップ |
|-----------|------|---------|
| Named→定義の解決 | ✅（validate.rs L95-99 で実装パターンあり） | パターン流用可能 |
| from推論（直前セグメント終了値 or 初期値）| ❌ | **Missing** — 変数別の最終値追跡が必要 |
| relative_to計算 | ❌（V11 排他バリデーションのみ） | **Missing** — from + relative_to → to の計算 |
| EasingFunction転写 | ✅ Clone derive あり | そのまま `.clone()` で転写可能 |
| duration=None → 即時遷移 | ❌ | **Missing** — duration 0 セグメント生成 |

**複雑度**: 中（R2 の変数別状態追跡と密結合）

### Requirement 4: メタ情報伝達

| 技術ニーズ | 既存 | ギャップ |
|-----------|------|---------|
| time_scale 格納 | `Storyboard.time_scale` あり | フィールドコピーのみ |
| loop_count 格納 | `Storyboard.loop_count` あり | フィールドコピーのみ |
| interruption_policy 格納 | `Storyboard.interruption_policy` あり | フィールドコピーのみ |

**複雑度**: 低（単純転写）

### Requirement 5: 割り切り情報

| 技術ニーズ | 既存 | ギャップ |
|-----------|------|---------|
| 変数型ヒント | `AnimationVariableDef` のバリアント判定 | match で抽出可能 |
| i64丸めヒント | ❌ | **Missing** — フラグ型の定義 |
| Object即時切り替えヒント | ❌ | **Missing** — フラグ型の定義 |
| typewriter文字列 | `AnimationVariableDef::Integer.typewriter` あり | 転写可能 |
| 合計再生時間 | ❌ | **Missing** — 全セグメント終了時刻の max 算出 |
| min/max値域 | `AnimationVariableDef` に格納済み | 転写可能 |

**複雑度**: 低〜中（型定義 + R2の計算結果に依存）

### Requirement 6: エラーハンドリング

| 技術ニーズ | 既存 | ギャップ |
|-----------|------|---------|
| DolaError 拡張 | 13バリアント + Display + Error 実装 | 新バリアント追加は容易 |
| コンパイル固有エラー | ❌ | **Missing** — 循環依存検出、時刻解決失敗等 |
| バリデーション前提条件 | `Validate` トレイト | コンパイル関数内で `validate()` を先行呼び出し or 呼び出し側に委ねる |

**複雑度**: 低

### Requirement 7: コンパイルAPI

| 技術ニーズ | 既存 | ギャップ |
|-----------|------|---------|
| コンパイル関数 | ❌ | **Missing** — 新規パブリック関数 |
| lib.rs エクスポート | pub use パターン確立済み | 追加のみ |
| Result戻り値型 | `Result<T, Vec<DolaError>>` パターン確立済み | パターン流用 |

**複雑度**: 低（R2, R3 の実装に依存）

---

## 3. 実装アプローチオプション

### Option A: dola クレート内に `compile.rs` モジュールを追加

**検討理由**: validate.rs と同レベルの新モジュールとして、既存構造に自然にフィット

- **追加ファイル**: `src/compile.rs`（+ 必要に応じて `src/compile/` ディレクトリ化）
- **変更ファイル**: `src/lib.rs`（mod宣言 + pub use 追加）、`src/error.rs`（新バリアント追加）
- **テスト**: `tests/compile_test.rs`

**Trade-offs**:
- ✅ validate.rs の参照解決パターンを内部で直接利用可能
- ✅ DolaDocument の全フィールドに直接アクセス
- ✅ 既存の公開API・テストパターン・serde規約に完全準拠
- ✅ 新規クレート作成のオーバーヘッドなし
- ❌ compile.rs が大きくなる場合、dola クレートの複雑度が上がる

### Option B: 別クレート `dola-compiler` を新設

**検討理由**: コンパイラの責務をデータ定義から完全分離

- **新クレート**: `crates/dola-compiler/`
- **依存**: `dola` クレートに依存

**Trade-offs**:
- ✅ 責務の完全分離（dola = 定義、dola-compiler = 変換）
- ✅ コンパイル不要なユースケース（定義の読み書きのみ）のバイナリサイズ削減
- ❌ `validate.rs` の内部ロジック（参照解決）を再実装 or pub(crate) を pub に変更する必要
- ❌ ワークスペースに新クレート追加のオーバーヘッド
- ❌ dola の内部型へのアクセスが制限される

### Option C: dola クレート内に `compile/` サブモジュールディレクトリ

**検討理由**: コンパイラのサブ責務（型定義、時刻解決、値解決）を分割

- **追加**: `src/compile/mod.rs`, `src/compile/types.rs`, `src/compile/resolve.rs` 等
- 内部構造は分割しつつ、外部インターフェースは Option A と同一

**Trade-offs**:
- ✅ 内部構造が整理される
- ✅ Option A の利点をすべて継承
- ❌ 初期段階でファイルが小さければ過剰設計の恐れ
- → 最初は Option A で開始し、成長に応じて C に移行する戦略が現実的

### 推奨アプローチ

**Option A を推奨**（成長時に Option C へ移行可能）

理由:
1. dolaは「宣言 + バリデーション + コンパイル」までを一つのクレートで完結させるのが自然
2. validate.rs の内部関数（キーフレーム名収集等）を `pub(crate)` で共有可能
3. 既存パターンとの一貫性が最も高い
4. 機能が成長した場合は `compile/` ディレクトリに分割する選択肢を保持

---

## 4. 実装複雑度・リスク評価

### 工数: **M（3-7日）**

- 型定義（R1, R4, R5）: ~1日
- キーフレーム時刻解決（R2）: ~2-3日 ←最大の工数
- トランジション解決（R3）: ~1日
- エラー型・API（R6, R7）: ~0.5日
- テスト: ~1-2日

**根拠**: 新パターン（キーフレームDAG時刻解決）が1つ、それ以外は確立済みパターンの活用。外部統合なし。

### リスク: **中（Medium）**

| リスク要因 | 詳細 | 緩和策 |
|-----------|------|--------|
| キーフレーム時刻解決の複雑度 | 前方参照・複数KF待ち・暗黙KF追跡の組み合わせ | 2パスアルゴリズム（1. KF依存グラフ構築 → 2. トポロジカル順序で時刻算出）の事前設計 |
| 同一変数の複数セグメント重複 | 異なるエントリが同一変数の同一時間帯にセグメントを配置する可能性 | バリデーション or コンパイルエラーとして扱う設計判断が必要 |
| between配置のduration自動決定 | between指定時、TransitionDef.durationを無視してKF間の時間幅で上書きするか？ | 要件で「from/toキーフレーム間の時間範囲にトランジションを配置する」とあるため、時間幅はKF間で決定するのが自然。設計フェーズで確定 |

---

## 5. 設計フェーズへの引き継ぎ事項

### 設計判断が必要な項目

1. **キーフレーム時刻解決アルゴリズム**: 線形スキャン vs トポロジカルソート vs 2パスの選択
2. **同一変数のセグメント重複ポリシー**: エラー vs 後勝ち vs マージ
3. **between配置時のduration処理**: TransitionDef.duration を無視してKF間幅で置き換えるか
4. **CompiledSegment の値型設計**: `f64` と `DynamicValue` を統合する enum か、ジェネリクスか
5. **validate.rs との共通ロジック抽出**: 参照解決をモジュール間で共有する方法

### Research Needed

1. キーフレーム依存グラフが循環する可能性と検出方法（既存バリデーションでは未対応）
2. `time_scale` の事前適用 vs ランタイム適用の詳細な境界（要件R4-4で「事前適用しない」と明記済み、確認のみ）
