# Implementation Plan

## Task Overview

全7要件を9個のメジャータスクに分割。データ構造定義とエラー型拡張は並行実装可能。キーフレーム依存解析以降は順次実装し、最終的にPublic API統合とテストカバレッジで完了する。

## Tasks

- [ ] 1. (P) コンパイル済みデータ構造の定義
- [ ] 1.1 (P) CompiledStoryboard 構造体の実装
  - ストーリーボード全体のコンパイル結果を保持するルート構造体を定義
  - timelines (BTreeMap), time_scale, loop_count, interruption_policy, total_base_duration フィールドを含む
  - serde derive (Serialize, Deserialize) を適用
  - _Requirements: 1.1, 1.5, 4.1, 4.2, 4.3, 4.4, 5.5, 7.4_

- [ ] 1.2 (P) CompiledVariableTimeline 構造体の実装
  - 変数ごとのセグメント列とランタイムヒントを保持する構造体を定義
  - variable_type, segments (Vec), base_duration, min_value, max_value フィールドを含む
  - serde derive を適用
  - _Requirements: 1.2, 5.1, 5.5, 5.6, 7.4_

- [ ] 1.3 (P) CompiledSegment 構造体の実装
  - 単一トランジションセグメントの全情報を保持する構造体を定義
  - start_time, end_time, from_value (TransitionValue), to_value, easing (Option) フィールドを含む
  - 即時遷移の場合は start_time == end_time となる設計を反映
  - Object型の場合は easing が None となる設計を反映
  - serde derive を適用
  - _Requirements: 1.3, 1.4, 3.4, 3.5, 7.4_

- [ ] 1.4 (P) VariableTypeHint enum の実装
  - ランタイムに変数型固有の処理方法を伝達する enum を定義
  - Float, Integer { typewriter: Option<String> }, Object バリアントを含む
  - #[serde(tag = "type", rename_all = "snake_case")] 属性を適用
  - serde derive を適用
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 7.4_

- [ ] 2. (P) コンパイル用エラー型の拡張
- [ ] 2.1 (P) DolaError に KeyframeCycle バリアントを追加
  - キーフレーム循環依存を表すバリアントを error.rs に追加
  - storyboard (String) と cycle (Vec&lt;String&gt;) フィールドを持つ
  - Display 実装に match arm を追加し、循環に含まれるKF名を表示
  - _Requirements: 6.2, 6.3_

- [ ] 2.2 (P) DolaError に CompileError バリアントを追加
  - コンパイル固有の汎用エラーを表すバリアントを error.rs に追加
  - storyboard (String), entry_index (usize), reason (String) フィールドを持つ
  - Display 実装に match arm を追加（セグメント重複、between delay超過など）
  - _Requirements: 6.3_

- [ ] 3. バリデーション統合の実装
- [ ] 3.1 compile_storyboard 内での doc.validate() 呼び出し
  - コンパイル関数の冒頭で doc.validate() を内部呼び出し
  - バリデーションエラーがあればそのまま返却（Vec&lt;DolaError&gt;）
  - 呼び出し側が validate を忘れるリスクをゼロにする設計（二重呼び出しのコストは無視できる）
  - _Requirements: 6.1_

- [ ] 3.2 validate.rs の collect_keyframe_names_from_ref を pub(crate) に昇格
  - 既存の private 関数を pub(crate) に変更
  - compile.rs からキーフレーム名収集ロジックを再利用可能にする
  - _Requirements: 6.1_

- [ ] 4. キーフレーム依存グラフの構築と解析
- [ ] 4.1 エントリ間の依存関係グラフを構築
  - 各エントリの配置パターン（sequential, at, between, pure KF）から依存エッジを抽出
  - "start" 疑似キーフレーム（時刻 = start_time）をルートノードとして扱う
  - collect_keyframe_names_from_ref を活用してキーフレーム参照を解決
  - グラフデータ構造（隣接リスト等）を内部実装
  - _Requirements: 2.3, 2.4_

- [ ] 4.2 循環依存の検出
  - DFS探索により依存グラフ内の循環を検出
  - 循環が見つかった場合は DolaError::KeyframeCycle を生成
  - cycle フィールドに循環パス上のKF名を格納
  - _Requirements: 6.2_

- [ ] 4.3 トポロジカルソートによるエントリ処理順序の決定
  - 循環がない場合、トポロジカルソートでエントリの処理順序を決定
  - 前方参照を含むキーフレームDAGを正しく解決するための順序を確定
  - _Requirements: 2.3, 2.4_

- [ ] 5. エントリ処理ロジックの実装
- [ ] 5.1 Sequential エントリの時刻計算
  - 初回エントリの場合: base_time = compile start_time, segment_start = base + delay
  - 連結エントリの場合: base_time = 同一変数の前セグメント end_time, segment_start = base + delay
  - segment_end = segment_start + duration
  - keyframe_time = segment_end
  - _Requirements: 2.1, 2.2, 2.5_

