# ギャップ分析: type-consolidation

## 1. 現状調査

### 1.1 幾何・空間型の完全インベントリ

wintf クレート内で発見された全幾何・空間型を、**体系別に分類**して列挙する。

---

#### 体系A: 2D座標ポイント型

**同じ概念「2D上の位置」を表すが、フィールド型・用途・定義箇所がバラバラ。**

| 型名                          | 定義箇所                          | フィールド                           | derive/属性                                  | 用途                                                                                                | 問題                      |
| ----------------------------- | --------------------------------- | ------------------------------------ | -------------------------------------------- | --------------------------------------------------------------------------------------------------- | ------------------------- |
| `PhysicalPoint`<br>(pointer)  | `ecs/pointer/types.rs:16`         | `x: i32, y: i32`                     | `Debug, Clone, Copy, Default, PartialEq, Eq` | ポインター座標（物理ピクセル整数）。`PointerState.client_point`、`PointerState.local_point`、drag系 | -                         |
| `PhysicalPoint`<br>(hit_test) | `ecs/layout/hit_test/mod.rs:47`   | `x: f32, y: f32`                     | `Debug, Clone, Copy, PartialEq`              | ヒットテスト関数の引数（浮動小数点スクリーン座標）                                                  | **同名異義の重複！**      |
| `Translate`                   | `ecs/transform/components.rs:7`   | `x: f32, y: f32`                     | `Default, Clone, Copy, Debug, PartialEq`     | CSS transform: translate 相当（非推奨モジュール）                                                   | 構造は`Offset`と同一      |
| `TransformOrigin`             | `ecs/transform/components.rs:102` | `x: f32, y: f32`                     | `Clone, Copy, Debug, PartialEq`              | 変換基準点（非推奨モジュール）                                                                      | 意味的には比率（0.0-1.0） |
| `PositionSample`              | `ecs/pointer/types.rs:207`        | `x: f32, y: f32, timestamp: Instant` | `Debug, Clone, Copy`                         | カーソル速度計算用サンプル                                                                          | ポイント＋タイムスタンプ  |
| Win32 `POINT`                 | （外部型）                        | `x: i32, y: i32`                     | -                                            | `WindowPos.position`、drag系、nchittest                                                             | ECS層に漏れ出し           |
| `Vector2` (windows_numerics)  | （外部型）                        | `X: f32, Y: f32`                     | -                                            | `D2DRectExt.offset()/.size()` の戻り値、描画系                                                      | 描画系で多用              |

**使用箇所の詳細:**

- `PhysicalPoint` (pointer/types.rs) → drag/accumulator.rs, drag/dispatch.rs, drag/mod.rs, drag/state.rs、`pub use` で `ecs/mod.rs` から re-export
- `PhysicalPoint` (hit_test/mod.rs) → 内部的に `hit_test()`, `hit_test_in_window()` で使用。mouse_move.rs 等では `use ... PhysicalPoint as HitTestPoint` でエイリアス回避
- Win32 `POINT` → window/window_pos.rs (WindowPos.position), drag/dispatch.rs, drag/state.rs, drag/context.rs, window_proc/dpi_helpers.rs, examples多数

---

#### 体系B: 2Dサイズ型

**「幅×高さ」を表す型がモジュール間で統一されていない。**

| 型名                         | 定義箇所                     | フィールド                                            | derive/属性                                         | 用途                             | 問題                           |
| ---------------------------- | ---------------------------- | ----------------------------------------------------- | --------------------------------------------------- | -------------------------------- | ------------------------------ |
| `Size`                       | `ecs/layout/metrics.rs:31`   | `width: f32, height: f32`                             | `Component, Debug, Clone, Copy, PartialEq, Default` | Arrangement.size、レイアウト全般 | layout層に閉じ込め             |
| `BoxSize`                    | `ecs/layout/box_style.rs:15` | `width: Option<Dimension>, height: Option<Dimension>` | `Debug, Clone, Copy, PartialEq, Default`            | BoxStyle.size値オブジェクト      | Dimension型（Auto/Px/%）       |
| `TextLayoutMetrics`          | `ecs/layout/metrics.rs:9`    | `width: f32, height: f32`                             | `Component, Debug, Clone, Copy, PartialEq, Default` | テキストレイアウトの物理サイズ   | `Size`とフィールド構成同一     |
| Win32 `SIZE`                 | （外部型）                   | `cx: i32, cy: i32`                                    | -                                                   | `WindowPos.size`、render.rs      | ECS層に漏れ出し                |
| `Vector2` (windows_numerics) | （外部型）                   | `X: f32, Y: f32`                                      | -                                                   | `D2DRectExt.size()` の戻り値     | サイズにもポイントにも使われる |

**使用箇所の詳細:**

