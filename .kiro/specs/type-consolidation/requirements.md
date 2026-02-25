# Requirements Document

## Project Description (Input)
wintf内でRECT/SIZEなど、中身が一致している似たような構造体が乱立している。全体の用途を調査し、同じ概念の構造体をひとまとめにし、どこかの`types`？かな、よく使われる型宣言名前空間に宣言を集約したい。多分ギャップ分析で全体調査になるかな？お願いします。

## Introduction

wintf クレート内で幾何学・空間系の構造体（`Size`, `Offset`, `Rect`, `PhysicalPoint` 等）が複数モジュールに分散・重複定義されている。これらを調査し、意味的に同一または類似した型を共通名前空間に集約し、コードベースの一貫性と保守性を向上させる。

### 現状の型分布（調査結果）

| 概念                       | 定義箇所                       | フィールド型                    | 問題                                    |
| -------------------------- | ------------------------------ | ------------------------------- | --------------------------------------- |
| `Size`                     | `ecs/layout/metrics.rs`        | `f32 × 2`                       | 唯一定義、レイアウト層に閉じ込め        |
| `Offset`                   | `ecs/layout/metrics.rs`        | `f32 × 2`                       | 唯一定義、レイアウト層に閉じ込め        |
| `LayoutScale`              | `ecs/layout/metrics.rs`        | `f32 × 2`                       | 唯一定義、レイアウト層に閉じ込め        |
| `Rect<T>`                  | `ecs/layout/dimension.rs`      | `T × 4 (left/right/top/bottom)` | ボックスモデル専用、汎用性不足          |
| `D2DRect`                  | `ecs/layout/rect.rs`           | `D2D_RECT_F` のエイリアス       | Win32型への直接依存                     |
| `PhysicalPoint` (pointer)  | `ecs/pointer/types.rs`         | `i32 × 2`                       | **同名型が2箇所に重複**                 |
| `PhysicalPoint` (hit_test) | `ecs/layout/hit_test/mod.rs`   | `f32 × 2`                       | **同名だがフィールド型が異なる**        |
| `Translate`                | `ecs/transform/components.rs`  | `f32 × 2`                       | `Offset` と構造同一（非推奨モジュール） |
| `Scale`                    | `ecs/transform/components.rs`  | `f32 × 2`                       | `LayoutScale` と構造同一（非推奨）      |
| `TransformOrigin`          | `ecs/transform/components.rs`  | `f32 × 2`                       | （非推奨モジュール）                    |
| `WindowPos`                | `ecs/window/window_pos.rs`     | Win32 `POINT`/`SIZE` を直接使用 | Win32型への直接依存                     |
| `Shape::Rect`              | `ecs/layout/hit_region/mod.rs` | `f32 × 4 (x/y/w/h)`             | インライン矩形定義                      |

## Requirements

### Requirement 1: 共通型モジュールの導入
**Objective:** As a wintf ライブラリ開発者, I want 幾何学・空間型を一箇所にまとめた共通型モジュールを持つ, so that 型定義の発見性が向上し、新規モジュール作成時の型選択が容易になる

#### Acceptance Criteria
1. The wintf shall `ecs/types/` または `ecs/common/types/` モジュールに共通幾何型を定義する共通型モジュールを提供する
2. The wintf shall 共通型モジュールを `pub use` で re-export し、既存のモジュール構成を壊さずにアクセス可能とする
3. The wintf shall 共通型モジュールの全型に `Debug`, `Clone`, `Copy`, `PartialEq` の最低限の derive を適用する

### Requirement 2: Point 型の統一
**Objective:** As a wintf ライブラリ開発者, I want `PhysicalPoint` の重複定義を解消し、用途に応じた型体系を持つ, so that 同名異義の型による混乱がなくなる

#### Acceptance Criteria
1. The wintf shall 整数座標ポイント型（`i32 × 2`）を共通型モジュールで一つだけ定義する
2. The wintf shall 浮動小数点座標ポイント型（`f32 × 2`）を共通型モジュールで一つだけ定義する
3. When `ecs/pointer/types.rs` が整数座標ポイントを必要とする場合, the wintf shall 共通型モジュールの整数ポイント型を使用する
4. When `ecs/layout/hit_test/` が浮動小数点座標ポイントを必要とする場合, the wintf shall 共通型モジュールの浮動小数点ポイント型を使用する
5. The wintf shall `PhysicalPoint` という同一名称の異なる定義を排除する