- [ ] 5.2 At 参照エントリの時刻計算
  - 参照先キーフレームの時刻を解決（トポロジカルソート順により既に確定済み）
  - offset があればそれを加算した時刻を base_time とする
  - segment_start = base + delay
  - segment_end = segment_start + duration
  - keyframe_time = segment_end
  - _Requirements: 2.3_

- [ ] 5.3 Between 配置エントリの時刻計算
  - from_KF時刻と to_KF時刻を解決（両方とも既に確定済み）
  - segment_start = from_KF時刻 + delay
  - segment_end = to_KF時刻
  - duration は to_KF時刻 - segment_start で自動決定（TransitionDef.duration は無視）
  - delay >= (to_KF時刻 - from_KF時刻) の場合は DolaError::CompileError を生成
  - keyframe_time = segment_end
  - _Requirements: 2.4_

- [ ] 5.4 Pure Keyframe エントリの時刻計算
  - at ベースの場合: KF時刻 + offset を keyframe_time とする
  - at なしの場合: 配列直前エントリの keyframe_time を継承
  - セグメントは生成しない（純粋な時刻マーカー）
  - _Requirements: 2.3_

- [ ] 5.5 Multiple キーフレーム参照の最遅時刻解決
  - KeyframeRef::Multiple([...]) の場合、全KFの時刻を確認し最遅時刻を使用
  - KeyframeRef::WithOffset { keyframes: Multiple([...]), offset } の場合、全KFの最遅時刻決定後に offset を加算
  - _Requirements: 2.3_

- [ ] 5.6 Named トランジションの解決
  - TransitionRef::Named が指定されている場合、DolaDocument の transition マップから定義を検索
  - 見つからない場合は DolaError::CompileError を生成
  - 見つかった場合は TransitionDef をインライン定義と同等に扱う
  - _Requirements: 3.1_

- [ ] 5.7 from 値の推論
  - TransitionDef に from が省略されている場合、同一変数の直前セグメント終了値を使用
  - 直前セグメントがない場合は変数の初期値を使用
  - _Requirements: 3.2_

- [ ] 5.8 to 値の計算
  - relative_to が指定されている場合、from + relative_to を to とする
  - relative_to がない場合は TransitionDef.to をそのまま使用
  - _Requirements: 3.3_

- [ ] 5.9 CompiledSegment の構築
  - 解決済みの時刻情報（start_time, end_time）と値情報（from_value, to_value）を結合
  - EasingFunction をそのまま転写（Object型の場合は None）
  - duration = 0 の場合は start_time == end_time の即時遷移セグメントとして構築
  - 各変数ごとにセグメントを一時的に蓄積（変数名→Vec&lt;CompiledSegment&gt; マップ）
  - _Requirements: 1.3, 3.4, 3.5_

- [ ] 6. 最終化処理の実装
- [ ] 6.1 変数ごとのセグメントを時刻順にソート
  - 各変数のセグメントベクタを start_time でソート
  - _Requirements: 1.2_

- [ ] 6.2 セグメント重複の検出
  - 同一変数内で時間的に重複するセグメント（前セグメントの end_time > 次セグメントの start_time）がある場合は DolaError::CompileError を生成
  - _Requirements: 1.2_

- [ ] 6.3 CompiledVariableTimeline の構築
  - ソート済みセグメント配列と変数型ヒント（VariableTypeHint）を結合
  - base_duration を算出（最終セグメント end_time - 初回セグメント start_time）
  - min_value / max_value を変数定義から転写（f64/i64のみ）
  - _Requirements: 1.2, 5.1, 5.5, 5.6_

- [ ] 6.4 CompiledStoryboard の構築
  - すべての CompiledVariableTimeline を BTreeMap に格納
  - time_scale, loop_count, interruption_policy を元のストーリーボードから転写
  - total_base_duration を全タイムラインの base_duration の最大値として算出
  - storyboard_name と start_time をメタ情報として含める
  - _Requirements: 1.1, 1.5, 4.1, 4.2, 4.3, 5.5_

- [ ] 7. compile_storyboard 関数本体の統合
- [ ] 7.1 関数シグネチャとドキュメントコメントの実装
  - `pub fn compile_storyboard(doc: &DolaDocument, storyboard_name: &str, start_time: f64) -> Result<CompiledStoryboard, Vec<DolaError>>` を定義
  - Preconditions, Postconditions, Invariants をドキュメントコメントに明記
  - _Requirements: 7.1, 7.2_