- `Size` → arrangement.rs, rect.rs (D2DRectExt.from_offset_size), hit_region/mod.rs, taffy_systems.rs、`pub use layout::*` で re-export
- `TextLayoutMetrics` → metrics.rs内で定義、文字列レイアウト用
- Win32 `SIZE` → window/window_pos.rs (WindowPos.size), graphics/render.rs, window_pos_systems.rs, examples多数

---

#### 体系C: 2Dオフセット/平行移動型

**「親からの相対位置」を表す型が複数存在。**

| 型名             | 定義箇所                        | フィールド                       | derive/属性                                | 用途                         | 問題                         |
| ---------------- | ------------------------------- | -------------------------------- | ------------------------------------------ | ---------------------------- | ---------------------------- |
| `Offset`         | `ecs/layout/metrics.rs:53`      | `x: f32, y: f32`                 | `Component, Debug, Clone, Copy, PartialEq` | Arrangement.offset、矩形構築 | layout層に閉じ込め           |
| `Translate`      | `ecs/transform/components.rs:7` | `x: f32, y: f32`                 | `Default, Clone, Copy, Debug, PartialEq`   | CSS translate相当            | `Offset`と構造同一（非推奨） |
| `CursorVelocity` | `ecs/pointer/types.rs:63`       | `x: f32, y: f32, magnitude: f32` | `Debug, Clone, Default, PartialEq`         | カーソル移動速度             | ベクトル＋ノルム             |

---

#### 体系D: スケール型

| 型名          | 定義箇所                         | フィールド               | derive/属性                                    | 用途                       | 問題                      |
| ------------- | -------------------------------- | ------------------------ | ---------------------------------------------- | -------------------------- | ------------------------- |
| `LayoutScale` | `ecs/layout/metrics.rs:66`       | `x: f32, y: f32`         | `Component, Debug, Clone, Copy, PartialEq`     | Arrangement.scale（DPI等） | layout層に閉じ込め        |
| `Scale`       | `ecs/transform/components.rs:26` | `x: f32, y: f32`         | `Clone, Copy, Debug, PartialEq`                | CSS scale相当（非推奨）    | `LayoutScale`と構造同一   |
| `DPI`         | `ecs/window/dpi.rs:23`           | `dpi_x: u16, dpi_y: u16` | `Component, Debug, Clone, Copy, PartialEq, Eq` | ウィンドウDPI値            | 整数DPI値、意味的に別概念 |

---

#### 体系E: 矩形型

**最も乱立が顕著。用途別に4つの異なる矩形表現が存在。**

| 型名                     | 定義箇所                          | フィールド                                     | derive/属性                                | 用途                                              | 問題                           |
| ------------------------ | --------------------------------- | ---------------------------------------------- | ------------------------------------------ | ------------------------------------------------- | ------------------------------ |
| `Rect<T>`                | `ecs/layout/dimension.rs:227`     | `left: T, right: T, top: T, bottom: T`         | `Debug, Clone, Copy, PartialEq, Component` | BoxMargin/BoxPadding/BoxInset の内部型            | ボックスモデル専用             |
| `D2DRect` (`D2D_RECT_F`) | `ecs/layout/rect.rs:7`            | `left: f32, top: f32, right: f32, bottom: f32` | -                                          | GlobalArrangement.bounds、描画バウンディング      | Win32型のエイリアス            |
| `Shape::Rect`            | `ecs/layout/hit_region/mod.rs:87` | `x: f32, y: f32, width: f32, height: f32`      | `Debug, Clone`                             | ヒット領域の矩形定義                              | enum variant内のインライン定義 |
| Win32 `D2D_RECT_F`       | （外部型）                        | `left, top, right, bottom: f32`                | -                                          | COM層描画コマンド(DrawRectangle, FillRectangle等) | COM層では直接使用が妥当        |
| Win32 `RECT`             | （外部型）                        | `left, top, right, bottom: i32`                | -                                          | COM ulw.rs、monitor.rs                            | 整数矩形、Win32 API直接        |

**使用箇所の詳細:**

- `Rect<T>` → box_style.rs (BoxMargin, BoxPadding, BoxInset)、taffy Rect への From 変換
- `D2DRect` → arrangement.rs (GlobalArrangement.bounds)、`D2DRectExt` トレイトで拡張メソッド追加
- `D2D_RECT_F` 直接使用 → com/d2d/command_types.rs (BlendImage, DrawGdiMetafile, DrawRectangle, FillRectangle 等多数)、widget/shapes/rectangle.rs、widget/bitmap_source/systems.rs、テスト多数

---

#### 体系F: 複合配置型（コンポーネント）