### Requirement 3: Size/Offset 型の共通化
**Objective:** As a wintf ライブラリ開発者, I want `Size`, `Offset`, `LayoutScale` を共通型として他モジュールからも参照しやすくする, so that レイアウト以外のモジュールでも一貫した空間型を使用できる

#### Acceptance Criteria
1. The wintf shall `Size`（`f32 × 2: width, height`）を共通型モジュールで定義する
2. The wintf shall `Offset`（`f32 × 2: x, y`）を共通型モジュールで定義する
3. The wintf shall `ecs/layout/metrics.rs` の `Size`, `Offset` を共通型モジュールからの re-export に置き換える
4. While `Arrangement` コンポーネントが `Size`, `Offset` を使用している場合, the wintf shall 共通型モジュールの定義を参照する
5. The wintf shall `LayoutScale` のスコープを評価し、レイアウト専用であれば `layout/` に残し、汎用であれば共通化する

### Requirement 4: Rect 型の整理
**Objective:** As a wintf ライブラリ開発者, I want 矩形を表す型を用途別に整理する, so that `D2D_RECT_F` への直接依存を減らし、型の意味が明確になる

#### Acceptance Criteria
1. The wintf shall ボックスモデル用の `Rect<T>`（`left/right/top/bottom`）をレイアウトモジュールに維持する
2. The wintf shall バウンディングボックス用途の矩形型（`x, y, width, height` または `left, top, right, bottom`）を共通型モジュールで定義する
3. The wintf shall `D2DRect` エイリアスの使用箇所を評価し、共通矩形型への段階的な移行パスを提供する
4. The wintf shall `Shape::Rect` variant（`hit_region` 内部の enum、`x/y/w/h` 表現）は本仕様のスコープ外とし、変更しない
5. The wintf shall 描画コマンド（`DrawRectangle`, `FillRectangle`）の `D2D_RECT_F` フィールドは COM 層の型として維持する（COM 層はWin32型を直接使用する設計方針に従う）

### Requirement 5: Transform 系型との境界整理
**Objective:** As a wintf ライブラリ開発者, I want 非推奨の `transform/` モジュールの型と共通型の関係を明確にする, so that 非推奨モジュールの将来的な削除時に影響範囲が限定される

#### Acceptance Criteria
1. The wintf shall `transform/components.rs` の `Translate`, `Scale` 等は非推奨モジュール内に維持し、共通型との統合は行わない
2. The wintf shall 非推奨モジュールに `#[deprecated]` 属性またはドキュメントによる非推奨マーキングを施す

### Requirement 6: Win32 型の抽象化境界
**Objective:** As a wintf ライブラリ開発者, I want ECS 層で Win32 の `POINT`/`SIZE` を直接使用する箇所を整理する, so that プラットフォーム依存の型が ECS コンポーネントの公開 API に漏れない

#### Acceptance Criteria
1. The wintf shall `WindowPos` コンポーネントで Win32 `POINT`/`SIZE` を使用している箇所を評価する
2. When ECS コンポーネントの公開フィールドが Win32 型（`POINT`, `SIZE` 等）を直接参照している場合, the wintf shall フィールド型が一致する共通型（整数座標の場合は `i32 × 2` の共通ポイント型、等）に置き換える
3. The wintf shall 共通型と Win32 型の相互変換を `From`/`Into` トレイトで提供する
4. The wintf shall COM 層（`src/com/`）内部では Win32 ネイティブ型の直接使用を許容する
4. The wintf shall 共通型から Win32 型（`POINT`, `SIZE`, `D2D_RECT_F`, `RECT` 等）への双方向変換を提供する

### Requirement 7: 後方互換性の維持
**Objective:** As a wintf ライブラリ利用者（areka クレート）, I want 型統合後も既存のコードがコンパイルできる, so that リファクタリングが既存機能を破壊しない

#### Acceptance Criteria
1. The wintf shall 既存の型パスからの `pub use` re-export により後方互換性を提供する
2. When 型が共通モジュールに移動された場合, the wintf shall 元のモジュールに `pub use` re-export エントリを残す
3. The wintf shall 全ての既存テスト（`tests/`）がリファクタリング後もパスする
4. The wintf shall 全てのサンプル（`examples/`）がリファクタリング後もコンパイル・実行可能である
5. If 型名の変更が必要な場合, the wintf shall `type OldName = NewName;` による型エイリアスで移行期間を提供する
