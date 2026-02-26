# Gap Analysis: wintf-P0-balloon01-core

| 項目             | 内容                                          |
| ---------------- | --------------------------------------------- |
| **対象仕様**     | wintf-P0-balloon01-core（バルーンコア子仕様） |
| **分析日**       | 2026-02-26                                    |
| **Requirements** | v1.0（8要件 / 39受入基準）                    |
| **分析範囲**     | crates/wintf/src/, crates/areka/src/main.rs   |

---

## 1. 現状調査サマリ

### 1.1 既存アセット

| アセット                             | パス                             | 関連性                                                                                                       |
| ------------------------------------ | -------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `Window` + `on_window_add` フック    | `ecs/window/components.rs`       | BalloonWindow の on_add パターンの直接テンプレート                                                           |
| `Visual` + `on_visual_add` フック    | `ecs/graphics/visual.rs`         | 子エンティティに自動挿入される。BalloonWindow が Visual を持てば Arrangement, BrushInherit, DComp も連鎖挿入 |
| `Arrangement` + `on_arrangement_add` | `ecs/layout/arrangement.rs`      | GlobalArrangement + ArrangementTreeChanged の自動挿入。レイアウト統合の基盤                                  |
| `WindowPos`                          | `ecs/window/window_pos.rs`       | 位置・サイズ・ZOrder。`Changed<WindowPos>` パターンで配置追従の発火基盤                                      |
| `WindowStyle`                        | `ecs/window/components.rs`       | `WS_POPUP \| WS_VISIBLE`, `WS_EX_LAYERED` のデフォルト。バルーン用にカスタマイズ必要                         |
| `Monitor`                            | `ecs/window/monitor.rs`          | `work_area: RECT` でデスクトップ領域境界を取得可能                                                           |
| `GraphicsCommandList`                | `ecs/graphics/command_list.rs`   | フレーム描画パイプラインの出力先                                                                             |
| `ClipShape`                          | `ecs/graphics/clip.rs`           | `RoundedRectangle`, `RoundedRectangleIndividual` — コンテンツ領域クリッピングに使用可能                      |
| D2D1 ジオメトリAPI                   | `com/d2d/mod.rs`                 | `create_path_geometry()`, `create_rounded_rectangle_geometry()`, `FillGeometry` — しっぽ・角丸描画の基盤     |
| `Rectangle` ウィジェット             | `ecs/widget/shapes/rectangle.rs` | `on_add`/`on_remove` フック + `GraphicsCommandList` 描画パターン。フレーム描画の参考実装                     |
| `SetWindowParentToLayoutRoot`        | `ecs/window/window_pos.rs`       | LayoutRoot 子設定コマンド。BalloonWindow 用にも必要                                                          |
| スケジュールパイプライン             | `ecs/world/schedule_labels.rs`   | PostLayout ステージが placement_system の実行位置候補                                                        |
| areka モック実装                     | `crates/areka/src/main.rs`       | 現行のアドホックなバルーン模擬。BalloonWindowMarker + 手動配置。将来の置き換え対象                           |

### 1.2 確立済みパターン

- **on_add フックチェーン**: `Window → Visual → Arrangement` の連鎖挿入パターン。BalloonWindow はこのチェーンの起点として同じパターンを踏襲する
- **DeferredWorld::commands()**: on_add フック内で子エンティティ spawn に使用（`on_window_add` で実証済み）
- **ChildOf 階層**: bevy_ecs 0.18 の `ChildOf(parent)` による親子関係構築
- **Changed<T> リアクティブクエリ**: `Changed<WindowPos>` による変更検知 → システム発火パターン（`sync_window_arrangement_from_window_pos`, `compositor_systems` で多用）
- **GraphicsCommandList パイプライン**: Draw ステージで CommandList 生成 → RenderSurface で合成 → Composition でコミット
- **CompositionMode 分岐**: ULW / DComp 両対応。`find_owner_window_composition_mode()` で ChildOf チェーンを辿りモード判定

### 1.3 コーディング規約

