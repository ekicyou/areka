# Research & Design Decisions

## Summary
- **Feature**: `dola-compiled-transition`
- **Discovery Scope**: Extension（既存dolaクレートに compile.rs モジュールを追加）
- **Key Findings**:
  - キーフレーム時刻解決にはトポロジカルソートベースの依存グラフ解決が必要
  - validate.rs の参照解決パターン（2パス名前収集）はコンパイラに直接流用可能
  - 既存型（TransitionValue, EasingFunction, InterruptionPolicy）をコンパイル済み構造体で再利用し、新規型定義を最小限に抑えられる

## Research Log

### キーフレーム時刻解決アルゴリズム
- **Context**: R2が要求するキーフレームDAGの時刻解決がコンパイラの核心的課題
- **Sources Consulted**: 既存 validate.rs（V6 キーフレーム参照検証）、storyboard.rs の KeyframeRef / BetweenKeyframes 型定義
- **Findings**:
  - エントリ配列順序 ≠ 時系列順序（at/betweenによる任意時点ジャンプ）
  - 前方参照が発生しうる（entry A が entry B のキーフレームを参照、B は A より後ろ）
  - 暗黙的キーフレーム（`__implicit_{idx}`）の時刻はエントリ終了時刻で決定
  - 同一変数の前エントリ連結は配列順序に基づく暗黙的依存
  - validate.rs は2パス名前収集（第1パスで全KF名収集、第2パスで参照検証）だが、時刻解決では依存順序の制御が必須
- **Implications**: 線形スキャンでは前方参照に対応不可。依存グラフ構築 → トポロジカルソートが必要。循環検出（R6-2）もグラフ構築時に自然に実現可能。

### validate.rs との共通ロジック
- **Context**: validate.rs とコンパイラで参照解決パターンが重複する
- **Sources Consulted**: validate.rs 全403行を精読
- **Findings**:
  - `collect_keyframe_names_from_ref()` — KeyframeRef からKF名を抽出。コンパイラでも依存グラフ構築に直接必要
  - Named→定義の解決パターン（L95-99）— `self.transition.get(name)` によるルックアップ。コンパイラでも同一パターンを使用
  - 2パスKF名収集ロジック — validate専用。コンパイラでは依存グラフで代替するため直接流用しない
  - 変数型チェック（V10/V13）— validate.rs の責務。コンパイラは validate() を前提条件とするため重複チェック不要
- **Implications**: `collect_keyframe_names_from_ref` を `pub(crate)` に昇格して共有。他のロジックは責務が異なるため別実装。

### CompiledSegment の値型設計
- **Context**: f64/i64 変数のスカラー値と Object 変数の DynamicValue を統一的に扱う型が必要
- **Sources Consulted**: TransitionValue（transition.rs）、DynamicValue（value.rs）、AnimationVariableDef（variable.rs）
- **Findings**:
  - 既存 `TransitionValue` enum が `Scalar(f64)` / `Dynamic(DynamicValue)` を持ち、セグメントの from/to 値として直接使用可能
  - i64 変数は f64 空間で補間し丸めるため、セグメント値は f64 で十分（VariableTypeHint で型を別途伝達）
  - enum アプローチ（Transition/InstantSwitch）は型安全だが、R1-3が全セグメントに統一フィールドを要求しており、統一 struct の方が合致
- **Implications**: CompiledSegment は統一 struct として定義。TransitionValue を from_value / to_value に使用。

### between 配置時の delay / duration 処理
- **Context**: between 指定時、TransitionDef の delay / duration フィールドの扱いが未定義
- **Sources Consulted**: R2-4、TransitionDef 型定義、gap-analysis.md §5 設計判断#3
- **Findings**:
  - R2-4は「from/toキーフレーム間の時間範囲にトランジションを配置する」と規定
  - duration: between が指定する時間範囲（to_time - from_time - delay）で上書きされるのが自然
  - delay: between の from_kf_time からのオフセットとして適用可能（ユーザが delay 付きのnamed transitionを between で使うケースは合理的）
  - delay >= (to_time - from_time) の場合はゼロまたは負の duration となり、コンパイルエラーとすべき
- **Implications**: between は duration を上書き、delay は維持。これにより at / sequential と一貫した delay セマンティクスを保持。

### セグメント重複ポリシー
- **Context**: 同一変数に対して複数エントリが同一時間帯にセグメントを配置する可能性
- **Sources Consulted**: R1-2（ギャップ許容）、gap-analysis.md §4 リスク評価
- **Findings**:
  - R1-2はギャップ許容を明記するが、重複（オーバーラップ）については言及なし
  - 重複は宣言的定義のミスである可能性が高い
  - 重複を許す場合、「後勝ち」「マージ」のセマンティクスが必要で複雑化
  - 重複をコンパイルエラーとすることで、宣言ミスを早期に検出でき安全
- **Implications**: 重複はコンパイルエラー。コンパイル後にセグメントを時刻順ソートし、隣接セグメント間で end_time > next.start_time を検出。

## Architecture Pattern Evaluation

| Option | 概要 | 長所 | 短所 | 備考 |
|--------|------|------|------|------|
| A: compile.rs | dolaクレート内に単一モジュール追加 | 既存パターンとの一貫性、pub(crate)共有可能、オーバーヘッド最小 | ファイルが大きくなる可能性 | 推奨。成長時にOption Cへ移行可能 |
| B: 別クレート | dola-compiler クレートを新設 | 責務の完全分離 | validate.rs の内部ロジック再実装が必要、ワークスペースオーバーヘッド | 過剰 |
| C: compile/ ディレクトリ | サブモジュール分割 | 内部構造が整理される | 初期段階では過剰設計 | Option Aの成長先 |