- [ ] 7.2 全サブステップのオーケストレーション
  - Task 3-6 で実装した各処理を正しい順序で呼び出し
  - フロー: Validate → Lookup → Build graph → Cycle detect → Toposort → Process entries → Sort → Overlap check → Build result
  - エラーが検出された段階で Vec&lt;DolaError&gt; に蓄積し、可能な限り処理を継続（循環依存検出時は中断）
  - _Requirements: 7.2_

- [ ] 8. Public API の統合
- [ ] 8.1 compile.rs モジュールの作成と基本エクスポート
  - src/compile.rs を新規作成し、すべての compiled 型と compile_storyboard 関数を配置
  - src/lib.rs に `mod compile;` を追加
  - _Requirements: 7.3_

- [ ] 8.2 Public API のフラットエクスポート
  - src/lib.rs に以下をエクスポート:
    - `pub use compile::{CompiledStoryboard, CompiledVariableTimeline, CompiledSegment, VariableTypeHint, compile_storyboard};`
  - 既存の public API との一貫性を保つ（フラットエクスポートパターン）
  - _Requirements: 7.3_

- [ ] 9. テストカバレッジの実装
- [ ] 9.1 データ構造のシリアライズ/デシリアライズテスト
  - tests/compile_test.rs を作成
  - CompiledStoryboard のJSON ラウンドトリップテストを実装
  - serde derive が正しく動作することを確認
  - _Requirements: 7.4_

- [ ] 9.2 時刻解決の単体テスト
  - 単純順次ストーリーボード（1変数・複数セグメント）の時刻解決テスト
  - at 参照（前方・後方）のテスト
  - between 配置のテスト
  - duration=0 即時遷移のテスト
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 3.5_

- [ ] 9.3 トランジション解決の単体テスト
  - Named トランジション解決テスト
  - from 推論テスト（直前セグメント終了値 / 初期値）
  - relative_to 計算テスト
  - Object 型即時切り替えテスト
  - _Requirements: 3.1, 3.2, 3.3, 1.4_

- [ ] 9.4 メタ情報とヒントの単体テスト
  - time_scale / loop_count / interruption_policy 伝達テスト
  - time_scale 非適用テスト（セグメント時刻が time_scale で変化しないこと）
  - 変数型ヒント（Float / Integer / Object）の正しい判定テスト
  - typewriter ヒント伝達テスト
  - 合計再生時間（base_duration / total_base_duration）計算テスト
  - min/max 値域伝達テスト
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [ ] 9.5 エラーハンドリングのテスト
  - validate 失敗パススルーテスト（不正ドキュメントのバリデーションエラー返却）
  - 循環依存検出テスト（A→B→A のKF循環）
  - セグメント重複検出テスト
  - between delay 超過テスト
  - 未定義ストーリーボード参照テスト
  - _Requirements: 6.1, 6.2, 6.3_

- [ ] 9.6 統合テストの実装
  - tests/compile_integration_test.rs を作成
  - 複合ストーリーボード（複数変数 + at/between/sequential 混在）のテスト
  - 全変数型混在（Float + Integer + Object）の同一ストーリーボードテスト
  - Builder → Compile のフローテスト（DolaDocumentBuilder で構築 → compile_storyboard）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 3.5, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 7.4_

## Requirements Coverage

全7要件（R1-R7）の計37個の受入基準をすべてタスクにマッピング済み。

| Requirement | Acceptance Criteria | Mapped Tasks |
|-------------|---------------------|--------------|
| R1 | 1.1-1.5 | 1.1, 1.2, 1.3, 5.9, 6.2, 6.3, 6.4, 9.6 |
| R2 | 2.1-2.5 | 5.1, 5.2, 5.3, 5.4, 5.5, 9.2, 9.6 |
| R3 | 3.1-3.5 | 5.6, 5.7, 5.8, 5.9, 9.3, 9.6 |
| R4 | 4.1-4.4 | 1.1, 6.4, 9.4, 9.6 |
| R5 | 5.1-5.6 | 1.2, 1.4, 6.3, 9.4, 9.6 |
| R6 | 6.1-6.3 | 2.1, 2.2, 3.1, 4.2, 7.2, 9.5, 9.6 |
| R7 | 7.1-7.4 | 1.1, 1.2, 1.3, 1.4, 7.1, 8.1, 8.2, 9.1, 9.6 |

## Implementation Notes

- **Parallel Execution**: Task 1 (データ構造) と Task 2 (エラー型) は完全に独立しているため並行実装可能
- **Test-Driven Development**: Task 9.1（データ構造テスト）は Task 1 完了後すぐ実装可能
- **Incremental Testing**: Task 9.2-9.5 は対応する実装タスク完了次第、段階的にテスト追加が推奨される
- **Final Validation**: Task 9.6（統合テスト）はすべての実装完了後に全機能を検証