- コンポーネント定義: `#[derive(Component)]` + `#[component(on_add = ...)]`
- フック関数シグネチャ: `fn on_xxx_add(mut world: DeferredWorld, context: HookContext)`
- 自動挿入前に `world.get::<T>(entity).is_none()` で既存チェック
- `unsafe impl Send for T {}`, `unsafe impl Sync for T {}` — Win32 ハンドル系に必要
- `tracing::debug!` / `tracing::error!` でログ出力

---

## 2. 要件別アセットマップ

### Req 1: 複合ウィジェットエンティティ階層の構築

| AC  | 技術要素                                  | 既存アセット                       | ギャップ                                                         |
| --- | ----------------------------------------- | ---------------------------------- | ---------------------------------------------------------------- |
| AC1 | BalloonWindow → BalloonFrame 子spawn      | `on_window_add` パターン           | **Missing**: `BalloonWindow` コンポーネント + on_add フック      |
| AC2 | BalloonFrame → BalloonContentArea 子spawn | `DeferredWorld::commands()`        | **Missing**: `BalloonFrame`, `BalloonContentArea` コンポーネント |
| AC3 | 3層エンティティ階層                       | `ChildOf` 階層パターン             | **Missing**: 階層構築ロジック（パターンは確立済み）              |
| AC4 | DeferredWorld::commands() 遅延実行        | `on_window_add` で実証済み         | ギャップなし（パターン流用）                                     |
| AC5 | Visual + Arrangement 自動挿入             | `on_visual_add` → Arrangement 連鎖 | ギャップなし（Visual 挿入すれば連鎖で Arrangement 自動付与）     |

**評価**: パターンは完全に確立済み。BalloonWindow / BalloonFrame / BalloonContentArea の3コンポーネント定義と on_add フック実装が必要。

---

### Req 2: バルーンウィンドウ生成

| AC  | 技術要素               | 既存アセット             | ギャップ                                                                             |
| --- | ---------------------- | ------------------------ | ------------------------------------------------------------------------------------ |
| AC1 | anchor フィールド      | —                        | **Missing**: `BalloonWindow.anchor: Entity` フィールド定義                           |
| AC2 | 1:N キャラ→バルーン    | Entity 参照パターン      | ギャップなし（ECS の自然なパターン）                                                 |
| AC3 | 透過ウィンドウ生成     | `WindowStyle` デフォルト | **Missing**: バルーン用 WindowStyle 定義（`WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` 追加） |
| AC4 | HWND↔Entity マッピング | `Window` システム一式    | ギャップなし（既存 Window をそのまま使用）                                           |
| AC5 | despawn 時リソース解放 | 既存 Window despawn 処理 | **Research Needed**: 子エンティティの cascade despawn が ChildOf で自動か要確認      |

**評価**: `Window` + `WindowStyle` + `WindowPos` の組み合わせで大部分が既存基盤で対応可能。BalloonWindow は `Window` を内包（同一エンティティに挿入）するため、HWND 管理は既存システムに委譲。

---

### Req 3: バルーンスキン定義インターフェース

| AC  | 技術要素               | 既存アセット                | ギャップ                                             |
| --- | ---------------------- | --------------------------- | ---------------------------------------------------- |
| AC1 | 背景定義（単色/画像）  | `Brush` コンポーネント      | **Missing**: `BalloonSkinDef` コンポーネント         |
| AC2 | 枠線パラメータ         | —                           | **Missing**: 枠線パラメータ構造体                    |
| AC3 | しっぽ定義             | —                           | **Missing**: しっぽ定義構造体                        |
| AC4 | パディング定義         | `BoxStyle.padding` パターン | **Missing**: BalloonSkinDef 内のパディングフィールド |
| AC5 | 不正入力フォールバック | —                           | **Missing**: バリデーション + デフォルトスキン定義   |

**評価**: `BalloonSkinDef` は完全新規。ただし親仕様の design.md に構造体定義の雛形あり。データ構造のみでロジック少量。

---

### Req 4: フレーム描画