| 型名                | 定義箇所                          | フィールド                                                      | derive/属性                                         | 用途                               |
| ------------------- | --------------------------------- | --------------------------------------------------------------- | --------------------------------------------------- | ---------------------------------- |
| `Arrangement`       | `ecs/layout/arrangement.rs:7`     | `offset: Offset, scale: LayoutScale, size: Size`                | `Component, Debug, Clone, Copy, PartialEq`          | ローカル配置情報                   |
| `GlobalArrangement` | `ecs/layout/arrangement.rs:77`    | `transform: Matrix3x2, bounds: D2DRect`                         | `Component, Debug, Clone, Copy, PartialEq`          | 累積変換＋スクリーンバウンディング |
| `Transform`         | `ecs/transform/components.rs:130` | `translate, scale, rotate, skew, origin`                        | `Component, Clone, Copy, Debug, Default, PartialEq` | 2D変換（非推奨）                   |
| `GlobalTransform`   | `ecs/transform/components.rs:160` | `Matrix3x2`                                                     | `Component, Clone, Copy, Debug, Default, PartialEq` | グローバル変換行列（非推奨）       |
| `WindowPos`         | `ecs/window/window_pos.rs:55`     | `zorder, position: Option<POINT>, size: Option<SIZE>, ...flags` | `Component, Debug, Clone, Copy, PartialEq`          | ウィンドウ位置・サイズ             |
| `BoxStyle`          | `ecs/layout/box_style.rs:100+`    | `size, margin, padding, position, inset, flex_*`                | `Component, Debug, Clone, Copy, PartialEq, Default` | 統合レイアウトスタイル             |

---

#### 体系G: ボックスモデル値オブジェクト

| 型名          | 定義箇所          | フィールド                    | 用途              |
| ------------- | ----------------- | ----------------------------- | ----------------- |
| `BoxSize`     | `box_style.rs:15` | `Option<Dimension> × 2`       | BoxStyle.size     |
| `BoxMargin`   | `box_style.rs:22` | `Rect<LengthPercentageAuto>`  | BoxStyle.margin   |
| `BoxPadding`  | `box_style.rs:26` | `Rect<LengthPercentage>`      | BoxStyle.padding  |
| `BoxPosition` | `box_style.rs:30` | `enum { Relative, Absolute }` | BoxStyle.position |
| `BoxInset`    | `box_style.rs:40` | `Rect<LengthPercentageAuto>`  | BoxStyle.inset    |

---

### 1.2 現在の re-export 構造

```
ecs/mod.rs
├── pub use layout::*           → Size, Offset, LayoutScale, Arrangement, GlobalArrangement,
│                                  D2DRect, D2DRectExt, Rect<T>, BoxStyle, BoxSize, ...全て
├── pub use pointer::{ PhysicalPoint, ... }  → pointer版PhysicalPoint
├── pub use transform::*        → Transform, GlobalTransform, Translate, Scale, ...
└── pub use window::{ WindowPos, DPI, ... }
```

**名前衝突の実態:**
- `PhysicalPoint` は `ecs/mod.rs` で pointer 版のみ re-export
- hit_test 版は `layout::hit_test::PhysicalPoint` としてのみアクセス可能
- 内部コードでは `PhysicalPoint as HitTestPoint` でエイリアス回避（mouse_move.rs, mouse_click.rs, mouse_dblclick_wheel.rs）

---

## 2. 要件とのギャップ分析

### 要件 → 既存資産マッピング

| 要件                       | 既存資産                                                  | ギャップ                                                                                       | 状態           |
| -------------------------- | --------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | -------------- |
| Req1: 共通型モジュール導入 | 該当なし（各サブモジュールに分散）                        | 共通型モジュール自体が存在しない                                                               | **Missing**    |
| Req2: Point型統一          | `PhysicalPoint`×2, Win32 `POINT`, `Vector2`               | 同名異義の重複、整数/浮動小数点の未分離                                                        | **Missing**    |
| Req3: Size/Offset共通化    | `Size`, `Offset` (layout内)                               | 定義場所がlayout層に閉じ込め                                                                                           | **Constraint** |
| Req4: Rect型整理           | `Rect<T>`, `D2DRect`, `Shape::Rect`, `D2D_RECT_F`, `RECT` | D2DRect を標準バウンディングボックス型として採用。ボックスモデル `Rect<T>` との使い分けを明確化                       | **Constraint** |
| Req5: Transform境界整理    | `transform/` モジュール全体が非推奨                       | `#[deprecated]` マーキングが一部のみ                                                                                   | **Constraint** |
| Req6: Win32型抽象化        | `WindowPos` が `POINT`/`SIZE` を直接保持                  | WindowPos はコンポーネント参照のためスコープ内。i32版共通型への置き換え + From/Into 変換が必要 | **Missing**    |
| Req7: 後方互換性           | `pub use` re-export パターンが既に存在                    | 移動後の re-export パスを追加する作業が必要                                                    | **Unknown**    |

---

## 3. 実装アプローチ選択肢

