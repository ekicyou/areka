# Requirements Document

## Introduction

伺か（Ukagaka）のSERIKO定義にインスパイアされた、ストーリーボードのループ間ランダム遅延機能の要件定義。
瞬きアニメーションのような「不定期に繰り返される自然な動作」を宣言的に定義可能にする。

現在のdolaクレートの `Storyboard` は `loop_count` による繰り返し再生に対応しているが、
各周回は即座に連続再生される。本機能により、周回間にランダムな待機時間（ループオフセット）を
挿入し、より自然で有機的なアニメーション表現を実現する。

## Project Description (Input)
ストーリーボードの繰り返し時に、すぐに繰り返すのではなく、ランダムな時間のoffsetを設定し、例えば瞬きアニメーションを実現したい。伺かのSERIKO定義をインスパイヤしたもの。

## Requirements

### Requirement 1: ループオフセット定義
**Objective:** アニメーション作成者として、ストーリーボードの各ループ間にランダムな待機時間の範囲を宣言的に指定したい。不定期に繰り返される自然な動作を定義できるようにするため。

#### Acceptance Criteria
1. The dola Storyboard shall ストーリーボード定義に `loop_offset` フィールド（省略可能）を持つ
2. When `loop_offset` が指定された場合, the dola Storyboard shall 最小値 `min`（f64秒、デフォルト 0.0）と最大値 `max`（f64秒、必須）の2つのパラメータで待機時間の範囲を表現する
3. Where `loop_offset` が省略された場合, the dola Storyboard shall 従来どおりループ間に遅延なしで即座に次の周回を開始する（後方互換性の維持）
4. The dola Storyboard shall `loop_offset` をJSON/TOML/YAMLすべてのシリアライズ形式で同等にサポートする

### Requirement 2: ランダム遅延のランタイム適用
**Objective:** ランタイムエンジンとして、各ループ周回の完了時にランダムな遅延を生成・適用し、自然な不定期再生を実現したい。再生エンジンの既存ループ制御を拡張するため。

#### Acceptance Criteria
1. When 周回が完了し `loop_offset` が定義されている場合, the dola Runtime shall `[min, max]` の範囲から一様乱数で遅延時間を生成し、次の周回開始前にその時間だけ待機する
2. When 無限ループ（`loop_count = -1`）と `loop_offset` が同時に指定された場合, the dola Runtime shall 各周回完了時にランダム遅延を適用し続ける
3. When `loop_count = 1`（繰り返しなし）の場合, the dola Runtime shall `loop_offset` を無視して単一再生を行う
4. While ランダム遅延で待機中, the dola Runtime shall アニメーション変数を最終値（前回周回の終了値）に維持する
5. The dola Runtime shall 各周回ごとに独立した乱数を生成し、毎回異なる待機時間とする
6. The dola Runtime shall `loop_offset` で生成されたランダム遅延に `time_scale` を適用しない（遅延時間は実時間ベースで一定。`time_scale` はアニメーション再生速度のみに影響する）

### Requirement 3: バリデーションルール
**Objective:** ドキュメント作成者として、不正なループオフセット定義を事前に検出したい。ランタイムエラーを防止し、デバッグを容易にするため。

#### Acceptance Criteria
1. If `loop_offset.min` が負の値の場合, the dola Validator shall バリデーションエラーを報告する
2. If `loop_offset.max` が負の値の場合, the dola Validator shall バリデーションエラーを報告する
3. If `loop_offset.min > loop_offset.max` の場合, the dola Validator shall 範囲逆転エラーを報告する
4. If `loop_offset` が指定されているが `loop_count = 1` の場合, the dola Validator shall 警告を報告する（エラーではなく警告）

### Requirement 4: JSON定義の簡潔な記法
**Objective:** アニメーション作成者として、シンプルなユースケース（瞬きなど）をできるだけ簡潔に記述したい。定義ファイルの可読性と記述効率を高めるため。

#### Acceptance Criteria
1. When 単一の数値が `loop_offset` に指定された場合, the dola Storyboard shall それを `max` として解釈し、`min` を 0.0 とする短縮形をサポートする
2. When オブジェクト形式で `loop_offset` が指定された場合, the dola Storyboard shall `{ "min": ..., "max": ... }` として解析する
3. The dola Storyboard shall 短縮形（数値）とオブジェクト形式の両方をデシリアライズ/シリアライズで正しく処理する

### Requirement 5: 割り込み・一時停止との整合性
**Objective:** ランタイムエンジンとして、既存の割り込み/一時停止メカニズムとループオフセットの一貫した動作を保証したい。既存の再生制御機能との互換性を維持するため。

#### Acceptance Criteria
1. When 遅延待機中にPauseが要求された場合, the dola Runtime shall 遅延の残り時間を保持し、Resume時に残り遅延から再開する
2. When 遅延待機中にCancelが要求された場合, the dola Runtime shall 遅延を即座に中断しストーリーボードをキャンセル状態にする
3. When 遅延待機中に割り込み（InterruptionPolicy に基づく）が発生した場合, the dola Runtime shall InterruptionPolicyに従った終了処理を実行する