| AC  | 技術要素                       | 既存アセット                                    | ギャップ                                                                            |
| --- | ------------------------------ | ----------------------------------------------- | ----------------------------------------------------------------------------------- |
| AC1 | CommandList によるフレーム描画 | `GraphicsCommandList`, `Rectangle` 描画パターン | **Missing**: `draw_balloon_frame` システム                                          |
| AC2 | 角丸矩形描画                   | `create_rounded_rectangle_geometry()`           | ギャップなし（D2D1 API 利用可能）                                                   |
| AC3 | 枠線描画                       | `D2D1DeviceContextExt`                          | ギャップなし（DrawGeometry API 利用可能）                                           |
| AC4 | しっぽ描画                     | `create_path_geometry()` — PathGeometry         | **Missing**: しっぽジオメトリ生成ロジック                                           |
| AC5 | SkinDef 変更時再描画           | `Changed<T>` パターン                           | **Missing**: `Changed<BalloonSkinDef>` リアクティブシステム                         |
| AC6 | ULW/DComp 両対応               | 既存 RenderSurface パイプライン                 | **Constraint**: 両モードのテスト必要。描画自体は CommandList 経由なのでモード非依存 |

**評価**: 描画パイプラインは `GraphicsCommandList` → `RenderSurface` で確立済み。しっぽの PathGeometry 構築が唯一のアルゴリズム的課題。角丸・枠線は既存 D2D1 API で対応。

---

### Req 5: バルーン配置制御

| AC  | 技術要素                     | 既存アセット                           | ギャップ                                                                      |
| --- | ---------------------------- | -------------------------------------- | ----------------------------------------------------------------------------- |
| AC1 | 配置方向 enum                | —                                      | **Missing**: `BalloonPlacement` コンポーネント（Auto/Right/Left/Above/Below） |
| AC2 | anchor の WindowPos 基準配置 | `WindowPos`, `Changed<WindowPos>`      | **Missing**: 位置算出ロジック                                                 |
| AC3 | 追従 system                  | スケジュールパイプライン（PostLayout） | **Missing**: `placement_system`                                               |
| AC4 | デスクトップ領域外自動反転   | `Monitor.work_area`                    | **Missing**: 領域判定 + 反転ロジック                                          |
| AC5 | オフセット設定               | —                                      | **Missing**: `BalloonWindow.offset` フィールド                                |

**評価**: 配置ロジックは完全新規だが、`WindowPos` + `Monitor.work_area` という既存基盤を活用。areka の `on_shell_drag` ハンドラ（ハードコード offset + `SetWindowPosCommand`）が概念実証となっているが、ECS システムとして再設計が必要。

---

### Req 6: バルーン表示制御

| AC  | 技術要素                 | 既存アセット                 | ギャップ                                               |
| --- | ------------------------ | ---------------------------- | ------------------------------------------------------ |
| AC1 | 表示/非表示制御          | `Visual.is_visible`          | ギャップなし（既存フィールド利用）                     |
| AC2 | 前面表示                 | `WindowPos.zorder_topmost()` | ギャップなし（既存メソッド利用）                       |
| AC3 | 非表示時エンティティ保持 | ECS 設計パターン             | ギャップなし（ShowWindow(SW_HIDE) + エンティティ保持） |
| AC4 | サイズ設定               | `WindowPos.with_size()`      | ギャップなし（既存メソッド利用）                       |

**評価**: 全 AC が既存コンポーネントの機能で対応可能。新規実装なし。

---

### Req 7: エラーハンドリングと堅牢性

| AC  | 技術要素                              | 既存アセット                  | ギャップ                                                          |
| --- | ------------------------------------- | ----------------------------- | ----------------------------------------------------------------- |
| AC1 | 無効 anchor → 非表示化                | `Visual.is_visible`           | **Missing**: anchor バリデーションロジック（placement_system 内） |
| AC2 | WindowPos 不在 → スキップ             | クエリの `Option<&WindowPos>` | **Missing**: placement_system 内のガード処理                      |
| AC3 | SkinDef 不在 → デフォルト             | —                             | **Missing**: デフォルト BalloonSkinDef 定義 + フォールバック      |
| AC4 | Win32 生成失敗 → ログ＋クリーンアップ | `tracing::error!`             | **Missing**: エラー分岐とクリーンアップコマンド                   |

**評価**: 個々のエラー処理は placement_system やフレーム描画システムに組み込む付随ロジック。独立したコンポーネント不要。

---

### Req 8: モジュール配置と拡張性