### Option A: 最小限の共通モジュール（ecs/types.rs）

**アプローチ**: `ecs/types.rs` を新設し、プリミティブ型（Point系、Size、Offset）のみを集約。既存モジュールは re-export で対応。

- **変更ファイル**: 新規1ファイル + 既存5-8ファイルの `use` パス変更
- **移動対象**: `Size`, `Offset`, `LayoutScale` (metrics.rs から), 新規 `Point` / `PointF` 型
- **既存維持**: `Rect<T>` は dimension.rs に残す、`D2DRect` は rect.rs に残す、transform系は触らない

**トレードオフ**:
- ✅ 変更範囲が小さく、リスク低
- ✅ Point重複を解消できる
- ✅ 後方互換性の維持が容易
- ❌ Rect系の整理は別フェーズに先送り
- ❌ Win32型変換は個別対応の必要あり

### Option B: 包括的な型モジュール（ecs/types/）

**アプローチ**: `ecs/types/` ディレクトリを新設し、プリミティブ型 + 矩形型 + Win32変換を包括的に集約。

- **変更ファイル**: 新規ディレクトリ(3-4ファイル) + 既存10-15ファイルの `use` パス変更
- **移動対象**: `Size`, `Offset`, `LayoutScale`, `Rect<T>`, 新規 Point系、新規バウンディング矩形型、Win32変換トレイト
- **既存維持**: `D2DRect` エイリアスは維持（新矩形型への From 追加）、transform系は触らない

**トレードオフ**:
- ✅ 型体系の完全な統一
- ✅ Win32型の抽象化境界が明確
- ✅ 新規モジュール開発時の型選択が容易
- ❌ 変更範囲が広く、テスト修正も多い
- ❌ 破壊的変更のリスクが高い

### Option C: ハイブリッド（段階的実装）

**アプローチ**: Phase 1 で Option A の最小限集約を実施し、Phase 2 で Rect 系・Win32 変換を追加。

- **Phase 1**: `ecs/types.rs` に Point系・Size・Offset を集約、PhysicalPoint 重複解消
- **Phase 2**: バウンディング矩形型の追加、Win32 From/Into 変換の追加
- **Phase 3**: `D2DRect` → 共通矩形型への段階的移行

**トレードオフ**:
- ✅ リスク分散、各フェーズで検証可能
- ✅ Phase 1 だけでも最大の課題（PhysicalPoint重複）を解決
- ✅ コンパイル通過を各段階で保証
- ❌ 複数回のリファクタリングが必要
- ❌ 中間状態での不整合期間あり

---

## 4. 複雑度・リスク評価

| 項目       | 評価          | 理由                                                                                                                                  |
| ---------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| **工数**   | **M (3-7日)** | 型移動自体は単純だが、re-export パス維持・テスト修正・PhysicalPoint統一が主要作業                                                     |
| **リスク** | **Medium**    | 既存パターン(`pub use` re-export)が確立済み。意味論的に同一の型が多い。ただしPhysicalPointはi32/f32で型が異なり、統一に設計判断が必要 |

---

## 5. 設計フェーズへの持ち越し事項

### Research Needed

1. **PhysicalPoint の統一方針** 【設計判断】: i32版（pointer用）とf32版（hit_test用）をどう統合するか
   - 案1: `PhysicalPoint<T>` ジェネリック化
   - 案2: `PhysicalPointI`（整数）/ `PhysicalPointF`（浮動小数点）に分離命名
   - 案3: `ScreenPoint`（i32、ポインター用）/ `LogicalPoint`（f32、レイアウト用）のように意味名で分離
2. **`TextLayoutMetrics` の扱い** 【解決済み】: `TextLayoutMetrics` は「テキストレイアウトの測定結果」というセマンティック型であり、汎用の `Size` とは意味が異なる。`metrics.rs` に維持し、統合は行わない。
3. **`D2DRectExt` トレイトの移行** 【設計判断】: 共通矩形型にメソッドを組み込むか、トレイトのまま維持か
4. **`LayoutScale` のスコープ判定** 【設計判断】: DPI専用か汎用スケールか（`DPI` コンポーネントとの関係整理）
5. **`PositionSample`, `CursorVelocity` の扱い** 【解決済み】: `PositionSample`（`x, y, timestamp`）、`CursorVelocity`（`x, y, magnitude`）は追加フィールドを持つポインターモジュール固有のドメイン型。共通化の対象外とする。

### 推奨アプローチ

**Option C（ハイブリッド段階的実装）** を推奨。

理由:
- Phase 1 で最も深刻な問題（PhysicalPoint重複、Size/Offset の閉じ込め）を解消
- Phase 2 以降は要件の優先度に応じて柔軟にスケジューリング可能
- 各 Phase 末でコンパイル通過・テストパスを保証でき、安全
