# Requirements Document

## Introduction

wintf の DPI 対応レイアウトシステムを修正する仕様である。現在、座標系の混在と二重スケーリング問題により、異なる DPI 設定のモニター間でウィンドウの描画サイズが正しくスケーリングされない。本仕様では、**論理ピクセル（logical px）を唯一の座標系**として確立し、DPI スケーリングを `Arrangement.scale` 経由で一元管理することで、Per-Monitor DPI Aware V2 環境下での正しい描画を実現する。

### 背景

#### 環境情報
- 左モニター (DISPLAY1, 非プライマリ): 200% DPI = 物理 1440×900px = 論理 720×450px
- 右モニター (DISPLAY2, プライマリ): 125% DPI = 物理 3072×1728px = 論理 2457×1382px
- プロセス DPI Awareness: `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`

#### バグの根本原因

座標系の混在と二重スケーリング問題。システムの数学的な変換連鎖:
```
GA.bounds.width = Arrangement.size.width × GA.transform.M11
GA.transform.M11 = prod(各祖先.scale.x)
```

正常な設計 (LayoutRoot → Window → 子):
```
LayoutRoot: scale=(1.0, 1.0)
Window:     scale=(DPI_scale, DPI_scale)
子:         scale=(1.0, 1.0)  ← Taffyが論理pxで計算した場合
→ GA.bounds.width = logical_px × DPI_scale = physical_px  ✓
```

現在の誤った状態 (`fix: LayoutRoot物理px系でのDPI二重スケーリング修正` 以降):
- `Arrangement.scale` を常に `(1.0, 1.0)` に固定
- `BoxStyle.size` に物理pxを直接設定
- → Direct2D の `SetTransform` でスケール 1.0 → DPI に関係なく同じ物理サイズで描画
- → **200% DPI 画面でも 125% DPI 画面でも見た目が同じになってしまう**

#### 修正前にあった別の問題 (修正で生じた問題の背景)
- `BoxStyle.size = physical_width / DPI` (論理 px) だった
- `Arrangement.scale = DPI.scale` (2.0 など) だった
- `GA.bounds.width = 800 (logical) × 2.0 = 1600 (physical)` → 正しい計算
- **しかし**: デモウィンドウが `BoxStyle 800×700 論理px` で 200% DPI モニターが `720×450 論理px` しかない → ウィンドウがモニターより大きい
- これを「ウィンドウが1600×1400pxに膨れた」と誤認して、原因を二重スケーリングと判断したが、実際は**デモのウィンドウサイズが大きすぎた**のが問題だった

---

## Requirements

### Requirement 1: BoxStyle 座標系の論理 px 統一

**Objective:** 開発者として、BoxStyle の座標系を論理ピクセルに統一したい。なぜなら、DPI に依存しない一貫したレイアウト定義を可能にし、DPI スケーリングの責務を明確に分離するためである。

#### Acceptance Criteria
1. The wintf layout system shall レイアウト計算において `BoxStyle` のサイズ・位置の値の単位を論理ピクセル（logical px = 96 DPI / 100% 相当）として扱う
2. When `BoxStyle.size` に値が設定される場合, the wintf layout system shall その値を論理ピクセル単位として解釈し、物理ピクセル値を直接設定してはならない
3. The wintf layout system shall 論理ピクセルから物理ピクセルへの変換を `Arrangement.scale` を介した変換連鎖（`GA.bounds.width = BoxStyle.size.width × GA.transform.M11`）で行う

### Requirement 2: Window エンティティの DPI スケール適用

**Objective:** 開発者として、Window エンティティの `Arrangement.scale` に Windows DPI スケールファクターを自動設定したい。なぜなら、DPI スケーリングを ECS の変換伝播メカニズムで一元管理し、子要素に正しい物理ピクセルサイズを自動的に伝播させるためである。

#### Acceptance Criteria
1. When `update_arrangements_system` が Window エンティティを処理する場合, the wintf layout system shall `Arrangement.scale` を `{x: DPI.scale_x(), y: DPI.scale_y()}` に設定する
2. While Window エンティティが DPI スケールを持つ間, the wintf layout system shall Window 以外の子エンティティの `Arrangement.scale` を `(1.0, 1.0)` のまま維持する（Taffy が論理 px で計算するため）
3. The wintf layout system shall `GlobalArrangement.bounds` を物理ピクセル単位で正しく算出する（`logical_px × DPI_scale = physical_px`）

### Requirement 3: WM_WINDOWPOSCHANGED での論理 px 変換

**Objective:** 開発者として、ウィンドウサイズ変更時に物理ピクセルを論理ピクセルに変換して BoxStyle に設定したい。なぜなら、Win32 API から受け取る物理ピクセル値を BoxStyle の論理ピクセル座標系に正しくマッピングするためである。

#### Acceptance Criteria
1. When `WM_WINDOWPOSCHANGED` メッセージを受信した場合, the wintf window handler shall `BoxStyle.size` を `physical_width / DPI.scale_x()`, `physical_height / DPI.scale_y()` （論理 px）で設定する
2. When DPI が変化した後に `WM_WINDOWPOSCHANGED` を受信した場合, the wintf window handler shall `DpiChangeContext` 経由で取得した最新の DPI 値を使用して論理 px 変換を行う