| AC  | 技術要素                          | 既存アセット                        | ギャップ                                      |
| --- | --------------------------------- | ----------------------------------- | --------------------------------------------- |
| AC1 | `ecs/widget/balloon/mod.rs`       | `ecs/widget/mod.rs`（balloon なし） | **Missing**: ディレクトリ + モジュール作成    |
| AC2 | `ecs/widget/balloon/frame.rs`     | —                                   | **Missing**: ファイル作成                     |
| AC3 | `ecs/widget/balloon/placement.rs` | —                                   | **Missing**: ファイル作成                     |
| AC4 | ChildOf 拡張ポイント              | `ChildOf` パターン確立済み          | ギャップなし（特別な拡張機構不要）            |
| AC5 | レイヤー依存方向の遵守            | COM → ECS → Message Handling        | ギャップなし（新モジュールは ECS レイヤー内） |

**評価**: ファイル構造のスキャフォールド。既存の `ecs/widget/` 配下に `balloon/` サブモジュールを追加し、`mod.rs` で pub mod 宣言するだけ。

---

## 3. ギャップサマリ

### Missing（新規作成が必要）

| #   | アイテム                                                          | 関連要件             | 複雑度                           |
| --- | ----------------------------------------------------------------- | -------------------- | -------------------------------- |
| M1  | `BalloonWindow` コンポーネント + `on_balloon_window_add` フック   | Req 1, 2             | 低（on_window_add テンプレート） |
| M2  | `BalloonFrame` コンポーネント                                     | Req 1, 4             | 低                               |
| M3  | `BalloonContentArea` コンポーネント                               | Req 1                | 低                               |
| M4  | `BalloonSkinDef` コンポーネント（背景・枠線・しっぽ・パディング） | Req 3                | 低（データ定義のみ）             |
| M5  | `BalloonPlacement` コンポーネント（方向 enum + Auto 判定）        | Req 5                | 低                               |
| M6  | `draw_balloon_frame` システム（CommandList 描画）                 | Req 4                | 中（しっぽ PathGeometry）        |
| M7  | `placement_system`（配置計算 + デスクトップ境界反転）             | Req 5                | 中                               |
| M8  | しっぽジオメトリ生成ロジック                                      | Req 4 AC4            | 中（PathGeometry 構築）          |
| M9  | デフォルト BalloonSkinDef + バリデーション                        | Req 3 AC5, Req 7 AC3 | 低                               |
| M10 | モジュール構造（balloon/mod.rs, frame.rs, placement.rs）          | Req 8                | 低（スキャフォールド）           |

### Research Needed（設計フェーズで調査）

| #   | アイテム                                                                         | 影響範囲  |
| --- | -------------------------------------------------------------------------------- | --------- |
| RN1 | `ChildOf` despawn 時の cascade 動作確認（bevy_ecs 0.18 の仕様）                  | Req 2 AC5 |
| RN2 | placement_system のスケジュール配置（PostLayout vs Update）                      | Req 5 AC3 |
| ~~RN3~~ | ~~BalloonWindow が Window を同一エンティティに共存させるか~~ **解決済み**: 親 design.md で同一エンティティ方式に決定済み（`on_balloon_window_add` で Window + WindowStyle + WindowPos + Visual を同一エンティティに挿入） | — |

### Constraint（既存アーキテクチャ制約）

| #   | 制約                                                                     | 影響      |
| --- | ------------------------------------------------------------------------ | --------- |
| C1  | on_add フック内で直接の子 spawn は不可（DeferredWorld::commands() 必須） | Req 1 AC4 |
| C2  | ULW / DComp 両モード対応必須                                             | Req 4 AC6 |
| C3  | BalloonContentArea は BalloonFrame の子（inherited-context.md 制約）     | Req 1 AC2 |
| C4  | Relation API 不採用（anchor は Entity フィールド方式）                   | Req 2 AC1 |

---

## 4. 実装アプローチ検討

### Option A: 既存コンポーネントの拡張

**適用可能性**: 低

既存の `Window` コンポーネントにバルーン固有のフィールドを追加する方法。