**選択**: Option A（compile.rs 単一モジュール）

## Design Decisions

### Decision: キーフレーム時刻解決アルゴリズム
- **Context**: エントリ間のキーフレーム依存による時刻解決が複雑
- **Alternatives Considered**:
  1. 線形スキャン — 単純だが前方参照に対応不可
  2. トポロジカルソート — 依存グラフに基づく順序決定
  3. 反復解決 — 未解決エントリを繰り返しスキャン
- **Selected Approach**: トポロジカルソート
- **Rationale**: 前方参照・相互依存を正しく処理し、循環検出もDFS中に自然に実現。既存validate.rsのKF名収集も依存グラフ構築に統合可能。
- **Trade-offs**: 依存グラフ構築のコード量は増えるが、正確性と循環検出の両立は他アプローチでは困難
- **Follow-up**: 依存グラフのエッジ種別（KF参照 / 同一変数連結 / 配列順序連結）を実装時に明確化

### Decision: between 配置時の delay / duration 処理
- **Context**: TransitionDef の delay / duration と between 指定の時間範囲の競合
- **Alternatives Considered**:
  1. delay も duration も無視 — between が完全に時間範囲を支配
  2. delay は維持、duration は上書き — delay は from_kf_time からのオフセット
  3. 両方維持 — between 内で delay + duration を使い、残りはギャップ
- **Selected Approach**: delay は維持、duration は上書き
- **Rationale**: delay の「遷移前待機」セマンティクスは配置パターンに依存しない普遍的概念。duration は between の from-to 間で自動決定される。named transition テンプレートを between で再利用する際、delay のみが意味を持つ。
- **Trade-offs**: delay >= (to_time - from_time) のエッジケース検出が必要
- **Follow-up**: なし

### Decision: CompiledSegment の値型
- **Context**: f64/i64 変数のスカラー値と Object 変数の DynamicValue を1つのセグメント型で表現
- **Alternatives Considered**:
  1. 統一 struct — 全セグメントが同一フィールド構造
  2. enum (Transition / InstantSwitch) — 変数型ごとに別バリアント
  3. ジェネリクス CompiledSegment&lt;V&gt; — 型パラメータで値型を制御
- **Selected Approach**: 統一 struct
- **Rationale**: R1-3が全セグメントに同一フィールド（start_time, end_time, from_value, to_value, easing）を要求。TransitionValue 再利用により既存パターンとの一貫性を維持。VariableTypeHint で型判別を行うため、セグメントレベルの型分岐は不要。
- **Trade-offs**: Object型の from_value は前セグメント終了値のコピーであり、やや冗長。しかし自己完結性・デバッグ容易性の利点が上回る。
- **Follow-up**: なし

### Decision: 同一変数セグメント重複ポリシー
- **Context**: 複数エントリが同一変数の同一時間帯にセグメントを生成する可能性
- **Alternatives Considered**:
  1. コンパイルエラー — 重複を検出してエラー報告
  2. 後勝ち — 後のセグメントが先のセグメントを上書き
  3. マージ — 重複区間を分割して統合
- **Selected Approach**: コンパイルエラー
- **Rationale**: 重複は宣言的定義のミスである可能性が高く、暗黙のマージや上書きよりも明示的なエラーが安全。ランタイム側の曖昧さを排除。
- **Trade-offs**: 意図的な重複（レイヤリング等）には対応できないが、現行要件のスコープ外
- **Follow-up**: なし

### Decision: 純粋KFエントリの時刻解決
- **Context**: variable/transition を持たない純粋キーフレームエントリの時刻をどう決定するか
- **Alternatives Considered**:
  1. at 参照必須 — 純粋KFは at がなければエラー
  2. 配列直前エントリのKF時刻を継承 — 暗黙的な「現在時刻カーソル」
  3. コンパイル開始時刻 — 常に start_time
- **Selected Approach**: at 参照時はその時刻を使用、at なしの場合は配列直前エントリのKF時刻を継承
- **Rationale**: 純粋KFは同期ポイントとして使われる。at ありなら明示的な時刻指定。at なしの場合は「ストーリーボードのこの位置の時刻」を表すため、直前エントリの完了時刻が自然。
- **Trade-offs**: 依存グラフに配列順序に基づくエッジが追加される
- **Follow-up**: なし

## Risks & Mitigations
- **キーフレーム時刻解決の複雑度** — トポロジカルソートにより正確な順序保証。テストケースで前方参照・複数KF待ち・暗黙KF追跡の組み合わせを網羅
- **セグメント重複の誤検出** — 浮動小数点比較に微小イプシロン許容を導入（同一時刻の end/start は重複としない）
- **大規模ストーリーボードの性能** — トポロジカルソートは O(V+E) であり、通常のストーリーボード規模（数十〜数百エントリ）で問題なし
- **validate.rs 変更の影響** — `collect_keyframe_names_from_ref` の可視性変更（private → pub(crate)）のみ。既存テストへの影響なし

## References
- dola crate 既存ソース: `crates/dola/src/` — validate.rs, storyboard.rs, transition.rs, variable.rs, easing.rs, error.rs
- gap-analysis.md: `.kiro/specs/dola-compiled-transition/gap-analysis.md` — 実装アプローチ評価とリスク分析
- 参考議論: https://claude.ai/share/fb7707ad-da54-45b9-be43-038eed693949 — 初期構想