### Requirement 4: WM_DPICHANGED でのウィンドウサイズ更新

**Objective:** 開発者として、DPI 変更時に `SetWindowPos` でウィンドウの物理サイズを更新したい。なぜなら、モニター間移動時に OS が提供する推奨サイズを適用し、見た目のサイズを保つためである。

#### Acceptance Criteria
1. When `WM_DPICHANGED` メッセージを受信した場合, the wintf window handler shall `SetWindowPos` 呼び出しから `SWP_NOSIZE` フラグを除去し、`suggested_rect` の幅・高さ（物理 px）をそのまま適用する
2. When `WM_DPICHANGED` に続いて `WM_WINDOWPOSCHANGED` が発行される場合, the wintf window handler shall `WM_WINDOWPOSCHANGED` ハンドラ内で新しい物理サイズを新 DPI で除算して `BoxStyle.size` を論理 px に変換し、座標系の一貫性を自動的に維持する

### Requirement 5: デモウィンドウサイズの適正化

**Objective:** 開発者として、`taffy_flex_demo.rs` のウィンドウサイズを 200% DPI モニターに収まるサイズに変更したい。なぜなら、マルチモニター環境でのテスト検証をどのモニターでも行えるようにするためである。

#### Acceptance Criteria
1. The `taffy_flex_demo` shall ウィンドウの `BoxStyle.size` を、200% DPI モニター（論理サイズ 720×450 px）に `find_non_primary_monitor_origin` が返す初期配置位置を考慮しても収まるサイズに設定する（具体的なサイズ値は実装時に決定する）
2. The `taffy_flex_demo` shall 全ての子要素がウィンドウ内に収まり、レイアウトが崩れない状態を維持する

### Requirement 6: デモ検証フローの自動化

**Objective:** 開発者として、デモ起動から DPI レイアウト検証ログ出力・終了までを手作業なしで完了したい。なぜなら、DPI 修正の正しさを高速かつ再現可能な形で確認できるようにするためである。

#### Acceptance Criteria
1. The `taffy_flex_demo` shall レイアウト安定後に DPI ダンプログを出力し、開発者の手作業なしに自動的に終了する
2. The `taffy_flex_demo` shall デモ起動から終了までの総所要時間を、手動操作が不要な最小限の待機時間に短縮する（現在の 60 秒から大幅に削減すること）

### Requirement 7: 描画矩形サイズのログ出力

**Objective:** 開発者として、各エンティティの DPI スケール・物理サイズ・論理サイズを INFO ログで確認したい。なぜなら、DPI スケーリングの正しさを実行時に検証可能にするためである。

#### Acceptance Criteria
1. The wintf layout system shall `dump_all_windows_dpi` または同等の関数で、各エンティティの `Arrangement.scale`（DPI スケール確認用）を INFO ログ出力する
2. The wintf layout system shall 各エンティティの `GlobalArrangement.bounds` の物理 px サイズ（実際に描画される矩形の物理サイズ）を INFO ログ出力する
3. The wintf layout system shall 各エンティティの `BoxStyle.size` の論理 px サイズを INFO ログ出力する
4. When `RUST_LOG=info` が設定されている場合, the wintf layout system shall 上記のログ情報をすべて出力する

---

## 確認基準

以下のすべてを満たすことで、本仕様の要件が充足される:

| 条件 | 期待値 |
|------|--------|
| Window 1 (125% DPI, 右モニター) | `GA.bounds` 幅 = `BoxStyle.width × 1.25` = 物理 px |
| Window 2 (200% DPI, 左モニター) | `GA.bounds` 幅 = `BoxStyle.width × 2.00` = 物理 px |
| 子要素 RedBox (200×100 logical px) @ Window 1 | `GA.bounds` 幅 = 250 物理 px |
| 子要素 RedBox (200×100 logical px) @ Window 2 | `GA.bounds` 幅 = 400 物理 px |
| 両ウィンドウの論理 px サイズ | 同一（同じ `BoxStyle` 値） |
| 両ウィンドウの物理 px サイズ | DPI に比例して異なる |

---

## 影響範囲

| ファイル | 変更内容 |
|---|---|
| `crates/wintf/src/ecs/layout/systems.rs` | `update_arrangements_system`: Window の scale を DPI から設定 |
| `crates/wintf/src/ecs/window_proc/handlers.rs` | `WM_WINDOWPOSCHANGED`: `physical / DPI` で BoxStyle.size を論理 px 設定 |
| `crates/wintf/src/ecs/window_proc/handlers.rs` | `WM_DPICHANGED`: `SWP_NOSIZE` を除去、suggested_rect のサイズを適用 |
| `crates/wintf/examples/taffy_flex_demo.rs` | ウィンドウ・子要素サイズを 200% DPI 画面収容サイズに縮小（具体値は実装時決定） |
| `crates/wintf/examples/taffy_flex_demo.rs` | 自動終了フロー（DPI ダンプ後に自動終了）、待機時間大幅削減 |