- BalloonWindow の責務（anchor, placement, offset）は Window の責務と明確に異なる
- Window の on_add フックにバルーン固有ロジックを入れると単一責任原則に違反
- 既存 Window のテストや他ウィジェットへの影響リスク

**トレードオフ**:
- ✅ ファイル数が増えない
- ❌ Window コンポーネントの肥大化
- ❌ 後続子仕様（balloon02〜08）の拡張が Window に密結合
- ❌ 不適切: バルーンは独立した概念

---

### Option B: 新規コンポーネント群の作成 ⭐

**適用可能性**: 高（推奨候補）

`ecs/widget/balloon/` に新規モジュールを作成し、BalloonWindow / BalloonFrame / BalloonContentArea / BalloonSkinDef / BalloonPlacement を独立コンポーネントとして定義。

- 明確な責務分離: バルーン固有ロジックは balloon/ モジュールに閉じる
- 既存 Window / Visual / Arrangement を on_add チェーンで活用（拡張ではなく構成）
- 後続子仕様は BalloonContentArea の ChildOf に新エンティティを追加するだけ

**統合ポイント**:
- `BalloonWindow` の on_add で `Window` + `WindowStyle` + `WindowPos` + `Visual` を同一エンティティに挿入
- `placement_system` は `Changed<WindowPos>` クエリで anchor の移動を検知
- `draw_balloon_frame` は既存 Draw スケジュールに登録
- `ecs/widget/mod.rs` に `pub mod balloon;` を追加

**トレードオフ**:
- ✅ 明確な責務分離と拡張性
- ✅ 既存コンポーネントへの影響ゼロ
- ✅ 後続子仕様の独立開発が容易
- ✅ テスト容易（balloon/ 単体でテスト可能）
- ❌ ファイル数が増加（3ファイル）
- ❌ on_add チェーンの理解が必要（ただし既存パターンと同一）

---

### Option C: ハイブリッドアプローチ

**適用可能性**: 不要

本仕様はグリーンフィールド実装（既存バルーンコードなし）であり、拡張対象の既存コンポーネントが存在しない。Option B が自然な選択であり、ハイブリッドの動機がない。

---

## 5. 工数・リスク評価

### 工数: **M（3〜7日）**

**根拠**: 新規コンポーネント定義は5つだが、いずれも既存 on_add パターンのテンプレート流用。描画システム（draw_balloon_frame）としっぽ PathGeometry 構築、placement_system のデスクトップ境界反転ロジックが主要な実装工数。ULW/DComp 両モードのテストも含む。

### リスク: **低**

**根拠**:
- 全コンポーネントが on_add フックチェーンという確立済みパターンに準拠
- 描画は GraphicsCommandList → RenderSurface の既存パイプラインを利用
- D2D1 ジオメトリ API（PathGeometry, RoundedRectangleGeometry）は既に COM ラッパーで利用可能
- 外部依存・未知技術なし
- areka モック実装が概念実証として存在

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option B（新規コンポーネント群の作成）** を推奨。グリーンフィールド実装であり、既存パターンへの適合性が高い。

### 設計フェーズで決定すべき事項

1. **RN2: placement_system のスケジュール配置** — PostLayout（レイアウト確定後）が最有力。Update では Arrangement 未確定のリスク
2. **しっぽ描画のジオメトリ設計** — PathGeometry によるカスタム形状生成の具体的なアルゴリズムと座標計算方式

> **解決済み RN3**: BalloonWindow と Window の関係は親 design.md で同一エンティティ方式に決定済み。`on_balloon_window_add` で `Window(ULW)` + `WindowStyle` + `WindowPos(TopMost)` + `Visual` を同一エンティティに挿入する。

### 持ち越し調査事項

- **RN1**: bevy_ecs 0.18 での `ChildOf` エンティティ despawn 時の cascade 動作（子自動 despawn か手動必要か）
- バルーンウィンドウの初期サイズ決定戦略（コンテンツ依存 vs 固定 vs SkinDef 指定）
- areka モック実装から BalloonWindow コンポーネントへの移行パス

---

*分析完了。要件の 70% 以上が既存パターンの直接流用またはパターン準拠で実装可能。主要な新規実装はフレーム描画システムと配置システムの2点に集中。*
